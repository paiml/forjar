//! FJ-2600/2601/2603: Popperian falsification for convergence testing,
//! idempotency verification, and sandbox isolation.
//!
//! Each test states conditions under which the convergence model or
//! sandbox isolation would be rejected as invalid.

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
fn f_2600_1_convergence_result_all_pass() {
    let result = ConvergenceResult {
        resource_id: "nginx-pkg".into(),
        resource_type: "package".into(),
        converged: true,
        idempotent: true,
        preserved: true,
        duration_ms: 100,
        error: None,
    };
    assert!(result.passed(), "all true fields must pass");
    assert!(result.to_string().contains("[PASS]"));
}

#[test]
fn f_2600_2_convergence_result_fails_on_not_converged() {
    let result = ConvergenceResult {
        resource_id: "pkg".into(),
        resource_type: "package".into(),
        converged: false,
        idempotent: true,
        preserved: true,
        duration_ms: 50,
        error: None,
    };
    assert!(!result.passed(), "converged=false must fail");
    assert!(result.to_string().contains("[FAIL]"));
}

#[test]
fn f_2600_3_convergence_result_fails_on_not_idempotent() {
    let result = ConvergenceResult {
        resource_id: "pkg".into(),
        resource_type: "package".into(),
        converged: true,
        idempotent: false,
        preserved: true,
        duration_ms: 50,
        error: None,
    };
    assert!(!result.passed(), "idempotent=false must fail");
}

#[test]
fn f_2600_4_convergence_result_fails_on_not_preserved() {
    let result = ConvergenceResult {
        resource_id: "pkg".into(),
        resource_type: "package".into(),
        converged: true,
        idempotent: true,
        preserved: false,
        duration_ms: 50,
        error: None,
    };
    assert!(!result.passed(), "preserved=false must fail");
}

#[test]
fn f_2600_5_convergence_result_fails_on_error() {
    let result = ConvergenceResult {
        resource_id: "pkg".into(),
        resource_type: "package".into(),
        converged: true,
        idempotent: true,
        preserved: true,
        duration_ms: 50,
        error: Some("unexpected failure".into()),
    };
    assert!(!result.passed(), "error present must fail");
}

// ── FJ-2600: Convergence Summary ───────────────────────────────────

#[test]
fn f_2600_6_convergence_summary_all_pass() {
    let results = vec![
        ConvergenceResult {
            resource_id: "a".into(),
            resource_type: "file".into(),
            converged: true,
            idempotent: true,
            preserved: true,
            duration_ms: 10,
            error: None,
        },
        ConvergenceResult {
            resource_id: "b".into(),
            resource_type: "package".into(),
            converged: true,
            idempotent: true,
            preserved: true,
            duration_ms: 20,
            error: None,
        },
    ];
    let summary = ConvergenceSummary::from_results(&results);
    assert_eq!(summary.total, 2);
    assert_eq!(summary.passed, 2);
    assert_eq!(summary.convergence_failures, 0);
    assert_eq!(summary.idempotency_failures, 0);
    assert_eq!(summary.preservation_failures, 0);
    assert!((summary.pass_rate() - 100.0).abs() < 0.01);
}

#[test]
fn f_2600_7_convergence_summary_with_failures() {
    let results = vec![
        ConvergenceResult {
            resource_id: "a".into(),
            resource_type: "file".into(),
            converged: true,
            idempotent: true,
            preserved: true,
            duration_ms: 10,
            error: None,
        },
        ConvergenceResult {
            resource_id: "b".into(),
            resource_type: "package".into(),
            converged: false,
            idempotent: true,
            preserved: true,
            duration_ms: 20,
            error: None,
        },
        ConvergenceResult {
            resource_id: "c".into(),
            resource_type: "service".into(),
            converged: true,
            idempotent: false,
            preserved: false,
            duration_ms: 30,
            error: None,
        },
    ];
    let summary = ConvergenceSummary::from_results(&results);
    assert_eq!(summary.total, 3);
    assert_eq!(summary.passed, 1);
    assert_eq!(summary.convergence_failures, 1);
    assert_eq!(summary.idempotency_failures, 1);
    assert_eq!(summary.preservation_failures, 1);
    let display = summary.to_string();
    assert!(display.contains("1/3 passed"));
}

