//! FJ-1360/1363: Convert execution, cache SSH command generation.
//! Usage: cargo test --test falsification_convert_cache_exec

use forjar::core::store::cache::CacheSource;
use forjar::core::store::cache_exec::{pull_command, push_command};
use forjar::core::store::convert::{
    ChangeType, ConversionChange, ConversionReport, ResourceConversion,
};
use forjar::core::store::convert_exec::apply_conversion;
use forjar::core::store::purity::PurityLevel;
use std::path::Path;

// ── helpers ──

fn ssh_source(host: &str, port: Option<u16>) -> CacheSource {
    CacheSource::Ssh {
        host: host.into(),
        user: "cache".into(),
        path: "/var/lib/forjar/cache".into(),
        port,
    }
}

fn local_source(path: &str) -> CacheSource {
    CacheSource::Local { path: path.into() }
}

fn write_yaml(dir: &Path, content: &str) -> std::path::PathBuf {
    let config_path = dir.join("forjar.yaml");
    std::fs::write(&config_path, content).unwrap();
    config_path
}

fn change(ct: ChangeType, desc: &str) -> ConversionChange {
    ConversionChange {
        change_type: ct,
        description: desc.into(),
    }
}

fn resource_conv(
    name: &str,
    provider: &str,
    current: PurityLevel,
    target: PurityLevel,
    auto_changes: Vec<ConversionChange>,
) -> ResourceConversion {
    ResourceConversion {
        name: name.into(),
        provider: provider.into(),
        current_purity: current,
        target_purity: target,
        auto_changes,
        manual_changes: vec![],
    }
}

// ── FJ-1360: pull_command ──

#[test]
fn pull_command_ssh_default_port() {
    let src = ssh_source("cache1.example.com", None);
    let staging = Path::new("/tmp/staging-abc123");
    let cmd = pull_command(&src, "blake3:abc123def456", staging);
    assert!(cmd.contains("rsync"));
    assert!(cmd.contains("cache@cache1.example.com"));
    assert!(cmd.contains("abc123def456"));
    // No custom SSH port flag (mkdir -p is fine)
    assert!(!cmd.contains("ssh -p "));
}

#[test]
fn pull_command_ssh_custom_port() {
    let src = ssh_source("cache2.internal", Some(2222));
    let staging = Path::new("/tmp/staging-xyz");
    let cmd = pull_command(&src, "blake3:deadbeef", staging);
    assert!(cmd.contains("-p 2222"));
    assert!(cmd.contains("cache@cache2.internal"));
}

#[test]
fn pull_command_ssh_strips_blake3_prefix() {
    let src = ssh_source("host", None);
    let staging = Path::new("/tmp/s");
    let cmd = pull_command(&src, "blake3:aabbccdd", staging);
    assert!(cmd.contains("aabbccdd/"));
    assert!(!cmd.contains("blake3:"));
}

#[test]
fn pull_command_ssh_no_prefix() {
    let src = ssh_source("host", None);
    let staging = Path::new("/tmp/s");
    let cmd = pull_command(&src, "rawhashthing", staging);
    assert!(cmd.contains("rawhashthing/"));
}

#[test]
fn pull_command_local() {
    let src = local_source("/mnt/cache");
    let staging = Path::new("/tmp/staging-local");
    let cmd = pull_command(&src, "blake3:abc123", staging);
    assert!(cmd.contains("cp -a"));
    assert!(cmd.contains("/mnt/cache/abc123"));
    assert!(cmd.contains("mkdir -p"));
}

// ── FJ-1360: push_command ──

#[test]
fn push_command_ssh_default_port() {
    let src = ssh_source("cache1.example.com", None);
    let store_dir = Path::new("/var/lib/forjar/store");
    let cmd = push_command(&src, "blake3:abc123", store_dir);
    assert!(cmd.contains("rsync"));
    assert!(cmd.contains("cache@cache1.example.com"));
    assert!(cmd.contains("/var/lib/forjar/store/abc123/"));
    assert!(!cmd.contains("-p "));
}

#[test]
fn push_command_ssh_custom_port() {
    let src = ssh_source("host", Some(2222));
    let store_dir = Path::new("/store");
    let cmd = push_command(&src, "blake3:deadbeef", store_dir);
    assert!(cmd.contains("-p 2222"));
    assert!(cmd.contains("/store/deadbeef/"));
}

#[test]
fn push_command_local() {
    let src = local_source("/backup/cache");
    let store_dir = Path::new("/var/lib/forjar/store");
    let cmd = push_command(&src, "blake3:ffee", store_dir);
    assert!(cmd.contains("cp -a"));
    assert!(cmd.contains("/var/lib/forjar/store/ffee"));
    assert!(cmd.contains("/backup/cache/ffee"));
}

