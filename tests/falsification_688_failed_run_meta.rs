//! Refs #688: a resource that FAILED must never be recorded as `converged`.
//!
//! THE FLAW THIS CLOSES. `ResourceRunStatus` had no failure variant: a failed
//! resource was `Converged { failed: true }`, and `#[serde(tag = "action")]`
//! writes the variant name, so the run's `meta.yaml` said
//!
//! ```yaml
//! guard:
//!   action: converged
//!   exit_code: 1
//!   failed: true
//! ```
//!
//! beside a summary of `converged: 0, failed: 1`. Anything that keys on
//! `action` — the field a reader is invited to key on — counted the failure
//! as a success.
//!
//! WHAT THIS TEST MUST NOT BECOME. A unit test that builds a status value and
//! serializes it proves the type, not the record: the writer could keep
//! choosing `Converged`. These tests run the BINARY on a fixture and read the
//! `meta.yaml` that apply actually wrote, and they read it as untyped YAML so
//! the assertion is about the bytes on disk, not about forjar's own reader.

use std::path::Path;
use std::process::Command;

/// One local machine and one task: its command exits `code`, and its
/// `completion_check` (what forjar asks the host afterwards) exits `check`.
fn fixture(dir: &Path, code: i32, check: i32) {
    let cfg = format!(
        "version: \"1.0\"\n\
         name: meta-fixture\n\
         machines:\n  box:\n    hostname: box\n    addr: 127.0.0.1\n\
         resources:\n  guard:\n    type: task\n    machine: box\n    \
         command: \"echo guard-check; exit {code}\"\n    \
         completion_check: \"exit {check}\"\n"
    );
    std::fs::write(dir.join("forjar.yaml"), cfg).unwrap();
}

/// Apply the fixture and return the one run's `meta.yaml`, untyped.
fn apply_and_read_meta(dir: &Path) -> serde_yaml_ng::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(dir)
        .args([
            "apply",
            "-f",
            "forjar.yaml",
            "--state-dir",
            "state",
            "--yes",
        ])
        .output()
        .expect("run forjar apply");
    let runs = dir.join("state/box/runs");
    let mut metas: Vec<_> = std::fs::read_dir(&runs)
        .unwrap_or_else(|e| {
            panic!(
                "no run dir at {} ({e}); apply said:\n{}",
                runs.display(),
                String::from_utf8_lossy(&out.stderr)
            )
        })
        .map(|e| e.unwrap().path().join("meta.yaml"))
        .filter(|p| p.exists())
        .collect();
    assert_eq!(metas.len(), 1, "one apply, one run: {metas:?}");
    let text = std::fs::read_to_string(metas.pop().unwrap()).unwrap();
    serde_yaml_ng::from_str(&text).unwrap()
}

#[test]
fn a_failed_task_is_recorded_as_failed() {
    let d = tempfile::tempdir().unwrap();
    fixture(d.path(), 1, 1);
    let meta = apply_and_read_meta(d.path());
    let row = &meta["resources"]["guard"];
    assert_eq!(
        row["action"].as_str(),
        Some("failed"),
        "#688: a task that exited 1 was recorded as {row:?}"
    );
    assert_eq!(row["exit_code"].as_i64(), Some(1), "{row:?}");
    assert_eq!(meta["summary"]["failed"].as_u64(), Some(1));
    assert_eq!(meta["summary"]["converged"].as_u64(), Some(0));
}

/// The other direction: `converged` still means a resource that succeeded.
#[test]
fn a_succeeding_task_is_recorded_as_converged() {
    let d = tempfile::tempdir().unwrap();
    fixture(d.path(), 0, 0);
    let meta = apply_and_read_meta(d.path());
    let row = &meta["resources"]["guard"];
    assert_eq!(row["action"].as_str(), Some("converged"), "{row:?}");
    assert_eq!(meta["summary"]["converged"].as_u64(), Some(1));
    assert_eq!(meta["summary"]["failed"].as_u64(), Some(0));
}

/// The command exits 0 but the host does not report the declared state. Apply
/// counts that as a failure; before #688 the run's `resources:` had NO row for
/// it at all, so the record said `failed: 0` while apply said `1 FAILED`.
#[test]
fn a_task_the_host_refuses_after_exit_0_is_recorded_as_failed() {
    let d = tempfile::tempdir().unwrap();
    fixture(d.path(), 0, 1);
    let meta = apply_and_read_meta(d.path());
    let row = &meta["resources"]["guard"];
    assert_eq!(
        row["action"].as_str(),
        Some("failed"),
        "#688: a resource the host refused was recorded as {row:?}"
    );
    assert_eq!(row["exit_code"].as_i64(), Some(0), "{row:?}");
    assert_eq!(meta["summary"]["failed"].as_u64(), Some(1));
}

/// Run dirs written before #688 carry `action: converged` + `failed: true`.
/// They must still read, and read as the failure they recorded.
#[test]
fn a_pre_688_failure_row_reads_as_failed() {
    let old = "run_id: r-old\nmachine: box\ncommand: apply\n\
               resources:\n  guard:\n    action: converged\n    exit_code: 1\n    \
               duration_secs: 0.5\n    failed: true\n\
               summary: {total: 1, converged: 0, noop: 0, failed: 1, skipped: 0}\n";
    let meta: forjar::core::types::RunMeta = serde_yaml_ng::from_str(old).unwrap();
    let again: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&serde_yaml_ng::to_string(&meta).unwrap()).unwrap();
    assert_eq!(
        again["resources"]["guard"]["action"].as_str(),
        Some("failed"),
        "{again:?}"
    );
}

/// A resource apply never runs because its dependency failed. Apply counts it
/// in `resources_failed`; before #688 the run's `resources:` had no row for it.
/// `continue_independent` keeps the run going past the failure, so the
/// dependent is reached and skipped rather than the machine stopping.
#[test]
fn a_resource_skipped_for_a_failed_dependency_is_recorded_as_skipped() {
    let d = tempfile::tempdir().unwrap();
    let cfg = "version: \"1.0\"\n\
               name: meta-fixture\n\
               machines:\n  box:\n    hostname: box\n    addr: 127.0.0.1\n\
               policy:\n  failure: continue_independent\n\
               resources:\n  guard:\n    type: task\n    machine: box\n    \
               command: \"exit 1\"\n    completion_check: \"exit 1\"\n  \
               after:\n    type: task\n    machine: box\n    depends_on: [guard]\n    \
               command: \"exit 0\"\n    completion_check: \"exit 0\"\n";
    std::fs::write(d.path().join("forjar.yaml"), cfg).unwrap();
    let meta = apply_and_read_meta(d.path());
    assert_eq!(
        meta["resources"]["guard"]["action"].as_str(),
        Some("failed")
    );
    let row = &meta["resources"]["after"];
    assert_eq!(
        row["action"].as_str(),
        Some("skipped"),
        "#688: a resource skipped for a failed dependency was recorded as {row:?}"
    );
    assert!(
        row["reason"].as_str().is_some_and(|r| r.contains("guard")),
        "the skip names the dependency that failed: {row:?}"
    );
    assert_eq!(meta["summary"]["skipped"].as_u64(), Some(1));
}