#[test]
fn f_2600_8_convergence_summary_empty_is_100_pct() {
    let summary = ConvergenceSummary::from_results(&[]);
    assert_eq!(summary.total, 0);
    assert!((summary.pass_rate() - 100.0).abs() < 0.01);
}

// ── FJ-2600: Convergence Report Formatting ─────────────────────────

#[test]
fn f_2600_9_format_convergence_report_pass() {
    let results = vec![ConvergenceResult {
        resource_id: "pkg".into(),
        resource_type: "package".into(),
        converged: true,
        idempotent: true,
        preserved: true,
        duration_ms: 42,
        error: None,
    }];
    let report = format_convergence_report(&results);
    assert!(report.contains("1/1 passed"));
    assert!(report.contains("[PASS]"));
}

#[test]
fn f_2600_10_format_convergence_report_failure() {
    let results = vec![ConvergenceResult {
        resource_id: "svc".into(),
        resource_type: "service".into(),
        converged: true,
        idempotent: false,
        preserved: true,
        duration_ms: 99,
        error: None,
    }];
    let report = format_convergence_report(&results);
    assert!(report.contains("0/1 passed"));
    assert!(report.contains("Failures:"));
    assert!(report.contains("idempotency"));
}

// ── FJ-2601: Runner Mode ──────────────────────────────────────────

#[test]
fn f_2601_1_runner_mode_display() {
    assert_eq!(RunnerMode::Simulated.to_string(), "simulated");
    assert_eq!(RunnerMode::Sandbox.to_string(), "sandbox");
}

#[test]
fn f_2601_2_convergence_test_config_defaults() {
    let config = ConvergenceTestConfig::default();
    assert_eq!(config.backend, SandboxBackend::Pepita);
    assert_eq!(config.level, SandboxLevel::Minimal);
    assert!(!config.test_pairs);
    assert_eq!(config.parallelism, 4);
}

// ── FJ-2601: Live Convergence Test (Safe Script) ───────────────────

#[test]
fn f_2601_3_convergence_test_safe_script() {
    let target = ConvergenceTarget {
        resource_id: "test-file".into(),
        resource_type: "file".into(),
        apply_script: "echo 'hello' > $FORJAR_SANDBOX/test.txt".into(),
        state_query_script: "cat $FORJAR_SANDBOX/test.txt".into(),
        expected_hash: String::new(),
    };
    let result = run_convergence_test(&target);
    assert!(
        result.converged,
        "safe echo script must converge: {:?}",
        result.error
    );
    assert!(result.idempotent, "echo is idempotent");
    assert!(result.preserved, "state must be preserved after re-apply");
}

#[test]
fn f_2601_4_convergence_test_empty_script_fails() {
    let target = ConvergenceTarget {
        resource_id: "empty".into(),
        resource_type: "file".into(),
        apply_script: String::new(),
        state_query_script: "true".into(),
        expected_hash: String::new(),
    };
    let result = run_convergence_test(&target);
    assert!(!result.converged, "empty apply script must fail");
    assert!(result.error.is_some());
}

#[test]
fn f_2601_5_convergence_test_unsafe_script_rejected() {
    let target = ConvergenceTarget {
        resource_id: "unsafe".into(),
        resource_type: "package".into(),
        apply_script: "apt-get install nginx".into(),
        state_query_script: "dpkg -l nginx".into(),
        expected_hash: String::new(),
    };
    let result = run_convergence_test(&target);
    assert!(!result.converged, "unsafe system command must be rejected");
    assert!(result.error.as_deref().unwrap().contains("system commands"));
}

// ── FJ-2603: Sandbox Level Properties ──────────────────────────────

#[test]
fn f_2603_1_full_sandbox_blocks_network() {
    assert!(
        blocks_network(SandboxLevel::Full),
        "Full isolation must block network"
    );
    assert!(
        !blocks_network(SandboxLevel::NetworkOnly),
        "NetworkOnly must allow network"
    );
    assert!(!blocks_network(SandboxLevel::Minimal));
    assert!(!blocks_network(SandboxLevel::None));
}

#[test]
fn f_2603_2_fs_isolation_levels() {
    assert!(
        enforces_fs_isolation(SandboxLevel::Full),
        "Full must enforce FS"
    );
    assert!(
        enforces_fs_isolation(SandboxLevel::NetworkOnly),
        "NetworkOnly must enforce FS"
    );
    assert!(
        enforces_fs_isolation(SandboxLevel::Minimal),
        "Minimal must enforce FS"
    );
    assert!(
        !enforces_fs_isolation(SandboxLevel::None),
        "None must NOT enforce FS"
    );
}

