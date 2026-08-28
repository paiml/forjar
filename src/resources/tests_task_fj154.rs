//! FJ-154 tests for the task handler — a config-derived name must never
//! escape the shell word it is interpolated into.
//!
//! Extracted from `task.rs` so the handler stays under the 500-line gate.

use super::task::*;
use crate::core::types::{MachineTarget, Resource, ResourceType, TaskMode};

fn service_resource(name: &str) -> Resource {
    Resource {
        resource_type: ResourceType::Task,
        machine: MachineTarget::Single("m1".to_string()),
        name: Some(name.to_string()),
        command: Some("/usr/bin/myd".to_string()),
        task_mode: Some(TaskMode::Service),
        ..Default::default()
    }
}

#[test]
fn fj154_service_name_slugified_in_log_path() {
    // Defect #13: a name with spaces/metachars must not split the log
    // redirect target. It is slugified to [A-Za-z0-9._-].
    let r = service_resource("x; rm -rf ~ #");
    let script = apply_script(&r);
    // Slug is "x--rm--rf"; the log redirect is a single quoted word.
    assert!(
        script.contains("> '/tmp/forjar-svc-x--rm--rf.log'"),
        "{script}"
    );
    // No bare `rm -rf ~` appears in the redirect target.
    assert!(!script.contains("/tmp/forjar-svc-x; rm -rf ~"), "{script}");
}

#[test]
fn fj154_service_log_and_pid_paths_consistent() {
    let r = service_resource("my svc");
    let apply = apply_script(&r);
    let check = check_script(&r);
    // Both apply and check agree on the slugified pidfile.
    assert!(apply.contains("'/tmp/forjar-svc-my-svc.pid'"), "{apply}");
    assert!(check.contains("'/tmp/forjar-svc-my-svc.pid'"), "{check}");
    assert!(apply.contains("'/tmp/forjar-svc-my-svc.log'"), "{apply}");
}

#[test]
fn fj154_service_benign_name_unchanged() {
    let r = service_resource("web");
    let script = apply_script(&r);
    assert!(script.contains("> '/tmp/forjar-svc-web.log'"), "{script}");
    assert!(script.contains("'/tmp/forjar-svc-web.pid'"), "{script}");
}

#[test]
fn fj154_output_artifacts_quoted() {
    let mut r = service_resource("t");
    r.task_mode = None;
    r.output_artifacts = vec!["/out/x';id;'".to_string()];
    let q = state_query_script(&r);
    assert!(q.contains("'\"'\"'"), "{q}");
    assert!(!q.contains("b3sum '/out/x';id"), "{q}");
}
