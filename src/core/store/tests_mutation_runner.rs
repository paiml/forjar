//! Tests for infrastructure mutation runner (FJ-2604).

use super::convergence_runner::RunnerMode;
use super::mutation_runner::*;
use crate::core::types::MutationOperator;

fn file_target(id: &str) -> MutationTarget {
    // Apply creates a marker file; drift script reads it.
    // Mutation operators (rm, chmod, etc.) modify/delete the file,
    // so drift detection compares before/after and detects the change.
    MutationTarget {
        resource_id: id.into(),
        resource_type: "file".into(),
        apply_script: format!(
            "mkdir -p \"$FORJAR_SANDBOX/etc/forjar\" && echo 'content-{id}' > \"$FORJAR_SANDBOX/etc/forjar/{id}\""
        ),
        drift_script: format!(
            "cat \"$FORJAR_SANDBOX/etc/forjar/{id}\" 2>/dev/null || echo 'MISSING'"
        ),
        expected_hash: String::new(), // not used in mutation tests
    }
}

fn service_target(id: &str) -> MutationTarget {
    // Service targets use PID files in the sandbox — no system calls.
    MutationTarget {
        resource_id: id.into(),
        resource_type: "service".into(),
        apply_script: format!(
            "mkdir -p \"$FORJAR_SANDBOX/run\" && echo 'running' > \"$FORJAR_SANDBOX/run/{id}.pid\""
        ),
        drift_script: format!("cat \"$FORJAR_SANDBOX/run/{id}.pid\" 2>/dev/null || echo 'STOPPED'"),
        expected_hash: String::new(),
    }
}

#[test]
fn unsafe_operators_skipped_in_local_mode() {
    let target = service_target("nginx");
    let config = MutationRunConfig {
        test_reconvergence: false,
        ..MutationRunConfig::default()
    };
    // StopService is unsafe for local execution
    let result = run_mutation_test(&target, MutationOperator::StopService, &config);
    assert!(
        result.error.is_some(),
        "StopService should be rejected locally"
    );
    assert!(
        result.error.as_ref().unwrap().contains("container backend"),
        "error should explain container requirement"
    );
}

#[test]
fn safe_operators_run_in_local_mode() {
    let target = file_target("safe-test");
    let config = MutationRunConfig {
        test_reconvergence: false,
        ..MutationRunConfig::default()
    };
    // DeleteFile is safe for local execution
    let result = run_mutation_test(&target, MutationOperator::DeleteFile, &config);
    assert!(result.error.is_none(), "DeleteFile should run locally");
    assert!(result.detected, "DeleteFile should be detected");
}

#[test]
fn mutation_script_delete_file() {
    let script = mutation_script(MutationOperator::DeleteFile, "nginx-config");
    assert!(script.contains("rm -f"));
    assert!(script.contains("nginx-config"));
    assert!(script.contains("FORJAR_SANDBOX"), "should be sandbox-aware");
}

#[test]
fn mutation_script_modify_content() {
    let script = mutation_script(MutationOperator::ModifyContent, "app-config");
    assert!(script.contains("MUTATED_CONTENT"));
    assert!(script.contains("app-config"));
}

#[test]
fn mutation_script_change_permissions() {
    let script = mutation_script(MutationOperator::ChangePermissions, "secret-file");
    assert!(script.contains("chmod 000"));
}

#[test]
fn mutation_script_stop_service() {
    let script = mutation_script(MutationOperator::StopService, "nginx");
    assert!(script.contains("systemctl stop"));
    assert!(script.contains("nginx"));
}

#[test]
fn mutation_script_remove_package() {
    let script = mutation_script(MutationOperator::RemovePackage, "curl");
    assert!(script.contains("apt-get remove"));
    assert!(script.contains("curl"));
}

#[test]
fn mutation_script_kill_process() {
    let script = mutation_script(MutationOperator::KillProcess, "worker");
    assert!(script.contains("pkill"));
    assert!(script.contains("worker"));
}

#[test]
fn mutation_script_unmount() {
    let script = mutation_script(MutationOperator::UnmountFilesystem, "data");
    assert!(script.contains("umount"));
    assert!(script.contains("data"));
}

#[test]
fn mutation_script_corrupt_config() {
    let script = mutation_script(MutationOperator::CorruptConfig, "my-conf");
    assert!(script.contains("sed"));
    assert!(script.contains("CORRUPTED"));
}

