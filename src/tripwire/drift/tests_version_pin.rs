//! forjar#613: a pinned binary whose live `--version` is not its pin is drift.

use super::version_pin::{
    declared_pin, evaluate, parse_probe, parse_semver, Live, Outcome, Pin, Semver, Upstream,
};
use super::*;
use crate::core::types::{MachineTarget, Resource, ResourceLock, StateLock};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;

fn local_machine() -> Machine {
    serde_yaml_ng::from_str("hostname: box\naddr: 127.0.0.1").unwrap()
}

/// Offline: no test reaches api.github.com. The upstream leg is covered by the
/// pure `evaluate` / `parse_probe` tests below.
fn offline() -> DriftOptions {
    DriftOptions {
        check_upstream: false,
        ..DriftOptions::default()
    }
}

/// A fake binary at `<dir>/ollama` whose `--version` prints `output`.
fn fake_binary(dir: &std::path::Path, output: &str) {
    let path = dir.join("ollama");
    std::fs::write(&path, format!("#!/bin/sh\nprintf '%s\\n' '{output}'\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// paiml/infra's shape: ollama pinned to v0.33.2.
fn ollama(dir: &std::path::Path) -> Resource {
    Resource {
        resource_type: ResourceType::GithubRelease,
        machine: MachineTarget::Single("box".to_string()),
        repo: Some("ollama/ollama".to_string()),
        tag: Some("v0.33.2".to_string()),
        asset_pattern: Some("*linux-amd64*".to_string()),
        binary: Some("ollama".to_string()),
        install_dir: Some(dir.display().to_string()),
        ..Default::default()
    }
}

fn config_with(id: &str, resource: Resource) -> IndexMap<String, Resource> {
    let mut resources = IndexMap::new();
    resources.insert(id.to_string(), resource);
    resources
}

fn lock_with(entries: Vec<(&str, ResourceLock)>) -> StateLock {
    StateLock {
        schema: "1.0".to_string(),
        machine: "box".to_string(),
        hostname: "box".to_string(),
        generated_at: "now".to_string(),
        generator: "test".to_string(),
        created_by: None,
        blake3_version: "1.8".to_string(),
        resources: entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    }
}

/// A converged lock entry whose baseline was taken AFTER the out-of-band
/// install, so the digest path compares the box with itself and sees nothing.
fn rebaselined_entry(resource: &Resource) -> ResourceLock {
    let q = crate::core::codegen::state_query_script(resource).unwrap();
    let out = crate::transport::exec_script(&local_machine(), &q).unwrap();
    let live = crate::tripwire::hasher::hash_string_or_sentinel(
        &crate::core::observation_mask::masked_for(&out.stdout, resource),
    );
    ResourceLock {
        resource_type: ResourceType::GithubRelease,
        status: ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        hash: "blake3:desired".to_string(),
        observed: Some(live),
        details: HashMap::new(),
    }
}

// ---------------------------------------------------------------------------
// The headline: RED before the fix, GREEN after.
// ---------------------------------------------------------------------------

#[test]
fn fj613_live_version_ahead_of_pin_is_drift_even_when_the_lock_agrees() {
    let dir = tempfile::tempdir().unwrap();
    fake_binary(dir.path(), "ollama version is 0.34.2");
    let r = ollama(dir.path());
    let lock = lock_with(vec![("ollama-binary", rebaselined_entry(&r))]);
    let resources = config_with("ollama-binary", r);

    let report = detect_drift_full_reported(&lock, &local_machine(), &resources, offline());

    let drifted: Vec<_> = report
        .findings
        .iter()
        .filter(|f| !f.is_unmeasured() && f.resource_id == "ollama-binary")
        .collect();
    assert_eq!(
        drifted.len(),
        1,
        "live ollama 0.34.2 under a v0.33.2 pin must be DRIFT; got {:?}",
        report.findings
    );
    assert_eq!(drifted[0].expected_hash, "version 0.33.2");
    assert_eq!(drifted[0].actual_hash, "version 0.34.2");
    assert!(
        drifted[0].detail.contains("DOWNGRADE"),
        "the operator must be told the apply would move the box backwards: {}",
        drifted[0].detail
    );
    // Offline is NAMED, never silently green.
    let lines = report.census.summary_lines().join("\n");
    assert!(
        lines.contains("version pin NOT checked 1: ollama-binary (upstream not checked (offline)"),
        "census must name the pin that was not compared with upstream:\n{lines}"
    );
}

#[test]
fn fj613_a_pin_never_applied_from_here_is_still_compared() {
    // infra's actual state: not in the lock at all.
    let dir = tempfile::tempdir().unwrap();
    fake_binary(dir.path(), "ollama version is 0.34.2");
    let lock = lock_with(vec![]);
    let resources = config_with("ollama-binary", ollama(dir.path()));

    let report = detect_drift_full_reported(&lock, &local_machine(), &resources, offline());

    assert!(
        report
            .findings
            .iter()
            .any(|f| version_pin::is_version_finding(f) && f.actual_hash == "version 0.34.2"),
        "got {:?}",
        report.findings
    );
    assert_eq!(report.census.inspected_total(), 1);
}

#[test]
fn fj613_lockless_run_compares_the_pin() {
    let dir = tempfile::tempdir().unwrap();
    fake_binary(dir.path(), "ollama version is 0.34.2");
    let resources = config_with("ollama-binary", ollama(dir.path()));

    let report = detect_drift_lockless("box", &local_machine(), &resources, offline());

    assert!(
        report.findings.iter().any(version_pin::is_version_finding),
        "got {:?}",
        report.findings
    );
}

#[test]
fn fj613_matching_version_is_clean_and_offline_is_still_named() {
    let dir = tempfile::tempdir().unwrap();
    fake_binary(dir.path(), "ollama version is 0.33.2");
    let resources = config_with("ollama-binary", ollama(dir.path()));

    let report =
        detect_drift_full_reported(&lock_with(vec![]), &local_machine(), &resources, offline());

    assert!(report.findings.is_empty(), "got {:?}", report.findings);
    assert_eq!(report.census.version_not_checked_ids().len(), 1);
}

#[test]
fn fj613_unparseable_version_is_unmeasured_never_clean() {
    let dir = tempfile::tempdir().unwrap();
    fake_binary(dir.path(), "usage: ollama [command]");
    let resources = config_with("ollama-binary", ollama(dir.path()));

    let report =
        detect_drift_full_reported(&lock_with(vec![]), &local_machine(), &resources, offline());

    assert_eq!(report.findings.len(), 1, "got {:?}", report.findings);
    assert!(report.findings[0].is_unmeasured());
    assert!(report.findings[0].detail.contains("cannot measure"));
    assert_eq!(report.census.unmeasured_total(), 1);
}

#[test]
fn fj613_remediation_wrapper_never_sees_version_findings() {
    // `detect_drift_full` feeds the apply gate: a version finding there would
    // mark the resource drifted and re-apply the older pin.
    let dir = tempfile::tempdir().unwrap();
    fake_binary(dir.path(), "ollama version is 0.34.2");
    let r = ollama(dir.path());
    let lock = lock_with(vec![("ollama-binary", rebaselined_entry(&r))]);
    let resources = config_with("ollama-binary", r);

    let findings = detect_drift_full(&lock, &local_machine(), &resources);

    assert!(findings.is_empty(), "got {findings:?}");
}

#[test]
fn fj613_disabled_surface_names_the_pins_it_skipped() {
    let dir = tempfile::tempdir().unwrap();
    fake_binary(dir.path(), "ollama version is 0.34.2");
    let resources = config_with("ollama-binary", ollama(dir.path()));
    let opts = DriftOptions {
        run_task_checks: false,
        check_version_pins: false,
        check_upstream: false,
    };

    let report = detect_drift_full_reported(&lock_with(vec![]), &local_machine(), &resources, opts);

    assert!(report.findings.is_empty());
    assert_eq!(
        report.census.version_not_checked_ids(),
        vec![("ollama-binary", "version pins not checked on this surface")]
    );
}

// ---------------------------------------------------------------------------
// Parsing.
// ---------------------------------------------------------------------------

fn v(major: u64, minor: u64, patch: u64) -> Semver {
    Semver {
        major,
        minor,
        patch,
    }
}

#[test]
fn fj613_parse_semver_reads_the_shapes_in_the_wild() {
    assert_eq!(parse_semver("ollama version is 0.34.2"), Some(v(0, 34, 2)));
    assert_eq!(parse_semver("forjar 1.32.0"), Some(v(1, 32, 0)));
    assert_eq!(parse_semver("v0.34.2"), Some(v(0, 34, 2)));
    assert_eq!(
        parse_semver("rclone v1.68.1\n- os/version: ubuntu 24.04"),
        Some(v(1, 68, 1))
    );
    assert_eq!(
        parse_semver(
            "Warning: could not connect to a running Ollama instance\nWarning: client version is 0.34.2"
        ),
        Some(v(0, 34, 2))
    );
    assert_eq!(parse_semver("tool 2.0.0-rc.1 (abc123)"), Some(v(2, 0, 0)));
}

#[test]
fn fj613_parse_semver_refuses_what_is_not_a_version() {
    assert_eq!(parse_semver(""), None);
    assert_eq!(parse_semver("usage: ollama [command]"), None);
    assert_eq!(parse_semver("ollama version 0.34"), None);
    assert_eq!(parse_semver("listening on 10.42.0.11"), None);
    assert_eq!(parse_semver("abc1.2.3"), None);
}

#[test]
fn fj613_declared_pin_from_tag() {
    let mut r = Resource {
        resource_type: ResourceType::GithubRelease,
        ..Default::default()
    };
    assert_eq!(declared_pin(&r), Pin::Latest);
    r.tag = Some("latest".into());
    assert_eq!(declared_pin(&r), Pin::Latest);
    r.tag = Some("v0.33.2".into());
    assert_eq!(declared_pin(&r), Pin::Version(v(0, 33, 2)));
    r.tag = Some("nightly".into());
    assert_eq!(declared_pin(&r), Pin::Unversioned("nightly".into()));
}

#[test]
fn fj613_parse_probe_splits_live_and_upstream() {
    let (live, up) = parse_probe(
        "@@forjar-live\nollama version is 0.34.2\n@@forjar-upstream\n\"tag_name\": \"v0.35.0\"\n",
        true,
    );
    assert_eq!(live, Live::Version(v(0, 34, 2)));
    assert_eq!(up, Upstream::Version(v(0, 35, 0)));

    let (live, up) = parse_probe(
        "@@forjar-missing\n@@forjar-upstream\nerror: curl: (6) Could not resolve host: api.github.com\n",
        true,
    );
    assert_eq!(live, Live::Missing);
    assert!(matches!(up, Upstream::Unmeasured(ref e) if e.contains("Could not resolve host")));

    let (_, up) = parse_probe("@@forjar-live\nx 1.0.0\n@@forjar-upstream\n", true);
    assert!(
        matches!(up, Upstream::Unmeasured(_)),
        "empty answer is not a version"
    );

    let (_, up) = parse_probe("@@forjar-live\nx 1.0.0\n", false);
    assert_eq!(up, Upstream::NotChecked);
}

// ---------------------------------------------------------------------------
// Verdicts ("latest CRUX tooling only").
// ---------------------------------------------------------------------------

#[test]
fn fj613_pin_behind_upstream_is_drift() {
    let out = evaluate(
        "ollama",
        "ollama/ollama",
        &Pin::Version(v(0, 33, 2)),
        &Live::Version(v(0, 33, 2)),
        &Upstream::Version(v(0, 34, 2)),
    );
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(
        matches!(&out[0], Outcome::Drift { expected, actual, .. } if expected == "latest 0.34.2" && actual == "pin 0.33.2"),
        "{out:?}"
    );
}

#[test]
fn fj613_pin_at_upstream_and_live_is_clean() {
    let out = evaluate(
        "ollama",
        "ollama/ollama",
        &Pin::Version(v(0, 34, 2)),
        &Live::Version(v(0, 34, 2)),
        &Upstream::Version(v(0, 34, 2)),
    );
    assert!(out.is_empty(), "{out:?}");
}

#[test]
fn fj613_upstream_unanswered_is_unmeasured() {
    let out = evaluate(
        "ollama",
        "ollama/ollama",
        &Pin::Version(v(0, 34, 2)),
        &Live::Version(v(0, 34, 2)),
        &Upstream::Unmeasured("curl: (22) 403".into()),
    );
    assert!(
        matches!(&out[..], [Outcome::Unmeasured(e)] if e.contains("403")),
        "{out:?}"
    );
}

#[test]
fn fj613_tag_latest_compares_live_with_upstream() {
    let out = evaluate(
        "ollama",
        "ollama/ollama",
        &Pin::Latest,
        &Live::Version(v(0, 33, 2)),
        &Upstream::Version(v(0, 34, 2)),
    );
    assert!(
        matches!(&out[..], [Outcome::Drift { detail, .. }] if detail.contains("upgrade")),
        "{out:?}"
    );

    let offline = evaluate(
        "ollama",
        "ollama/ollama",
        &Pin::Latest,
        &Live::Version(v(0, 33, 2)),
        &Upstream::NotChecked,
    );
    assert!(
        matches!(&offline[..], [Outcome::NotChecked(_)]),
        "{offline:?}"
    );
}

#[test]
fn fj613_nightly_is_named_not_passed() {
    let out = evaluate(
        "apr",
        "paiml/aprender",
        &Pin::Unversioned("nightly".into()),
        &Live::Missing,
        &Upstream::NotChecked,
    );
    assert!(
        matches!(&out[..], [Outcome::NotChecked(w)] if w.contains("nightly")),
        "{out:?}"
    );
}
