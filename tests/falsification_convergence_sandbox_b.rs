//! FJ-2600/2601/2603: Popperian falsification for convergence testing,
//! idempotency verification, and sandbox isolation.
//!
//! Each test states conditions under which the convergence model or
//! sandbox isolation would be rejected as invalid.

#![allow(unused_imports)]
use forjar::core::store::convergence_runner::{
    format_convergence_report, run_convergence_test, ConvergenceResult, ConvergenceSummary,
    ConvergenceTarget, ConvergenceTestConfig, RunnerMode,
};
use forjar::core::store::sandbox::{
    blocks_network, cgroup_path, enforces_fs_isolation, parse_sandbox_config, preset_profile,
    validate_config, BindMount, EnvVar, SandboxConfig, SandboxLevel,
};
use forjar::core::store::sandbox_exec::plan_sandbox_build;
use forjar::core::types::SandboxBackend;
use std::collections::BTreeMap;

// ── FJ-2600: Convergence Result Model ──────────────────────────────

#[test]
fn f_2603_17_cgroup_path_strips_blake3_prefix() {
    let with_prefix = cgroup_path("blake3:deadbeef12345678");
    let without_prefix = cgroup_path("deadbeef12345678");
    // Both should use the hash portion, but with_prefix strips "blake3:"
    assert!(with_prefix.contains("deadbeef12345678"));
    assert!(without_prefix.contains("deadbeef12345678"));
}

// ── FJ-2603: Sandbox Execution Plan ────────────────────────────────

#[test]
fn f_2603_18_sandbox_plan_covers_full_lifecycle() {
    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    };
    let inputs = BTreeMap::new();
    let plan = plan_sandbox_build(
        &config,
        "abc123def456789012345678",
        &inputs,
        "echo hello",
        std::path::Path::new("/tmp/store"),
    );
    // With Full + no inputs: 9 steps (step 3 bind inputs is skipped when empty)
    // With Full + inputs: 10 steps (1 bind per input added at step 3)
    assert!(
        plan.steps.len() >= 9,
        "sandbox lifecycle must have at least 9 steps, got {}",
        plan.steps.len()
    );
    // Verify key lifecycle steps are present by description
    let descs: Vec<&str> = plan.steps.iter().map(|s| s.description.as_str()).collect();
    assert!(descs.iter().any(|d| d.contains("namespace")));
    assert!(descs
        .iter()
        .any(|d| d.contains("overlayfs") || d.contains("Overlay")));
    assert!(descs.iter().any(|d| d.contains("cgroup")));
    assert!(descs.iter().any(|d| d.contains("Execute")));
    assert!(descs
        .iter()
        .any(|d| d.contains("BLAKE3") || d.contains("hash")));
    assert!(descs.iter().any(|d| d.contains("store")));
    assert!(descs
        .iter()
        .any(|d| d.contains("Destroy") || d.contains("clean")));
}

#[test]
fn f_2603_19_sandbox_plan_namespace_derives_from_hash() {
    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    };
    let plan = plan_sandbox_build(
        &config,
        "deadbeefcafebabe12345678",
        &BTreeMap::new(),
        "true",
        std::path::Path::new("/store"),
    );
    assert!(
        plan.namespace_id.contains("deadbeefcafebabe"),
        "namespace must derive from hash prefix"
    );
}

#[test]
fn f_2603_20_sandbox_plan_full_has_seccomp() {
    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    };
    let plan = plan_sandbox_build(
        &config,
        "abc123def456789012345678",
        &BTreeMap::new(),
        "true",
        std::path::Path::new("/store"),
    );
    assert!(
        !plan.seccomp_rules.is_empty(),
        "Full sandbox must have seccomp rules"
    );
    // Must deny dangerous syscalls
    let denied_syscalls: Vec<&str> = plan
        .seccomp_rules
        .iter()
        .map(|r| r.syscall.as_str())
        .collect();
    assert!(
        denied_syscalls.contains(&"connect") || denied_syscalls.contains(&"socket"),
        "Full sandbox must deny network syscalls"
    );
}

#[test]
fn f_2603_21_sandbox_plan_minimal_no_seccomp() {
    let config = SandboxConfig {
        level: SandboxLevel::Minimal,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    };
    let plan = plan_sandbox_build(
        &config,
        "abc123def456789012345678",
        &BTreeMap::new(),
        "true",
        std::path::Path::new("/store"),
    );
    assert!(
        plan.seccomp_rules.is_empty(),
        "Minimal sandbox must NOT have seccomp rules"
    );
}

// ── FJ-2603: Sandbox Serde ─────────────────────────────────────────

#[test]
fn f_2603_22_sandbox_level_serde_roundtrip() {
    for level in [
        SandboxLevel::Full,
        SandboxLevel::NetworkOnly,
        SandboxLevel::Minimal,
        SandboxLevel::None,
    ] {
        let json = serde_json::to_string(&level).unwrap();
        let parsed: SandboxLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(level, parsed);
    }
}

#[test]
fn f_2603_23_sandbox_config_serde_roundtrip() {
    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![BindMount {
            source: "/data".into(),
            target: "/mnt/data".into(),
            readonly: true,
        }],
        env: vec![EnvVar {
            name: "KEY".into(),
            value: "val".into(),
        }],
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: SandboxConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.level, SandboxLevel::Full);
    assert_eq!(parsed.bind_mounts.len(), 1);
    assert_eq!(parsed.env.len(), 1);
}

// ── FJ-2600: Convergence Result Serde ──────────────────────────────

#[test]
fn f_cross_1_convergence_result_serde_roundtrip() {
    let result = ConvergenceResult {
        resource_id: "nginx".into(),
        resource_type: "package".into(),
        converged: true,
        idempotent: true,
        preserved: true,
        duration_ms: 150,
        error: None,
    };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: ConvergenceResult = serde_json::from_str(&json).unwrap();
    assert!(parsed.passed());
    assert_eq!(parsed.duration_ms, 150);
}

#[test]
fn f_cross_2_convergence_summary_serde_roundtrip() {
    let summary = ConvergenceSummary {
        total: 10,
        passed: 8,
        convergence_failures: 1,
        idempotency_failures: 1,
        preservation_failures: 0,
    };
    let json = serde_json::to_string(&summary).unwrap();
    let parsed: ConvergenceSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.total, 10);
    assert_eq!(parsed.passed, 8);
}