#[test]
fn push_command_strips_blake3_prefix() {
    let src = ssh_source("host", None);
    let store_dir = Path::new("/store");
    let cmd = push_command(&src, "blake3:abcdef1234567890", store_dir);
    assert!(!cmd.contains("blake3:"));
    assert!(cmd.contains("abcdef1234567890"));
}

#[test]
fn push_command_no_prefix() {
    let src = ssh_source("host", None);
    let store_dir = Path::new("/store");
    let cmd = push_command(&src, "rawhash", store_dir);
    assert!(cmd.contains("rawhash"));
}

// ── FJ-1363: apply_conversion — no changes ──

#[test]
fn apply_conversion_no_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = write_yaml(
        tmp.path(),
        "resources:\n  - name: nginx\n    type: package\n",
    );
    let report = ConversionReport {
        resources: vec![],
        current_purity: PurityLevel::Impure,
        projected_purity: PurityLevel::Impure,
        auto_change_count: 0,
        manual_change_count: 0,
    };
    let result = apply_conversion(&config_path, &report).unwrap();
    assert_eq!(result.changes_applied, 0);
    assert_eq!(result.lock_pins_generated, 0);
    assert_eq!(result.new_purity, PurityLevel::Impure);
}

// ── FJ-1363: apply_conversion — version pin ──

#[test]
fn apply_conversion_adds_version_pin() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = write_yaml(
        tmp.path(),
        "resources:\n  - name: nginx\n    type: package\n",
    );
    let report = ConversionReport {
        resources: vec![resource_conv(
            "nginx",
            "apt",
            PurityLevel::Impure,
            PurityLevel::Pinned,
            vec![change(ChangeType::AddVersionPin, "Add version pin")],
        )],
        current_purity: PurityLevel::Impure,
        projected_purity: PurityLevel::Pinned,
        auto_change_count: 1,
        manual_change_count: 0,
    };
    let result = apply_conversion(&config_path, &report).unwrap();
    assert_eq!(result.changes_applied, 1);
    assert_eq!(result.new_purity, PurityLevel::Pinned);

    let updated = std::fs::read_to_string(&config_path).unwrap();
    assert!(updated.contains("version"));
}

// ── FJ-1363: apply_conversion — store flag ──

#[test]
fn apply_conversion_adds_store_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = write_yaml(
        tmp.path(),
        "resources:\n  - name: myapp\n    type: file\n    version: '1.0'\n",
    );
    let report = ConversionReport {
        resources: vec![resource_conv(
            "myapp",
            "cargo",
            PurityLevel::Pinned,
            PurityLevel::Pure,
            vec![change(ChangeType::EnableStore, "Enable store")],
        )],
        current_purity: PurityLevel::Pinned,
        projected_purity: PurityLevel::Pure,
        auto_change_count: 1,
        manual_change_count: 0,
    };
    let result = apply_conversion(&config_path, &report).unwrap();
    assert_eq!(result.changes_applied, 1);

    let updated = std::fs::read_to_string(&config_path).unwrap();
    assert!(updated.contains("store"));
}

// ── FJ-1363: apply_conversion — lock pin ──

#[test]
fn apply_conversion_generates_lock_pin() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = write_yaml(
        tmp.path(),
        "resources:\n  - name: curl\n    type: package\n",
    );
    let report = ConversionReport {
        resources: vec![resource_conv(
            "curl",
            "apt",
            PurityLevel::Impure,
            PurityLevel::Pinned,
            vec![change(ChangeType::GenerateLockPin, "Generate lock pin")],
        )],
        current_purity: PurityLevel::Impure,
        projected_purity: PurityLevel::Pinned,
        auto_change_count: 1,
        manual_change_count: 0,
    };
    let result = apply_conversion(&config_path, &report).unwrap();
    assert_eq!(result.changes_applied, 1);
    assert_eq!(result.lock_pins_generated, 1);

    let lock_path = tmp.path().join("forjar.inputs.lock.yaml");
    assert!(lock_path.exists());
}

// ── FJ-1363: apply_conversion — backup ──

#[test]
fn apply_conversion_creates_backup() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = write_yaml(tmp.path(), "resources:\n  - name: foo\n    type: package\n");
    let report = ConversionReport {
        resources: vec![resource_conv(
            "foo",
            "apt",
            PurityLevel::Impure,
            PurityLevel::Pinned,
            vec![change(ChangeType::AddVersionPin, "pin")],
        )],
        current_purity: PurityLevel::Impure,
        projected_purity: PurityLevel::Pinned,
        auto_change_count: 1,
        manual_change_count: 0,
    };
    let result = apply_conversion(&config_path, &report).unwrap();
    assert!(result.backup_path.exists());
    assert!(result.backup_path.display().to_string().contains(".bak"));
}

