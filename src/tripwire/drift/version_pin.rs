//! forjar#613: a versioned binary is drift when its LIVE version is not its PIN.
//!
//! # The defect
//!
//! paiml/infra pins ollama to `v0.33.2` on lambda-labs, and the box runs
//! `0.34.2`, which was installed outside forjar. Nothing said so:
//!
//! - `github_release`'s check is `[ -x <install_dir>/<binary> ]`, which tests
//!   presence only. It PRINTS `--version` and never compares it with `tag:`.
//! - The state-query path hashes that output and compares the hash with the
//!   lock's baseline. A baseline taken after the out-of-band install, or no
//!   baseline at all (`declared here, absent from the lock`), compares the box
//!   with itself or with nothing, and the result is clean.
//! - `plan` reads only the lock.
//!
//! So the one command that would act, `apply`, downgrades the box, and nothing
//! warns before it does.
//!
//! # Why this asks the DECLARATION and not the lock
//!
//! The pin is a claim written in the config, and `<bin> --version` is a
//! question the box can answer right now. Like `task_check`, this is an
//! ASSERTION with no baseline: a live `0.34.2` under a `v0.33.2` pin is drift
//! whether the lock recorded anything or not. So the detector walks the
//! declared resources for the machine, not `lock.resources`.
//!
//! # Latest CRUX tooling only
//!
//! A pin that matches the box but is behind the repo's latest GitHub release is
//! drift too. The upstream answer comes from the same endpoint family and the
//! same `curl` that `github_release::apply_script` already uses to fetch the
//! asset, run on the same target. That check needs the network, so
//! `--offline` turns it off. The census then NAMES every resource whose pin was
//! not compared with upstream. A skipped check must never read as a passed one.
//!
//! # What this never does
//!
//! It never feeds a remediation path. `detect_drift_full`, which the apply gate
//! and the pull agent call, turns this detector off: re-applying a pin that is
//! behind the live binary IS the downgrade this detector exists to report.

use super::census::DriftCensus;
use super::ignore::should_ignore_drift;
use super::unmeasured::{self, Reading};
use super::DriftFinding;
use crate::core::shell_escape::{is_valid_repo, sh_squote};
use crate::core::types::{Machine, Resource, ResourceType};
use std::fmt;

/// `MAJOR.MINOR.PATCH`. Pre-release and build suffixes are not compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Semver {
    /// Major.
    pub major: u64,
    /// Minor.
    pub minor: u64,
    /// Patch.
    pub patch: u64,
}

impl fmt::Display for Semver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The first `MAJOR.MINOR.PATCH` in `text`, with an optional leading `v`.
///
/// It reads version output in the shapes found in the wild:
/// `ollama version is 0.34.2`, `forjar 1.32.0`, `v0.34.2`, and ollama's
/// `Warning: could not connect to a running Ollama instance` followed by
/// `Warning: client version is 0.34.2`. A number that is part of a longer
/// dotted run (`10.42.0.11`) or glued to letters (`x86_64`) is not a version.
/// `None` means there was no version in the output. It is never taken as a match.
pub fn parse_semver(text: &str) -> Option<Semver> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() && starts_token(bytes, i) {
            if let Some((v, end)) = triple_at(bytes, i) {
                if !continues_dotted(bytes, end) {
                    return Some(v);
                }
            }
        }
        i += 1;
    }
    None
}

/// A version starts a token: at the start of the text, after a non-alphanumeric
/// byte that is not a dot, or after a `v` that itself starts a token.
fn starts_token(bytes: &[u8], i: usize) -> bool {
    let boundary = |j: usize| {
        j == 0
            || !(bytes[j - 1].is_ascii_alphanumeric()
                || bytes[j - 1] == b'.'
                || bytes[j - 1] == b'_')
    };
    if boundary(i) {
        return true;
    }
    matches!(bytes[i - 1], b'v' | b'V') && boundary(i - 1)
}

/// `(version, index just past it)` for `N.N.N` starting at `i`.
fn triple_at(bytes: &[u8], mut i: usize) -> Option<(Semver, usize)> {
    let mut parts = [0u64; 3];
    for (n, part) in parts.iter_mut().enumerate() {
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return None;
        }
        *part = std::str::from_utf8(&bytes[start..i]).ok()?.parse().ok()?;
        if n < 2 {
            if bytes.get(i) != Some(&b'.') {
                return None;
            }
            i += 1;
        }
    }
    Some((
        Semver {
            major: parts[0],
            minor: parts[1],
            patch: parts[2],
        },
        i,
    ))
}

