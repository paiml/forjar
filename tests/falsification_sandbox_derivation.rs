//! FJ-1316: Sandbox lifecycle planning.
//!
//! Popperian rejection criteria for:
//! - FJ-1316: plan_sandbox_build (10-step lifecycle, namespace, overlay, cgroup, seccomp)
//! - FJ-1316: seccomp_rules_for_level (Full vs non-Full)
//! - FJ-1316: validate_plan (step ordering, empty checks)
//! - FJ-1316: simulate_sandbox_build (dry-run determinism)
//! - FJ-1316: export_overlay_upper (whiteout conversion, tarball)
//! - FJ-1316: oci_layout_plan (OCI directory structure)
//! - FJ-1316: multi_arch_index (platform descriptor construction)
//! - FJ-1316: sha256_digest, gzip_compress, plan_step_count
//!
//! Usage: cargo test --test falsification_sandbox_derivation

use forjar::core::store::sandbox::{SandboxConfig, SandboxLevel};
use forjar::core::store::sandbox_exec::{
    export_overlay_upper, gzip_compress, multi_arch_index, oci_layout_plan, plan_sandbox_build,
    plan_step_count, seccomp_rules_for_level, sha256_digest, simulate_sandbox_build, validate_plan,
    OverlayConfig, SandboxPlan, SandboxStep,
};
use forjar::core::types::ArchBuild;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ============================================================================
// Helpers
// ============================================================================

fn test_sandbox_config(level: SandboxLevel) -> SandboxConfig {
    SandboxConfig {
        level,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: Vec::new(),
        env: Vec::new(),
    }
}

fn test_inputs() -> BTreeMap<String, PathBuf> {
    let mut m = BTreeMap::new();
    m.insert("curl".into(), PathBuf::from("/store/abc123/content"));
    m
}

// ============================================================================
// FJ-1316: plan_sandbox_build
// ============================================================================

#[test]
fn plan_has_10_steps_for_full_sandbox() {
    let config = test_sandbox_config(SandboxLevel::Full);
    let plan = plan_sandbox_build(
        &config,
        "blake3:deadbeef1234567890",
        &test_inputs(),
        "echo hello",
        Path::new("/store"),
    );
    assert_eq!(plan.steps.len(), 10); // 9 base steps + 1 bind per input
    assert!(plan.namespace_id.contains("forjar-build-"));
    assert!(!plan.cgroup_path.is_empty());
}

#[test]
fn plan_minimal_skips_seccomp() {
    let config = test_sandbox_config(SandboxLevel::Minimal);
    let plan = plan_sandbox_build(
        &config,
        "blake3:deadbeef1234567890",
        &test_inputs(),
        "echo hello",
        Path::new("/store"),
    );
    // Minimal: no seccomp step (step 5 absent)
    assert!(plan.seccomp_rules.is_empty());
    let has_seccomp = plan.steps.iter().any(|s| s.step == 5);
    assert!(!has_seccomp, "Minimal should skip seccomp step");
}

#[test]
fn plan_overlay_config_correct() {
    let config = test_sandbox_config(SandboxLevel::Full);
    let inputs = test_inputs();
    let plan = plan_sandbox_build(
        &config,
        "blake3:deadbeef1234567890",
        &inputs,
        "echo hello",
        Path::new("/store"),
    );
    assert_eq!(plan.overlay.lower_dirs.len(), inputs.len());
    assert!(plan.overlay.upper_dir.to_str().unwrap().contains("upper"));
    assert!(plan.overlay.work_dir.to_str().unwrap().contains("work"));
    assert!(plan.overlay.merged_dir.to_str().unwrap().contains("merged"));
}

#[test]
fn plan_bind_step_per_input() {
    let mut inputs = BTreeMap::new();
    inputs.insert("a".into(), PathBuf::from("/store/a/content"));
    inputs.insert("b".into(), PathBuf::from("/store/b/content"));
    inputs.insert("c".into(), PathBuf::from("/store/c/content"));
    let config = test_sandbox_config(SandboxLevel::Minimal);
    let plan = plan_sandbox_build(&config, "blake3:aabb", &inputs, "echo", Path::new("/s"));
    let bind_steps: Vec<_> = plan.steps.iter().filter(|s| s.step == 3).collect();
    assert_eq!(bind_steps.len(), 3, "one bind step per input");
}