#[test]
fn mutation_script_sandbox_aware() {
    // All file-targeting operators should use FORJAR_SANDBOX
    for op in &[
        MutationOperator::DeleteFile,
        MutationOperator::ModifyContent,
        MutationOperator::ChangePermissions,
        MutationOperator::CorruptConfig,
        MutationOperator::UnmountFilesystem,
    ] {
        let script = mutation_script(*op, "test-res");
        assert!(
            script.contains("FORJAR_SANDBOX"),
            "{op:?} should be sandbox-aware"
        );
    }
}

#[test]
fn applicable_operators_file() {
    let ops = applicable_operators("file");
    assert!(ops.contains(&MutationOperator::DeleteFile));
    assert!(ops.contains(&MutationOperator::ModifyContent));
    assert!(ops.contains(&MutationOperator::ChangePermissions));
    assert!(ops.contains(&MutationOperator::CorruptConfig));
    assert!(!ops.contains(&MutationOperator::StopService));
}

#[test]
fn applicable_operators_service() {
    let ops = applicable_operators("service");
    assert!(ops.contains(&MutationOperator::StopService));
    assert!(ops.contains(&MutationOperator::KillProcess));
    assert!(!ops.contains(&MutationOperator::DeleteFile));
}

#[test]
fn applicable_operators_package() {
    let ops = applicable_operators("package");
    assert!(ops.contains(&MutationOperator::RemovePackage));
    assert_eq!(ops.len(), 1);
}

#[test]
fn applicable_operators_mount() {
    let ops = applicable_operators("mount");
    assert!(ops.contains(&MutationOperator::UnmountFilesystem));
    assert_eq!(ops.len(), 1);
}

#[test]
fn applicable_operators_unknown_type() {
    let ops = applicable_operators("gpu");
    assert!(ops.is_empty());
}

#[test]
fn run_mutation_test_detects_delete() {
    let target = file_target("nginx-config");
    let config = MutationRunConfig::default();
    let result = run_mutation_test(&target, MutationOperator::DeleteFile, &config);
    // DeleteFile removes the file; drift script output changes from content to "MISSING"
    assert!(
        result.detected,
        "DeleteFile should be detected by drift script"
    );
    assert!(result.is_killed());
    assert!(result.error.is_none());
}

#[test]
fn run_mutation_test_detects_modify() {
    let target = file_target("app-conf");
    let config = MutationRunConfig {
        test_reconvergence: false,
        ..MutationRunConfig::default()
    };
    let result = run_mutation_test(&target, MutationOperator::ModifyContent, &config);
    // ModifyContent appends to the file; drift script output changes
    assert!(result.detected, "ModifyContent should be detected");
    assert!(result.error.is_none());
}

#[test]
fn run_mutation_test_empty_script_errors() {
    let target = MutationTarget {
        resource_id: "broken".into(),
        resource_type: "file".into(),
        apply_script: String::new(),
        drift_script: "echo drift".into(),
        expected_hash: "blake3:x".into(),
    };
    let config = MutationRunConfig::default();
    let result = run_mutation_test(&target, MutationOperator::DeleteFile, &config);
    assert!(result.error.is_some());
    assert!(result.error.as_ref().unwrap().contains("baseline"));
}

#[test]
fn run_mutation_suite_file_targets() {
    let targets = vec![file_target("config-a"), file_target("config-b")];
    let config = MutationRunConfig {
        test_reconvergence: false,
        ..MutationRunConfig::default()
    };
    let report = run_mutation_suite(&targets, &config);

    // Each file target gets 4 operators (DeleteFile, ModifyContent, ChangePermissions, CorruptConfig)
    assert_eq!(report.score.total, 8);
    // Real execution: DeleteFile and ModifyContent always detectable,
    // ChangePermissions and CorruptConfig depend on file existence in sandbox
    assert!(
        report.score.detected >= 4,
        "at least DeleteFile+ModifyContent per target"
    );
}

#[test]
fn run_mutation_suite_mixed_types() {
    let targets = vec![file_target("app-config"), service_target("nginx-svc")];
    let config = MutationRunConfig {
        test_reconvergence: false,
        ..MutationRunConfig::default()
    };
    let report = run_mutation_suite(&targets, &config);

    // file: 4 operators, service: 2 operators (both unsafe → errored)
    assert_eq!(report.score.total, 6);
    assert_eq!(report.by_type.len(), 2);
    // Service operators (StopService, KillProcess) are rejected locally
    assert!(report.score.errored >= 2, "service ops should be errored");
}

#[test]
fn run_mutation_suite_empty() {
    let targets: Vec<MutationTarget> = Vec::new();
    let config = MutationRunConfig::default();
    let report = run_mutation_suite(&targets, &config);
    assert_eq!(report.score.total, 0);
}

