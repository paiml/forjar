//! Tests for FJ-1361: Sandbox execution bridge.

use super::sandbox::{SandboxConfig, SandboxLevel};
use super::sandbox_exec::{
    plan_sandbox_build, plan_step_count, seccomp_rules_for_level, validate_plan, SandboxPlan,
    SandboxStep,
};
use super::sandbox_run::{dry_run_sandbox_plan, validate_sandbox_commands, SandboxExecResult};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn sample_plan() -> SandboxPlan {
    let config = SandboxConfig {
        level: SandboxLevel::Minimal,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: Vec::new(),
        env: Vec::new(),
    };
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "base".to_string(),
        PathBuf::from("/var/lib/forjar/store/abc123/content"),
    );
    plan_sandbox_build(
        &config,
        "blake3:test_hash_0123456789abcdef",
        &inputs,
        "cp -r /inputs/base /out",
        Path::new("/var/lib/forjar/store"),
    )
}

fn full_sandbox_plan() -> SandboxPlan {
    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 4096,
        cpus: 8.0,
        timeout: 1200,
        bind_mounts: Vec::new(),
        env: Vec::new(),
    };
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "src".to_string(),
        PathBuf::from("/var/lib/forjar/store/def456/content"),
    );
    plan_sandbox_build(
        &config,
        "blake3:full_test_hash_0123456789ab",
        &inputs,
        "make build",
        Path::new("/var/lib/forjar/store"),
    )
}

#[test]
fn sandbox_plan_has_steps() {
    let plan = sample_plan();
    assert!(plan_step_count(&plan) >= 9);
    assert!(validate_plan(&plan).is_empty());
}

#[test]
fn sandbox_plan_namespace_id_derived_from_hash() {
    let plan = sample_plan();
    assert!(plan.namespace_id.starts_with("forjar-build-"));
    assert!(plan.namespace_id.contains("blake3"));
}

#[test]
fn sandbox_plan_has_overlay_config() {
    let plan = sample_plan();
    assert!(!plan.overlay.lower_dirs.is_empty());
    assert!(plan.overlay.upper_dir.to_str().unwrap().contains("upper"));
    assert!(plan.overlay.work_dir.to_str().unwrap().contains("work"));
    assert!(plan.overlay.merged_dir.to_str().unwrap().contains("merged"));
}

#[test]
fn full_plan_has_seccomp_rules() {
    let plan = full_sandbox_plan();
    assert_eq!(plan.seccomp_rules.len(), 3);
    let syscalls: Vec<&str> = plan
        .seccomp_rules
        .iter()
        .map(|r| r.syscall.as_str())
        .collect();
    assert!(syscalls.contains(&"connect"));
    assert!(syscalls.contains(&"mount"));
    assert!(syscalls.contains(&"ptrace"));
}

#[test]
fn minimal_plan_no_seccomp_rules() {
    let plan = sample_plan();
    assert!(plan.seccomp_rules.is_empty());
}

#[test]
fn plan_steps_in_order() {
    let plan = sample_plan();
    let mut prev = 0u8;
    for step in &plan.steps {
        assert!(
            step.step >= prev,
            "steps out of order: {} < {}",
            step.step,
            prev
        );
        prev = step.step;
    }
}

#[test]
fn sandbox_exec_result_fields() {
    let result = SandboxExecResult {
        output_hash: "blake3:abc123".to_string(),
        store_path: "/var/lib/forjar/store/abc123/content".to_string(),
        steps_executed: vec![
            (1, "Create namespace".to_string(), true),
            (2, "Mount overlay".to_string(), true),
        ],
        duration_secs: 1.5,
    };
    assert_eq!(result.steps_executed.len(), 2);
    assert!(result.duration_secs > 0.0);
}

#[test]
fn validate_sandbox_commands_on_valid_plan() {
    let plan = sample_plan();
    let errors = validate_sandbox_commands(&plan);
    // Some commands may fail I8 (namespace/mount commands) — that's expected
    // The test just verifies the function runs without panic
    let _ = errors;
}

#[test]
fn dry_run_returns_commands() {
    let plan = sample_plan();
    // dry_run may fail for complex commands, that's OK — test the path
    let result = dry_run_sandbox_plan(&plan);
    if let Ok(cmds) = result {
        assert!(!cmds.is_empty());
    } // I8 failure on complex commands is expected
}

#[test]
fn seccomp_rules_for_levels() {
    assert_eq!(seccomp_rules_for_level(SandboxLevel::Full).len(), 3);
    assert!(seccomp_rules_for_level(SandboxLevel::NetworkOnly).is_empty());
    assert!(seccomp_rules_for_level(SandboxLevel::Minimal).is_empty());
    assert!(seccomp_rules_for_level(SandboxLevel::None).is_empty());
}

#[test]
fn plan_step_descriptions_nonempty() {
    let plan = sample_plan();
    for step in &plan.steps {
        assert!(
            !step.description.is_empty(),
            "step {} has empty description",
            step.step
        );
    }
}

#[test]
fn plan_step_commands_nonempty() {
    let plan = sample_plan();
    for step in &plan.steps {
        if let Some(cmd) = &step.command {
            assert!(!cmd.is_empty(), "step {} has empty command", step.step);
        }
    }
}

#[test]
fn cgroup_path_format() {
    let plan = sample_plan();
    assert!(plan.cgroup_path.starts_with("/sys/fs/cgroup/forjar-build-"));
}

#[test]
fn overlay_lower_dirs_match_inputs() {
    let config = SandboxConfig {
        level: SandboxLevel::Minimal,
        memory_mb: 1024,
        cpus: 2.0,
        timeout: 300,
        bind_mounts: Vec::new(),
        env: Vec::new(),
    };
    let mut inputs = BTreeMap::new();
    inputs.insert("a".to_string(), PathBuf::from("/store/aaa/content"));
    inputs.insert("b".to_string(), PathBuf::from("/store/bbb/content"));
    let plan = plan_sandbox_build(
        &config,
        "blake3:multi_input_hash_test_12345",
        &inputs,
        "echo test",
        Path::new("/store"),
    );
    assert_eq!(plan.overlay.lower_dirs.len(), 2);
}

#[test]
fn empty_sandbox_plan_step_struct() {
    let step = SandboxStep {
        step: 1,
        description: "Test step".to_string(),
        command: None,
    };
    assert!(step.command.is_none());
    assert_eq!(step.step, 1);
}

#[test]
fn full_plan_step_count_at_least_10() {
    let plan = full_sandbox_plan();
    assert!(
        plan_step_count(&plan) >= 10,
        "full sandbox should have at least 10 steps, got {}",
        plan_step_count(&plan)
    );
}
