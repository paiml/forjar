//! GH-254: a task must not report converged without reaching its declared state.
//!
//! `completion_check` was consulted only as a guard on whether to RUN the
//! command. It was never re-evaluated afterwards, so `converged` meant "the
//! command exited 0", not "the resource is in the state it declares". The lock
//! then recorded success and the next `plan` reported `no changes` over a host
//! that had never converged.
//!
//! This is not hypothetical. On paiml/infra's `lean-toolchain`, `sudo: true`
//! made `$HOME=/root`, so the Lean toolchain installed where the runner user
//! could not read it. Every command in the script succeeded. `forjar apply`
//! reported `1 converged, 0 failed`. `command -v lean` failed immediately
//! afterwards.
//!
//! The distinction these tests pin is between two different failures that
//! previously looked identical — except that the second looked like a success:
//!
//!   * the command errored                      -> already reported as failure
//!   * the command ran and achieved nothing     -> reported as CONVERGED
//!
//! The emitted script is EXECUTED here rather than pattern-matched. A
//! `script.contains("completion_check")` assertion would pass on a script that
//! never runs the check, which is the class of test that let the original
//! defect through.

use forjar::core::types::{MachineTarget, Resource, ResourceType};
use std::process::Command;

fn task(command: &str, completion_check: Option<&str>) -> Resource {
    Resource {
        resource_type: ResourceType::Task,
        machine: MachineTarget::Single("local".to_string()),
        command: Some(command.to_string()),
        completion_check: completion_check.map(str::to_string),
        ..Default::default()
    }
}

/// Run an emitted apply script under bash and return its exit status.
fn run(script: &str) -> std::process::Output {
    Command::new("bash")
        .arg("-c")
        .arg(script)
        .output()
        .expect("bash must run")
}

#[test]
fn a_command_that_succeeds_without_converging_fails() {
    // THE REGRESSION. `true` exits 0 and achieves nothing; `false` is a
    // completion_check that can never hold. Before GH-254 this emitted a script
    // that exited 0, and forjar reported the resource converged.
    let script = forjar::resources::task::apply_script(&task("true", Some("false")));
    let out = run(&script);

    assert!(
        !out.status.success(),
        "a task whose completion_check still fails must NOT converge.\n\
         script:\n{script}"
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not-converged"),
        "the failure must say the declared state was not reached, so it is \
         distinguishable from a command error: {stderr}"
    );
}

#[test]
fn a_command_that_genuinely_converges_still_passes() {
    // The gate must be passable, or it trains people to delete it. Here the
    // command actually produces the condition the check tests.
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("done");
    let script = forjar::resources::task::apply_script(&task(
        &format!("touch {}", marker.display()),
        Some(&format!("test -f {}", marker.display())),
    ));

    let out = run(&script);
    assert!(
        out.status.success(),
        "a task that reaches its declared state must converge.\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(marker.exists());
}

#[test]
fn a_failing_command_is_still_reported_as_a_command_failure() {
    // The two failures must stay distinguishable. A command that errors should
    // NOT be relabelled as a convergence problem — that would trade one
    // misleading report for another.
    let script = forjar::resources::task::apply_script(&task("exit 3", Some("true")));
    let out = run(&script);

    assert!(!out.status.success(), "a failing command must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not-converged"),
        "a command error must not be reported as a convergence failure: {stderr}"
    );
}

#[test]
fn a_task_without_a_completion_check_is_unaffected() {
    // No check declared means nothing to verify. This must not become a way to
    // fail tasks that never made the claim in the first place.
    let script = forjar::resources::task::apply_script(&task("true", None));
    let out = run(&script);
    assert!(
        out.status.success(),
        "a task with no completion_check must behave as before.\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn the_check_runs_after_the_command_not_before() {
    // Ordering is the whole point. If the verification ran first it would test
    // the pre-command state and pass vacuously for any task whose check
    // happened to hold already — which is exactly the guard semantics being
    // fixed. Here the command CREATES the condition, so a check evaluated
    // before it would fail.
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("created-by-the-command");
    assert!(!marker.exists());

    let script = forjar::resources::task::apply_script(&task(
        &format!("touch {}", marker.display()),
        Some(&format!("test -f {}", marker.display())),
    ));
    let out = run(&script);

    assert!(
        out.status.success(),
        "the verification must run AFTER the command, or a task that creates \
         its own precondition can never pass.\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
