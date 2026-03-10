//! FJ-1359/1361/1362: Provider exec helpers, sandbox dry-run, sync parsing.
//! Usage: cargo test --test falsification_provider_sandbox_sync

use forjar::core::store::provider::ImportProvider;
use forjar::core::store::provider_exec::{
    atomic_move_to_store, build_staging_script, dir_stats, hash_staging_dir, walkdir,
};
use forjar::core::store::sandbox_exec::{OverlayConfig, SandboxPlan, SandboxStep};
use forjar::core::store::sandbox_run::{dry_run_sandbox_plan, validate_sandbox_commands};
use forjar::core::store::sync_exec::{parse_provider, tempdir_for_reimport};
use std::path::PathBuf;

// ── helpers ──

fn make_plan(commands: Vec<Option<&str>>) -> SandboxPlan {
    SandboxPlan {
        steps: commands
            .into_iter()
            .enumerate()
            .map(|(i, cmd)| SandboxStep {
                description: format!("step {}", i + 1),
                command: cmd.map(|s| s.to_string()),
                step: (i + 1) as u8,
            })
            .collect(),
        namespace_id: "test-ns".into(),
        overlay: OverlayConfig {
            lower_dirs: vec![PathBuf::from("/store/abc/content")],
            upper_dir: PathBuf::from("/tmp/upper"),
            work_dir: PathBuf::from("/tmp/work"),
            merged_dir: PathBuf::from("/tmp/merged"),
        },
        seccomp_rules: vec![],
        cgroup_path: "/sys/fs/cgroup/forjar-test".into(),
    }
}

// ── FJ-1359: build_staging_script ──

#[test]
fn staging_script_contains_export_and_mkdir() {
    let script = build_staging_script("apt-get download nginx", std::path::Path::new("/tmp/stage"));
    assert!(script.contains("export STAGING='/tmp/stage'"));
    assert!(script.contains("mkdir -p \"$STAGING\""));
    assert!(script.contains("apt-get download nginx"));
}

#[test]
fn staging_script_preserves_command() {
    let script = build_staging_script(
        "cargo install --root $STAGING ripgrep",
        std::path::Path::new("/staging/abc"),
    );
    assert!(script.contains("cargo install --root $STAGING ripgrep"));
}

// ── FJ-1359: hash_staging_dir ──

#[test]
fn hash_staging_dir_with_files() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("a.txt"), "hello").unwrap();
    std::fs::write(tmp.path().join("b.txt"), "world").unwrap();
    let hash = hash_staging_dir(tmp.path()).unwrap();
    assert!(hash.starts_with("blake3:"));
    assert!(hash.len() > 10);
}

#[test]
fn hash_staging_dir_deterministic() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("file.txt"), "content").unwrap();
    let h1 = hash_staging_dir(tmp.path()).unwrap();
    let h2 = hash_staging_dir(tmp.path()).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn hash_staging_dir_empty_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let result = hash_staging_dir(tmp.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("empty"));
}

#[test]
fn hash_staging_dir_nested_files() {
    let tmp = tempfile::tempdir().unwrap();
    let sub = tmp.path().join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("nested.txt"), "deep").unwrap();
    let hash = hash_staging_dir(tmp.path()).unwrap();
    assert!(hash.starts_with("blake3:"));
}

#[test]
fn hash_staging_dir_different_content_different_hash() {
    let t1 = tempfile::tempdir().unwrap();
    std::fs::write(t1.path().join("f.txt"), "aaa").unwrap();
    let t2 = tempfile::tempdir().unwrap();
    std::fs::write(t2.path().join("f.txt"), "bbb").unwrap();
    let h1 = hash_staging_dir(t1.path()).unwrap();
    let h2 = hash_staging_dir(t2.path()).unwrap();
    assert_ne!(h1, h2);
}

// ── FJ-1359: atomic_move_to_store ──

#[test]
fn atomic_move_renames_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("staging");
    let dst = tmp.path().join("store/hash/content");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("data.bin"), "payload").unwrap();

    atomic_move_to_store(&src, &dst).unwrap();
    assert!(!src.exists());
    assert!(dst.exists());
    assert!(dst.join("data.bin").exists());
}

#[test]
fn atomic_move_creates_parent_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("staging");
    let dst = tmp.path().join("deep/nested/store/content");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("file"), "x").unwrap();

    atomic_move_to_store(&src, &dst).unwrap();
    assert!(dst.join("file").exists());
}

// ── FJ-1359: dir_stats ──

