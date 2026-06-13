//! Tests for FJ-1346: FAR binary format encode/decode.

use super::far::{
    decode_far_manifest, encode_far, FarFileEntry, FarManifest, FarProvenance, FAR_MAGIC,
};

fn sample_manifest() -> FarManifest {
    FarManifest {
        name: "numpy".to_string(),
        version: "1.26.4".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:aabbccdd".to_string(),
        tree_hash: "blake3:11223344".to_string(),
        file_count: 2,
        total_size: 100,
        files: vec![
            FarFileEntry {
                path: "lib/numpy/__init__.py".to_string(),
                size: 60,
                blake3: "blake3:aaaa".to_string(),
            },
            FarFileEntry {
                path: "lib/numpy/core.so".to_string(),
                size: 40,
                blake3: "blake3:bbbb".to_string(),
            },
        ],
        provenance: FarProvenance {
            origin_provider: "conda".to_string(),
            origin_ref: Some("conda-forge".to_string()),
            origin_hash: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 1.0.0".to_string(),
        },
        kernel_contracts: None,
    }
}

fn sample_chunks() -> Vec<([u8; 32], Vec<u8>)> {
    let data1 = vec![0xAA; 64];
    let data2 = vec![0xBB; 48];
    let h1 = blake3::hash(&data1);
    let h2 = blake3::hash(&data2);
    vec![(*h1.as_bytes(), data1), (*h2.as_bytes(), data2)]
}

#[test]
fn test_fj1346_magic_bytes() {
    assert_eq!(FAR_MAGIC.len(), 12);
    assert_eq!(&FAR_MAGIC[..10], b"FORJAR-FAR");
    assert_eq!(FAR_MAGIC[10], 0x00);
    assert_eq!(FAR_MAGIC[11], 0x01);
}

#[test]
fn test_fj1346_manifest_serde_roundtrip() {
    let m = sample_manifest();
    let yaml = serde_yaml_ng::to_string(&m).unwrap();
    let parsed: FarManifest = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(m, parsed);
}

#[test]
fn test_fj1346_encode_decode_roundtrip() {
    let manifest = sample_manifest();
    let chunks = sample_chunks();

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();

    // Verify magic at the start
    assert_eq!(&buf[..12], FAR_MAGIC);

    // Decode and verify
    let (decoded_manifest, decoded_entries) = decode_far_manifest(buf.as_slice()).unwrap();
    assert_eq!(decoded_manifest, manifest);
    assert_eq!(decoded_entries.len(), 2);

    // Verify chunk table entries
    assert_eq!(decoded_entries[0].hash, chunks[0].0);
    assert_eq!(decoded_entries[1].hash, chunks[1].0);
    assert_eq!(decoded_entries[0].offset, 0);
    assert!(decoded_entries[1].offset > 0);
}

#[test]
fn test_fj1346_empty_archive() {
    let manifest = FarManifest {
        name: "empty".to_string(),
        version: "0.0.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:0000".to_string(),
        tree_hash: "blake3:0000".to_string(),
        file_count: 0,
        total_size: 0,
        files: vec![],
        provenance: FarProvenance {
            origin_provider: "test".to_string(),
            origin_ref: None,
            origin_hash: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 1.0.0".to_string(),
        },
        kernel_contracts: None,
    };
    let chunks: Vec<([u8; 32], Vec<u8>)> = vec![];

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();

    let (decoded, entries) = decode_far_manifest(buf.as_slice()).unwrap();
    assert_eq!(decoded, manifest);
    assert!(entries.is_empty());
}

#[test]
fn test_fj1346_bad_magic_error() {
    let bad = b"NOT-A-FAR\x00\x00\x00extra_data_here";
    let result = decode_far_manifest(bad.as_slice());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid FAR magic"));
}

#[test]
fn test_fj1346_chunk_entry_offsets_sequential() {
    let manifest = sample_manifest();
    let data1 = vec![0xCC; 128];
    let data2 = vec![0xDD; 256];
    let data3 = vec![0xEE; 64];
    let h1 = blake3::hash(&data1);
    let h2 = blake3::hash(&data2);
    let h3 = blake3::hash(&data3);
    let chunks = vec![
        (*h1.as_bytes(), data1),
        (*h2.as_bytes(), data2),
        (*h3.as_bytes(), data3),
    ];

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();

    let (_, entries) = decode_far_manifest(buf.as_slice()).unwrap();
    assert_eq!(entries.len(), 3);

    // Offsets must be sequential: each starts after the previous
    for i in 1..entries.len() {
        assert_eq!(
            entries[i].offset,
            entries[i - 1].offset + entries[i - 1].length
        );
    }
}

