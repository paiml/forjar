//! FJ-2001/1347/2102: State database, chunker, and OCI layer builder falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-2001: State database
//!   - open_state_db: schema creation with WAL + FTS5
//!   - schema_version / set_schema_version: pragma access
//!   - fts5_search: full-text search with hyphen sanitization
//!   - list_all_resources: non-FTS listing
//! - FJ-1347: Chunker
//!   - chunk_bytes: fixed-size 64KB splitting with BLAKE3
//!   - tree_hash: binary Merkle tree computation
//!   - reassemble: chunk reassembly to original data
//!   - chunk_directory: directory → FAR entry pipeline
//! - FJ-2102: OCI layer builder
//!   - build_layer: deterministic tar + dual digest
//!   - compute_dual_digest: BLAKE3 + SHA-256
//!   - write_oci_layout: layout directory structure
//!   - LayerEntry: file/dir normalization
//!
//! Usage: cargo test --test falsification_store_db_chunker_layer

use forjar::core::store::chunker::{chunk_bytes, reassemble, tree_hash, CHUNK_SIZE};
use forjar::core::store::db::{
    fts5_search, list_all_resources, open_state_db, schema_version, set_schema_version,
};
use forjar::core::store::layer_builder::{build_layer, compute_dual_digest, LayerEntry};
use forjar::core::types::OciLayerConfig;

// ============================================================================
// FJ-2001: open_state_db
// ============================================================================

#[test]
fn db_open_in_memory() {
    let conn = open_state_db(std::path::Path::new(":memory:")).unwrap();
    // Tables should exist
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='machines'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn db_open_creates_all_tables() {
    let conn = open_state_db(std::path::Path::new(":memory:")).unwrap();
    let tables: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };
    assert!(tables.contains(&"machines".to_string()));
    assert!(tables.contains(&"generations".to_string()));
    assert!(tables.contains(&"resources".to_string()));
    assert!(tables.contains(&"events".to_string()));
    assert!(tables.contains(&"destroy_log".to_string()));
    assert!(tables.contains(&"drift_findings".to_string()));
    assert!(tables.contains(&"ingest_cursor".to_string()));
}

#[test]
fn db_open_creates_fts5_tables() {
    let conn = open_state_db(std::path::Path::new(":memory:")).unwrap();
    let tables: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '%fts%' ORDER BY name")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };
    assert!(tables.iter().any(|t| t.contains("resources_fts")));
}

// ============================================================================
// FJ-2001: schema_version / set_schema_version
// ============================================================================

#[test]
fn db_schema_version_default_zero() {
    let conn = open_state_db(std::path::Path::new(":memory:")).unwrap();
    let ver = schema_version(&conn).unwrap();
    assert_eq!(ver, 0);
}

#[test]
fn db_schema_version_set_and_read() {
    let conn = open_state_db(std::path::Path::new(":memory:")).unwrap();
    set_schema_version(&conn, 42).unwrap();
    assert_eq!(schema_version(&conn).unwrap(), 42);
}

// ============================================================================
// FJ-2001: fts5_search
// ============================================================================

fn setup_fts_db() -> rusqlite::Connection {
    let conn = open_state_db(std::path::Path::new(":memory:")).unwrap();
    conn.execute(
        "INSERT INTO machines (name, first_seen, last_seen) VALUES ('m1', 'now', 'now')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO generations (generation_num, run_id, config_hash, created_at) VALUES (1, 'r1', 'h1', 'now')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO resources (resource_id, machine_id, generation_id, resource_type, status, applied_at, packages) \
         VALUES ('nginx-pkg', 1, 1, 'package', 'converged', 'now', 'nginx')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO resources (resource_id, machine_id, generation_id, resource_type, status, applied_at, path) \
         VALUES ('app-config', 1, 1, 'file', 'converged', 'now', '/etc/app/config.yaml')",
        [],
    )
    .unwrap();
    // Rebuild FTS index
    conn.execute(
        "INSERT INTO resources_fts(resources_fts) VALUES('rebuild')",
        [],
    )
    .unwrap();
    conn
}

#[test]
fn db_fts5_search_by_package() {
    let conn = setup_fts_db();
    let results = fts5_search(&conn, "nginx", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].resource_id, "nginx-pkg");
}

#[test]
fn db_fts5_search_by_path() {
    let conn = setup_fts_db();
    let results = fts5_search(&conn, "config", 10).unwrap();
    assert!(!results.is_empty());
}

#[test]
fn db_fts5_search_empty_query() {
    let conn = setup_fts_db();
    let results = fts5_search(&conn, "", 10).unwrap();
    assert!(results.is_empty());
}

