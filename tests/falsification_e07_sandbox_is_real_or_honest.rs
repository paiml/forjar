//! FJ-410 (E07): Sandbox is Real or Honest
//!
//! WHAT WAS OBSERVABLY WRONG:
//! The `execute_sandbox_plan` implementation used a fake script with unrunnable shell commands
//! (e.g. `seccomp-bpf` without the binary, `nsenter --target $PID` with unbound PID).
//! It also faked the output hash by hashing string paths instead of the actual file contents.
//!
//! WHY THESE ASSERTIONS:
//! To satisfy the "honest subset" of the fix:
//! We assert that calling `execute_sandbox_plan` returns a clear "not implemented" error
//! detailing what is missing (delegation to namespace, hash_directory, atomic_move).
//!
//! WHAT WE LEFT OUT AND WHY:
//! We left out full delegation to `pepita` namespace, Rust-side `content_hash`, and
//! `atomic_move_to_store`. Full delegation requires wiring the I/O of sandbox creation,
//! execution tracking, and store atomic operations correctly across multiple modules,
//! which is risky to rush in a single refactor. Returning the honest error avoids giving
//! the user a false sense of security with fake hashes and fake execution logs.

use forjar::core::store::sandbox::{SandboxConfig, SandboxLevel};
use forjar::core::store::sandbox_exec::plan_sandbox_build;
use forjar::core::store::sandbox_run::execute_sandbox_plan;

use std::collections::BTreeMap;
use std::path::PathBuf;

#[test]
fn test_e07_execute_sandbox_plan_returns_honest_error() {
    // Gate on `unshare` availability
    let unshare_check = std::process::Command::new("which").arg("unshare").output();
    if unshare_check.is_err() || !unshare_check.unwrap().status.success() {
        println!("Skipping test: 'unshare' command is not available on this system");
        return;
    }

    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 512,
        cpus: 1.0,
        timeout: 30,
        bind_mounts: vec![],
        env: vec![],
    };

    let mut input_paths = BTreeMap::new();
    input_paths.insert("base".to_string(), PathBuf::from("/tmp/dummy-base"));

    let script = "echo hi > $out/x";
    let store_dir = PathBuf::from("/tmp/forjar-store");

    let plan = plan_sandbox_build(
        &config,
        "blake3:dummyhash",
        &input_paths,
        script,
        &store_dir,
    );

    let machine = forjar::core::types::Machine::ssh("localhost", "127.0.0.1", "root");

    let result = execute_sandbox_plan(&plan, script, &machine, &store_dir, Some(30));

    assert!(
        result.is_err(),
        "Expected execute_sandbox_plan to fail with honest error"
    );
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("not implemented") && err_msg.contains("sandbox execution"),
        "Expected honest error about missing sandbox execution, got: {}",
        err_msg
    );
}

/// The plan keeps its lifecycle, but a step whose mechanism does not exist on
/// any host carries no command and says so — no script that would fail at
/// runtime, no silent skip.
#[test]
fn the_plan_names_the_steps_it_cannot_run() {
    let config = forjar::core::store::sandbox::preset_profile("full").expect("full profile");
    let mut inputs = BTreeMap::new();
    inputs.insert("src".to_string(), PathBuf::from("/store/abc/content"));
    let plan = plan_sandbox_build(
        &config,
        "blake3:aabbcc",
        &inputs,
        "echo build",
        std::path::Path::new("/var/forjar/store"),
    );
    let unavailable: Vec<_> = plan
        .steps
        .iter()
        .filter(|s| s.description.contains("NOT EXECUTABLE"))
        .collect();
    assert_eq!(
        unavailable.len(),
        2,
        "seccomp-bpf and forjar-hash-dir are the two mechanisms that do not exist; got {:?}",
        plan.steps
            .iter()
            .map(|s| &s.description)
            .collect::<Vec<_>>()
    );
    for s in &unavailable {
        assert!(
            s.command.is_none(),
            "step {} still emits a command: {:?}",
            s.step,
            s.command
        );
    }
    for s in &plan.steps {
        if let Some(c) = &s.command {
            assert!(
                !c.contains("seccomp-bpf") && !c.contains("forjar-hash-dir"),
                "step {} invokes a binary that does not exist: {c}",
                s.step
            );
        }
    }
}
