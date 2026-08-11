//! Regression tests for the P3-observ dogfood defects (issue #208).
//!
//! Every test here corresponds to a defect reproduced twice against the
//! published forjar 1.12.3. They are grouped by root cause, not by symptom.

use crate::core::{executor, parser, types};
use std::path::{Path, PathBuf};

// ── helpers ────────────────────────────────────────────────────────────────

/// Write a localhost config with one file resource and one task resource.
fn write_cfg(dir: &Path, target: &Path) -> PathBuf {
    let yaml = format!(
        "version: \"1.0\"\nname: observ\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  hello-file:\n    type: file\n    machine: local\n    path: {}\n    content: \"hello v1\"\n",
        target.display()
    );
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).expect("write config");
    p
}

/// Converge a config locally with an explicit run id.
fn apply_with_run_id(config_path: &Path, state_dir: &Path, run_id: &str) {
    let config = parser::parse_and_validate(config_path).expect("parse");
    let order = crate::core::resolver::build_execution_order(&config).expect("order");
    let cfg = executor::ApplyConfig {
        config: &config,
        state_dir,
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: None,
        tag_filter: None,
        group_filter: None,
        timeout_secs: None,
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: Some(run_id.to_string()),
        refresh: false,
        force_tag: None,
    };
    let _ = order;
    executor::apply(&cfg).expect("apply");
}

/// Create a synthetic run directory with a meta.yaml and one log file.
fn make_run(state_dir: &Path, machine: &str, run_id: &str, started_at: Option<&str>) -> PathBuf {
    let dir = state_dir.join(machine).join("runs").join(run_id);
    std::fs::create_dir_all(&dir).expect("mkdir run");
    let started = match started_at {
        Some(s) => format!("started_at: \"{s}\"\n"),
        None => "started_at: null\n".to_string(),
    };
    std::fs::write(
        dir.join("meta.yaml"),
        format!(
            "run_id: {run_id}\nmachine: {machine}\ncommand: apply\n{started}\
resources: {{}}\nsummary:\n  total: 0\n  converged: 0\n  noop: 0\n  failed: 0\n  skipped: 0\n"
        ),
    )
    .expect("write meta");
    std::fs::write(dir.join("a.create.log"), "log body\n").expect("write log");
    dir
}

// ── one apply == one run id, one script, one discoverable action ───────────

/// Covers three defects that share a single apply:
///   * logs-run-id-disagrees-with-history-audit-state-query
///   * logs-script-flag-noop-scripts-never-recorded
///   * logs-resource-filter-drops-the-matching-resource
#[test]
fn one_apply_produces_one_run_id_a_script_and_a_findable_resource_log() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let target = tmp.path().join("hello.txt");
    let cfg = write_cfg(tmp.path(), &target);
    let state_dir = tmp.path().join("state");
    apply_with_run_id(&cfg, &state_dir, "r-observ-test");

    // The apply really happened (non-regression guard).
    assert!(target.exists(), "apply must still converge the resource");

    // 1. The run-log directory carries the id the caller minted…
    let run_dir = state_dir.join("local").join("runs").join("r-observ-test");
    assert!(
        run_dir.is_dir(),
        "run dir for the caller's run id must exist, found: {:?}",
        std::fs::read_dir(state_dir.join("local").join("runs"))
            .map(|d| d.flatten().map(|e| e.file_name()).collect::<Vec<_>>())
    );

    // …and so does the event stream. Previously these were two different ids.
    let events =
        std::fs::read_to_string(state_dir.join("local").join("events.jsonl")).expect("events");
    assert!(
        events.contains("r-observ-test"),
        "event log must carry the SAME run id as the run dir: {events}"
    );

    // 2. The generated script is persisted, not a 0-byte sidecar.
    let script = std::fs::read_to_string(run_dir.join("hello-file.script")).expect("script");
    assert!(
        !script.trim().is_empty(),
        "the executed script must be recorded for `logs --script`"
    );
    let log = std::fs::read_to_string(run_dir.join("hello-file.create.log")).expect("log");
    assert!(
        !log.contains("script_hash: \n"),
        "script_hash must not be blank: {log}"
    );

    // 3. `--resource` finds the action forjar actually recorded (`create`),
    //    not the hardcoded apply/check/destroy guesses.
    let actions = super::logs::actions_for_resource(&run_dir, "hello-file");
    assert_eq!(
        actions,
        vec!["create".to_string()],
        "resource filter must discover the recorded action"
    );
    assert!(
        super::logs::actions_for_resource(&run_dir, "no-such-resource").is_empty(),
        "an unknown resource must not match anything"
    );

    // 4. `history --resource` sees this resource in the machine event log.
    let found = super::history_resource::collect_resource_events(&state_dir, None, "hello-file")
        .expect("collect events");
    assert!(
        !found.is_empty(),
        "history --resource must find the resource's events in state/<machine>/events.jsonl"
    );
    let other = super::history_resource::collect_resource_events(&state_dir, None, "not-a-resource")
        .expect("collect events");
    assert!(
        other.is_empty(),
        "the filter must still exclude non-matching resources"
    );
}

