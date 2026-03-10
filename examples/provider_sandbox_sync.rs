//! FJ-1359/1361/1362: Provider exec helpers, sandbox validation, sync parsing.
//!
//! Usage: cargo run --example provider_sandbox_sync

use forjar::core::store::provider_exec::{
    build_staging_script, dir_stats, hash_staging_dir, walkdir,
};
use forjar::core::store::sandbox_exec::{OverlayConfig, SandboxPlan, SandboxStep};
use forjar::core::store::sandbox_run::{dry_run_sandbox_plan, validate_sandbox_commands};
use forjar::core::store::sync_exec::{parse_provider, tempdir_for_reimport};
use std::path::PathBuf;

fn main() {
    println!("Forjar: Provider Exec, Sandbox Validation & Sync Parsing");
    println!("{}", "=".repeat(58));

    // ── FJ-1359: Provider execution helpers ──
    println!("\n[FJ-1359] Provider Execution Helpers:");

    let staging_script = build_staging_script(
        "cargo install --root $STAGING ripgrep",
        "/tmp/staging".as_ref(),
    );
    println!(
        "  Staging script:\n    {}",
        staging_script.replace('\n', "\n    ")
    );

    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("bin/rg"), "binary-content").ok();
    std::fs::create_dir_all(tmp.path().join("bin")).unwrap();
    std::fs::write(tmp.path().join("bin/rg"), "binary-content").unwrap();
    std::fs::write(tmp.path().join("bin/rg.1"), "manpage").unwrap();
    let hash = hash_staging_dir(tmp.path()).unwrap();
    println!("  Staging hash: {hash}");

    let (count, size) = dir_stats(tmp.path());
    println!("  Files: {count}, Size: {size} bytes");

    let entries = walkdir(tmp.path()).unwrap();
    for (path, sz) in &entries {
        println!("    {} ({sz} B)", path.display());
    }

    // ── FJ-1361: Sandbox dry-run validation ──
    println!("\n[FJ-1361] Sandbox Dry-Run Validation:");

    let plan = SandboxPlan {
        steps: vec![
            SandboxStep {
                description: "Create namespace".into(),
                command: Some("echo 'creating namespace'".into()),
                step: 1,
            },
            SandboxStep {
                description: "Mount overlay".into(),
                command: None, // informational
                step: 2,
            },
            SandboxStep {
                description: "Execute build".into(),
                command: Some("echo 'building...'".into()),
                step: 3,
            },
            SandboxStep {
                description: "Extract outputs".into(),
                command: Some("echo 'extracting'".into()),
                step: 4,
            },
        ],
        namespace_id: "forjar-build-demo".into(),
        overlay: OverlayConfig {
            lower_dirs: vec![PathBuf::from("/store/abc/content")],
            upper_dir: PathBuf::from("/tmp/upper"),
            work_dir: PathBuf::from("/tmp/work"),
            merged_dir: PathBuf::from("/tmp/merged"),
        },
        seccomp_rules: vec![],
        cgroup_path: "/sys/fs/cgroup/forjar-demo".into(),
    };

    let cmds = dry_run_sandbox_plan(&plan).unwrap();
    println!("  Commands ({} executable steps):", cmds.len());
    for (i, cmd) in cmds.iter().enumerate() {
        println!("    {}: {cmd}", i + 1);
    }

    let errors = validate_sandbox_commands(&plan);
    println!("  Validation errors: {}", errors.len());

    // ── FJ-1362: Sync exec parsing ──
    println!("\n[FJ-1362] Sync Execution Parsing:");

    let providers = [
        "apt",
        "cargo",
        "uv",
        "nix",
        "docker",
        "tofu",
        "terraform",
        "apr",
    ];
    for p in &providers {
        let parsed = parse_provider(p).unwrap();
        println!("  {p} → {parsed:?}");
    }

    let path = tempdir_for_reimport("blake3:abcdef1234567890aabbccddeeff0011");
    println!("\n  Reimport tempdir: {}", path.display());

    println!("\n{}", "=".repeat(58));
    println!("All provider/sandbox/sync criteria survived.");
}
