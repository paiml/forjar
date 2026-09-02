//! FJ-2729 (PMAT-199): the MCP tools must agree with the CLI they mirror.
//!
//! Both defects below were found by driving the PUBLISHED 1.12.0 binary over
//! stdio and comparing each tool against its CLI counterpart on the same
//! project. Neither was visible from the schema, from `tools/list`, or from a
//! handler test that only asserted "returns Ok".

use super::handlers::*;
use super::types::*;
use crate::mcp::handlers::DriftHandler;
use pforge_runtime::Handler;

fn project(dir: &std::path::Path) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"
version: "1.0"
name: parity
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  real:
    type: file
    machine: local
    path: {}
    content: hi
  action:
    type: task
    machine: local
    phony: true
    command: "echo run"
"#,
            dir.join("f.txt").display()
        ),
    )
    .unwrap();
    cfg
}

/// Write a lock marking `real` converged, the way a successful apply would.
fn converged_lock(dir: &std::path::Path, cfg: &std::path::Path) {
    let config = crate::core::parser::parse_and_validate(cfg).unwrap();
    let hash = crate::core::planner::hash_desired_state(&config.resources["real"]);
    let md = dir.join("state").join("local");
    std::fs::create_dir_all(&md).unwrap();
    std::fs::write(
        md.join("state.lock.yaml"),
        format!(
            "schema: \"1.0\"\nmachine: local\nhostname: localhost\ngenerated_at: now\n\
             generator: test\nblake3_version: \"1\"\nresources:\n  real:\n    type: file\n\
             \x20   status: converged\n    hash: \"{hash}\"\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("f.txt"), "hi").unwrap();
}

#[tokio::test]
async fn plan_reports_only_real_changes_not_every_resource() {
    // `exec_plan.changes` carries EVERY resource with its action, NoOp
    // included; `cli::plan` filters those out before counting. The MCP handler
    // did not, so a fully converged project reported all of its resources as
    // pending. Verified on the published binary: CLI said "0 to change", MCP
    // said 6 changes.
    let d = tempfile::tempdir().unwrap();
    let cfg = project(d.path());
    converged_lock(d.path(), &cfg);

    let out = PlanHandler
        .handle(PlanInput {
            path: cfg.display().to_string(),
            state_dir: Some(d.path().join("state").display().to_string()),
            resource: None,
            tag: None,
        })
        .await
        .expect("plan runs");

    assert!(
        !out.changes.iter().any(|c| c.resource_id == "real"),
        "a converged resource must not be reported as a change: {:?}",
        out.changes
    );
    assert!(
        !out.changes.iter().any(|c| c.resource_id == "action"),
        "a phony resource is goal-only and must not appear in a bulk plan: {:?}",
        out.changes
    );
}

#[tokio::test]
async fn plan_still_reports_a_genuine_change() {
    // The guard against "fixed" meaning "always empty".
    let d = tempfile::tempdir().unwrap();
    let cfg = project(d.path());

    let out = PlanHandler
        .handle(PlanInput {
            path: cfg.display().to_string(),
            state_dir: Some(d.path().join("state").display().to_string()),
            resource: None,
            tag: None,
        })
        .await
        .expect("plan runs");

    assert!(
        out.changes.iter().any(|c| c.resource_id == "real"),
        "nothing has been applied, so `real` must be planned: {:?}",
        out.changes
    );
}

#[tokio::test]
async fn status_finds_machines_whose_state_is_a_directory() {
    // A machine's state is `state/<machine>/state.lock.yaml`. The handler
    // scanned for files with a `.json` extension, which forjar has never
    // written, so it returned an empty list for every project ever. Verified
    // on the published binary: CLI printed `Machine: local (localhost)` while
    // MCP returned `{"machines": []}`.
    let d = tempfile::tempdir().unwrap();
    let cfg = project(d.path());
    converged_lock(d.path(), &cfg);

    let out = StatusHandler
        .handle(StatusInput {
            path: None,
            state_dir: Some(d.path().join("state").display().to_string()),
            machine: None,
        })
        .await
        .expect("status runs");

    assert_eq!(
        out.machines.len(),
        1,
        "expected the `local` machine: {:?}",
        out.machines
    );
    assert_eq!(out.machines[0].name, "local");
    assert_eq!(out.machines[0].resource_count, 1);
}

#[tokio::test]
async fn status_ignores_a_directory_with_no_lock() {
    // Presence of the lock is the test, so a stray directory under the state
    // dir is not reported as a machine.
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("state").join("not-a-machine")).unwrap();

    let out = StatusHandler
        .handle(StatusInput {
            path: None,
            state_dir: Some(d.path().join("state").display().to_string()),
            machine: None,
        })
        .await
        .expect("status runs");

    assert!(out.machines.is_empty(), "{:?}", out.machines);
}

// ── GH-208: state_dir must follow the config, not the process cwd ──────────
//
// Every parity test above passes an explicit ABSOLUTE `state_dir` — exactly the
// case that always worked. That is why they all passed while the published
// binary told an MCP client a tampered machine was clean.
//
// These omit `state_dir`, which is what a real client does. No cwd juggling is
// needed to prove the point: the fixture lives in a tempdir while the test
// process runs from the crate root, so cwd is ALREADY not the config's
// directory — which is precisely the situation of an MCP stdio server, whose
// working directory is chosen by the client. (`set_current_dir` is disallowed in
// this repo for being process-global and flaky, and would add nothing here.)

#[tokio::test]
async fn plan_without_state_dir_finds_state_beside_the_config() {
    let d = tempfile::tempdir().unwrap();
    let cfg = project(d.path());
    converged_lock(d.path(), &cfg);

    let out = PlanHandler
        .handle(PlanInput {
            path: cfg.display().to_string(), // absolute
            state_dir: None,                 // <- the real-client case
            resource: None,
            tag: None,
        })
        .await
        .expect("plan runs");

    assert!(
        !out.changes.iter().any(|c| c.resource_id == "real"),
        "a converged project addressed by absolute path must not report CREATE \
         just because the server's cwd is elsewhere (GH-208): {:?}",
        out.changes
    );
}

#[tokio::test]
async fn drift_without_state_dir_sees_real_drift() {
    let d = tempfile::tempdir().unwrap();
    let cfg = project(d.path());

    // File drift compares `details.content_hash` against the file at
    // `details.path` (tripwire::drift::check_file_resource_drift), so the lock
    // must carry BOTH — the shared `converged_lock` helper records only the
    // desired-state hash, which is enough for `plan` but cannot express drift.
    let target = d.path().join("f.txt");
    std::fs::write(&target, "hi").unwrap();
    let content_hash = crate::tripwire::hasher::hash_file(&target).unwrap();
    let md = d.path().join("state").join("local");
    std::fs::create_dir_all(&md).unwrap();
    std::fs::write(
        md.join("state.lock.yaml"),
        format!(
            "schema: \"1.0\"\nmachine: local\nhostname: localhost\ngenerated_at: now\n\
             generator: test\nblake3_version: \"1\"\nresources:\n  real:\n    type: file\n\
             \x20   status: converged\n    hash: \"h\"\n    details:\n      path: \"{}\"\n\
             \x20     content_hash: \"{}\"\n",
            target.display(),
            content_hash
        ),
    )
    .unwrap();

    // Tamper: the managed file no longer matches the recorded content hash.
    std::fs::write(&target, "TAMPERED").unwrap();

    let out = DriftHandler
        .handle(DriftInput {
            path: cfg.display().to_string(),
            state_dir: None,
            machine: None,
        })
        .await
        .expect("drift runs");

    assert!(
        out.drifted,
        "drift is the tripwire tool: reporting a tampered machine as clean \
         because the server's cwd is elsewhere is the worst outcome it has \
         (GH-208). findings={:?} unchecked={:?}",
        out.findings, out.unchecked
    );
}

/// FJQ: `forjar lint` and `forjar_lint` used to disagree about the same file.
///
/// `cli/lint.rs` dropped every `SC1*` diagnostic and every line inside a
/// heredoc body; `mcp/handlers.rs` dropped neither, and listed advisory
/// diagnostics the CLI only tallied. Same verb, two answers, and no test
/// compared them — `tests_parity.rs` covered plan and drift and never lint.
///
/// The assertion is on the RENDERED lines, not on a count: two surfaces can
/// agree on "7 findings" while describing different sevens.
#[tokio::test]
async fn lint_reports_the_same_findings_on_both_surfaces() {
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("forjar.yaml");
    // The Stripe-shaped fixture is assembled at runtime: the detector matches
    // `[sr]k_(live|test)_[A-Za-z0-9]{20,}`, and so does GitHub push protection,
    // which blocked a push of this repo over the literal form.
    let fake_key = format!("sk_{}_{}", "live", "A".repeat(24));
    std::fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: parity
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  leaky:
    type: file
    machine: local
    path: /etc/leaky.conf
    content: "api_key={fake_key}"
"#
        ),
    )
    .unwrap();

    let config = crate::core::parser::parse_and_validate(&cfg).unwrap();
    let text = std::fs::read_to_string(&cfg).unwrap();
    let cli_lines = crate::core::quality_gate::evaluate(
        &config,
        Some(&text),
        &crate::core::quality_gate::GateThresholds::default(),
    )
    .render();

    let out = LintHandler
        .handle(LintInput {
            path: cfg.display().to_string(),
            max_cyclomatic: None,
        })
        .await
        .expect("lint runs");

    assert!(!cli_lines.is_empty(), "the fixture must produce a finding");
    for line in &cli_lines {
        assert!(
            out.warnings.contains(line),
            "the MCP verb did not report a line the CLI reports: {line:?}\nmcp={:?}",
            out.warnings
        );
    }
    assert!(!out.gate_passed, "a plaintext API key must fail the gate");
    assert_eq!(out.error_count, 1, "findings={:?}", out.findings);
}
