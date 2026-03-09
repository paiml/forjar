//! FJ-1307/1304/1301: Input closure, reference scanning, and store metadata falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-1307: Input closure tracking
//!   - input_closure: transitive dependency resolution
//!   - closure_hash: deterministic composite hash
//!   - all_closures: batch closure computation
//!   - Cycle protection
//! - FJ-1304: Reference scanning for store paths
//!   - scan_file_refs: BLAKE3 hash detection in file content
//!   - is_valid_blake3_hash: validation
//!   - scan_directory_refs: recursive directory scanning
//! - FJ-1301: Store metadata
//!   - new_meta: field initialization
//!   - write_meta / read_meta roundtrip
//!   - StoreMeta serde (YAML)
//!
//! Usage: cargo test --test falsification_store_closure_ref

use forjar::core::store::closure::{all_closures, closure_hash, input_closure, ResourceInputs};
use forjar::core::store::meta::{new_meta, read_meta, write_meta, Provenance, StoreMeta};
use forjar::core::store::reference::{is_valid_blake3_hash, scan_directory_refs, scan_file_refs};
use std::collections::{BTreeMap, BTreeSet};

// ============================================================================
// FJ-1307: input_closure
// ============================================================================

#[test]
fn closure_single_resource_no_deps() {
    let mut graph = BTreeMap::new();
    graph.insert(
        "A".into(),
        ResourceInputs {
            input_hashes: vec!["h1".into(), "h2".into()],
            depends_on: vec![],
        },
    );
    let closure = input_closure("A", &graph);
    assert_eq!(closure, vec!["h1", "h2"]);
}

#[test]
fn closure_transitive_deps() {
    let mut graph = BTreeMap::new();
    graph.insert(
        "A".into(),
        ResourceInputs {
            input_hashes: vec!["ha".into()],
            depends_on: vec![],
        },
    );
    graph.insert(
        "B".into(),
        ResourceInputs {
            input_hashes: vec!["hb".into()],
            depends_on: vec!["A".into()],
        },
    );
    graph.insert(
        "C".into(),
        ResourceInputs {
            input_hashes: vec!["hc".into()],
            depends_on: vec!["B".into()],
        },
    );
    let closure = input_closure("C", &graph);
    assert!(closure.contains(&"ha".to_string()));
    assert!(closure.contains(&"hb".to_string()));
    assert!(closure.contains(&"hc".to_string()));
    assert_eq!(closure.len(), 3);
}

#[test]
fn closure_diamond_deduplicates() {
    let mut graph = BTreeMap::new();
    graph.insert(
        "A".into(),
        ResourceInputs {
            input_hashes: vec!["shared".into()],
            depends_on: vec![],
        },
    );
    graph.insert(
        "B".into(),
        ResourceInputs {
            input_hashes: vec!["hb".into()],
            depends_on: vec!["A".into()],
        },
    );
    graph.insert(
        "C".into(),
        ResourceInputs {
            input_hashes: vec!["hc".into()],
            depends_on: vec!["A".into()],
        },
    );
    graph.insert(
        "D".into(),
        ResourceInputs {
            input_hashes: vec!["hd".into()],
            depends_on: vec!["B".into(), "C".into()],
        },
    );
    let closure = input_closure("D", &graph);
    // shared appears once despite two paths to A
    assert_eq!(
        closure.iter().filter(|h| *h == "shared").count(),
        1,
        "diamond should not duplicate shared inputs"
    );
    assert_eq!(closure.len(), 4); // shared, hb, hc, hd
}

#[test]
fn closure_cycle_protection() {
    let mut graph = BTreeMap::new();
    graph.insert(
        "A".into(),
        ResourceInputs {
            input_hashes: vec!["ha".into()],
            depends_on: vec!["B".into()],
        },
    );
    graph.insert(
        "B".into(),
        ResourceInputs {
            input_hashes: vec!["hb".into()],
            depends_on: vec!["A".into()],
        },
    );
    // Should not infinite loop
    let closure = input_closure("A", &graph);
    assert!(closure.contains(&"ha".to_string()));
    assert!(closure.contains(&"hb".to_string()));
}