/// `1.2.3.4` is not a semver, and neither is its `1.2.3`.
fn continues_dotted(bytes: &[u8], end: usize) -> bool {
    bytes.get(end) == Some(&b'.') && bytes.get(end + 1).is_some_and(u8::is_ascii_digit)
}

/// What the declaration pins the binary to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pin {
    /// `tag: v0.33.2`.
    Version(Semver),
    /// `tag: latest`, or no tag (`apply_script` defaults to `latest`).
    Latest,
    /// A tag that names no version, such as `nightly`.
    Unversioned(String),
}

/// The pin a `github_release` declares.
pub fn declared_pin(resource: &Resource) -> Pin {
    match resource.tag.as_deref() {
        None | Some("latest") => Pin::Latest,
        Some(tag) => {
            parse_semver(tag).map_or_else(|| Pin::Unversioned(tag.to_string()), Pin::Version)
        }
    }
}

/// Is this a resource whose version this detector can reason about?
pub fn is_versioned_binary(resource: &Resource) -> bool {
    resource.resource_type == ResourceType::GithubRelease
        && resource.state.as_deref() != Some("absent")
}

/// What the live binary said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Live {
    /// Not installed at the declared path. Plan and the state query own that.
    Missing,
    /// `--version` printed this.
    Version(Semver),
    /// `--version` printed no version. The first line is kept as evidence.
    Unparseable(String),
}

/// What the repo's latest GitHub release is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Upstream {
    /// `--offline`, or a surface that does not reach the network.
    NotChecked,
    /// `releases/latest` answered with this tag.
    Version(Semver),
    /// Asked and got no usable answer. The string says why.
    Unmeasured(String),
}

/// One verdict about one resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The declared and live versions differ, or the pin is behind upstream.
    Drift {
        /// What the declaration requires.
        expected: String,
        /// What was found.
        actual: String,
        /// Operator-facing explanation.
        detail: String,
    },
    /// The question was asked and could not be answered.
    Unmeasured(String),
    /// The question was not asked. It is named, never counted as a pass.
    NotChecked(String),
}

/// Compare the pin with the live binary and with upstream. Pure; no I/O.
pub fn evaluate(
    binary: &str,
    repo: &str,
    pin: &Pin,
    live: &Live,
    upstream: &Upstream,
) -> Vec<Outcome> {
    let mut out = Vec::new();
    let expected = match pin {
        Pin::Unversioned(tag) => {
            out.push(Outcome::NotChecked(format!(
                "tag '{tag}' names no version, so the live {binary} was not compared with it"
            )));
            return out;
        }
        Pin::Version(p) => Some(*p),
        Pin::Latest => match upstream {
            Upstream::Version(u) => Some(*u),
            _ => None,
        },
    };
    match (live, expected) {
        (Live::Unparseable(raw), _) => out.push(Outcome::Unmeasured(format!(
            "cannot measure: `{binary} --version` printed no version (\"{raw}\")"
        ))),
        (Live::Version(l), Some(e)) if *l != e => out.push(Outcome::Drift {
            expected: format!("version {e}"),
            actual: format!("version {l}"),
            detail: format!(
                "live {binary} is {l}, declared {} is {e}; apply would {} it",
                if matches!(pin, Pin::Latest) {
                    "latest"
                } else {
                    "pin"
                },
                if *l > e { "DOWNGRADE" } else { "upgrade" }
            ),
        }),
        _ => {}
    }
    match (pin, upstream) {
        (Pin::Version(p), Upstream::Version(u)) if p < u => out.push(Outcome::Drift {
            expected: format!("latest {u}"),
            actual: format!("pin {p}"),
            detail: format!("pin {p} is behind the latest {repo} release {u}"),
        }),
        (_, Upstream::Unmeasured(why)) => out.push(Outcome::Unmeasured(format!(
            "latest {repo} release not measured: {why}"
        ))),
        (Pin::Version(p), Upstream::NotChecked) => out.push(Outcome::NotChecked(format!(
            "upstream not checked (offline): pin {p} was not compared with the latest {repo} release"
        ))),
        (Pin::Latest, Upstream::NotChecked) => out.push(Outcome::NotChecked(
            "upstream not checked (offline): tag 'latest' cannot be compared without it"
                .to_string(),
        )),
        _ => {}
    }
    out
}