#[test]
fn run_meta_records_a_start_timestamp() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let target = tmp.path().join("hello.txt");
    let cfg = write_cfg(tmp.path(), &target);
    let state_dir = tmp.path().join("state");
    apply_with_run_id(&cfg, &state_dir, "r-ts-test");
    let meta = std::fs::read_to_string(
        state_dir
            .join("local")
            .join("runs")
            .join("r-ts-test")
            .join("meta.yaml"),
    )
    .expect("meta");
    assert!(
        !meta.contains("started_at: null"),
        "a run without a start timestamp makes retention ordering arbitrary: {meta}"
    );
}

#[test]
fn history_resource_errors_when_there_is_no_state_dir() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let missing = tmp.path().join("nope");
    let err = super::history_resource::cmd_history_resource(&missing, None, "x", 10, false)
        .expect_err("a missing state dir must be reported, not silently empty");
    assert!(err.contains("does not exist"), "{err}");
}

// ── retention ordering ─────────────────────────────────────────────────────

#[test]
fn gc_keeps_the_newest_runs_not_an_arbitrary_subset() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let state_dir = tmp.path();
    // 15 runs, oldest first, with ids that do NOT sort in creation order so a
    // name-based or readdir-based sort cannot accidentally pass.
    let ids: Vec<String> = (0..15).map(|i| format!("r-{:04x}", (15 - i) * 977)).collect();
    for (i, id) in ids.iter().enumerate() {
        make_run(state_dir, "local", id, Some(&format!("2026-01-01T00:00:{i:02}Z")));
    }

    let retention = types::LogRetention {
        keep_runs: 10,
        ..Default::default()
    };
    super::logs_gc::cmd_logs_gc(state_dir, false, false, true, Some(&retention)).expect("gc");

    let surviving: std::collections::HashSet<String> =
        std::fs::read_dir(state_dir.join("local").join("runs"))
            .expect("read runs")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

    assert_eq!(surviving.len(), 10, "retention must keep exactly 10 runs");
    for id in ids.iter().take(5) {
        assert!(
            !surviving.contains(id),
            "the 5 OLDEST runs must be the ones deleted; {id} survived"
        );
    }
    for id in ids.iter().skip(5) {
        assert!(
            surviving.contains(id),
            "a newer run must never be deleted; {id} was"
        );
    }
}

