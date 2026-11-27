//! #1294 regression tests for phony task convergence. Split out of
//! tests_task.rs, which is over the 500-line file-health limit.

use super::task::*;
use crate::core::types::{MachineTarget, Resource, ResourceType};

/// #1294: a phony task (from `forjar import-makefile`) names an action with no
/// artifact to observe. It runs on every explicit request and exits 0. Its
/// post-apply check must then report converged, not `task=pending`, or JIDOKA
/// flags a successful run as NOT CONVERGED.
#[test]
fn a_phony_task_with_nothing_to_observe_converges_after_its_run() {
    let r = Resource {
        resource_type: ResourceType::Task,
        machine: MachineTarget::Single("m1".to_string()),
        command: Some("true".to_string()),
        phony: true,
        ..Default::default()
    };
    let script = check_script(&r);
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(&script)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "phony check must pass:\n{script}\n{stdout}"
    );
    assert!(stdout.contains("task=completed"), "{stdout}");
    assert!(!stdout.contains("task=pending"), "{stdout}");
}