#[test]
fn closure_missing_resource_returns_empty() {
    let graph = BTreeMap::new();
    let closure = input_closure("MISSING", &graph);
    assert!(closure.is_empty());
}

// ============================================================================
// FJ-1307: closure_hash determinism
// ============================================================================

#[test]
fn closure_hash_deterministic() {
    let closure = vec!["h1".into(), "h2".into(), "h3".into()];
    let h1 = closure_hash(&closure);
    let h2 = closure_hash(&closure);
    assert_eq!(h1, h2, "same closure should produce same hash");
}

#[test]
fn closure_hash_differs_for_different_inputs() {
    let c1 = vec!["h1".into(), "h2".into()];
    let c2 = vec!["h1".into(), "h3".into()];
    assert_ne!(closure_hash(&c1), closure_hash(&c2));
}

// ============================================================================
// FJ-1307: all_closures
// ============================================================================

#[test]
fn all_closures_batch() {
    let mut graph = BTreeMap::new();
    graph.insert(
        "A".into(),
        ResourceInputs {
            input_hashes: vec!["ha".into()],
            depends_on: vec![],
        },
    );
    graph.insert(
        "B".into(),
        ResourceInputs {
            input_hashes: vec!["hb".into()],
            depends_on: vec!["A".into()],
        },
    );
    let closures = all_closures(&graph);
    assert_eq!(closures.len(), 2);
    assert_eq!(closures["A"], vec!["ha"]);
    assert!(closures["B"].contains(&"ha".to_string()));
    assert!(closures["B"].contains(&"hb".to_string()));
}

// ============================================================================
// FJ-1304: is_valid_blake3_hash
// ============================================================================

#[test]
fn valid_blake3_hash() {
    let hash = format!("blake3:{}", "a".repeat(64));
    assert!(is_valid_blake3_hash(&hash));
}

#[test]
fn invalid_blake3_no_prefix() {
    let hash = "a".repeat(64);
    assert!(!is_valid_blake3_hash(&hash));
}

#[test]
fn invalid_blake3_short_hash() {
    let hash = format!("blake3:{}", "a".repeat(63));
    assert!(!is_valid_blake3_hash(&hash));
}

#[test]
fn invalid_blake3_non_hex() {
    let hash = format!("blake3:{}", "g".repeat(64));
    assert!(!is_valid_blake3_hash(&hash));
}

#[test]
fn invalid_blake3_empty() {
    assert!(!is_valid_blake3_hash(""));
    assert!(!is_valid_blake3_hash("blake3:"));
}

// ============================================================================
// FJ-1304: scan_file_refs
// ============================================================================

#[test]
fn scan_refs_finds_known_hash() {
    let hash = format!("blake3:{}", "a".repeat(64));
    let content = format!("path: /store/{hash}/content");
    let mut known = BTreeSet::new();
    known.insert(hash.clone());
    let refs = scan_file_refs(content.as_bytes(), &known);
    assert!(refs.contains(&hash));
}

#[test]
fn scan_refs_ignores_unknown_hash() {
    let hash = format!("blake3:{}", "a".repeat(64));
    let content = format!("path: /store/{hash}/content");
    let known = BTreeSet::new(); // empty
    let refs = scan_file_refs(content.as_bytes(), &known);
    assert!(refs.is_empty());
}

#[test]
fn scan_refs_multiple_in_one_file() {
    let h1 = format!("blake3:{}", "a".repeat(64));
    let h2 = format!("blake3:{}", "b".repeat(64));
    let content = format!("dep1: {h1}\ndep2: {h2}");
    let mut known = BTreeSet::new();
    known.insert(h1.clone());
    known.insert(h2.clone());
    let refs = scan_file_refs(content.as_bytes(), &known);
    assert_eq!(refs.len(), 2);
}

#[test]
fn scan_refs_empty_content() {
    let known = BTreeSet::new();
    let refs = scan_file_refs(b"", &known);
    assert!(refs.is_empty());
}

