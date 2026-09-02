//! `Cargo.lock` must not pin a crate release that crates.io has yanked, and the
//! audit lane must be wired to say so.
//!
//! forjar#364. `cargo package --locked` during the 1.21.0 release preflight
//! printed, and kept printing on every release since:
//!
//!   warning: package `spin v0.9.8` in Cargo.lock is yanked in registry
//!   `crates-io`, consider updating to a version that is not yanked
//!
//! The pin is transitive — `spin 0.9.8` is reached only through `wasmi 0.40.0`,
//! the WASM plugin runtime — and it sat there for months because BOTH halves of
//! the standing gate were blind to it in different ways:
//!
//!   * `cargo deny check` (audit.yml) resolves a FEATURE GRAPH, and
//!     `deny.toml`'s `all-features = false` keeps the optional `wasmi` out of
//!     it. Measured: with `yanked = "deny"` and `all-features = false` the run
//!     exits 0 with zero mentions of `yanked`. It never saw the crate.
//!   * `cargo audit`, in the SAME job, reads `Cargo.lock` directly with no
//!     feature resolution, and HAS been reporting it on every daily cron run:
//!
//! ```text
//! Crate:     spin
//! Version:   0.9.8
//! Warning:   yanked
//! Dependency tree: spin 0.9.8 └── wasmi 0.40.0 └── forjar
//! ```
//!
//!     It exits 0 anyway, because a yank is a warning class:
//!     `warning: 2 allowed warnings found`. The gate saw it every day for
//!     months and had no way to fail.
//!
//! So the issue's own question — "whether the gate treats yanks as a finding"
//! — has a measured answer: one tool cannot see it and the other sees it and
//! shrugs. `cargo audit --deny yanked` exits 1 on the pre-fix lockfile
//! (`error: 1 denied warning found!`), which is the half that generalises: it
//! catches the NEXT yank, not just this one.
//!
//! # Why this test asserts both halves
//!
//! Assertion (a) alone is a frozen denylist — it pins today's known-bad triple
//! and would never notice a crate yanked tomorrow. Assertion (b) alone proves
//! only that a flag is present in a workflow. Together: (b) delegates the open
//! class to the tool that enumerates yanks upstream, and (a) is the offline,
//! hermetic tripwire that keeps a regenerated lockfile from silently walking
//! back to the exact release we already know is bad.
//!
//! The triple includes the CHECKSUM, not just name+version. A lockfile
//! regenerated against a stale registry cache can re-pin the same version; the
//! checksum is what identifies the exact artifact upstream withdrew.
//!
//! This test reads repo files through `CARGO_MANIFEST_DIR` and opens no
//! network connection — the same idiom as
//! `tests/falsification_hosted_jobs_do_not_cache_target.rs`.

use std::path::{Path, PathBuf};

/// A crate release crates.io has yanked, identified by its exact artifact.
struct YankedRelease {
    name: &'static str,
    version: &'static str,
    checksum: &'static str,
    /// Why it is here, and what to move to.
    note: &'static str,
}

/// Releases verified yanked against the local crates.io index at the time this
/// guard was written. A whole-lockfile sweep of all 518 checksummed packages
/// against that index found exactly one, so this list was complete on that day
/// — which is precisely why assertion (b) exists to cover every day after.
const KNOWN_YANKED: &[YankedRelease] = &[YankedRelease {
    name: "spin",
    version: "0.9.8",
    checksum: "6980e8d7511241f8acf4aebddbb1ff938df5eebe98691418c4468d0b72a96a67",
    note: "yanked upstream; 0.9.9 is unyanked and carries a real soundness fix \
           (a ManuallyDrop guard around assume_init_read in Once::into_inner). \
           `cargo update -p spin` moves the pin without touching Cargo.toml, \
           because the requirement is already ^0.9.",
}];

fn repo_file(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// One `[[package]]` stanza from `Cargo.lock`, reduced to the fields that
/// identify the artifact.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct LockPackage {
    name: String,
    version: String,
    checksum: String,
}

