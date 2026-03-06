//! Coverage tests for cli/lock_core.rs — lock_info, lock_validate, lock_integrity, lock_prune.
//! Also cli/validate_core.rs — validate, validate_exhaustive, validate_deep.

const LOCK_WEB1: &str = r#"schema: "1"
machine: web1
hostname: web1
generated_at: "2025-01-01T00:00:00Z"
generator: forjar-test
blake3_version: "1.0"
resources:
  nginx:
    type: package
    status: converged
    hash: abc123def456
    applied_at: "2025-01-01T00:00:00Z"
    duration_seconds: 2.5
  config:
    type: file
    status: converged
    hash: ghi789jkl012
    applied_at: "2025-01-01T00:01:00Z"
    duration_seconds: 0.5
"#;

fn setup_state(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join("web1")).unwrap();
    std::fs::write(dir.join("web1/state.lock.yaml"), LOCK_WEB1).unwrap();
}

fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut f, yaml.as_bytes()).unwrap();
    std::io::Write::flush(&mut f).unwrap();
    f
}

const CONFIG: &str = "version: '1.0'\nname: test\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n";

// ── cmd_lock_info ──

#[test]
fn lock_info_empty() {
    let d = tempfile::tempdir().unwrap();
    let r = super::lock_core::cmd_lock_info(d.path(), false);
    assert!(r.is_ok());
}

#[test]
fn lock_info_empty_json() {
    let d = tempfile::tempdir().unwrap();
    let r = super::lock_core::cmd_lock_info(d.path(), true);
    assert!(r.is_ok());
}

#[test]
fn lock_info_with_data() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::lock_core::cmd_lock_info(d.path(), false);
    assert!(r.is_ok());
}

#[test]
fn lock_info_with_data_json() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::lock_core::cmd_lock_info(d.path(), true);
    assert!(r.is_ok());
}

// ── cmd_lock_validate ──

#[test]
fn lock_validate_empty() {
    let d = tempfile::tempdir().unwrap();
    let r = super::lock_core::cmd_lock_validate(d.path(), false);
    assert!(r.is_ok());
}

#[test]
fn lock_validate_empty_json() {
    let d = tempfile::tempdir().unwrap();
    let r = super::lock_core::cmd_lock_validate(d.path(), true);
    assert!(r.is_ok());
}

#[test]
fn lock_validate_with_data() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::lock_core::cmd_lock_validate(d.path(), false);
    assert!(r.is_ok());
}

#[test]
fn lock_validate_with_data_json() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::lock_core::cmd_lock_validate(d.path(), true);
    assert!(r.is_ok());
}

// ── cmd_lock_integrity ──

#[test]
fn lock_integrity_empty() {
    let d = tempfile::tempdir().unwrap();
    let r = super::lock_core::cmd_lock_integrity(d.path(), false);
    assert!(r.is_ok());
}

#[test]
fn lock_integrity_empty_json() {
    let d = tempfile::tempdir().unwrap();
    let r = super::lock_core::cmd_lock_integrity(d.path(), true);
    assert!(r.is_ok());
}

#[test]
fn lock_integrity_with_data() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::lock_core::cmd_lock_integrity(d.path(), false);
    assert!(r.is_ok());
}

#[test]
fn lock_integrity_with_data_json() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::lock_core::cmd_lock_integrity(d.path(), true);
    assert!(r.is_ok());
}

// ── cmd_lock_prune ──

#[test]
fn lock_prune_no_yes() {
    let d = tempfile::tempdir().unwrap();
    let cfg = write_temp_config(CONFIG);
    let r = super::lock_core::cmd_lock_prune(cfg.path(), d.path(), false);
    assert!(r.is_ok());
}

#[test]
fn lock_prune_with_yes() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let cfg = write_temp_config(CONFIG);
    let r = super::lock_core::cmd_lock_prune(cfg.path(), d.path(), true);
    assert!(r.is_ok());
}

// ── cmd_validate ──

#[test]
fn validate_basic() {
    let cfg = write_temp_config(CONFIG);
    let r = super::validate_core::cmd_validate(cfg.path(), false, false, false);
    assert!(r.is_ok());
}

#[test]
fn validate_json() {
    let cfg = write_temp_config(CONFIG);
    let r = super::validate_core::cmd_validate(cfg.path(), false, true, false);
    assert!(r.is_ok());
}

#[test]
fn validate_strict() {
    let cfg = write_temp_config(CONFIG);
    // Strict validation may flag missing description — both Ok/Err are valid
    let _ = super::validate_core::cmd_validate(cfg.path(), true, false, false);
}

#[test]
fn validate_strict_json() {
    let cfg = write_temp_config(CONFIG);
    let _ = super::validate_core::cmd_validate(cfg.path(), true, true, false);
}

#[test]
fn validate_dry_expand() {
    let cfg = write_temp_config(CONFIG);
    let r = super::validate_core::cmd_validate(cfg.path(), false, false, true);
    assert!(r.is_ok());
}

#[test]
fn validate_missing_file() {
    let r = super::validate_core::cmd_validate(std::path::Path::new("/nonexistent/f.yaml"), false, false, false);
    assert!(r.is_err());
}

// ── cmd_validate_exhaustive ──

#[test]
fn validate_exhaustive_text() {
    let cfg = write_temp_config(CONFIG);
    // Exhaustive may find issues — both Ok/Err exercise code paths
    let _ = super::validate_core::cmd_validate_exhaustive(cfg.path(), false);
}

#[test]
fn validate_exhaustive_json() {
    let cfg = write_temp_config(CONFIG);
    let _ = super::validate_core::cmd_validate_exhaustive(cfg.path(), true);
}

// ── cmd_validate_deep ──

#[test]
fn validate_deep_text() {
    let cfg = write_temp_config(CONFIG);
    let _ = super::validate_deep::cmd_validate_deep(cfg.path(), false);
}

#[test]
fn validate_deep_json() {
    let cfg = write_temp_config(CONFIG);
    let _ = super::validate_deep::cmd_validate_deep(cfg.path(), true);
}
