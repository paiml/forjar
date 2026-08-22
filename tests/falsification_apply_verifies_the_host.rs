//! `apply` must ask the HOST before writing Converged.
//!
//! THE FLAW THIS CLOSES.
//!
//! forjar's executor defined "converged" as
//! `hash(config_now) == hash(config_at_last_apply)` — a statement about the
//! lock file, never about the machine. `apply` was the only verb that never
//! asked a question: `check_script` had 16 call sites and not one was on the
//! apply path (it was reachable only through opt-in `--refresh`).
//!
//! So whatever proxy a resource author picked became the system's definition of
//! reality, and the lock laundered it into a permanent claim. Measured
//! 2026-08-19: mount's apply guarded on `mountpoint -q <path>`, so changing the
//! declared `source` reported `1 converged` on two hosts that both kept the old
//! share mounted.
//!
//! This is Terraform's `AssertObjectCompatible` in miniature — after apply, core
//! asks the host whether the declared state is there. A resource does not get to
//! answer on its own behalf.
//!
//! WHAT THIS TEST MUST NOT BECOME. The verification is only as honest as the
//! check it runs, so a test that stubs a check which always passes would go
//! green while proving nothing. Every case here therefore drives the verdict
//! from a REAL host condition that the test controls.

use forjar::core::executor::output_verify;
use forjar::core::types::{Machine, MachineTarget, Resource, ResourceType};
use std::fs;

fn localhost() -> Machine {
    Machine {
        hostname: "localhost".into(),
        addr: "localhost".into(),
        user: whoami(),
        arch: "x86_64".into(),
        ssh_key: None,
        roles: vec![],
        transport: Some("local".into()),
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

fn whoami() -> String {
    std::env::var("USER").unwrap_or_else(|_| "root".into())
}

/// A `file` resource: its check asserts the path exists, which the test controls
/// by creating or not creating it. No stubbing — a real condition on a real host.
fn file_at(path: &std::path::Path) -> Resource {
    Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("localhost".into()),
        path: Some(path.to_string_lossy().into_owned()),
        content: Some("declared".into()),
        ..Default::default()
    }
}

#[test]
fn a_host_that_does_not_have_the_declared_state_is_not_converged() {
    // THE REGRESSION. Apply "succeeded" (we do not run it), but the host does
    // not have the file. Before this change the executor wrote Converged into
    // the lock on the strength of an exit code alone.
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("never-created");

    let err = output_verify::verify_against_host(&file_at(&missing), &localhost());

    assert!(
        err.is_some(),
        "the declared file is absent on the host, so apply must NOT be allowed \
         to report converged. This returning None is exactly the silence that \
         let a wrong mount report `1 converged` on two machines."
    );
    let msg = err.unwrap();
    assert!(
        msg.contains("does not report the declared state"),
        "the failure must say the HOST disagrees, distinguishable from a command \
         error: {msg}"
    );
}

#[test]
fn a_host_that_does_have_the_declared_state_is_converged() {
    // The gate must be passable, or it trains people to set FORJAR_VERIFY=warn
    // and never unset it.
    let dir = tempfile::tempdir().unwrap();
    let present = dir.path().join("created");
    fs::write(&present, "declared").unwrap();

    assert!(
        output_verify::verify_against_host(&file_at(&present), &localhost()).is_none(),
        "the declared file exists on the host; verification must pass"
    );
}

#[test]
fn the_escape_hatch_is_a_separate_policy_and_is_currently_off() {
    // FORJAR_VERIFY=warn suppresses the gate entirely, so its state is worth
    // pinning: if this ever reads false in normal runs, verification has quietly
    // become permanent-warn and every assertion above is inert.
    //
    // The policy is a separate function precisely so it can be checked without
    // mutating process env — this crate forbids `unsafe`, and `set_var` is
    // unsafe in edition 2024. That constraint pushed the hatch out of the
    // logic, which is the better shape anyway.
    assert!(
        output_verify::verification_enabled(),
        "post-apply verification must be ON by default; FORJAR_VERIFY=warn is a \
         migration hatch with a removal date, not a setting"
    );
}
