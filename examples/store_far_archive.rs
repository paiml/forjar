//! FAR (Forjar ARchive) binary format — encode, decode, inspect, verify.
//!
//! Demonstrates the FAR archive format used for content-addressed
//! distribution of store entries, kernel contracts, and model artifacts.
//!
//! Run: `cargo run --example store_far_archive`

use forjar::core::store::chunker::{chunk_bytes, tree_hash, CHUNK_SIZE};
use forjar::core::store::far::{
    decode_far_manifest, encode_far, FarFileEntry, FarManifest, FarProvenance, KernelContractInfo,
    FAR_MAGIC,
};

fn main() {
    println!("=== FAR Archive Format Demo ===\n");
    demo_magic_and_layout();
    demo_encode_decode();
    demo_chunking_merkle();
    demo_kernel_contracts();
    demo_streaming_decode();
    demo_integrity_verification();
    println!("\n=== All FAR archive demos passed ===");
}

/// 1. FAR magic bytes and binary layout.
fn demo_magic_and_layout() {
    println!("--- 1. FAR Magic and Binary Layout ---");
    assert_eq!(FAR_MAGIC.len(), 12, "FAR magic must be 12 bytes");
    assert_eq!(&FAR_MAGIC[..10], b"FORJAR-FAR");
    println!(
        "  Magic: {:?} ({} bytes)",
        std::str::from_utf8(&FAR_MAGIC[..10]).unwrap(),
        FAR_MAGIC.len()
    );
    println!("  Layout: magic(12) → manifest_len(8) → zstd(manifest)");
    println!("        → chunk_count(8) → chunk_table(48*N)");
    println!("        → zstd(chunks) → sig_len(8) → sig");
    println!("  Binary format verified\n");
}

/// 2. Encode and decode a FAR archive roundtrip.
fn demo_encode_decode() {
    println!("--- 2. Encode/Decode Roundtrip ---");

    let manifest = sample_manifest(None);
    let data = b"hello forjar store";
    let hash = *blake3::hash(data).as_bytes();
    let chunks = vec![(hash, data.to_vec())];

    // Encode
    let mut buffer = Vec::new();
    encode_far(&manifest, &chunks, &mut buffer).unwrap();
    println!("  Encoded: {} bytes ({} chunk)", buffer.len(), chunks.len());

    // Verify magic at start
    assert_eq!(&buffer[..12], FAR_MAGIC);
    println!("  Magic verified at offset 0");

    // Decode
    let (decoded_manifest, decoded_chunks) = decode_far_manifest(buffer.as_slice()).unwrap();
    assert_eq!(decoded_manifest, manifest);
    assert_eq!(decoded_chunks.len(), 1);
    assert_eq!(decoded_chunks[0].hash, hash);
    println!(
        "  Decoded: {} v{} ({} files, {} chunks)",
        decoded_manifest.name,
        decoded_manifest.version,
        decoded_manifest.file_count,
        decoded_chunks.len()
    );
    println!("  Roundtrip verified\n");
}

/// 3. Chunking and Merkle tree hashing.
fn demo_chunking_merkle() {
    println!("--- 3. Chunking and Merkle Tree ---");
    println!("  CHUNK_SIZE = {} bytes (64KB)", CHUNK_SIZE);

    // Single chunk
    let small = vec![42u8; 100];
    let chunks = chunk_bytes(&small);
    assert_eq!(chunks.len(), 1);
    println!("  100 bytes → {} chunk", chunks.len());

    // Multiple chunks
    let large = vec![0u8; CHUNK_SIZE * 3 + 500];
    let chunks = chunk_bytes(&large);
    assert_eq!(chunks.len(), 4);
    println!(
        "  {} bytes → {} chunks (3 full + 1 partial)",
        large.len(),
        chunks.len()
    );

    // Merkle tree
    let root = tree_hash(&chunks);
    let root2 = tree_hash(&chunks);
    assert_eq!(root, root2, "deterministic Merkle root");
    let hex_root: String = root[..8].iter().map(|b| format!("{b:02x}")).collect();
    println!("  Merkle root: blake3:{hex_root}...");

    // Different data → different root
    let other = chunk_bytes(&[1u8; CHUNK_SIZE * 2]);
    let other_root = tree_hash(&other);
    assert_ne!(root, other_root);
    println!("  Different data → different Merkle root");

    // Empty
    let empty_root = tree_hash(&[]);
    let expected = *blake3::hash(b"").as_bytes();
    assert_eq!(empty_root, expected);
    println!("  Empty chunks → blake3('') Merkle root");
    println!("  Chunking and Merkle tree verified\n");
}