#[test]
fn db_fts5_search_hyphen_safe() {
    // Hyphens in FTS5 are interpreted as NOT — our sanitizer should handle this
    let conn = setup_fts_db();
    let results = fts5_search(&conn, "nginx-pkg", 10).unwrap();
    // Should not error, even with hyphen
    assert!(!results.is_empty());
}

#[test]
fn db_list_all_resources() {
    let conn = setup_fts_db();
    let results = list_all_resources(&conn, 50).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn db_list_all_resources_with_limit() {
    let conn = setup_fts_db();
    let results = list_all_resources(&conn, 1).unwrap();
    assert_eq!(results.len(), 1);
}

// ============================================================================
// FJ-1347: chunk_bytes
// ============================================================================

#[test]
fn chunker_small_data_single_chunk() {
    let data = vec![0xABu8; 100];
    let chunks = chunk_bytes(&data);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].data.len(), 100);
    assert_eq!(chunks[0].hash, *blake3::hash(&data).as_bytes());
}

#[test]
fn chunker_exact_chunk_size() {
    let data = vec![0xCDu8; CHUNK_SIZE];
    let chunks = chunk_bytes(&data);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].data.len(), CHUNK_SIZE);
}

#[test]
fn chunker_two_chunks() {
    let data = vec![0xEFu8; CHUNK_SIZE + 1];
    let chunks = chunk_bytes(&data);
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].data.len(), CHUNK_SIZE);
    assert_eq!(chunks[1].data.len(), 1);
}

#[test]
fn chunker_empty_data() {
    let chunks = chunk_bytes(&[]);
    assert!(chunks.is_empty());
}

#[test]
fn chunker_hash_deterministic() {
    let data = vec![0x42u8; 200];
    let c1 = chunk_bytes(&data);
    let c2 = chunk_bytes(&data);
    assert_eq!(c1[0].hash, c2[0].hash);
}

// ============================================================================
// FJ-1347: tree_hash
// ============================================================================

#[test]
fn chunker_tree_hash_single_chunk() {
    let chunks = chunk_bytes(&[1, 2, 3]);
    let hash = tree_hash(&chunks);
    assert_eq!(hash, chunks[0].hash, "single chunk tree hash == chunk hash");
}

#[test]
fn chunker_tree_hash_two_chunks() {
    let data = vec![0xAAu8; CHUNK_SIZE * 2];
    let chunks = chunk_bytes(&data);
    let hash = tree_hash(&chunks);
    // Should be BLAKE3(chunk0_hash || chunk1_hash)
    let mut hasher = blake3::Hasher::new();
    hasher.update(&chunks[0].hash);
    hasher.update(&chunks[1].hash);
    let expected = *hasher.finalize().as_bytes();
    assert_eq!(hash, expected);
}

#[test]
fn chunker_tree_hash_empty() {
    let hash = tree_hash(&[]);
    let expected = *blake3::hash(b"").as_bytes();
    assert_eq!(hash, expected);
}

#[test]
fn chunker_tree_hash_odd_chunks() {
    // 3 chunks: two paired, one promoted
    let data = vec![0xBBu8; CHUNK_SIZE * 3];
    let chunks = chunk_bytes(&data);
    assert_eq!(chunks.len(), 3);
    let hash = tree_hash(&chunks);
    // hash should be BLAKE3(BLAKE3(c0||c1) || c2)
    let mut h01 = blake3::Hasher::new();
    h01.update(&chunks[0].hash);
    h01.update(&chunks[1].hash);
    let node01 = *h01.finalize().as_bytes();
    let mut root = blake3::Hasher::new();
    root.update(&node01);
    root.update(&chunks[2].hash);
    let expected = *root.finalize().as_bytes();
    assert_eq!(hash, expected);
}

#[test]
fn chunker_tree_hash_deterministic() {
    let data = vec![0xCCu8; CHUNK_SIZE * 5];
    let chunks = chunk_bytes(&data);
    let h1 = tree_hash(&chunks);
    let h2 = tree_hash(&chunks);
    assert_eq!(h1, h2);
}

// ============================================================================
// FJ-1347: reassemble
// ============================================================================

#[test]
fn chunker_reassemble_roundtrip() {
    let data = vec![0xDDu8; CHUNK_SIZE * 3 + 42];
    let chunks = chunk_bytes(&data);
    let reassembled = reassemble(&chunks);
    assert_eq!(reassembled, data);
}

#[test]
fn chunker_reassemble_single_chunk() {
    let data = b"hello world";
    let chunks = chunk_bytes(data);
    let reassembled = reassemble(&chunks);
    assert_eq!(reassembled, data);
}

#[test]
fn chunker_reassemble_empty() {
    let reassembled = reassemble(&[]);
    assert!(reassembled.is_empty());
}

