//! FJ-1300/1307/1347: Content chunking, input closures, store paths.
//! Usage: cargo test --test falsification_chunker_closure_path

use forjar::core::store::chunker::{chunk_bytes, reassemble, tree_hash, CHUNK_SIZE};
use forjar::core::store::closure::{all_closures, closure_hash, input_closure, ResourceInputs};
use forjar::core::store::path::{store_entry_path, store_path, STORE_BASE};
use std::collections::BTreeMap;

// ── FJ-1347: chunk_bytes ──

#[test]
fn chunk_small_data() {
    let data = b"hello forjar";
    let chunks = chunk_bytes(data);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].data, data);
    assert_eq!(chunks[0].hash.len(), 32);
}

#[test]
fn chunk_exact_boundary() {
    let data = vec![0u8; CHUNK_SIZE];
    let chunks = chunk_bytes(&data);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].data.len(), CHUNK_SIZE);
}

#[test]
fn chunk_two_chunks() {
    let data = vec![0u8; CHUNK_SIZE + 1];
    let chunks = chunk_bytes(&data);
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].data.len(), CHUNK_SIZE);
    assert_eq!(chunks[1].data.len(), 1);
}

#[test]
fn chunk_empty() {
    let chunks = chunk_bytes(b"");
    assert!(chunks.is_empty());
}

#[test]
fn chunk_hashes_differ() {
    let c1 = chunk_bytes(b"aaa");
    let c2 = chunk_bytes(b"bbb");
    assert_ne!(c1[0].hash, c2[0].hash);
}

// ── FJ-1347: reassemble ──

#[test]
fn reassemble_roundtrip() {
    let data = b"forjar content addressing is deterministic";
    let chunks = chunk_bytes(data);
    let reassembled = reassemble(&chunks);
    assert_eq!(reassembled, data);
}

#[test]
fn reassemble_multi_chunk() {
    let data = vec![42u8; CHUNK_SIZE * 3 + 100];
    let chunks = chunk_bytes(&data);
    assert_eq!(chunks.len(), 4);
    assert_eq!(reassemble(&chunks), data);
}

#[test]
fn reassemble_empty() {
    let result = reassemble(&[]);
    assert!(result.is_empty());
}

// ── FJ-1347: tree_hash ──

#[test]
fn tree_hash_deterministic() {
    let chunks = chunk_bytes(b"forjar merkle tree");
    let h1 = tree_hash(&chunks);
    let h2 = tree_hash(&chunks);
    assert_eq!(h1, h2);
}

#[test]
fn tree_hash_sensitive() {
    let c1 = chunk_bytes(b"data-a");
    let c2 = chunk_bytes(b"data-b");
    assert_ne!(tree_hash(&c1), tree_hash(&c2));
}

#[test]
fn tree_hash_empty() {
    let h = tree_hash(&[]);
    assert_eq!(h.len(), 32);
}

#[test]
fn tree_hash_multiple_chunks() {
    let data = vec![1u8; CHUNK_SIZE * 5];
    let chunks = chunk_bytes(&data);
    let h = tree_hash(&chunks);
    assert_eq!(h.len(), 32);
}

// ── FJ-1307: input_closure ──

fn graph(items: &[(&str, &[&str], &[&str])]) -> BTreeMap<String, ResourceInputs> {
    items
        .iter()
        .map(|(name, hashes, deps)| {
            (
                name.to_string(),
                ResourceInputs {
                    input_hashes: hashes.iter().map(|s| s.to_string()).collect(),
                    depends_on: deps.iter().map(|s| s.to_string()).collect(),
                },
            )
        })
        .collect()
}

#[test]
fn closure_no_deps() {
    let g = graph(&[("a", &["h1", "h2"], &[])]);
    let c = input_closure("a", &g);
    assert_eq!(c, vec!["h1", "h2"]);
}