// ============================================================================
// FJ-1304: scan_directory_refs
// ============================================================================

#[test]
fn scan_dir_refs_recursive() {
    let dir = tempfile::tempdir().unwrap();
    let h1 = format!("blake3:{}", "c".repeat(64));

    // Create nested file with store reference
    let sub = dir.path().join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("file.txt"), format!("ref: {h1}")).unwrap();

    let mut known = BTreeSet::new();
    known.insert(h1.clone());

    let refs = scan_directory_refs(dir.path(), &known).unwrap();
    assert!(refs.contains(&h1));
}

#[test]
fn scan_dir_refs_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let known = BTreeSet::new();
    let refs = scan_directory_refs(dir.path(), &known).unwrap();
    assert!(refs.is_empty());
}

// ============================================================================
// FJ-1301: new_meta
// ============================================================================

#[test]
fn new_meta_fields() {
    let meta = new_meta("sh", "rh", &["h1".into()], "x86_64", "apt");
    assert_eq!(meta.schema, "1.0");
    assert_eq!(meta.store_hash, "sh");
    assert_eq!(meta.recipe_hash, "rh");
    assert_eq!(meta.input_hashes, vec!["h1"]);
    assert_eq!(meta.arch, "x86_64");
    assert_eq!(meta.provider, "apt");
    assert!(!meta.created_at.is_empty());
    assert!(meta.generator.starts_with("forjar"));
    assert!(meta.references.is_empty());
    assert!(meta.provenance.is_none());
}

// ============================================================================
// FJ-1301: write_meta / read_meta roundtrip
// ============================================================================

#[test]
fn write_read_meta_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let entry_dir = dir.path().join("store").join("entry1");

    let meta = new_meta(
        "sh1",
        "rh1",
        &["h1".into(), "h2".into()],
        "aarch64",
        "cargo",
    );
    write_meta(&entry_dir, &meta).unwrap();

    let loaded = read_meta(&entry_dir).unwrap();
    assert_eq!(loaded.store_hash, "sh1");
    assert_eq!(loaded.recipe_hash, "rh1");
    assert_eq!(loaded.input_hashes, vec!["h1", "h2"]);
    assert_eq!(loaded.arch, "aarch64");
    assert_eq!(loaded.provider, "cargo");
}

#[test]
fn write_meta_with_provenance() {
    let dir = tempfile::tempdir().unwrap();
    let entry_dir = dir.path().join("entry2");

    let mut meta = new_meta("sh2", "rh2", &[], "x86_64", "apt");
    meta.provenance = Some(Provenance {
        origin_provider: "apt".into(),
        origin_ref: Some("nginx=1.24.0-2".into()),
        origin_hash: Some("upstream-hash".into()),
        derived_from: None,
        derivation_depth: 0,
    });
    write_meta(&entry_dir, &meta).unwrap();

    let loaded = read_meta(&entry_dir).unwrap();
    let prov = loaded.provenance.unwrap();
    assert_eq!(prov.origin_provider, "apt");
    assert_eq!(prov.origin_ref.as_deref(), Some("nginx=1.24.0-2"));
    assert_eq!(prov.origin_hash.as_deref(), Some("upstream-hash"));
}

#[test]
fn read_meta_missing_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let result = read_meta(dir.path());
    assert!(result.is_err());
}

// ============================================================================
// FJ-1301: StoreMeta serde roundtrip
// ============================================================================

#[test]
fn store_meta_yaml_roundtrip() {
    let meta = new_meta("sh", "rh", &["h1".into()], "x86_64", "apt");
    let yaml = serde_yaml_ng::to_string(&meta).unwrap();
    let parsed: StoreMeta = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(parsed.store_hash, "sh");
}

#[test]
fn store_meta_json_roundtrip() {
    let meta = new_meta("sh", "rh", &[], "x86_64", "cargo");
    let json = serde_json::to_string(&meta).unwrap();
    let parsed: StoreMeta = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.provider, "cargo");
}
