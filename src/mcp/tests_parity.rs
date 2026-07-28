//! FJ-2729 (PMAT-199): the MCP tools must agree with the CLI they mirror.
//!
//! Both defects below were found by driving the PUBLISHED 1.12.0 binary over
//! stdio and comparing each tool against its CLI counterpart on the same
//! project. Neither was visible from the schema, from `tools/list`, or from a
//! handler test that only asserted "returns Ok".

use super::handlers::*;
use super::types::*;
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
            state_dir: Some(d.path().join("state").display().to_string()),
            machine: None,
        })
        .await
        .expect("status runs");

    assert!(out.machines.is_empty(), "{:?}", out.machines);
}