#[test]
fn dir_stats_counts_files_and_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("a"), "hello").unwrap(); // 5 bytes
    std::fs::write(tmp.path().join("b"), "world!").unwrap(); // 6 bytes
    let (count, size) = dir_stats(tmp.path());
    assert_eq!(count, 2);
    assert_eq!(size, 11);
}

#[test]
fn dir_stats_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let (count, size) = dir_stats(tmp.path());
    assert_eq!(count, 0);
    assert_eq!(size, 0);
}

#[test]
fn dir_stats_nested() {
    let tmp = tempfile::tempdir().unwrap();
    let sub = tmp.path().join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("f"), "data").unwrap(); // 4 bytes
    std::fs::write(tmp.path().join("top"), "hi").unwrap(); // 2 bytes
    let (count, size) = dir_stats(tmp.path());
    assert_eq!(count, 2);
    assert_eq!(size, 6);
}

// ── FJ-1359: walkdir ──

#[test]
fn walkdir_lists_files() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("x.txt"), "abc").unwrap();
    let entries = walkdir(tmp.path()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, 3); // 3 bytes
}

#[test]
fn walkdir_recursive() {
    let tmp = tempfile::tempdir().unwrap();
    let d = tmp.path().join("a/b");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("deep.txt"), "deep").unwrap();
    std::fs::write(tmp.path().join("top.txt"), "top").unwrap();
    let entries = walkdir(tmp.path()).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn walkdir_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let entries = walkdir(tmp.path()).unwrap();
    assert!(entries.is_empty());
}

// ── FJ-1361: dry_run_sandbox_plan ──

#[test]
fn dry_run_collects_commands() {
    let plan = make_plan(vec![Some("echo step1"), None, Some("echo step3")]);
    let cmds = dry_run_sandbox_plan(&plan).unwrap();
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[0], "echo step1");
    assert_eq!(cmds[1], "echo step3");
}

#[test]
fn dry_run_empty_plan() {
    let plan = make_plan(vec![]);
    let cmds = dry_run_sandbox_plan(&plan).unwrap();
    assert!(cmds.is_empty());
}

#[test]
fn dry_run_all_informational() {
    let plan = make_plan(vec![None, None]);
    let cmds = dry_run_sandbox_plan(&plan).unwrap();
    assert!(cmds.is_empty());
}

// ── FJ-1361: validate_sandbox_commands ──

#[test]
fn validate_commands_all_safe() {
    let plan = make_plan(vec![Some("echo hello"), Some("ls /tmp")]);
    let errors = validate_sandbox_commands(&plan);
    assert!(errors.is_empty());
}

#[test]
fn validate_commands_with_none() {
    let plan = make_plan(vec![None, Some("echo ok"), None]);
    let errors = validate_sandbox_commands(&plan);
    assert!(errors.is_empty());
}

// ── FJ-1362: parse_provider ──

#[test]
fn parse_provider_all_valid() {
    let cases = [
        ("apt", ImportProvider::Apt),
        ("cargo", ImportProvider::Cargo),
        ("uv", ImportProvider::Uv),
        ("nix", ImportProvider::Nix),
        ("docker", ImportProvider::Docker),
        ("tofu", ImportProvider::Tofu),
        ("terraform", ImportProvider::Terraform),
        ("apr", ImportProvider::Apr),
    ];
    for (name, expected) in &cases {
        let result = parse_provider(name).unwrap();
        assert_eq!(result, *expected, "failed for {name}");
    }
}

#[test]
fn parse_provider_unknown() {
    let result = parse_provider("homebrew");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown"));
}

#[test]
fn parse_provider_case_sensitive() {
    let result = parse_provider("APT");
    assert!(result.is_err());
}

// ── FJ-1362: tempdir_for_reimport ──

#[test]
fn tempdir_path_contains_hash_prefix() {
    let path = tempdir_for_reimport("blake3:abcdef1234567890aabbccdd");
    assert!(path.display().to_string().contains("abcdef1234567890"));
    assert!(path
        .display()
        .to_string()
        .starts_with("/tmp/forjar-reimport-"));
}

#[test]
fn tempdir_path_strips_blake3() {
    let path = tempdir_for_reimport("blake3:1234567890abcdef");
    let s = path.display().to_string();
    assert!(!s.contains("blake3:"));
}

#[test]
fn tempdir_path_raw_hash() {
    let path = tempdir_for_reimport("aabbccdd11223344");
    let s = path.display().to_string();
    assert!(s.contains("aabbccdd11223344"));
}

#[test]
fn tempdir_path_short_hash() {
    let path = tempdir_for_reimport("ab");
    let s = path.display().to_string();
    assert!(s.contains("ab"));
}
