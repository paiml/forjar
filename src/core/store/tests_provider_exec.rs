//! Tests for FJ-1359: Provider execution bridge.

use super::provider::{
    all_providers, import_command, validate_import, ImportConfig, ImportProvider,
};
use super::provider_exec::{build_staging_script, hash_staging_dir, ExecutionContext};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn sample_config(provider: ImportProvider, reference: &str) -> ImportConfig {
    ImportConfig {
        provider,
        reference: reference.to_string(),
        version: Some("1.0".to_string()),
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    }
}

#[test]
fn all_providers_generate_valid_commands() {
    let refs = [
        (ImportProvider::Apt, "curl"),
        (ImportProvider::Cargo, "ripgrep"),
        (ImportProvider::Uv, "requests"),
        (ImportProvider::Nix, "nixpkgs#ripgrep"),
        (ImportProvider::Docker, "alpine"),
        (ImportProvider::Tofu, "./infra"),
        (ImportProvider::Terraform, "./infra"),
        (ImportProvider::Apr, "mistral-7b"),
    ];

    for (provider, reference) in refs {
        let config = sample_config(provider, reference);
        let cmd = import_command(&config);
        assert!(!cmd.is_empty(), "{provider:?} produced empty command");
        assert!(
            !cmd.contains('\0'),
            "{provider:?} command contains null byte"
        );
    }
}

#[test]
fn validate_import_rejects_empty_reference() {
    let config = ImportConfig {
        provider: ImportProvider::Apt,
        reference: String::new(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let errors = validate_import(&config);
    assert!(!errors.is_empty());
    assert!(errors[0].contains("reference"));
}

#[test]
fn validate_import_rejects_empty_arch() {
    let config = ImportConfig {
        provider: ImportProvider::Cargo,
        reference: "ripgrep".to_string(),
        version: None,
        arch: String::new(),
        options: BTreeMap::new(),
    };
    let errors = validate_import(&config);
    assert!(errors.iter().any(|e| e.contains("arch")));
}

#[test]
fn build_staging_script_sets_staging_env() {
    let staging = PathBuf::from("/tmp/forjar-staging/test123");
    let script = build_staging_script("apt-get install -y curl", &staging);
    assert!(script.contains("export STAGING='/tmp/forjar-staging/test123'"));
    assert!(script.contains("mkdir -p \"$STAGING\""));
    assert!(script.contains("apt-get install -y curl"));
}

#[test]
fn hash_staging_dir_with_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file1.txt"), b"hello").unwrap();
    std::fs::write(dir.path().join("file2.txt"), b"world").unwrap();

    let hash = hash_staging_dir(dir.path()).unwrap();
    assert!(hash.starts_with("blake3:"));
    assert_eq!(hash.len(), 64 + 7); // blake3: prefix + 64 hex chars
}

#[test]
fn hash_staging_dir_deterministic() {
    let dir1 = tempfile::tempdir().unwrap();
    std::fs::write(dir1.path().join("a.txt"), b"content").unwrap();
    std::fs::write(dir1.path().join("b.txt"), b"other").unwrap();

    let dir2 = tempfile::tempdir().unwrap();
    std::fs::write(dir2.path().join("a.txt"), b"content").unwrap();
    std::fs::write(dir2.path().join("b.txt"), b"other").unwrap();

    let hash1 = hash_staging_dir(dir1.path()).unwrap();
    let hash2 = hash_staging_dir(dir2.path()).unwrap();
    assert_eq!(
        hash1, hash2,
        "identical content must produce identical hash"
    );
}

#[test]
fn hash_staging_dir_empty_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let result = hash_staging_dir(dir.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("empty"));
}

#[test]
fn hash_staging_dir_with_subdirectories() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("nested.txt"), b"nested content").unwrap();
    std::fs::write(dir.path().join("root.txt"), b"root content").unwrap();

    let hash = hash_staging_dir(dir.path()).unwrap();
    assert!(hash.starts_with("blake3:"));
}

#[test]
fn hash_staging_dir_different_content_different_hash() {
    let dir1 = tempfile::tempdir().unwrap();
    std::fs::write(dir1.path().join("f.txt"), b"version1").unwrap();

    let dir2 = tempfile::tempdir().unwrap();
    std::fs::write(dir2.path().join("f.txt"), b"version2").unwrap();

    let hash1 = hash_staging_dir(dir1.path()).unwrap();
    let hash2 = hash_staging_dir(dir2.path()).unwrap();
    assert_ne!(
        hash1, hash2,
        "different content must produce different hash"
    );
}

#[test]
fn execution_context_fields() {
    let ctx = ExecutionContext {
        store_dir: PathBuf::from("/var/lib/forjar/store"),
        staging_dir: PathBuf::from("/tmp/forjar-staging"),
        machine: crate::core::types::Machine {
            hostname: "test".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: Vec::new(),
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        },
        timeout_secs: Some(300),
    };
    assert_eq!(ctx.store_dir.display().to_string(), "/var/lib/forjar/store");
    assert_eq!(ctx.timeout_secs, Some(300));
}

#[test]
fn all_eight_providers_covered() {
    let providers = all_providers();
    assert_eq!(providers.len(), 8);
}

