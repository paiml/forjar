//! Additional coverage tests for store execution modules (FJ-1372).
//!
//! Targets untested paths in cache_exec, sandbox_run, sync_exec,
//! and pin_resolve that don't require real transport.

use super::cache::CacheSource;
use super::cache_exec::{pull_command, push_command, CachePullResult};
use super::pin_resolve::{parse_resolved_version, pin_hash, resolution_command, ResolvedPin};
use super::sandbox_exec::{SandboxPlan, SandboxStep};
use super::sandbox_run::{dry_run_sandbox_plan, validate_sandbox_commands, SandboxExecResult};
use super::substitution::{SubstitutionOutcome, SubstitutionPlan, SubstitutionStep};
use std::path::{Path, PathBuf};

// ── CachePullResult fields ──────────────────────────────────────

#[test]
fn cache_pull_result_clone() {
    let r = CachePullResult {
        store_hash: "blake3:aaa".to_string(),
        store_path: "/store/aaa/content".to_string(),
        bytes_transferred: 4096,
        verified: true,
    };
    let r2 = r.clone();
    assert_eq!(r2.store_hash, "blake3:aaa");
    assert_eq!(r2.bytes_transferred, 4096);
    assert!(r2.verified);
}

#[test]
fn cache_pull_result_unverified() {
    let r = CachePullResult {
        store_hash: "blake3:bbb".to_string(),
        store_path: "/store/bbb".to_string(),
        bytes_transferred: 0,
        verified: false,
    };
    assert!(!r.verified);
    assert_eq!(r.bytes_transferred, 0);
}

// ── pull_command edge cases ─────────────────────────────────────

#[test]
fn pull_command_local_path_with_spaces() {
    let src = CacheSource::Local {
        path: "/mnt/my cache".to_string(),
    };
    let cmd = pull_command(&src, "blake3:abc", Path::new("/tmp/staging dir"));
    assert!(cmd.contains("/mnt/my cache/abc/."));
}

#[test]
fn push_command_local_path_with_spaces() {
    let src = CacheSource::Local {
        path: "/mnt/my cache".to_string(),
    };
    let cmd = push_command(&src, "blake3:abc", Path::new("/store dir"));
    assert!(cmd.contains("/store dir/abc"));
}

#[test]
fn pull_command_ssh_default_port() {
    let src = CacheSource::Ssh {
        host: "cache.lan".to_string(),
        user: "fj".to_string(),
        path: "/cache".to_string(),
        port: None,
    };
    let cmd = pull_command(&src, "blake3:xyz", Path::new("/tmp/s"));
    // No port flag should be present when port is None
    assert!(!cmd.contains("-p 2222"));
    assert!(cmd.contains("fj@cache.lan"));
    assert!(cmd.contains("/cache/xyz/"));
}

#[test]
fn push_command_ssh_custom_port() {
    let src = CacheSource::Ssh {
        host: "cache.lan".to_string(),
        user: "fj".to_string(),
        path: "/cache".to_string(),
        port: Some(9922),
    };
    let cmd = push_command(&src, "abc", Path::new("/store"));
    assert!(cmd.contains("-p 9922"));
}

// ── substitution outcome tests ──────────────────────────────────

#[test]
fn substitution_local_hit_outcome() {
    let plan = SubstitutionPlan {
        outcome: SubstitutionOutcome::LocalHit {
            store_path: "/store/aaa".to_string(),
        },
        steps: vec![SubstitutionStep::CheckLocalStore {
            store_hash: "blake3:aaa".to_string(),
            found: true,
        }],
        store_hash: "blake3:aaa".to_string(),
    };
    match &plan.outcome {
        SubstitutionOutcome::LocalHit { store_path } => {
            assert!(store_path.contains("aaa"));
        }
        _ => panic!("expected LocalHit"),
    }
}

#[test]
fn substitution_cache_miss_outcome() {
    let plan = SubstitutionPlan {
        outcome: SubstitutionOutcome::CacheMiss {
            store_hash: "blake3:miss".to_string(),
        },
        steps: vec![],
        store_hash: "blake3:miss".to_string(),
    };
    match &plan.outcome {
        SubstitutionOutcome::CacheMiss { store_hash } => {
            assert!(store_hash.contains("miss"));
        }
        _ => panic!("expected CacheMiss"),
    }
}

// ── pin_resolve edge cases ──────────────────────────────────────

#[test]
fn parse_apt_output_with_extra_whitespace() {
    let output = "curl:\n  Installed: 7.88.1\n  Candidate:   8.0.0   \n";
    let v = parse_resolved_version("apt", output).unwrap();
    assert_eq!(v, "8.0.0");
}

#[test]
fn parse_cargo_output_no_equals() {
    let output = "no results found";
    assert!(parse_resolved_version("cargo", output).is_none());
}

#[test]
fn parse_cargo_output_with_comment() {
    let output = "serde = \"1.0.200\"    # Serialization framework";
    let v = parse_resolved_version("cargo", output).unwrap();
    assert_eq!(v, "1.0.200");
}

#[test]
fn parse_uv_no_available_versions_prefix() {
    // Fallback: first line
    let output = "1.5.0";
    let v = parse_resolved_version("uv", output).unwrap();
    assert_eq!(v, "1.5.0");
}

#[test]
fn parse_pip_available_versions() {
    let output = "Available versions: 2.0.0, 1.9.0, 1.8.0";
    let v = parse_resolved_version("pip", output).unwrap();
    assert_eq!(v, "2.0.0");
}