/// 4. FAR with kernel contract metadata.
fn demo_kernel_contracts() {
    println!("--- 4. Kernel Contract Metadata ---");

    let kernel_info = KernelContractInfo {
        model_type: "llama".to_string(),
        required_ops: vec![
            "rmsnorm".into(),
            "silu".into(),
            "rope".into(),
            "swiglu".into(),
            "gqa".into(),
            "softmax".into(),
            "matmul".into(),
        ],
        coverage_pct: 85.7,
    };

    let manifest = sample_manifest(Some(kernel_info));
    let chunks = vec![(*blake3::hash(b"contract").as_bytes(), b"contract".to_vec())];

    let mut buffer = Vec::new();
    encode_far(&manifest, &chunks, &mut buffer).unwrap();

    let (decoded, _) = decode_far_manifest(buffer.as_slice()).unwrap();
    let kc = decoded.kernel_contracts.unwrap();
    assert_eq!(kc.model_type, "llama");
    assert_eq!(kc.required_ops.len(), 7);
    assert!((kc.coverage_pct - 85.7).abs() < 0.01);
    println!(
        "  Model: {} ({} ops, {:.1}% coverage)",
        kc.model_type,
        kc.required_ops.len(),
        kc.coverage_pct
    );
    println!("  Ops: {}", kc.required_ops.join(", "));
    println!("  Kernel contract metadata roundtrip verified\n");
}

/// 5. Streaming decode (manifest only, no full load).
fn demo_streaming_decode() {
    println!("--- 5. Streaming Decode ---");

    // Create archive with many chunks
    let data = vec![99u8; CHUNK_SIZE * 5];
    let chunk_list = chunk_bytes(&data);
    let chunk_pairs: Vec<([u8; 32], Vec<u8>)> = chunk_list
        .iter()
        .map(|c| (c.hash, c.data.clone()))
        .collect();

    let manifest = FarManifest {
        name: "large-payload".to_string(),
        version: "1.0.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:abc123".to_string(),
        tree_hash: format!(
            "blake3:{}",
            tree_hash(&chunk_list)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        file_count: 1,
        total_size: data.len() as u64,
        files: vec![FarFileEntry {
            path: "payload.bin".to_string(),
            size: data.len() as u64,
            blake3: format!(
                "blake3:{}",
                blake3::hash(&data)
                    .as_bytes()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
        }],
        provenance: FarProvenance {
            origin_provider: "test".to_string(),
            origin_ref: None,
            origin_hash: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            generator: "forjar-example".to_string(),
        },
        kernel_contracts: None,
    };

    let mut buffer = Vec::new();
    encode_far(&manifest, &chunk_pairs, &mut buffer).unwrap();
    println!(
        "  Archive size: {} bytes ({} chunks, {} raw bytes)",
        buffer.len(),
        chunk_pairs.len(),
        data.len()
    );

    // Decode only manifest and chunk table (no chunk data read)
    let (m, entries) = decode_far_manifest(buffer.as_slice()).unwrap();
    assert_eq!(m.name, "large-payload");
    assert_eq!(entries.len(), 5);
    println!(
        "  Streaming decode: manifest + {} chunk entries",
        entries.len()
    );
    println!("  Chunk table:");
    for (i, e) in entries.iter().enumerate() {
        println!(
            "    [{i}] hash={}... offset={} len={}",
            e.hash[..4]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            e.offset,
            e.length
        );
    }
    println!("  Streaming decode verified\n");
}

/// 6. Integrity verification: detect corruption.
fn demo_integrity_verification() {
    println!("--- 6. Integrity Verification ---");

    // Valid archive
    let manifest = sample_manifest(None);
    let data = b"integrity test data";
    let hash = *blake3::hash(data).as_bytes();
    let chunks = vec![(hash, data.to_vec())];

    let mut buffer = Vec::new();
    encode_far(&manifest, &chunks, &mut buffer).unwrap();

    // Valid decode
    let result = decode_far_manifest(buffer.as_slice());
    assert!(result.is_ok());
    println!("  Valid archive: decode OK");

    // Corrupt magic
    let mut corrupted = buffer.clone();
    corrupted[0] = 0xFF;
    let result = decode_far_manifest(corrupted.as_slice());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid FAR magic"));
    println!("  Corrupt magic: rejected (invalid FAR magic)");

    // Truncated archive
    let truncated = &buffer[..12]; // only magic
    let result = decode_far_manifest(truncated);
    assert!(result.is_err());
    println!("  Truncated archive: rejected");

    // Verify chunk hash matches
    let (_, entries) = decode_far_manifest(buffer.as_slice()).unwrap();
    assert_eq!(entries[0].hash, hash);
    let recomputed = *blake3::hash(data).as_bytes();
    assert_eq!(entries[0].hash, recomputed);
    println!("  Chunk hash verification: BLAKE3 matches");
    println!("  Integrity verification passed\n");
}

fn sample_manifest(kernel: Option<KernelContractInfo>) -> FarManifest {
    FarManifest {
        name: "nginx".to_string(),
        version: "1.24.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:abc123".to_string(),
        tree_hash: "blake3:def456".to_string(),
        file_count: 1,
        total_size: 18,
        files: vec![FarFileEntry {
            path: "bin/nginx".to_string(),
            size: 18,
            blake3: "blake3:aabbcc".to_string(),
        }],
        provenance: FarProvenance {
            origin_provider: "apt".to_string(),
            origin_ref: Some("nginx".to_string()),
            origin_hash: Some("sha256:original".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            generator: "forjar-example".to_string(),
        },
        kernel_contracts: kernel,
    }
}