// ── FJ-1363: apply_conversion — multiple changes ──

#[test]
fn apply_conversion_multiple_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = write_yaml(
        tmp.path(),
        "resources:\n  - name: a\n    type: package\n  - name: b\n    type: file\n    version: '2.0'\n",
    );
    let report = ConversionReport {
        resources: vec![
            resource_conv(
                "a",
                "apt",
                PurityLevel::Impure,
                PurityLevel::Pinned,
                vec![change(ChangeType::AddVersionPin, "pin a")],
            ),
            resource_conv(
                "b",
                "cargo",
                PurityLevel::Pinned,
                PurityLevel::Pure,
                vec![change(ChangeType::EnableStore, "store b")],
            ),
        ],
        current_purity: PurityLevel::Impure,
        projected_purity: PurityLevel::Pure,
        auto_change_count: 2,
        manual_change_count: 0,
    };
    let result = apply_conversion(&config_path, &report).unwrap();
    assert_eq!(result.changes_applied, 2);
}

// ── FJ-1363: apply_conversion — nonexistent resource ──

#[test]
fn apply_conversion_nonexistent_resource_zero_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = write_yaml(
        tmp.path(),
        "resources:\n  - name: real\n    type: package\n",
    );
    let report = ConversionReport {
        resources: vec![resource_conv(
            "nonexistent",
            "apt",
            PurityLevel::Impure,
            PurityLevel::Pinned,
            vec![change(ChangeType::AddVersionPin, "pin")],
        )],
        current_purity: PurityLevel::Impure,
        projected_purity: PurityLevel::Pinned,
        auto_change_count: 1,
        manual_change_count: 0,
    };
    let result = apply_conversion(&config_path, &report).unwrap();
    assert_eq!(result.changes_applied, 0);
}

// ── FJ-1363: apply_conversion — invalid YAML ──

#[test]
fn apply_conversion_invalid_yaml() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = write_yaml(tmp.path(), "{{{{invalid yaml");
    let report = ConversionReport {
        resources: vec![],
        current_purity: PurityLevel::Impure,
        projected_purity: PurityLevel::Impure,
        auto_change_count: 1,
        manual_change_count: 0,
    };
    let result = apply_conversion(&config_path, &report);
    assert!(result.is_err());
}

// ── FJ-1363: apply_conversion — missing file ──

#[test]
fn apply_conversion_missing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = tmp.path().join("nonexistent.yaml");
    let report = ConversionReport {
        resources: vec![],
        current_purity: PurityLevel::Impure,
        projected_purity: PurityLevel::Impure,
        auto_change_count: 1,
        manual_change_count: 0,
    };
    let result = apply_conversion(&config_path, &report);
    assert!(result.is_err());
}

// ── FJ-1363: apply_conversion — already has version ──

#[test]
fn apply_conversion_skip_existing_version() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = write_yaml(
        tmp.path(),
        "resources:\n  - name: nginx\n    type: package\n    version: '1.24'\n",
    );
    let report = ConversionReport {
        resources: vec![resource_conv(
            "nginx",
            "apt",
            PurityLevel::Pinned,
            PurityLevel::Pinned,
            vec![change(ChangeType::AddVersionPin, "pin")],
        )],
        current_purity: PurityLevel::Pinned,
        projected_purity: PurityLevel::Pinned,
        auto_change_count: 1,
        manual_change_count: 0,
    };
    let result = apply_conversion(&config_path, &report).unwrap();
    assert_eq!(result.changes_applied, 0);
}

// ── FJ-1363: apply_conversion — already has store ──

#[test]
fn apply_conversion_skip_existing_store() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = write_yaml(
        tmp.path(),
        "resources:\n  - name: myapp\n    type: file\n    store: true\n",
    );
    let report = ConversionReport {
        resources: vec![resource_conv(
            "myapp",
            "cargo",
            PurityLevel::Pure,
            PurityLevel::Pure,
            vec![change(ChangeType::EnableStore, "store")],
        )],
        current_purity: PurityLevel::Pure,
        projected_purity: PurityLevel::Pure,
        auto_change_count: 1,
        manual_change_count: 0,
    };
    let result = apply_conversion(&config_path, &report).unwrap();
    assert_eq!(result.changes_applied, 0);
}
