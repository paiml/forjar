//! Tests for infrastructure mutation runner (FJ-2604).

use super::mutation_runner::*;
use crate::core::types::MutationOperator;

fn file_target(id: &str) -> MutationTarget {
    MutationTarget {
        resource_id: id.into(),
        resource_type: "file".into(),
        apply_script: format!("echo 'apply {id}'"),
        drift_script: format!("echo 'drift {id}'"),
        expected_hash: "blake3:expected".into(),
    }
}

fn service_target(id: &str) -> MutationTarget {
    MutationTarget {
        resource_id: id.into(),
        resource_type: "service".into(),
        apply_script: format!("systemctl start {id}"),
        drift_script: format!("systemctl is-active {id}"),
        expected_hash: "blake3:expected".into(),
    }
}

#[test]
fn mutation_script_delete_file() {
    let script = mutation_script(MutationOperator::DeleteFile, "nginx-config");
    assert!(script.contains("rm -f"));
    assert!(script.contains("nginx-config"));
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
fn run_mutation_test_detects() {
    let target = file_target("nginx-config");
    let config = MutationRunConfig::default();
    let result = run_mutation_test(&target, MutationOperator::DeleteFile, &config);
    assert!(result.detected);
    assert!(result.is_killed());
    assert!(result.reconverged.unwrap_or(false));
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
    let targets = vec![
        file_target("config-a"),
        file_target("config-b"),
    ];
    let config = MutationRunConfig::default();
    let report = run_mutation_suite(&targets, &config);

    // Each file target gets 4 operators (DeleteFile, ModifyContent, ChangePermissions, CorruptConfig)
    assert_eq!(report.score.total, 8);
    assert_eq!(report.score.detected, 8);
    assert_eq!(report.score.grade(), 'A');
}

#[test]
fn run_mutation_suite_mixed_types() {
    let targets = vec![
        file_target("app-config"),
        service_target("nginx"),
    ];
    let config = MutationRunConfig::default();
    let report = run_mutation_suite(&targets, &config);

    // file: 4 operators, service: 2 operators
    assert_eq!(report.score.total, 6);
    assert_eq!(report.by_type.len(), 2);
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
        ..MutationRunConfig::default()
    };
    let report = run_mutation_parallel(targets, &config);
    // 2 file targets * 4 ops + 1 service target * 2 ops = 10
    assert_eq!(report.score.total, 10);
    assert_eq!(report.score.detected, 10);
}

#[test]
fn format_mutation_run_output() {
    let targets = vec![file_target("app")];
    let config = MutationRunConfig::default();
    let report = run_mutation_suite(&targets, &config);
    let output = format_mutation_run(&report);
    assert!(output.contains("Grade A"));
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
fn mutations_per_resource_limit() {
    let targets = vec![file_target("big")];
    let config = MutationRunConfig {
        mutations_per_resource: 2,
        ..MutationRunConfig::default()
    };
    let report = run_mutation_suite(&targets, &config);
    assert_eq!(report.score.total, 2);
}