const LIVE: &str = "@@forjar-live";
const MISSING: &str = "@@forjar-missing";
const UPSTREAM: &str = "@@forjar-upstream";

/// The one read-only query: run `--version`, and optionally ask upstream.
///
/// The upstream leg is `github_release::apply_script`'s own fetch — `curl`
/// against `api.github.com/repos/<repo>/releases/...`, on the target — pointed
/// at `releases/latest`. A curl failure prints its reason, which becomes an
/// UNMEASURED verdict, never a pass.
pub fn probe_script(resource: &Resource, check_upstream: bool) -> String {
    let binary = resource.binary.as_deref().unwrap_or("unknown");
    let install_dir = resource.install_dir.as_deref().unwrap_or("/usr/local/bin");
    let bin = sh_squote(&format!("{install_dir}/{binary}"));
    let mut s = format!(
        "if [ -x {bin} ]; then echo '{LIVE}'; {bin} --version 2>&1 | head -n 20; \
         else echo '{MISSING}'; fi\n"
    );
    if check_upstream {
        let repo = resource.repo.as_deref().unwrap_or("unknown/unknown");
        let url = sh_squote(&format!(
            "https://api.github.com/repos/{repo}/releases/latest"
        ));
        s.push_str(&format!(
            "echo '{UPSTREAM}'\n\
             if R=$(curl -fsS --max-time 20 {url} 2>&1); then \
             printf '%s\\n' \"$R\" | grep -o '\"tag_name\": *\"[^\"]*\"' | head -n 1; \
             else printf 'error: %s\\n' \"$(printf '%s' \"$R\" | head -n 1)\"; fi\n"
        ));
    }
    s.push_str("exit 0\n");
    s
}

/// Split the probe's stdout into its live and upstream answers.
pub fn parse_probe(stdout: &str, check_upstream: bool) -> (Live, Upstream) {
    let (head, tail) = match stdout.split_once(UPSTREAM) {
        Some((h, t)) => (h, Some(t)),
        None => (stdout, None),
    };
    let live = if head.contains(MISSING) {
        Live::Missing
    } else {
        let text = head.split_once(LIVE).map_or(head, |(_, t)| t);
        match parse_semver(text) {
            Some(v) => Live::Version(v),
            None => Live::Unparseable(first_line(text)),
        }
    };
    let upstream = match (check_upstream, tail) {
        (false, _) => Upstream::NotChecked,
        (true, None) => Upstream::Unmeasured("the probe printed no upstream answer".into()),
        (true, Some(t)) => {
            let t = t.trim();
            if let Some(err) = t.strip_prefix("error:") {
                Upstream::Unmeasured(format!("curl: {}", err.trim()))
            } else {
                match parse_semver(t) {
                    Some(v) => Upstream::Version(v),
                    None => Upstream::Unmeasured(format!(
                        "no semver tag_name in the release API answer (\"{}\")",
                        first_line(t)
                    )),
                }
            }
        }
    };
    (live, upstream)
}

fn first_line(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("no output");
    line.chars().take(200).collect()
}

/// Every verdict for one machine's versioned binaries, in declaration order.
pub struct VersionPinVerdicts {
    /// Drift and unmeasured findings, in the shape every drift consumer reads.
    pub findings: Vec<DriftFinding>,
    /// `(resource id, why)` for every question that was not asked.
    pub not_checked: Vec<(String, String)>,
    /// Resource ids whose live binary was asked.
    pub inspected: Vec<String>,
}

/// Check every declared versioned binary on `machine_name`.
///
/// `resources` must already be template-resolved (PMAT-197).
pub fn check_version_pins(
    machine_name: &str,
    machine: &Machine,
    resources: &indexmap::IndexMap<String, Resource>,
    check_upstream: bool,
) -> VersionPinVerdicts {
    let mut v = VersionPinVerdicts {
        findings: Vec::new(),
        not_checked: Vec::new(),
        inspected: Vec::new(),
    };
    for (id, resource) in resources {
        if !is_versioned_binary(resource)
            || !resource.machine.iter().any(|m| m == machine_name)
            || should_ignore_drift(id, resources)
        {
            continue;
        }
        check_one(id, resource, machine, check_upstream, &mut v);
    }
    v
}

