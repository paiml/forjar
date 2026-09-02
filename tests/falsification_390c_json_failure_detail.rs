//! Refs #390-C: machine-readable output must carry the failure, not `null`.
//!
//! THE FLAW THIS CLOSES.
//!
//! `--json`, `--output events` and `--report` emitted `"error": null` for every
//! FAILED resource, making them strictly WORSE than the console — which at least
//! printed stderr. For a CI pipeline that is the surface that matters, and it
//! carried nothing.
//!
//! Two independent causes, which had to be fixed together:
//!
//! 1. `build_resource_reports` read the error out of `details["error"]`, a key
//!    `record_failure` never wrote (`details: HashMap::new()`). So the field was
//!    unconditionally `None`.
//!
//! 2. `build_resource_reports(lock)` mapped the WHOLE persisted lock rather than
//!    this run's outcomes, so stale statuses and durations were reprinted for
//!    resources the run never executed. Filling the error without fixing this
//!    would have upgraded an obviously-contentless stale row into a convincing
//!    wrong one.
//!
//! WHY IT WAS SAFE TO FIX ONLY NOW. The failure string lands in
//! `state.lock.yaml`, which is re-serialised and blake3-sidecarred every run and
//! commonly committed. #390 bounded all six `record_failure` call sites first;
//! before that an unbounded stderr could have gone into a hashed, committed file.

use std::process::Command;

fn run(dir: &std::path::Path, args: &[&str]) -> (String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("forjar must run");
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.success(),
    )
}

fn write_cfg(dir: &std::path::Path, name: &str, body: &str) -> String {
    let p = dir.join(format!("{name}.yaml"));
    std::fs::write(
        &p,
        format!(
            r#"version: '1.0'
name: jsonfail
machines:
  local:
    hostname: localhost
    addr: localhost
    transport: local
resources:
{body}"#
        ),
    )
    .unwrap();
    p.to_string_lossy().into_owned()
}

fn failing(dir: &std::path::Path) -> String {
    format!(
        "  boom:\n    machine: local\n    type: task\n    working_dir: {}\n    \
         command: |\n      echo \"JSONC_STDERR\" >&2\n    completion_check: |\n      \
         test -e /nonexistent-390c\n",
        dir.display()
    )
}

/// Pull `machines[].resource_reports` out of an `apply --json` payload.
///
/// NOTE the path. The reports are NOT top-level — the first version of this test
/// looked for `resource_reports` at the root, found nothing, and passed
/// vacuously against a build with the fix reverted.
fn reports(out: &str) -> Vec<serde_json::Value> {
    // `run()` merges stdout and stderr, and forjar prints a human summary line
    // after the JSON, so `from_str` sees trailing characters. Take the FIRST
    // complete value and stop there.
    let start = out.find('{').expect("apply --json must emit an object");
    let v: serde_json::Value = serde_json::Deserializer::from_str(&out[start..])
        .into_iter::<serde_json::Value>()
        .next()
        .expect("apply --json must emit at least one JSON value")
        .expect("apply --json must be valid JSON");
    v["machines"]
        .as_array()
        .expect("machines[] must be present")
        .iter()
        .flat_map(|m| {
            m["resource_reports"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .collect()
}

#[test]
fn a_failed_resource_carries_its_error_in_json() {
    // THE REGRESSION. Before the fix this field was unconditionally null, making
    // --json strictly worse than the console for a CI pipeline.
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_cfg(dir.path(), "c", &failing(dir.path()));
    let (out, ok) = run(
        dir.path(),
        &["apply", "--yes", "-f", &cfg, "--state-dir", "st", "--json"],
    );
    assert!(!ok, "the fixture must fail, or there is nothing to report");

    let rs = reports(&out);
    let boom = rs
        .iter()
        .find(|r| r["resource_id"] == "boom")
        .expect("the failed resource must appear in resource_reports");
    assert_eq!(boom["status"], "failed");
    assert!(
        !boom["error"].is_null(),
        "a FAILED resource reports \"error\": null — the machine-readable surface \
         carries less than the console.\n{boom}"
    );
    let err = boom["error"].as_str().unwrap_or("");
    assert!(
        err.contains("NOT CONVERGED") || err.contains("JSONC_STDERR"),
        "the error is non-null but says nothing about what failed: {err:?}"
    );
}

#[test]
fn the_lock_records_the_failure_text() {
    // The mechanism, asserted directly: record_failure must write
    // details["error"], which is what build_resource_reports reads.
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_cfg(dir.path(), "c", &failing(dir.path()));
    run(
        dir.path(),
        &["apply", "--yes", "-f", &cfg, "--state-dir", "st"],
    );

    let lock = std::fs::read_to_string(dir.path().join("st/local/state.lock.yaml"))
        .expect("a lock must exist after a failed apply");
    assert!(
        lock.contains("error:"),
        "the lock entry for a failed resource carries no error key:\n{lock}"
    );
    assert!(
        lock.contains("NOT CONVERGED") || lock.contains("JSONC_STDERR"),
        "the recorded error is present but says nothing about what failed:\n{lock}"
    );
}

#[test]
fn a_report_does_not_include_resources_this_run_never_touched() {
    // The second half. Two configs sharing a state dir: applying only the second
    // must not reprint a stale row for the first. Asserted directly — the first
    // version wrapped this in `if let`, so it passed silently whenever the shape
    // was not what it expected.
    let dir = tempfile::tempdir().unwrap();
    let first = format!(
        "  alpha:\n    machine: local\n    type: file\n    path: {}/a.txt\n    content: \"a\"\n",
        dir.path().display()
    );
    let second = format!(
        "  beta:\n    machine: local\n    type: file\n    path: {}/b.txt\n    content: \"b\"\n",
        dir.path().display()
    );
    let c1 = write_cfg(dir.path(), "one", &first);
    let c2 = write_cfg(dir.path(), "two", &second);

    run(
        dir.path(),
        &["apply", "--yes", "-f", &c1, "--state-dir", "st"],
    );
    let (out, _) = run(
        dir.path(),
        &["apply", "--yes", "-f", &c2, "--state-dir", "st", "--json"],
    );

    let ids: Vec<String> = reports(&out)
        .iter()
        .filter_map(|r| r["resource_id"].as_str().map(String::from))
        .collect();
    assert!(
        ids.iter().any(|i| i == "beta"),
        "the resource this run DID apply is missing: {ids:?}"
    );
    assert!(
        !ids.iter().any(|i| i == "alpha"),
        "the report includes `alpha`, which this run never executed — stale lock \
         rows are being reprinted as this run's outcome: {ids:?}"
    );
}
