//! Coverage tests for meta.rs — new_meta, write_meta/read_meta round-trip,
//! provenance, error cases.

use super::meta::*;

// ── new_meta ────────────────────────────────────────────────────

#[test]
fn new_meta_fields() {
    let m = new_meta(
        "blake3:abc",
        "blake3:recipe1",
        &["blake3:in1".to_string()],
        "x86_64",
        "apt",
    );
    assert_eq!(m.schema, "1.0");
    assert_eq!(m.store_hash, "blake3:abc");
    assert_eq!(m.recipe_hash, "blake3:recipe1");
    assert_eq!(m.input_hashes, vec!["blake3:in1"]);
    assert_eq!(m.arch, "x86_64");
    assert_eq!(m.provider, "apt");
    assert!(!m.created_at.is_empty());
    assert!(m.generator.contains("forjar"));
    assert!(m.references.is_empty());
    assert!(m.provenance.is_none());
}

#[test]
fn new_meta_empty_inputs() {
    let m = new_meta("blake3:abc", "blake3:r1", &[], "aarch64", "cargo");
    assert!(m.input_hashes.is_empty());
    assert_eq!(m.arch, "aarch64");
    assert_eq!(m.provider, "cargo");
}

// ── write_meta + read_meta round-trip ───────────────────────────

#[test]
fn write_read_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let entry_dir = dir.path().join("abc123");
    let mut meta = new_meta("blake3:abc123", "blake3:r", &[], "x86_64", "apt");
    meta.references = vec!["blake3:ref1".to_string()];

    write_meta(&entry_dir, &meta).unwrap();
    let loaded = read_meta(&entry_dir).unwrap();
    assert_eq!(loaded, meta);
}

#[test]
fn write_read_with_provenance() {
    let dir = tempfile::tempdir().unwrap();
    let entry_dir = dir.path().join("def456");
    let mut meta = new_meta("blake3:def456", "blake3:r", &[], "x86_64", "cargo");
    meta.provenance = Some(Provenance {
        origin_provider: "github".to_string(),
        origin_ref: Some("https://github.com/test/repo".to_string()),
        origin_hash: Some("abc123".to_string()),
        derived_from: None,
        derivation_depth: 0,
    });

    write_meta(&entry_dir, &meta).unwrap();
    let loaded = read_meta(&entry_dir).unwrap();
    assert_eq!(loaded.provenance, meta.provenance);
}

#[test]
fn write_read_with_derivation() {
    let dir = tempfile::tempdir().unwrap();
    let entry_dir = dir.path().join("ghi789");
    let mut meta = new_meta("blake3:ghi789", "blake3:r", &[], "x86_64", "nix");
    meta.provenance = Some(Provenance {
        origin_provider: "nix".to_string(),
        origin_ref: None,
        origin_hash: None,
        derived_from: Some("blake3:base_entry".to_string()),
        derivation_depth: 2,
    });

    write_meta(&entry_dir, &meta).unwrap();
    let loaded = read_meta(&entry_dir).unwrap();
    let prov = loaded.provenance.unwrap();
    assert_eq!(prov.derived_from, Some("blake3:base_entry".to_string()));
    assert_eq!(prov.derivation_depth, 2);
}

// ── read_meta: error cases ──────────────────────────────────────

#[test]
fn read_meta_nonexistent() {
    let dir = tempfile::tempdir().unwrap();
    let result = read_meta(&dir.path().join("nonexistent"));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cannot read"));
}

#[test]
fn read_meta_invalid_yaml() {
    let dir = tempfile::tempdir().unwrap();
    let entry_dir = dir.path().join("bad");
    std::fs::create_dir_all(&entry_dir).unwrap();
    std::fs::write(entry_dir.join("meta.yaml"), "invalid: yaml: [broken").unwrap();
    let result = read_meta(&entry_dir);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid meta.yaml"));
}

// ── write_meta: creates parent dirs ─────────────────────────────

