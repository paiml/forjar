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