/// Parse `Cargo.lock` into its `[[package]]` stanzas.
///
/// Deliberately a small hand parser rather than a grep: a grep for
/// `version = "0.9.8"` matches any of ~500 packages, and a grep for the
/// checksum alone would be satisfied by the string appearing in a comment.
/// Only the (name, version, checksum) TRIPLE inside one stanza identifies a
/// release.
fn parse_lockfile(text: &str) -> Vec<LockPackage> {
    let mut out = Vec::new();
    let mut cur: Option<LockPackage> = None;
    for line in text.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            if let Some(p) = cur.take() {
                out.push(p);
            }
            cur = Some(LockPackage::default());
            continue;
        }
        // Any other table header ends the current stanza.
        if line.starts_with('[') {
            if let Some(p) = cur.take() {
                out.push(p);
            }
            continue;
        }
        let Some(pkg) = cur.as_mut() else { continue };
        if let Some(v) = scalar(line, "name") {
            pkg.name = v;
        } else if let Some(v) = scalar(line, "version") {
            pkg.version = v;
        } else if let Some(v) = scalar(line, "checksum") {
            pkg.checksum = v;
        }
    }
    if let Some(p) = cur.take() {
        out.push(p);
    }
    out
}

/// `key = "value"` -> `value`, for the flat scalar keys of a lock stanza.
fn scalar(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// (a) THE PIN. No `[[package]]` stanza may name a release we know is yanked.
#[test]
fn cargo_lock_pins_no_known_yanked_release() {
    let text =
        std::fs::read_to_string(repo_file("Cargo.lock")).expect("Cargo.lock must be readable");
    let packages = parse_lockfile(&text);

    // A structural guard that scanned nothing would pass loudly. Print the
    // denominator and refuse an empty parse.
    assert!(
        packages.len() > 100,
        "parsed only {} [[package]] stanzas from Cargo.lock -- the parser is \
         broken, and a guard that scans nothing is not a guard",
        packages.len()
    );
    eprintln!(
        "scanned {} locked packages against {} known-yanked release(s)",
        packages.len(),
        KNOWN_YANKED.len()
    );

    let mut found = Vec::new();
    for bad in KNOWN_YANKED {
        for pkg in &packages {
            if pkg.name == bad.name && pkg.version == bad.version && pkg.checksum == bad.checksum {
                found.push(format!(
                    "  {} {} (checksum {}...)\n    {}",
                    bad.name,
                    bad.version,
                    &bad.checksum[..16],
                    bad.note
                ));
            }
        }
    }

    assert!(
        found.is_empty(),
        "Cargo.lock pins {} release(s) that crates.io has yanked:\n{}\n\
         A yank is upstream saying that exact artifact was wrong. \
         `cargo package --locked` warns about it on every release preflight.",
        found.len(),
        found.join("\n")
    );
}

/// (b) THE LANE. The audit workflow's `cargo audit` must deny yanks.
///
/// Without `--deny yanked`, `cargo audit` prints the finding and exits 0 —
/// which is exactly the state that let #364 sit unnoticed through months of
/// daily cron runs. This is the assertion that survives the next yank.
#[test]
fn audit_workflow_denies_yanked_crates() {
    let path = repo_file(".github/workflows/audit.yml");
    let text = std::fs::read_to_string(&path).expect("audit.yml must be readable");

    // Find the actual invocation line, not a mention in a comment.
    let invocations: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .filter(|l| l.starts_with("cargo audit"))
        .collect();

    assert_eq!(
        invocations.len(),
        1,
        "expected exactly one `cargo audit` invocation in {}, found {}: {:?}\n\
         If the lane was restructured, update this guard deliberately rather \
         than letting it scan nothing.",
        path.display(),
        invocations.len(),
        invocations
    );

    let invocation = invocations[0];
    assert!(
        invocation.contains("--deny yanked"),
        "the audit lane's cargo-audit invocation does not deny yanked crates:\n\
         \x20   {invocation}\n\
         Without --deny yanked, `cargo audit` reports a yanked pin as \
         `Warning: yanked` and still exits 0 (`warning: N allowed warnings \
         found`), so the daily audit cannot fail on it -- which is how #364 \
         survived. Note this denies the yanked CLASS only: `unsound` and \
         `unmaintained` stay warnings and will not red the lane."
    );
}