#[test]
fn write_meta_creates_nested_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let entry_dir = dir.path().join("a/b/c/entry");
    let meta = new_meta("blake3:x", "blake3:r", &[], "x86_64", "apt");
    write_meta(&entry_dir, &meta).unwrap();
    assert!(entry_dir.join("meta.yaml").exists());
}

// ── StoreMeta/Provenance equality ───────────────────────────────

#[test]
fn provenance_equality() {
    let a = Provenance {
        origin_provider: "apt".to_string(),
        origin_ref: None,
        origin_hash: None,
        derived_from: None,
        derivation_depth: 0,
    };
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn store_meta_equality() {
    let a = new_meta("blake3:a", "blake3:r", &[], "x86_64", "apt");
    let b = a.clone();
    assert_eq!(a, b);
}

// ── write_meta overwrites existing ──────────────────────────────

#[test]
fn write_meta_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let entry_dir = dir.path().join("overwrite");
    let meta1 = new_meta("blake3:v1", "blake3:r", &[], "x86_64", "apt");
    write_meta(&entry_dir, &meta1).unwrap();

    let meta2 = new_meta("blake3:v2", "blake3:r", &[], "aarch64", "cargo");
    write_meta(&entry_dir, &meta2).unwrap();

    let loaded = read_meta(&entry_dir).unwrap();
    assert_eq!(loaded.store_hash, "blake3:v2");
    assert_eq!(loaded.arch, "aarch64");
}

// ── write_meta: temp file is cleaned up ─────────────────────────

#[test]
fn write_meta_no_tmp_file_left() {
    let dir = tempfile::tempdir().unwrap();
    let entry_dir = dir.path().join("clean");
    let meta = new_meta("blake3:c", "blake3:r", &[], "x86_64", "apt");
    write_meta(&entry_dir, &meta).unwrap();
    // Verify no .tmp file remains
    let tmp = entry_dir.join("meta.yaml.tmp");
    assert!(!tmp.exists());
}

// ── new_meta: multiple inputs ───────────────────────────────────

#[test]
fn new_meta_multiple_inputs() {
    let inputs = vec![
        "blake3:in1".to_string(),
        "blake3:in2".to_string(),
        "blake3:in3".to_string(),
    ];
    let m = new_meta("blake3:x", "blake3:r", &inputs, "x86_64", "apt");
    assert_eq!(m.input_hashes.len(), 3);
}

// ── read_meta: references preserved ─────────────────────────────

#[test]
fn write_read_preserves_references() {
    let dir = tempfile::tempdir().unwrap();
    let entry_dir = dir.path().join("refs");
    let mut meta = new_meta("blake3:r", "blake3:r", &[], "x86_64", "apt");
    meta.references = vec!["blake3:ref1".to_string(), "blake3:ref2".to_string()];
    write_meta(&entry_dir, &meta).unwrap();
    let loaded = read_meta(&entry_dir).unwrap();
    assert_eq!(loaded.references.len(), 2);
}

// ── provenance: all fields populated ────────────────────────────

#[test]
fn provenance_all_fields() {
    let dir = tempfile::tempdir().unwrap();
    let entry_dir = dir.path().join("full-prov");
    let mut meta = new_meta("blake3:fp", "blake3:r", &[], "x86_64", "nix");
    meta.provenance = Some(Provenance {
        origin_provider: "nix".to_string(),
        origin_ref: Some("nixpkgs/unstable".to_string()),
        origin_hash: Some("abc123".to_string()),
        derived_from: Some("blake3:parent".to_string()),
        derivation_depth: 3,
    });
    write_meta(&entry_dir, &meta).unwrap();
    let loaded = read_meta(&entry_dir).unwrap();
    let prov = loaded.provenance.unwrap();
    assert_eq!(prov.origin_provider, "nix");
    assert_eq!(prov.origin_ref, Some("nixpkgs/unstable".to_string()));
    assert_eq!(prov.origin_hash, Some("abc123".to_string()));
    assert_eq!(prov.derived_from, Some("blake3:parent".to_string()));
    assert_eq!(prov.derivation_depth, 3);
}
