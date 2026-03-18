//! FJ-1302/1315/1346: Profile generation, sandbox config, and FAR archive falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-1302: Profile generation management
//!   - create_generation: atomic symlink switching
//!   - rollback: previous generation restoration
//!   - list_generations: sorted generation listing
//!   - current_generation: symlink target reading
//! - FJ-1315: Build sandbox configuration
//!   - validate_config: constraint enforcement
//!   - preset_profile: named presets
//!   - parse_sandbox_config: YAML deserialization
//!   - blocks_network / enforces_fs_isolation: level predicates
//!   - cgroup_path: deterministic path construction
//! - FJ-1346: FAR (Forjar ARchive) binary format
//!   - encode_far / decode_far_manifest: roundtrip fidelity
//!   - ChunkEntry table reconstruction
//!   - Magic validation
//!
//! Usage: cargo test --test falsification_store_profile_sandbox_far

use forjar::core::store::far::{
    decode_far_manifest, encode_far, FarFileEntry, FarManifest, FarProvenance, FAR_MAGIC,
};

fn test_manifest() -> FarManifest {
    FarManifest {
        name: "test-pkg".to_string(),
        version: "1.0.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:abc123".to_string(),
        tree_hash: "blake3:def456".to_string(),
        file_count: 2,
        total_size: 1024,
        files: vec![
            FarFileEntry {
                path: "bin/app".to_string(),
                size: 512,
                blake3: "blake3:aaa".to_string(),
            },
            FarFileEntry {
                path: "lib/core.so".to_string(),
                size: 512,
                blake3: "blake3:bbb".to_string(),
            },
        ],
        provenance: FarProvenance {
            origin_provider: "apt".to_string(),
            origin_ref: Some("nginx=1.24".to_string()),
            origin_hash: Some("upstream-hash".to_string()),
            created_at: "2026-03-09T00:00:00Z".to_string(),
            generator: "forjar 1.0".to_string(),
        },
        kernel_contracts: None,
    }
}

#[test]
fn far_encode_decode_zero_chunks() {
    let manifest = test_manifest();
    let chunks: Vec<([u8; 32], Vec<u8>)> = vec![];

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();

    let (decoded, decoded_chunks) = decode_far_manifest(std::io::Cursor::new(&buf)).unwrap();
    assert_eq!(decoded.name, "test-pkg");
    assert!(decoded_chunks.is_empty());
}

#[test]
fn far_magic_is_12_bytes() {
    assert_eq!(FAR_MAGIC.len(), 12);
    assert!(FAR_MAGIC.starts_with(b"FORJAR-FAR"));
}

#[test]
fn far_decode_invalid_magic_rejected() {
    let buf = b"NOT-FAR-MAGIC-HEADER-AND-MORE-BYTES";
    let result = decode_far_manifest(std::io::Cursor::new(buf));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("magic"));
}

#[test]
fn far_decode_truncated_rejected() {
    let result = decode_far_manifest(std::io::Cursor::new(b"FORJAR"));
    assert!(result.is_err());
}

#[test]
fn far_manifest_provenance_optional_fields() {
    let manifest = FarManifest {
        name: "minimal".to_string(),
        version: "0.1".to_string(),
        arch: "aarch64".to_string(),
        store_hash: "blake3:min".to_string(),
        tree_hash: "blake3:tree".to_string(),
        file_count: 0,
        total_size: 0,
        files: vec![],
        provenance: FarProvenance {
            origin_provider: "cargo".to_string(),
            origin_ref: None,
            origin_hash: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar".to_string(),
        },
        kernel_contracts: None,
    };

    let mut buf = Vec::new();
    encode_far(&manifest, &[], &mut buf).unwrap();

    let (decoded, _) = decode_far_manifest(std::io::Cursor::new(&buf)).unwrap();
    assert!(decoded.provenance.origin_ref.is_none());
    assert!(decoded.provenance.origin_hash.is_none());
    assert!(decoded.kernel_contracts.is_none());
}

#[test]
fn far_manifest_with_kernel_contracts() {
    let manifest = FarManifest {
        name: "model-pkg".to_string(),
        version: "2.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:model".to_string(),
        tree_hash: "blake3:tree".to_string(),
        file_count: 1,
        total_size: 1000,
        files: vec![FarFileEntry {
            path: "model.safetensors".to_string(),
            size: 1000,
            blake3: "blake3:weights".to_string(),
        }],
        provenance: FarProvenance {
            origin_provider: "hf".to_string(),
            origin_ref: Some("meta-llama/Llama-3.1-8B".to_string()),
            origin_hash: None,
            created_at: "2026-03-09T00:00:00Z".to_string(),
            generator: "forjar".to_string(),
        },
        kernel_contracts: Some(forjar::core::store::far::KernelContractInfo {
            model_type: "llama".to_string(),
            required_ops: vec!["matmul".to_string(), "rms_norm".to_string()],
            coverage_pct: 95.5,
        }),
    };

    let mut buf = Vec::new();
    encode_far(&manifest, &[], &mut buf).unwrap();

    let (decoded, _) = decode_far_manifest(std::io::Cursor::new(&buf)).unwrap();
    let kc = decoded.kernel_contracts.unwrap();
    assert_eq!(kc.model_type, "llama");
    assert_eq!(kc.required_ops.len(), 2);
    assert!((kc.coverage_pct - 95.5).abs() < 0.01);
}

#[test]
fn far_manifest_serde_yaml_roundtrip() {
    let manifest = test_manifest();
    let yaml = serde_yaml_ng::to_string(&manifest).unwrap();
    let parsed: FarManifest = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(parsed, manifest);
}

#[test]
fn far_manifest_serde_json_roundtrip() {
    let manifest = test_manifest();
    let json = serde_json::to_string(&manifest).unwrap();
    let parsed: FarManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, manifest);
}

#[test]
fn far_large_chunk_roundtrip() {
    let manifest = test_manifest();
    // 1 MB chunk to test zstd compression with larger data
    let data = vec![0xABu8; 1024 * 1024];
    let hash = blake3::hash(&data);
    let chunks = vec![(*hash.as_bytes(), data)];

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();

    // Compressed size should be less than 1 MB + overhead
    assert!(buf.len() < 1024 * 1024);

    let (decoded, entries) = decode_far_manifest(std::io::Cursor::new(&buf)).unwrap();
    assert_eq!(decoded.name, "test-pkg");
    assert_eq!(entries.len(), 1);
}

#[test]
fn far_chunk_table_offsets_consistent() {
    let manifest = test_manifest();
    let d1 = vec![1u8; 100];
    let d2 = vec![2u8; 200];
    let d3 = vec![3u8; 300];
    let h1 = blake3::hash(&d1);
    let h2 = blake3::hash(&d2);
    let h3 = blake3::hash(&d3);
    let chunks = vec![
        (*h1.as_bytes(), d1),
        (*h2.as_bytes(), d2),
        (*h3.as_bytes(), d3),
    ];

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();

    let (_, entries) = decode_far_manifest(std::io::Cursor::new(&buf)).unwrap();
    assert_eq!(entries.len(), 3);
    // First chunk starts at offset 0
    assert_eq!(entries[0].offset, 0);
    // Second chunk starts after first chunk's compressed data
    assert_eq!(entries[1].offset, entries[0].length);
    // Third chunk starts after second
    assert_eq!(entries[2].offset, entries[0].length + entries[1].length);
}