fn check_one(
    id: &str,
    resource: &Resource,
    machine: &Machine,
    check_upstream: bool,
    v: &mut VersionPinVerdicts,
) {
    let repo = resource.repo.as_deref().unwrap_or("unknown/unknown");
    let binary = resource.binary.as_deref().unwrap_or("unknown");
    let pin = declared_pin(resource);
    if !is_valid_repo(repo) {
        v.findings.push(finding(
            id,
            "a valid repo",
            "ERROR",
            format!("invalid github repo slug: {repo}"),
        ));
        return;
    }
    // A tag with no version needs no probe: there is nothing to compare.
    if let Pin::Unversioned(_) = pin {
        for o in evaluate(binary, repo, &pin, &Live::Missing, &Upstream::NotChecked) {
            push_outcome(id, o, v);
        }
        return;
    }
    let script = probe_script(resource, check_upstream);
    let out = match unmeasured::read(machine, &script) {
        Reading::Answered(out) if out.success() => out,
        Reading::Answered(out) => {
            v.findings.push(DriftFinding::unmeasured(
                id,
                ResourceType::GithubRelease,
                &pin_label(&pin),
                format!(
                    "version probe failed (exit {}): {}",
                    out.exit_code,
                    first_line(&out.stderr)
                ),
            ));
            return;
        }
        Reading::Unmeasured(why) => {
            v.findings.push(DriftFinding::unmeasured(
                id,
                ResourceType::GithubRelease,
                &pin_label(&pin),
                why,
            ));
            return;
        }
    };
    let (live, upstream) = parse_probe(&out.stdout, check_upstream);
    if live != Live::Missing {
        v.inspected.push(id.to_string());
    }
    for o in evaluate(binary, repo, &pin, &live, &upstream) {
        push_outcome(id, o, v);
    }
}

fn pin_label(pin: &Pin) -> String {
    match pin {
        Pin::Version(p) => format!("version {p}"),
        Pin::Latest => "latest".to_string(),
        Pin::Unversioned(t) => t.clone(),
    }
}

fn push_outcome(id: &str, o: Outcome, v: &mut VersionPinVerdicts) {
    match o {
        Outcome::Drift {
            expected,
            actual,
            detail,
        } => v.findings.push(finding(id, &expected, &actual, detail)),
        Outcome::Unmeasured(why) => v.findings.push(DriftFinding::unmeasured(
            id,
            ResourceType::GithubRelease,
            "a measured version",
            why,
        )),
        Outcome::NotChecked(why) => v.not_checked.push((id.to_string(), why)),
    }
}

/// A version finding. The hash fields carry versions, never digests, because
/// no digest was taken.
fn finding(id: &str, expected: &str, actual: &str, detail: String) -> DriftFinding {
    DriftFinding {
        resource_id: id.to_string(),
        resource_type: ResourceType::GithubRelease,
        expected_hash: expected.to_string(),
        actual_hash: actual.to_string(),
        detail,
    }
}

/// Fold one machine's verdicts into a drift run's census and findings.
pub(super) fn detect(
    machine_name: &str,
    machine: &Machine,
    resources: &indexmap::IndexMap<String, Resource>,
    opts: super::DriftOptions,
    census: &mut DriftCensus,
) -> Vec<DriftFinding> {
    if !opts.check_version_pins {
        for (id, r) in resources {
            if is_versioned_binary(r) && r.machine.iter().any(|m| m == machine_name) {
                census.version_not_checked(id, "version pins not checked on this surface".into());
            }
        }
        return Vec::new();
    }
    let v = check_version_pins(machine_name, machine, resources, opts.check_upstream);
    for id in &v.inspected {
        census.inspected(id, &ResourceType::GithubRelease);
    }
    for (id, why) in v.not_checked {
        census.version_not_checked(&id, why);
    }
    v.findings
}

/// Is this finding one of this detector's drift verdicts?
///
/// Its `expected_hash` holds a version (`version 0.33.2`, `latest 0.34.2`), and
/// no digest-based detector writes that, because a digest is `blake3:…`.
pub fn is_version_finding(f: &DriftFinding) -> bool {
    f.resource_type == ResourceType::GithubRelease
        && !f.is_unmeasured()
        && (f.expected_hash.starts_with("version ") || f.expected_hash.starts_with("latest "))
}