#[test]
fn parse_docker_multiline() {
    let output = "sha256:abc123def456";
    let v = parse_resolved_version("docker", output).unwrap();
    assert_eq!(v, "sha256:abc123def456");
}

#[test]
fn parse_apr_with_trailing_newlines() {
    let output = "3.1\n\n";
    let v = parse_resolved_version("apr", output).unwrap();
    assert_eq!(v, "3.1");
}

#[test]
fn parse_unknown_provider_none() {
    assert!(parse_resolved_version("custom-provider", "1.0").is_none());
}

#[test]
fn resolution_command_tofu_none() {
    // tofu is not supported for version resolution
    assert!(resolution_command("tofu", "module").is_none());
}

#[test]
fn resolution_command_terraform_none() {
    assert!(resolution_command("terraform", "module").is_none());
}

#[test]
fn pin_hash_empty_version() {
    let h = pin_hash("apt", "curl", "");
    assert!(h.starts_with("blake3:"));
}

#[test]
fn resolved_pin_eq() {
    let a = ResolvedPin {
        name: "curl".to_string(),
        provider: "apt".to_string(),
        version: "7.0".to_string(),
        hash: pin_hash("apt", "curl", "7.0"),
    };
    let b = a.clone();
    assert_eq!(a, b);
}

// ── sandbox_run: SandboxExecResult paths ────────────────────────

#[test]
fn sandbox_exec_result_with_many_steps() {
    let result = SandboxExecResult {
        output_hash: "blake3:out123".to_string(),
        store_path: "/store/out123/content".to_string(),
        steps_executed: vec![
            (1, "Create namespace".to_string(), true),
            (2, "Mount overlay".to_string(), true),
            (3, "Run script".to_string(), true),
            (4, "Capture output".to_string(), true),
            (5, "Cleanup".to_string(), true),
        ],
        duration_secs: 12.5,
    };
    assert_eq!(result.steps_executed.len(), 5);
    assert!(result.steps_executed.iter().all(|(_, _, ok)| *ok));
    assert!(result.duration_secs > 10.0);
}

#[test]
fn sandbox_exec_result_with_failure() {
    let result = SandboxExecResult {
        output_hash: String::new(),
        store_path: String::new(),
        steps_executed: vec![
            (1, "Create namespace".to_string(), true),
            (2, "Run script".to_string(), false),
        ],
        duration_secs: 0.5,
    };
    assert!(!result.steps_executed[1].2);
}

#[test]
fn dry_run_sandbox_plan_empty_steps() {
    let plan = SandboxPlan {
        namespace_id: "forjar-build-empty".to_string(),
        steps: vec![],
        overlay: super::sandbox_exec::OverlayConfig {
            lower_dirs: vec![],
            upper_dir: PathBuf::from("/tmp/upper"),
            work_dir: PathBuf::from("/tmp/work"),
            merged_dir: PathBuf::from("/tmp/merged"),
        },
        cgroup_path: "/sys/fs/cgroup/forjar-build-empty".to_string(),
        seccomp_rules: vec![],
    };
    let cmds = dry_run_sandbox_plan(&plan).unwrap();
    assert!(cmds.is_empty());
}

#[test]
fn validate_sandbox_commands_empty_plan() {
    let plan = SandboxPlan {
        namespace_id: "empty".to_string(),
        steps: vec![],
        overlay: super::sandbox_exec::OverlayConfig {
            lower_dirs: vec![],
            upper_dir: PathBuf::from("/tmp/u"),
            work_dir: PathBuf::from("/tmp/w"),
            merged_dir: PathBuf::from("/tmp/m"),
        },
        cgroup_path: "/sys/fs/cgroup/empty".to_string(),
        seccomp_rules: vec![],
    };
    let errors = validate_sandbox_commands(&plan);
    assert!(errors.is_empty());
}

#[test]
fn validate_sandbox_commands_info_step_no_command() {
    let plan = SandboxPlan {
        namespace_id: "info".to_string(),
        steps: vec![SandboxStep {
            step: 1,
            description: "Informational".to_string(),
            command: None,
        }],
        overlay: super::sandbox_exec::OverlayConfig {
            lower_dirs: vec![],
            upper_dir: PathBuf::from("/tmp/u"),
            work_dir: PathBuf::from("/tmp/w"),
            merged_dir: PathBuf::from("/tmp/m"),
        },
        cgroup_path: "/sys/fs/cgroup/info".to_string(),
        seccomp_rules: vec![],
    };
    let errors = validate_sandbox_commands(&plan);
    assert!(errors.is_empty());
}

#[test]
fn dry_run_with_valid_simple_commands() {
    let plan = SandboxPlan {
        namespace_id: "simple".to_string(),
        steps: vec![
            SandboxStep {
                step: 1,
                description: "echo".to_string(),
                command: Some("echo hello".to_string()),
            },
            SandboxStep {
                step: 2,
                description: "ls".to_string(),
                command: Some("ls /tmp".to_string()),
            },
        ],
        overlay: super::sandbox_exec::OverlayConfig {
            lower_dirs: vec![],
            upper_dir: PathBuf::from("/tmp/u"),
            work_dir: PathBuf::from("/tmp/w"),
            merged_dir: PathBuf::from("/tmp/m"),
        },
        cgroup_path: "/sys/fs/cgroup/simple".to_string(),
        seccomp_rules: vec![],
    };
    let cmds = dry_run_sandbox_plan(&plan).unwrap();
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[0], "echo hello");
    assert_eq!(cmds[1], "ls /tmp");
}