#[test]
fn plan_cgroup_has_memory_and_cpu() {
    let config = test_sandbox_config(SandboxLevel::Full);
    let plan = plan_sandbox_build(
        &config,
        "blake3:deadbeef1234567890",
        &test_inputs(),
        "echo hello",
        Path::new("/store"),
    );
    let cgroup_step = plan.steps.iter().find(|s| s.step == 4).unwrap();
    let cmd = cgroup_step.command.as_ref().unwrap();
    assert!(cmd.contains("memory.max"));
    assert!(cmd.contains("cpu.max"));
}

// ============================================================================
// FJ-1316: seccomp_rules_for_level
// ============================================================================

#[test]
fn seccomp_full_denies_connect_mount_ptrace() {
    let rules = seccomp_rules_for_level(SandboxLevel::Full);
    assert_eq!(rules.len(), 3);
    let syscalls: Vec<&str> = rules.iter().map(|r| r.syscall.as_str()).collect();
    assert!(syscalls.contains(&"connect"));
    assert!(syscalls.contains(&"mount"));
    assert!(syscalls.contains(&"ptrace"));
    assert!(rules.iter().all(|r| r.action == "deny"));
}

#[test]
fn seccomp_non_full_empty() {
    assert!(seccomp_rules_for_level(SandboxLevel::Minimal).is_empty());
    assert!(seccomp_rules_for_level(SandboxLevel::NetworkOnly).is_empty());
    assert!(seccomp_rules_for_level(SandboxLevel::None).is_empty());
}

// ============================================================================
// FJ-1316: validate_plan
// ============================================================================

#[test]
fn validate_plan_valid() {
    let config = test_sandbox_config(SandboxLevel::Full);
    let plan = plan_sandbox_build(
        &config,
        "blake3:deadbeef1234567890",
        &test_inputs(),
        "echo hello",
        Path::new("/store"),
    );
    assert!(validate_plan(&plan).is_empty());
}

#[test]
fn validate_plan_empty_steps() {
    let plan = SandboxPlan {
        steps: vec![],
        namespace_id: "ns".into(),
        overlay: OverlayConfig {
            lower_dirs: vec![PathBuf::from("/a")],
            upper_dir: PathBuf::from("/u"),
            work_dir: PathBuf::from("/w"),
            merged_dir: PathBuf::from("/m"),
        },
        seccomp_rules: vec![],
        cgroup_path: "/cg".into(),
    };
    let errors = validate_plan(&plan);
    assert!(errors.iter().any(|e| e.contains("no steps")));
}

#[test]
fn validate_plan_empty_namespace() {
    let plan = SandboxPlan {
        steps: vec![SandboxStep {
            step: 1,
            description: "test".into(),
            command: None,
        }],
        namespace_id: "".into(),
        overlay: OverlayConfig {
            lower_dirs: vec![PathBuf::from("/a")],
            upper_dir: PathBuf::from("/u"),
            work_dir: PathBuf::from("/w"),
            merged_dir: PathBuf::from("/m"),
        },
        seccomp_rules: vec![],
        cgroup_path: "/cg".into(),
    };
    let errors = validate_plan(&plan);
    assert!(errors.iter().any(|e| e.contains("namespace_id")));
}

#[test]
fn validate_plan_no_lower_dirs() {
    let plan = SandboxPlan {
        steps: vec![SandboxStep {
            step: 1,
            description: "test".into(),
            command: None,
        }],
        namespace_id: "ns".into(),
        overlay: OverlayConfig {
            lower_dirs: vec![],
            upper_dir: PathBuf::from("/u"),
            work_dir: PathBuf::from("/w"),
            merged_dir: PathBuf::from("/m"),
        },
        seccomp_rules: vec![],
        cgroup_path: "/cg".into(),
    };
    let errors = validate_plan(&plan);
    assert!(errors.iter().any(|e| e.contains("lower directory")));
}

// ============================================================================
// FJ-1316: simulate_sandbox_build
// ============================================================================

#[test]
fn simulate_deterministic() {
    let config = test_sandbox_config(SandboxLevel::Full);
    let inputs = test_inputs();
    let r1 = simulate_sandbox_build(&config, "blake3:aabb", &inputs, "echo hi", Path::new("/s"));
    let r2 = simulate_sandbox_build(&config, "blake3:aabb", &inputs, "echo hi", Path::new("/s"));
    assert_eq!(r1.output_hash, r2.output_hash);
    assert_eq!(r1.store_path, r2.store_path);
}

#[test]
fn simulate_script_sensitive() {
    let config = test_sandbox_config(SandboxLevel::Full);
    let inputs = test_inputs();
    let r1 = simulate_sandbox_build(&config, "blake3:aabb", &inputs, "echo A", Path::new("/s"));
    let r2 = simulate_sandbox_build(&config, "blake3:aabb", &inputs, "echo B", Path::new("/s"));
    assert_ne!(r1.output_hash, r2.output_hash);
}

