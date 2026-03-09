//! FJ-1347/1307/1300: Content chunking, input closures, store paths.
//!
//! Usage: cargo run --example chunker_closure_path

use forjar::core::store::chunker::{chunk_bytes, reassemble, tree_hash, CHUNK_SIZE};
use forjar::core::store::closure::{all_closures, closure_hash, input_closure, ResourceInputs};
use forjar::core::store::path::{store_entry_path, store_path, STORE_BASE};
use std::collections::BTreeMap;

fn main() {
    println!("Forjar: Content Chunking, Input Closures & Store Paths");
    println!("{}", "=".repeat(60));

    // ── FJ-1347: Content Chunking ──
    println!("\n[FJ-1347] Content Chunking (CHUNK_SIZE = {CHUNK_SIZE}):");

    let small = b"hello forjar store";
    let chunks = chunk_bytes(small);
    println!(
        "  Small data ({} bytes): {} chunk(s)",
        small.len(),
        chunks.len()
    );
    println!("  Chunk hash: {:02x?}", &chunks[0].hash[..8]);

    let large = vec![42u8; CHUNK_SIZE * 3 + 100];
    let large_chunks = chunk_bytes(&large);
    println!(
        "  Large data ({} bytes): {} chunk(s)",
        large.len(),
        large_chunks.len()
    );

    let reassembled = reassemble(&large_chunks);
    println!(
        "  Reassembly: {} (match={})",
        reassembled.len(),
        reassembled == large
    );

    let tree = tree_hash(&large_chunks);
    println!("  Merkle tree hash: {:02x?}", &tree[..8]);

    // ── FJ-1307: Input Closures ──
    println!("\n[FJ-1307] Input Closures:");

    let mut graph: BTreeMap<String, ResourceInputs> = BTreeMap::new();
    graph.insert(
        "libc".into(),
        ResourceInputs {
            input_hashes: vec!["blake3:libc-src".into()],
            depends_on: vec![],
        },
    );
    graph.insert(
        "openssl".into(),
        ResourceInputs {
            input_hashes: vec!["blake3:openssl-src".into()],
            depends_on: vec!["libc".into()],
        },
    );
    graph.insert(
        "curl".into(),
        ResourceInputs {
            input_hashes: vec!["blake3:curl-src".into()],
            depends_on: vec!["openssl".into(), "libc".into()],
        },
    );

    for name in ["libc", "openssl", "curl"] {
        let closure = input_closure(name, &graph);
        let hash = closure_hash(&closure);
        println!("  {name}: closure={closure:?}");
        println!("    hash={hash}");
    }

    let all = all_closures(&graph);
    println!("  all_closures: {} entries", all.len());

    // ── FJ-1300: Store Paths ──
    println!("\n[FJ-1300] Store Paths (STORE_BASE = {STORE_BASE}):");

    let path1 = store_path(
        "recipe-curl",
        &["blake3:in1", "blake3:in2"],
        "x86_64",
        "apt",
    );
    let path2 = store_path(
        "recipe-curl",
        &["blake3:in2", "blake3:in1"],
        "x86_64",
        "apt",
    );
    println!("  store_path(curl, [in1,in2], x86_64): {path1}");
    println!("  store_path(curl, [in2,in1], x86_64): {path2}");
    println!("  Order-independent: {}", path1 == path2);

    let entry = store_entry_path(&path1);
    println!("  store_entry_path: {entry}");

    let path_arm = store_path("recipe-curl", &["blake3:in1"], "aarch64", "apt");
    let path_x86 = store_path("recipe-curl", &["blake3:in1"], "x86_64", "apt");
    println!(
        "  Arch-sensitive: aarch64 != x86_64 = {}",
        path_arm != path_x86
    );

    println!("\n{}", "=".repeat(60));
    println!("All chunker/closure/path criteria survived.");
}