#[test]
fn closure_transitive() {
    let g = graph(&[
        ("a", &["h1"], &[]),
        ("b", &["h2"], &["a"]),
        ("c", &["h3"], &["b"]),
    ]);
    let c = input_closure("c", &g);
    assert!(c.contains(&"h1".to_string()));
    assert!(c.contains(&"h2".to_string()));
    assert!(c.contains(&"h3".to_string()));
}

#[test]
fn closure_diamond() {
    let g = graph(&[
        ("base", &["h0"], &[]),
        ("left", &["h1"], &["base"]),
        ("right", &["h2"], &["base"]),
        ("top", &["h3"], &["left", "right"]),
    ]);
    let c = input_closure("top", &g);
    assert_eq!(c.len(), 4); // h0, h1, h2, h3 (deduplicated)
}

#[test]
fn closure_missing_resource() {
    let g = graph(&[("a", &["h1"], &["missing"])]);
    let c = input_closure("a", &g);
    assert!(c.contains(&"h1".to_string()));
    // "missing" just stops traversal — no error
}

#[test]
fn closure_cycle_safe() {
    let g = graph(&[("a", &["h1"], &["b"]), ("b", &["h2"], &["a"])]);
    let c = input_closure("a", &g);
    assert!(c.contains(&"h1".to_string()));
    assert!(c.contains(&"h2".to_string()));
}

// ── FJ-1307: closure_hash ──

#[test]
fn closure_hash_deterministic() {
    let c = vec!["h1".into(), "h2".into()];
    let h1 = closure_hash(&c);
    let h2 = closure_hash(&c);
    assert_eq!(h1, h2);
    assert!(h1.starts_with("blake3:"));
}

#[test]
fn closure_hash_sensitive() {
    let c1 = vec!["h1".into()];
    let c2 = vec!["h2".into()];
    assert_ne!(closure_hash(&c1), closure_hash(&c2));
}

// ── FJ-1307: all_closures ──

#[test]
fn all_closures_computes_all() {
    let g = graph(&[("a", &["h1"], &[]), ("b", &["h2"], &["a"])]);
    let closures = all_closures(&g);
    assert_eq!(closures.len(), 2);
    assert_eq!(closures["a"], vec!["h1"]);
    assert!(closures["b"].contains(&"h1".to_string()));
    assert!(closures["b"].contains(&"h2".to_string()));
}

// ── FJ-1300: store_path ──

#[test]
fn store_path_deterministic() {
    let h1 = store_path("recipe1", &["in1", "in2"], "x86_64", "apt");
    let h2 = store_path("recipe1", &["in1", "in2"], "x86_64", "apt");
    assert_eq!(h1, h2);
    assert!(h1.starts_with("blake3:"));
}

#[test]
fn store_path_sensitive_to_recipe() {
    let h1 = store_path("recipe-a", &["in1"], "x86_64", "apt");
    let h2 = store_path("recipe-b", &["in1"], "x86_64", "apt");
    assert_ne!(h1, h2);
}

#[test]
fn store_path_sensitive_to_arch() {
    let h1 = store_path("r", &["in1"], "x86_64", "apt");
    let h2 = store_path("r", &["in1"], "aarch64", "apt");
    assert_ne!(h1, h2);
}

#[test]
fn store_path_order_independent() {
    let h1 = store_path("r", &["b", "a", "c"], "x86_64", "apt");
    let h2 = store_path("r", &["a", "c", "b"], "x86_64", "apt");
    assert_eq!(h1, h2, "inputs should be sorted internally");
}

// ── FJ-1300: store_entry_path ──

#[test]
fn entry_path_strips_prefix() {
    let p = store_entry_path("blake3:abcdef1234567890");
    assert_eq!(p, format!("{STORE_BASE}/abcdef1234567890"));
}

#[test]
fn entry_path_no_prefix() {
    let p = store_entry_path("abcdef1234567890");
    assert_eq!(p, format!("{STORE_BASE}/abcdef1234567890"));
}

#[test]
fn store_base_is_var_lib() {
    assert_eq!(STORE_BASE, "/var/lib/forjar/store");
}
