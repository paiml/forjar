//! Tests for FJ-1310: Lock file format.

use super::lockfile::{
    check_completeness, check_staleness, parse_lockfile, read_lockfile, write_lockfile, LockFile,
    Pin,
};
use std::collections::BTreeMap;

fn sample_lockfile() -> LockFile {
    let mut pins = BTreeMap::new();
    pins.insert(
        "nginx".to_string(),
        Pin {
            provider: "apt".to_string(),
            version: Some("1.24.0-1ubuntu1".to_string()),
            hash: "blake3:abc123".to_string(),
            git_rev: None,
            pin_type: None,
        },
    );
    pins.insert(
        "ripgrep".to_string(),
        Pin {
            provider: "cargo".to_string(),
            version: Some("14.1.0".to_string()),
            hash: "blake3:def456".to_string(),
            git_rev: None,
            pin_type: None,
        },
    );
    LockFile {
        schema: "1.0".to_string(),
        pins,
    }
}

#[test]
fn test_fj1310_parse_lockfile() {
    let yaml = r#"
schema: "1.0"
pins:
  nginx:
    provider: apt
    version: "1.24.0"
    hash: "blake3:abc123"
  my-recipe:
    provider: recipe
    hash: "blake3:def456"
    git_rev: "a1b2c3d4"
    type: recipe
"#;
    let lf = parse_lockfile(yaml).unwrap();
    assert_eq!(lf.schema, "1.0");
    assert_eq!(lf.pins.len(), 2);
    assert_eq!(lf.pins["nginx"].provider, "apt");
    assert_eq!(lf.pins["nginx"].version, Some("1.24.0".to_string()));
    assert_eq!(lf.pins["my-recipe"].git_rev, Some("a1b2c3d4".to_string()));
    assert_eq!(lf.pins["my-recipe"].pin_type, Some("recipe".to_string()));
}

#[test]
fn test_fj1310_parse_invalid_yaml() {
    let result = parse_lockfile("not: [valid: yaml");
    assert!(result.is_err());
}

#[test]
fn test_fj1310_serde_roundtrip() {
    let lf = sample_lockfile();
    let yaml = serde_yaml_ng::to_string(&lf).unwrap();
    let parsed: LockFile = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(lf, parsed);
}

#[test]
fn test_fj1310_optional_fields_omitted() {
    let lf = sample_lockfile();
    let yaml = serde_yaml_ng::to_string(&lf).unwrap();
    assert!(!yaml.contains("git_rev"));
    assert!(!yaml.contains("type:"));
}

#[test]
fn test_fj1310_write_read_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.inputs.lock.yaml");
    let lf = sample_lockfile();
    write_lockfile(&path, &lf).unwrap();

    assert!(path.exists());
    let read_back = read_lockfile(&path).unwrap();
    assert_eq!(lf, read_back);
}

#[test]
fn test_fj1310_write_creates_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub").join("deep").join("lock.yaml");
    let lf = sample_lockfile();
    write_lockfile(&path, &lf).unwrap();
    assert!(path.exists());
}

#[test]
fn test_fj1310_write_atomic_rename() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lock.yaml");
    let lf = sample_lockfile();
    write_lockfile(&path, &lf).unwrap();
    // Temp file should not remain
    let tmp = path.with_extension("lock.yaml.tmp");
    assert!(!tmp.exists());
}

#[test]
fn test_fj1310_read_not_found() {
    let result = read_lockfile(std::path::Path::new("/no/such/lock.yaml"));
    assert!(result.is_err());
}

#[test]
fn test_fj1310_staleness_none() {
    let lf = sample_lockfile();
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:abc123".to_string());
    current.insert("ripgrep".to_string(), "blake3:def456".to_string());
    let stale = check_staleness(&lf, &current);
    assert!(stale.is_empty());
}

#[test]
fn test_fj1310_staleness_detected() {
    let lf = sample_lockfile();
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:CHANGED".to_string());
    current.insert("ripgrep".to_string(), "blake3:def456".to_string());
    let stale = check_staleness(&lf, &current);
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].name, "nginx");
    assert_eq!(stale[0].locked_hash, "blake3:abc123");
    assert_eq!(stale[0].current_hash, "blake3:CHANGED");
}

#[test]
fn test_fj1310_staleness_missing_current() {
    let lf = sample_lockfile();
    let current = BTreeMap::new(); // no current hashes
    let stale = check_staleness(&lf, &current);
    // Missing from current is not stale (just not resolved yet)
    assert!(stale.is_empty());
}

#[test]
fn test_fj1310_completeness_all_present() {
    let lf = sample_lockfile();
    let inputs = vec!["nginx".to_string(), "ripgrep".to_string()];
    let missing = check_completeness(&lf, &inputs);
    assert!(missing.is_empty());
}

#[test]
fn test_fj1310_completeness_missing() {
    let lf = sample_lockfile();
    let inputs = vec![
        "nginx".to_string(),
        "ripgrep".to_string(),
        "python".to_string(),
    ];
    let missing = check_completeness(&lf, &inputs);
    assert_eq!(missing, vec!["python"]);
}

#[test]
fn test_fj1310_completeness_empty_lock() {
    let lf = LockFile {
        schema: "1.0".to_string(),
        pins: BTreeMap::new(),
    };
    let inputs = vec!["nginx".to_string()];
    let missing = check_completeness(&lf, &inputs);
    assert_eq!(missing, vec!["nginx"]);
}

#[test]
fn test_fj1310_completeness_empty_inputs() {
    let lf = sample_lockfile();
    let missing = check_completeness(&lf, &[]);
    assert!(missing.is_empty());
}

#[test]
fn test_fj1310_empty_lockfile() {
    let lf = LockFile {
        schema: "1.0".to_string(),
        pins: BTreeMap::new(),
    };
    let yaml = serde_yaml_ng::to_string(&lf).unwrap();
    let parsed = parse_lockfile(&yaml).unwrap();
    assert_eq!(parsed.pins.len(), 0);
}