#[test]
fn test_fj1346_provenance_optional_fields() {
    let m = FarManifest {
        name: "test".to_string(),
        version: "1.0".to_string(),
        arch: "aarch64".to_string(),
        store_hash: "blake3:xx".to_string(),
        tree_hash: "blake3:yy".to_string(),
        file_count: 0,
        total_size: 0,
        files: vec![],
        provenance: FarProvenance {
            origin_provider: "apt".to_string(),
            origin_ref: None,
            origin_hash: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 1.0.0".to_string(),
        },
        kernel_contracts: None,
    };
    let yaml = serde_yaml_ng::to_string(&m).unwrap();
    // Optional None fields should be omitted
    assert!(!yaml.contains("origin_ref"));
    assert!(!yaml.contains("origin_hash"));
}

#[test]
fn test_fj1346_truncated_input_error() {
    // Just the magic, nothing else
    let result = decode_far_manifest(FAR_MAGIC.as_slice());
    assert!(result.is_err());
}

// ── #154 (#17/#24, #18/#25): untrusted header lengths must not OOM/panic ──

/// Build a FAR header: magic + 8-byte LE manifest_len, optionally followed by
/// 8-byte LE chunk_count and arbitrary trailing bytes.
fn header_bytes(manifest_len: u64, rest: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(FAR_MAGIC);
    buf.extend_from_slice(&manifest_len.to_le_bytes());
    buf.extend_from_slice(rest);
    buf
}

#[test]
fn test_gh154_manifest_len_absurd_over_short_input_errors() {
    // Header claims a multi-GB manifest but supplies almost no bytes.
    // Must return Err (rejected by the MAX bound) — not OOM/abort/panic.
    let bad = header_bytes(u64::MAX, &[0xAB; 4]);
    let result = decode_far_manifest(bad.as_slice());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("manifest too large"));
}

#[test]
fn test_gh154_manifest_len_moderate_but_truncated_errors() {
    // A length within the sane bound but with no actual bytes following must
    // fail via the length-limited read (graceful Err), never a huge allocation.
    let bad = header_bytes(4096, &[]);
    let result = decode_far_manifest(bad.as_slice());
    assert!(result.is_err());
}

#[test]
fn test_gh154_chunk_count_absurd_over_short_input_errors() {
    // Valid magic + manifest, then an absurd chunk_count over a truncated table.
    // with_capacity(chunk_count * 48) used to capacity-overflow; now we grow the
    // Vec on each read so a truncated table fails gracefully via read_exact.
    let manifest = sample_manifest();
    let chunks: Vec<([u8; 32], Vec<u8>)> = vec![];
    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();
    // Overwrite the chunk_count field (immediately after the manifest) — find it
    // by truncating to right after the encoded chunk_count then patching. Simpler:
    // re-encode with empty chunks (chunk_count=0 sits at a known suffix) is awkward,
    // so craft a fresh minimal valid prefix and append a huge count.
    let mut compressed = Vec::new();
    let yaml = serde_yaml_ng::to_string(&manifest).unwrap();
    let cc = zstd::encode_all(yaml.as_bytes(), 3).unwrap();
    compressed.extend_from_slice(FAR_MAGIC);
    compressed.extend_from_slice(&(cc.len() as u64).to_le_bytes());
    compressed.extend_from_slice(&cc);
    compressed.extend_from_slice(&u64::MAX.to_le_bytes()); // absurd chunk_count
    let result = decode_far_manifest(compressed.as_slice());
    assert!(result.is_err());
}

#[test]
fn test_gh154_valid_roundtrip_still_works() {
    // Regression guard: the bounds checks must not break the happy path.
    let manifest = sample_manifest();
    let chunks = sample_chunks();
    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();
    let (decoded, entries) = decode_far_manifest(buf.as_slice()).unwrap();
    assert_eq!(decoded, manifest);
    assert_eq!(entries.len(), 2);
}

proptest::proptest! {
    /// #154: arbitrary header bytes (magic + manifest_len + chunk_count + junk)
    /// must never panic/OOM — decode either succeeds or returns Err.
    #[test]
    fn prop_gh154_arbitrary_header_no_panic(
        manifest_len in proptest::prelude::any::<u64>(),
        chunk_count in proptest::prelude::any::<u64>(),
        tail in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..64),
    ) {
        let mut buf = Vec::new();
        buf.extend_from_slice(FAR_MAGIC);
        buf.extend_from_slice(&manifest_len.to_le_bytes());
        buf.extend_from_slice(&chunk_count.to_le_bytes());
        buf.extend_from_slice(&tail);
        // Result is intentionally ignored: the property is "does not panic/abort".
        let _ = decode_far_manifest(buf.as_slice());
    }
}