#[test]
fn import_command_includes_version_for_apt() {
    let config = sample_config(ImportProvider::Apt, "curl");
    let cmd = import_command(&config);
    assert!(cmd.contains("=1.0"), "apt command should include =version");
}

#[test]
fn import_command_includes_version_for_cargo() {
    let config = sample_config(ImportProvider::Cargo, "ripgrep");
    let cmd = import_command(&config);
    assert!(
        cmd.contains("--version 1.0"),
        "cargo command should include --version"
    );
}

#[test]
fn import_command_includes_version_for_uv() {
    let config = sample_config(ImportProvider::Uv, "requests");
    let cmd = import_command(&config);
    assert!(
        cmd.contains("requests==1.0"),
        "uv command should include ==version"
    );
}

#[test]
fn staging_script_is_valid_shell() {
    let staging = PathBuf::from("/tmp/staging");
    let script = build_staging_script("echo hello", &staging);
    // Should be parseable as valid shell
    assert!(script.starts_with("export STAGING="));
    assert!(script.lines().count() >= 3);
}

#[test]
fn validate_nix_reference_format() {
    let config = ImportConfig {
        provider: ImportProvider::Nix,
        reference: "invalid".to_string(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let errors = validate_import(&config);
    assert!(
        errors.iter().any(|e| e.contains("flake format")),
        "nix without # should warn"
    );
}

#[test]
fn validate_docker_no_spaces() {
    let config = ImportConfig {
        provider: ImportProvider::Docker,
        reference: "has space".to_string(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let errors = validate_import(&config);
    assert!(errors.iter().any(|e| e.contains("spaces")));
}

#[test]
fn hash_staging_preserves_filename_in_hash() {
    // Different filenames with same content should produce different hashes
    let dir1 = tempfile::tempdir().unwrap();
    std::fs::write(dir1.path().join("alpha.txt"), b"same").unwrap();

    let dir2 = tempfile::tempdir().unwrap();
    std::fs::write(dir2.path().join("beta.txt"), b"same").unwrap();

    let hash1 = hash_staging_dir(dir1.path()).unwrap();
    let hash2 = hash_staging_dir(dir2.path()).unwrap();
    assert_ne!(
        hash1, hash2,
        "different filenames should produce different hashes"
    );
}

// ===== provider_exec helper function tests =====

use super::provider_exec::{atomic_move_to_store, dir_stats, walkdir};

#[test]
fn atomic_move_to_store_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let staging = dir.path().join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::write(staging.join("file.txt"), b"test").unwrap();

    let target = dir.path().join("store/abc123/content");
    atomic_move_to_store(&staging, &target).unwrap();

    assert!(target.join("file.txt").exists());
    assert!(!staging.exists());
}

#[test]
fn atomic_move_to_store_creates_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let staging = dir.path().join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::write(staging.join("data.bin"), b"data").unwrap();

    let target = dir.path().join("deep/nested/store/content");
    atomic_move_to_store(&staging, &target).unwrap();

    assert!(target.join("data.bin").exists());
}

#[test]
fn dir_stats_counts_files_and_bytes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"hello").unwrap();
    std::fs::write(dir.path().join("b.txt"), b"world!").unwrap();

    let (count, size) = dir_stats(dir.path());
    assert_eq!(count, 2);
    assert_eq!(size, 11); // 5 + 6
}

#[test]
fn dir_stats_recursive() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("nested.txt"), b"nested").unwrap();
    std::fs::write(dir.path().join("root.txt"), b"root").unwrap();

    let (count, _size) = dir_stats(dir.path());
    assert_eq!(count, 2);
}

#[test]
fn dir_stats_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let (count, size) = dir_stats(dir.path());
    assert_eq!(count, 0);
    assert_eq!(size, 0);
}

#[test]
fn walkdir_returns_files_with_sizes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"alpha").unwrap();
    let sub = dir.path().join("inner");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("b.txt"), b"beta").unwrap();

    let results = walkdir(dir.path()).unwrap();
    assert_eq!(results.len(), 2);
    let total_size: u64 = results.iter().map(|(_, s)| *s).sum();
    assert_eq!(total_size, 9); // 5 + 4
}

#[test]
fn walkdir_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let results = walkdir(dir.path()).unwrap();
    assert!(results.is_empty());
}

#[test]
fn walkdir_nonexistent_dir_errors() {
    let result = walkdir(std::path::Path::new("/nonexistent_walkdir_test"));
    assert!(result.is_err());
}

#[test]
fn build_staging_script_preserves_command() {
    let script = build_staging_script(
        "apt-get install -y nginx=1.24.0",
        &PathBuf::from("/tmp/staging-test"),
    );
    assert!(script.contains("nginx=1.24.0"));
    assert!(script.contains("STAGING"));
}

#[test]
fn hash_staging_with_deeply_nested_structure() {
    let dir = tempfile::tempdir().unwrap();
    let deep = dir.path().join("a/b/c/d");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("leaf.txt"), b"leaf node").unwrap();

    let hash = hash_staging_dir(dir.path()).unwrap();
    assert!(hash.starts_with("blake3:"));
}