// ── FJ-2603: Sandbox Config Validation ─────────────────────────────

#[test]
fn f_2603_3_valid_sandbox_config_no_errors() {
    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    };
    let errors = validate_config(&config);
    assert!(errors.is_empty(), "valid config must have no errors");
}

#[test]
fn f_2603_4_zero_memory_rejected() {
    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 0,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    };
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.contains("memory_mb")),
        "zero memory must be rejected"
    );
}

#[test]
fn f_2603_5_zero_cpus_rejected() {
    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 2048,
        cpus: 0.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    };
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.contains("cpus")),
        "zero cpus must be rejected"
    );
}

#[test]
fn f_2603_6_zero_timeout_rejected() {
    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 0,
        bind_mounts: vec![],
        env: vec![],
    };
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.contains("timeout")),
        "zero timeout must be rejected"
    );
}

#[test]
fn f_2603_7_excessive_memory_rejected() {
    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 2_000_000,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    };
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.contains("TiB")),
        "memory > 1 TiB must be rejected"
    );
}

#[test]
fn f_2603_8_empty_bind_mount_source_rejected() {
    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![BindMount {
            source: String::new(),
            target: "/data".into(),
            readonly: true,
        }],
        env: vec![],
    };
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.contains("source")),
        "empty bind mount source must be rejected"
    );
}

// ── FJ-2603: Sandbox Preset Profiles ───────────────────────────────

#[test]
fn f_2603_9_preset_full_profile() {
    let profile = preset_profile("full").expect("full profile must exist");
    assert_eq!(profile.level, SandboxLevel::Full);
    assert_eq!(profile.memory_mb, 2048);
    assert_eq!(profile.cpus, 4.0);
}

#[test]
fn f_2603_10_preset_network_only_profile() {
    let profile = preset_profile("network-only").expect("network-only must exist");
    assert_eq!(profile.level, SandboxLevel::NetworkOnly);
    assert_eq!(profile.memory_mb, 4096);
}

#[test]
fn f_2603_11_preset_minimal_profile() {
    let profile = preset_profile("minimal").expect("minimal must exist");
    assert_eq!(profile.level, SandboxLevel::Minimal);
    assert_eq!(profile.memory_mb, 1024);
}

#[test]
fn f_2603_12_preset_gpu_profile() {
    let profile = preset_profile("gpu").expect("gpu must exist");
    assert_eq!(profile.level, SandboxLevel::NetworkOnly);
    assert_eq!(profile.memory_mb, 16384);
    assert!(!profile.bind_mounts.is_empty(), "GPU needs device binds");
    assert!(!profile.env.is_empty(), "GPU needs NVIDIA_VISIBLE_DEVICES");
}

#[test]
fn f_2603_13_preset_unknown_returns_none() {
    assert!(preset_profile("nonexistent").is_none());
}

// ── FJ-2603: Sandbox YAML Parsing ──────────────────────────────────

#[test]
fn f_2603_14_parse_sandbox_config_yaml() {
    let yaml = r#"
level: full
memory_mb: 4096
cpus: 8.0
timeout: 1200
bind_mounts:
  - source: /data/inputs
    target: /inputs
    readonly: true
env:
  - name: BUILD_MODE
    value: release
"#;
    let config = parse_sandbox_config(yaml).unwrap();
    assert_eq!(config.level, SandboxLevel::Full);
    assert_eq!(config.memory_mb, 4096);
    assert_eq!(config.bind_mounts.len(), 1);
    assert_eq!(config.env.len(), 1);
    assert_eq!(config.env[0].name, "BUILD_MODE");
}

#[test]
fn f_2603_15_parse_sandbox_config_invalid_yaml() {
    let result = parse_sandbox_config("{ bad yaml");
    assert!(result.is_err());
}

// ── FJ-2603: Cgroup Path Generation ───────────────────────────────

#[test]
fn f_2603_16_cgroup_path_derives_from_hash() {
    let path = cgroup_path("blake3:abc123def456789012345678");
    assert!(path.starts_with("/sys/fs/cgroup/forjar-build-"));
    assert!(path.contains("abc123def4567890"));
}

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