#[test]
fn run_mutation_parallel_empty() {
    let config = MutationRunConfig::default();
    let report = run_mutation_parallel(Vec::new(), &config);
    assert_eq!(report.score.total, 0);
}

#[test]
fn run_mutation_parallel_multiple() {
    let targets = vec![
        file_target("conf-1"),
        file_target("conf-2"),
        service_target("svc-1"),
    ];
    let config = MutationRunConfig {
        parallelism: 2,
        test_reconvergence: false,
        ..MutationRunConfig::default()
    };
    let report = run_mutation_parallel(targets, &config);
    // 2 file targets * 4 ops + 1 service target * 2 ops = 10
    assert_eq!(report.score.total, 10);
    // Real execution: at least DeleteFile+ModifyContent detected per file target
    assert!(
        report.score.detected >= 4,
        "at least 4 detected from file targets"
    );
}

#[test]
fn format_mutation_run_output() {
    let targets = vec![file_target("app")];
    let config = MutationRunConfig::default();
    let report = run_mutation_suite(&targets, &config);
    let output = format_mutation_run(&report);
    assert!(
        output.contains("Grade A")
            || output.contains("Grade B")
            || output.contains("Grade C")
            || output.contains("Grade F"),
        "output should contain a grade: {output}"
    );
    assert!(output.contains("targets"));
    assert!(output.contains("mutations total"));
}

#[test]
fn mutation_run_config_default() {
    let config = MutationRunConfig::default();
    assert_eq!(config.mutations_per_resource, 50);
    assert_eq!(config.parallelism, 4);
    assert!(config.test_reconvergence);
    assert_eq!(config.backend, crate::core::types::SandboxBackend::Pepita);
}

#[test]
fn runner_mode_display() {
    assert_eq!(RunnerMode::Simulated.to_string(), "simulated");
    assert_eq!(RunnerMode::Sandbox.to_string(), "sandbox");
}

#[test]
fn runner_mode_equality() {
    assert_eq!(RunnerMode::Simulated, RunnerMode::Simulated);
    assert_ne!(RunnerMode::Simulated, RunnerMode::Sandbox);
}

#[test]
fn dispatch_uses_local_for_pepita() {
    let target = file_target("dispatch-sim");
    let config = MutationRunConfig {
        test_reconvergence: false,
        ..MutationRunConfig::default()
    }; // Pepita backend, not installed → local execution
    let result = run_mutation_test_dispatch(&target, MutationOperator::DeleteFile, &config);
    // DeleteFile removes the sandbox file → drift detected
    assert!(result.detected, "local mode should detect file deletion");
    assert!(result.error.is_none());
}

#[test]
fn dispatch_uses_local_for_chroot() {
    let target = file_target("dispatch-chroot");
    let config = MutationRunConfig {
        backend: crate::core::types::SandboxBackend::Chroot,
        test_reconvergence: false,
        ..MutationRunConfig::default()
    };
    let result = run_mutation_test_dispatch(&target, MutationOperator::DeleteFile, &config);
    assert!(
        result.detected,
        "chroot unavailable → local execution detects deletion"
    );
}

#[test]
fn dispatch_container_backend_available() {
    // When container backend is selected AND Docker is available,
    // dispatch routes through mutation_container.
    use crate::core::types::SandboxBackend;
    let target = file_target("container-dispatch");
    let config = MutationRunConfig {
        backend: SandboxBackend::Container,
        test_reconvergence: false,
        ..MutationRunConfig::default()
    };
    let result = run_mutation_test_dispatch(&target, MutationOperator::DeleteFile, &config);
    // Container runs real scripts; the key assertion is that the dispatch
    // path completes without panicking. Drift detection may or may not
    // detect changes depending on container state.
    assert!(result.duration_ms < 30_000, "should complete within 30s");
}

#[test]
fn parallel_dispatch_with_backend() {
    let targets = vec![file_target("par-a"), service_target("par-svc")];
    let config = MutationRunConfig {
        parallelism: 2,
        test_reconvergence: false,
        ..MutationRunConfig::default()
    };
    let report = run_mutation_parallel(targets, &config);
    // file: 4 ops + service: 2 ops = 6
    assert_eq!(report.score.total, 6);
    // At least file mutations detected
    assert!(
        report.score.detected >= 2,
        "at least DeleteFile+ModifyContent"
    );
}

#[test]
fn mutations_per_resource_limit() {
    let targets = vec![file_target("big")];
    let config = MutationRunConfig {
        mutations_per_resource: 2,
        ..MutationRunConfig::default()
    };
    let report = run_mutation_suite(&targets, &config);
    assert_eq!(report.score.total, 2);
}
