//! Tests for FJ-1347: fixed-size chunker with BLAKE3 hashing.

use super::chunker::{chunk_bytes, chunk_directory, reassemble, tree_hash, ChunkData, CHUNK_SIZE};

#[test]
fn test_fj1347_chunk_size_constant() {
    assert_eq!(CHUNK_SIZE, 65536);
}

#[test]
fn test_fj1347_empty_input() {
    let chunks = chunk_bytes(&[]);
    assert!(chunks.is_empty());
}

#[test]
fn test_fj1347_single_chunk() {
    let data = vec![0xAA; 100];
    let chunks = chunk_bytes(&data);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].data.len(), 100);
    assert_eq!(chunks[0].hash, *blake3::hash(&data).as_bytes());
}

#[test]
fn test_fj1347_exact_boundary() {
    let data = vec![0xBB; CHUNK_SIZE];
    let chunks = chunk_bytes(&data);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].data.len(), CHUNK_SIZE);
}

#[test]
fn test_fj1347_multi_chunk() {
    let data = vec![0xCC; CHUNK_SIZE * 3 + 42];
    let chunks = chunk_bytes(&data);
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[0].data.len(), CHUNK_SIZE);
    assert_eq!(chunks[1].data.len(), CHUNK_SIZE);
    assert_eq!(chunks[2].data.len(), CHUNK_SIZE);
    assert_eq!(chunks[3].data.len(), 42);
}

#[test]
fn test_fj1347_chunk_hashes_correct() {
    let data = vec![0xDD; CHUNK_SIZE + 10];
    let chunks = chunk_bytes(&data);
    let expected_first = *blake3::hash(&data[..CHUNK_SIZE]).as_bytes();
    let expected_second = *blake3::hash(&data[CHUNK_SIZE..]).as_bytes();
    assert_eq!(chunks[0].hash, expected_first);
    assert_eq!(chunks[1].hash, expected_second);
}

#[test]
fn test_fj1347_reassemble_roundtrip() {
    let data: Vec<u8> = (0..200_000u32).map(|i| (i % 256) as u8).collect();
    let chunks = chunk_bytes(&data);
    let rebuilt = reassemble(&chunks);
    assert_eq!(rebuilt, data);
}

#[test]
fn test_fj1347_reassemble_empty() {
    let chunks: Vec<ChunkData> = vec![];
    let rebuilt = reassemble(&chunks);
    assert!(rebuilt.is_empty());
}

#[test]
fn test_fj1347_tree_hash_empty() {
    let chunks: Vec<ChunkData> = vec![];
    let h = tree_hash(&chunks);
    assert_eq!(h, *blake3::hash(b"").as_bytes());
}

#[test]
fn test_fj1347_tree_hash_single() {
    let data = vec![0xEE; 100];
    let chunks = chunk_bytes(&data);
    let h = tree_hash(&chunks);
    // Single chunk: tree hash == chunk hash
    assert_eq!(h, chunks[0].hash);
}

#[test]
fn test_fj1347_tree_hash_deterministic() {
    let data = vec![0xFF; CHUNK_SIZE * 5 + 99];
    let chunks1 = chunk_bytes(&data);
    let chunks2 = chunk_bytes(&data);
    assert_eq!(tree_hash(&chunks1), tree_hash(&chunks2));
}

#[test]
fn test_fj1347_tree_hash_different_data() {
    let a = chunk_bytes(&[0xAA; CHUNK_SIZE * 2]);
    let b = chunk_bytes(&[0xBB; CHUNK_SIZE * 2]);
    assert_ne!(tree_hash(&a), tree_hash(&b));
}

#[test]
fn test_fj1347_chunk_directory() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("hello.txt"), "hello world").unwrap();
    std::fs::create_dir(tmp.path().join("sub")).unwrap();
    std::fs::write(tmp.path().join("sub/nested.txt"), "nested").unwrap();

    let (chunks, entries) = chunk_directory(tmp.path()).unwrap();
    assert!(!chunks.is_empty());
    assert_eq!(entries.len(), 2);

    let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
    assert!(paths.contains(&"hello.txt"));
    assert!(paths.contains(&"sub/nested.txt"));
}

#[test]
fn test_fj1347_chunk_directory_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let (chunks, entries) = chunk_directory(tmp.path()).unwrap();
    // Empty tar still produces a small chunk (tar footer)
    assert!(entries.is_empty());
    assert!(!chunks.is_empty()); // tar footer
}

#[test]
fn test_fj1347_chunk_directory_file_hashes() {
    let tmp = tempfile::tempdir().unwrap();
    let content = b"deterministic content";
    std::fs::write(tmp.path().join("file.txt"), content).unwrap();

    let (_, entries) = chunk_directory(tmp.path()).unwrap();
    assert_eq!(entries.len(), 1);
    let expected = format!("blake3:{}", blake3::hash(content).to_hex());
    assert_eq!(entries[0].blake3, expected);
    assert_eq!(entries[0].size, content.len() as u64);
}