// ============================================================================
// FJ-1347: chunk_directory
// ============================================================================

#[test]
fn chunker_chunk_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file1.txt"), "hello").unwrap();
    std::fs::write(dir.path().join("file2.txt"), "world").unwrap();

    let (chunks, entries) = forjar::core::store::chunker::chunk_directory(dir.path()).unwrap();
    assert!(!chunks.is_empty());
    assert_eq!(entries.len(), 2);
    // Entries should be sorted
    assert!(entries[0].path <= entries[1].path);
    // All entries have blake3: prefix
    for e in &entries {
        assert!(e.blake3.starts_with("blake3:"));
    }
}

// ============================================================================
// FJ-2102: compute_dual_digest
// ============================================================================

#[test]
fn layer_dual_digest_deterministic() {
    let data = b"hello OCI layer content";
    let d1 = compute_dual_digest(data);
    let d2 = compute_dual_digest(data);
    assert_eq!(d1.blake3, d2.blake3);
    assert_eq!(d1.sha256, d2.sha256);
    assert_eq!(d1.size_bytes, data.len() as u64);
}

#[test]
fn layer_dual_digest_different_data() {
    let d1 = compute_dual_digest(b"data-a");
    let d2 = compute_dual_digest(b"data-b");
    assert_ne!(d1.blake3, d2.blake3);
    assert_ne!(d1.sha256, d2.sha256);
}

#[test]
fn layer_dual_digest_empty() {
    let d = compute_dual_digest(b"");
    assert_eq!(d.size_bytes, 0);
    assert!(!d.blake3.is_empty());
    assert!(!d.sha256.is_empty());
}

// ============================================================================
// FJ-2102: LayerEntry
// ============================================================================

#[test]
fn layer_entry_file_normalizes_path() {
    let entry = LayerEntry::file("/etc/app/config.yaml", b"key: value", 0o644);
    // Leading slash should be stripped
    assert!(!entry.path.starts_with('/'));
    assert_eq!(entry.path, "etc/app/config.yaml");
    assert!(!entry.is_dir);
}

#[test]
fn layer_entry_dir_appends_slash() {
    let entry = LayerEntry::dir("etc/app", 0o755);
    assert!(entry.path.ends_with('/'));
    assert!(entry.is_dir);
    assert!(entry.content.is_empty());
}

#[test]
fn layer_entry_file_preserves_content() {
    let content = b"#!/bin/sh\nexec app";
    let entry = LayerEntry::file("usr/bin/app", content, 0o755);
    assert_eq!(entry.content, content);
    assert_eq!(entry.mode, 0o755);
}

// ============================================================================
// FJ-2102: build_layer
// ============================================================================

#[test]
fn layer_build_deterministic() {
    let entries = vec![
        LayerEntry::dir("etc", 0o755),
        LayerEntry::file("etc/app.conf", b"key=value\n", 0o644),
    ];
    let config = OciLayerConfig::default();
    let (r1, d1) = build_layer(&entries, &config).unwrap();
    let (r2, d2) = build_layer(&entries, &config).unwrap();
    assert_eq!(r1.digest, r2.digest);
    assert_eq!(r1.diff_id, r2.diff_id);
    assert_eq!(r1.store_hash, r2.store_hash);
    assert_eq!(d1, d2);
}

#[test]
fn layer_build_produces_valid_digests() {
    let entries = vec![LayerEntry::file("hello.txt", b"hello world", 0o644)];
    let config = OciLayerConfig::default();
    let (result, _data) = build_layer(&entries, &config).unwrap();
    assert!(result.digest.starts_with("sha256:"));
    assert!(result.diff_id.starts_with("sha256:"));
    assert!(result.store_hash.starts_with("blake3:"));
    assert!(result.compressed_size > 0);
    assert!(result.uncompressed_size > 0);
}

#[test]
fn layer_build_empty_entries() {
    let config = OciLayerConfig::default();
    let (result, _data) = build_layer(&[], &config).unwrap();
    assert!(result.digest.starts_with("sha256:"));
    assert_eq!(result.file_count, 0);
}

#[test]
fn layer_build_different_content_different_digests() {
    let config = OciLayerConfig::default();
    let entries_a = vec![LayerEntry::file("a.txt", b"aaa", 0o644)];
    let entries_b = vec![LayerEntry::file("a.txt", b"bbb", 0o644)];
    let (ra, _) = build_layer(&entries_a, &config).unwrap();
    let (rb, _) = build_layer(&entries_b, &config).unwrap();
    assert_ne!(ra.digest, rb.digest);
    assert_ne!(ra.store_hash, rb.store_hash);
}