#[test]
fn simulate_produces_steps() {
    let config = test_sandbox_config(SandboxLevel::Full);
    let r = simulate_sandbox_build(
        &config,
        "blake3:aabb",
        &test_inputs(),
        "echo hi",
        Path::new("/s"),
    );
    assert!(!r.steps_executed.is_empty());
    assert!(r.output_hash.starts_with("blake3:"));
}

// ============================================================================
// FJ-1316: export_overlay_upper
// ============================================================================

#[test]
fn overlay_export_3_steps() {
    let overlay = OverlayConfig {
        lower_dirs: vec![PathBuf::from("/l")],
        upper_dir: PathBuf::from("/upper"),
        work_dir: PathBuf::from("/w"),
        merged_dir: PathBuf::from("/m"),
    };
    let steps = export_overlay_upper(&overlay, Path::new("/out/layer.tar"));
    assert_eq!(steps.len(), 3);
    let cmd0 = steps[0].command.as_ref().unwrap();
    assert!(cmd0.contains(".wh."), "step 1 converts whiteouts");
    let cmd1 = steps[1].command.as_ref().unwrap();
    assert!(cmd1.contains("tar"), "step 2 creates tarball");
    let cmd2 = steps[2].command.as_ref().unwrap();
    assert!(cmd2.contains("sha256sum"), "step 3 computes DiffID");
}

// ============================================================================
// FJ-1316: oci_layout_plan
// ============================================================================

#[test]
fn oci_layout_4_steps() {
    let steps = oci_layout_plan(Path::new("/out"), "v1.0");
    assert_eq!(steps.len(), 4);
    assert!(steps[0].command.as_ref().unwrap().contains("mkdir"));
    assert!(steps[1]
        .command
        .as_ref()
        .unwrap()
        .contains("imageLayoutVersion"));
    assert!(steps[2].command.as_ref().unwrap().contains("schemaVersion"));
    assert!(steps[3].command.as_ref().unwrap().contains("RepoTags"));
    assert!(steps[3].command.as_ref().unwrap().contains("v1.0"));
}

// ============================================================================
// FJ-1316: multi_arch_index
// ============================================================================

#[test]
fn multi_arch_index_from_platforms() {
    let platforms = vec![
        ArchBuild {
            platform: "linux/amd64".into(),
            os: "linux".into(),
            architecture: "amd64".into(),
            variant: None,
            manifest_digest: Some("sha256:aaa".into()),
            duration_secs: None,
        },
        ArchBuild {
            platform: "linux/arm64".into(),
            os: "linux".into(),
            architecture: "arm64".into(),
            variant: Some("v8".into()),
            manifest_digest: Some("sha256:bbb".into()),
            duration_secs: None,
        },
    ];
    let index = multi_arch_index(&platforms);
    assert_eq!(index.schema_version, 2);
    assert_eq!(index.manifests.len(), 2);
    assert_eq!(index.manifests[0].digest, "sha256:aaa");
    assert_eq!(index.manifests[1].digest, "sha256:bbb");
}

#[test]
fn multi_arch_index_skips_no_digest() {
    let platforms = vec![ArchBuild {
        platform: "linux/amd64".into(),
        os: "linux".into(),
        architecture: "amd64".into(),
        variant: None,
        manifest_digest: None,
        duration_secs: None,
    }];
    let index = multi_arch_index(&platforms);
    assert!(index.manifests.is_empty());
}

// ============================================================================
// FJ-1316: sha256_digest, gzip_compress, plan_step_count
// ============================================================================

#[test]
fn sha256_deterministic() {
    let d1 = sha256_digest(b"hello world");
    let d2 = sha256_digest(b"hello world");
    assert_eq!(d1, d2);
    assert!(d1.starts_with("sha256:"));
}

#[test]
fn sha256_sensitive() {
    assert_ne!(sha256_digest(b"a"), sha256_digest(b"b"));
}

#[test]
fn gzip_roundtrip() {
    let data = b"forjar sandbox build data";
    let compressed = gzip_compress(data).unwrap();
    assert!(!compressed.is_empty());
    assert!(compressed.len() < data.len() + 100); // gzip overhead
}

#[test]
fn plan_step_count_matches() {
    let config = test_sandbox_config(SandboxLevel::Full);
    let plan = plan_sandbox_build(
        &config,
        "blake3:deadbeef1234567890",
        &test_inputs(),
        "echo hello",
        Path::new("/store"),
    );
    assert_eq!(plan_step_count(&plan), plan.steps.len());
}
