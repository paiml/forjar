//! Tests for FJ-1301: store metadata with provenance.

use super::meta::{new_meta, read_meta, write_meta, Provenance, StoreMeta};

fn sample_meta() -> StoreMeta {
    StoreMeta {
        schema: "1.0".to_string(),
        store_hash: "blake3:aabb".to_string(),
        recipe_hash: "blake3:ccdd".to_string(),
        input_hashes: vec!["blake3:1111".to_string(), "blake3:2222".to_string()],
        arch: "x86_64".to_string(),
        provider: "apt".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 1.0.0".to_string(),
        references: vec!["blake3:ref1".to_string()],
        provenance: None,
    }
}

#[test]
fn test_fj1301_serde_roundtrip() {
    let meta = sample_meta();
    let yaml = serde_yaml_ng::to_string(&meta).unwrap();
    let parsed: StoreMeta = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(meta, parsed);
}

#[test]
fn test_fj1301_yaml_format() {
    let meta = sample_meta();
    let yaml = serde_yaml_ng::to_string(&meta).unwrap();
    assert!(yaml.contains("store_hash:"));
    assert!(yaml.contains("recipe_hash:"));
    assert!(yaml.contains("input_hashes:"));
    assert!(yaml.contains("blake3:aabb"));
}

#[test]
fn test_fj1301_write_read_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let store_dir = dir.path().join("entry");
    let meta = sample_meta();
    write_meta(&store_dir, &meta).unwrap();
    let loaded = read_meta(&store_dir).unwrap();
    assert_eq!(meta, loaded);
}

#[test]
fn test_fj1301_atomic_write_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let store_dir = dir.path().join("entry");
    let meta = sample_meta();
    write_meta(&store_dir, &meta).unwrap();
    assert!(store_dir.join("meta.yaml").exists());
    assert!(!store_dir.join("meta.yaml.tmp").exists());
}

#[test]
fn test_fj1301_read_missing_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let result = read_meta(dir.path());
    assert!(result.is_err());
}

#[test]
fn test_fj1301_provenance_none_omitted() {
    let meta = sample_meta();
    let yaml = serde_yaml_ng::to_string(&meta).unwrap();
    assert!(
        !yaml.contains("provenance:"),
        "None provenance must be omitted"
    );
}

#[test]
fn test_fj1301_provenance_full() {
    let mut meta = sample_meta();
    meta.provenance = Some(Provenance {
        origin_provider: "cargo".to_string(),
        origin_ref: Some("https://crates.io/crates/serde".to_string()),
        origin_hash: Some("abc123".to_string()),
        derived_from: Some("blake3:parent".to_string()),
        derivation_depth: 2,
    });
    let yaml = serde_yaml_ng::to_string(&meta).unwrap();
    let parsed: StoreMeta = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(meta.provenance, parsed.provenance);
    assert_eq!(parsed.provenance.unwrap().derivation_depth, 2);
}

#[test]
fn test_fj1301_provenance_minimal() {
    let mut meta = sample_meta();
    meta.provenance = Some(Provenance {
        origin_provider: "apt".to_string(),
        origin_ref: None,
        origin_hash: None,
        derived_from: None,
        derivation_depth: 0,
    });
    let yaml = serde_yaml_ng::to_string(&meta).unwrap();
    let parsed: StoreMeta = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(parsed.provenance.unwrap().origin_provider, "apt");
}

#[test]
fn test_fj1301_new_meta_constructor() {
    let meta = new_meta(
        "blake3:store",
        "blake3:recipe",
        &["blake3:in1".to_string()],
        "aarch64",
        "cargo",
    );
    assert_eq!(meta.schema, "1.0");
    assert_eq!(meta.store_hash, "blake3:store");
    assert_eq!(meta.arch, "aarch64");
    assert_eq!(meta.provider, "cargo");
    assert!(meta.provenance.is_none());
    assert!(meta.references.is_empty());
}

#[test]
fn test_fj1301_write_creates_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let deep = dir.path().join("a").join("b").join("c");
    let meta = sample_meta();
    write_meta(&deep, &meta).unwrap();
    assert!(deep.join("meta.yaml").exists());
}
