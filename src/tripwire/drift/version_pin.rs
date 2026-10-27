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

use crate::core::types::{Resource, ResourceType};

pub use super::semver::{parse_semver, Semver};

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