#[test]
fn gc_orders_runs_without_timestamps_by_mtime() {
    // The published 1.12.3 wrote `started_at: null` on EVERY run, so the
    // retention sort compared "" to "" and the stable sort fell back to readdir
    // (hash) order — which is how `--gc` deleted the 2nd-newest run and kept the
    // oldest. Runs written by older forjars still look like this on disk, so the
    // order must be total even without timestamps.
    let tmp = tempfile::tempdir().expect("tempdir");
    let state_dir = tmp.path();
    let ids: Vec<String> = (0..6).map(|i| format!("r-{:04x}", (6 - i) * 977)).collect();
    for id in &ids {
        make_run(state_dir, "local", id, None);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let retention = types::LogRetention {
        keep_runs: 3,
        ..Default::default()
    };
    super::logs_gc::cmd_logs_gc(state_dir, false, false, true, Some(&retention)).expect("gc");

    let surviving: std::collections::HashSet<String> =
        std::fs::read_dir(state_dir.join("local").join("runs"))
            .expect("read runs")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
    assert_eq!(surviving.len(), 3);
    for id in ids.iter().skip(3) {
        assert!(
            surviving.contains(id),
            "the 3 newest runs must survive; {id} was deleted (surviving: {surviving:?})"
        );
    }
}

// ── follow ─────────────────────────────────────────────────────────────────

#[test]
fn follow_resolves_the_requested_run_not_the_newest() {
    let tmp = tempfile::tempdir().expect("tempdir");
    make_run(tmp.path(), "local", "r-old", Some("2026-01-01T00:00:00Z"));
    make_run(tmp.path(), "local", "r-new", Some("2026-01-01T00:00:09Z"));

    let newest = super::logs::resolve_follow_target(tmp.path(), None, None, false)
        .expect("resolve")
        .expect("a run");
    assert_eq!(newest.run_id, "r-new");

    let requested = super::logs::resolve_follow_target(tmp.path(), None, Some("r-old"), false)
        .expect("resolve")
        .expect("a run");
    assert_eq!(
        requested.run_id, "r-old",
        "--follow must honour --run rather than always watching the newest run"
    );
}

#[test]
fn follow_refuses_an_unknown_run_id() {
    let tmp = tempfile::tempdir().expect("tempdir");
    make_run(tmp.path(), "local", "r-new", Some("2026-01-01T00:00:09Z"));
    let err = super::logs::resolve_follow_target(tmp.path(), None, Some("r-nope"), false)
        .expect_err("an unknown run id must be refused, not silently redirected");
    assert!(err.contains("r-nope"), "{err}");
}

// ── anomaly ────────────────────────────────────────────────────────────────

#[test]
fn min_events_filters_the_analyzed_population() {
    let metrics = vec![
        ("local:a".to_string(), 3u32, 3u32, 0u32),
        ("local:b".to_string(), 1u32, 0u32, 0u32),
    ];
    assert_eq!(super::observe_anomaly::analyzed_count(&metrics, 1), 2);
    assert_eq!(super::observe_anomaly::analyzed_count(&metrics, 3), 1);
    assert_eq!(
        super::observe_anomaly::analyzed_count(&metrics, 9999),
        0,
        "--min-events 9999 must not claim to have analyzed anything"
    );
}

#[test]
fn a_lone_resource_failing_half_its_applies_is_an_anomaly() {
    // The dogfood case: ONE resource, 3 failures out of 6 applies. Every
    // detector before this fix was population-relative, so a population of one
    // could never be flagged.
    let metrics = vec![("local:flaky-task".to_string(), 3u32, 3u32, 0u32)];
    let findings = crate::tripwire::anomaly::detect_anomalies(&metrics, 3);
    assert_eq!(findings.len(), 1, "3/6 failures must be reported");
    assert!(
        findings[0].reasons.iter().any(|r| r.contains("failure rate")),
        "{:?}",
        findings[0].reasons
    );
    assert!(matches!(
        findings[0].status,
        crate::tripwire::anomaly::DriftStatus::Warning | crate::tripwire::anomaly::DriftStatus::Drift
    ));
}

#[test]
fn a_healthy_resource_is_not_flagged() {
    // Non-regression guard: the new absolute detector must not cry wolf.
    let metrics = vec![("local:steady".to_string(), 10u32, 0u32, 0u32)];
    assert!(crate::tripwire::anomaly::detect_anomalies(&metrics, 3).is_empty());
}

#[test]
fn a_130x_duration_outlier_is_detected() {
    let samples = vec![0.033, 0.033, 0.032, 0.032, 0.031, 4.033];
    let (score, reason) = crate::tripwire::anomaly::duration_outlier(&samples)
        .expect("a 130x outlier must be detected");
    assert!(score > 0.5, "score {score}");
    assert!(reason.contains("duration outlier"), "{reason}");
}

#[test]
fn steady_durations_are_not_flagged_as_outliers() {
    let samples = vec![0.033, 0.033, 0.032, 0.032, 0.031, 0.034];
    assert!(
        crate::tripwire::anomaly::duration_outlier(&samples).is_none(),
        "steady durations must not be reported as an outlier"
    );
    assert!(
        crate::tripwire::anomaly::duration_outlier(&[1.0, 9.0]).is_none(),
        "two samples are not enough evidence"
    );
}

#[test]
fn duration_detection_respects_min_events() {
    let durations = vec![(
        "local:t".to_string(),
        vec![0.03, 0.03, 0.03, 0.03, 0.03, 4.0],
    )];
    assert_eq!(
        crate::tripwire::anomaly::detect_duration_anomalies(&durations, 3).len(),
        1
    );
    assert!(
        crate::tripwire::anomaly::detect_duration_anomalies(&durations, 9999).is_empty(),
        "--min-events must gate the duration detector too"
    );
}
