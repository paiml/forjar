//! Store operation benchmarks — demonstrate the benchmarked operations.
//!
//! Shows the 10 store operations benchmarked by Criterion.rs in
//! `benches/store_bench.rs` and `benches/core_bench.rs`.
//!
//! Run benchmarks: `cargo bench --bench store_bench`
//! Run this demo: `cargo run --example store_benchmarks`

use forjar::core::purifier::validate_script;
use forjar::core::store::chunker::{chunk_bytes, reassemble, tree_hash, CHUNK_SIZE};
use forjar::core::store::closure::{all_closures, closure_hash, ResourceInputs};
use forjar::core::store::far::{decode_far_manifest, encode_far};
use forjar::core::store::far::{FarFileEntry, FarManifest, FarProvenance};
use forjar::core::store::path::store_path;
use forjar::core::store::purity::{classify, PuritySignals};
use forjar::core::store::repro_score::{compute_score, grade, ReproInput};
use forjar::core::store::secret_scan::scan_text;
use std::collections::BTreeMap;
use std::time::Instant;

fn main() {
    println!("=== Store Benchmarks Demo ===\n");
    println!("Demonstrates the 10 operations benchmarked by Criterion.rs.");
    println!("For real benchmarks, run: cargo bench --bench store_bench\n");

    bench_demo_store_path();
    bench_demo_purity();
    bench_demo_closure();
    bench_demo_repro_score();
    bench_demo_far_encode();
    bench_demo_far_decode();
    bench_demo_chunking();
    bench_demo_merkle();
    bench_demo_secret_scan();
    bench_demo_bash_validate();
    print_summary();
    println!("\n=== All benchmark demos passed ===");
}

/// 1. Store path hashing (BLAKE3 composite).
fn bench_demo_store_path() {
    println!("--- 1. Store Path Hash ---");
    let recipe = "blake3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    let inputs = [
        "blake3:1111111111111111111111111111111111111111111111111111111111111111",
        "blake3:2222222222222222222222222222222222222222222222222222222222222222",
    ];

    let start = Instant::now();
    for _ in 0..10_000 {
        let _ = store_path(recipe, &inputs, "x86_64", "apt");
    }
    let elapsed = start.elapsed();

    let path = store_path(recipe, &inputs, "x86_64", "apt");
    assert!(path.starts_with("blake3:"));
    println!("  10K iterations: {elapsed:?}");
    println!("  Result: {}...{}", &path[..20], &path[path.len() - 8..]);
}

/// 2. Purity classification.
fn bench_demo_purity() {
    println!("--- 2. Purity Classify ---");
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        dep_levels: vec![],
    };

    let start = Instant::now();
    for _ in 0..100_000 {
        let _ = classify("nginx", &signals);
    }
    let elapsed = start.elapsed();

    let result = classify("nginx", &signals);
    println!("  100K iterations: {elapsed:?}");
    println!("  Pure signals → {:?}", result.level);
}

/// 3. Closure hash computation.
fn bench_demo_closure() {
    println!("--- 3. Closure Hash ---");
    let mut graph = BTreeMap::new();
    graph.insert(
        "nginx".to_string(),
        ResourceInputs {
            input_hashes: vec!["blake3:aaaa".to_string()],
            depends_on: vec!["openssl".to_string()],
        },
    );
    graph.insert(
        "openssl".to_string(),
        ResourceInputs {
            input_hashes: vec!["blake3:bbbb".to_string()],
            depends_on: vec![],
        },
    );

    let start = Instant::now();
    for _ in 0..10_000 {
        let closures = all_closures(&graph);
        for (_name, closure) in &closures {
            let _ = closure_hash(closure);
        }
    }
    let elapsed = start.elapsed();

    let closures = all_closures(&graph);
    println!("  10K iterations: {elapsed:?}");
    println!("  Closure entries: {}", closures.len());
}

/// 4. Reproducibility scoring.
fn bench_demo_repro_score() {
    println!("--- 4. Repro Score ---");
    let inputs = vec![
        ReproInput {
            name: "nginx".to_string(),
            purity: forjar::core::store::purity::PurityLevel::Pure,
            has_store: true,
            has_lock_pin: true,
        },
        ReproInput {
            name: "openssl".to_string(),
            purity: forjar::core::store::purity::PurityLevel::Pinned,
            has_store: true,
            has_lock_pin: true,
        },
    ];

    let start = Instant::now();
    for _ in 0..100_000 {
        let _ = compute_score(&inputs);
    }
    let elapsed = start.elapsed();

    let result = compute_score(&inputs);
    println!("  100K iterations: {elapsed:?}");
    println!(
        "  Score: {:.1} (Grade {})",
        result.composite,
        grade(result.composite)
    );
}

/// 5. FAR archive encoding.
fn bench_demo_far_encode() {
    println!("--- 5. FAR Encode ---");
    let manifest = sample_manifest();
    let data = vec![42u8; CHUNK_SIZE];
    let hash = *blake3::hash(&data).as_bytes();
    let chunks = vec![(hash, data)];

    let start = Instant::now();
    for _ in 0..1_000 {
        let mut buf = Vec::with_capacity(CHUNK_SIZE + 512);
        encode_far(&manifest, &chunks, &mut buf).unwrap();
    }
    let elapsed = start.elapsed();

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();
    println!("  1K iterations (64KB payload): {elapsed:?}");
    println!("  Output size: {} bytes", buf.len());
}

/// 6. FAR archive decoding.
fn bench_demo_far_decode() {
    println!("--- 6. FAR Decode ---");
    let manifest = sample_manifest();
    let data = vec![42u8; CHUNK_SIZE];
    let hash = *blake3::hash(&data).as_bytes();
    let mut buf = Vec::new();
    encode_far(&manifest, &[(hash, data)], &mut buf).unwrap();

    let start = Instant::now();
    for _ in 0..1_000 {
        let _ = decode_far_manifest(buf.as_slice()).unwrap();
    }
    let elapsed = start.elapsed();

    let (m, entries) = decode_far_manifest(buf.as_slice()).unwrap();
    println!("  1K iterations: {elapsed:?}");
    println!("  Decoded: {} ({} chunks)", m.name, entries.len());
}

/// 7. Fixed-size chunking (64KB).
fn bench_demo_chunking() {
    println!("--- 7. Chunking ---");
    let data = vec![0u8; CHUNK_SIZE * 10]; // 640KB

    let start = Instant::now();
    for _ in 0..100 {
        let chunks = chunk_bytes(&data);
        let _ = reassemble(&chunks);
    }
    let elapsed = start.elapsed();

    let chunks = chunk_bytes(&data);
    println!(
        "  100 iterations (640KB → {} chunks + reassemble): {elapsed:?}",
        chunks.len()
    );
}

/// 8. Merkle tree hash.
fn bench_demo_merkle() {
    println!("--- 8. Merkle Tree ---");
    let data = vec![0u8; CHUNK_SIZE * 10];
    let chunks = chunk_bytes(&data);

    let start = Instant::now();
    for _ in 0..10_000 {
        let _ = tree_hash(&chunks);
    }
    let elapsed = start.elapsed();

    let root = tree_hash(&chunks);
    let hex: String = root[..8].iter().map(|b| format!("{b:02x}")).collect();
    println!("  10K iterations (10 chunks): {elapsed:?}");
    println!("  Root: blake3:{hex}...");
}

/// 9. Secret scanning.
fn bench_demo_secret_scan() {
    println!("--- 9. Secret Scan ---");
    let clean = "This is a perfectly normal configuration value with no secrets.";
    let dirty = "key=AKIAIOSFODNN7EXAMPLE password=supersecret123456";

    let start = Instant::now();
    for _ in 0..10_000 {
        let _ = scan_text(clean);
        let _ = scan_text(dirty);
    }
    let elapsed = start.elapsed();

    let clean_findings = scan_text(clean);
    let dirty_findings = scan_text(dirty);
    println!("  10K iterations (clean+dirty): {elapsed:?}");
    println!(
        "  Clean: {} findings, Dirty: {} findings",
        clean_findings.len(),
        dirty_findings.len()
    );
}

/// 10. Bash validation (I8).
fn bench_demo_bash_validate() {
    println!("--- 10. Bash Validate ---");
    let scripts = [
        "echo hello",
        "apt-get install -y nginx=1.24.0",
        "set -euo pipefail; make -j$(nproc)",
    ];

    let start = Instant::now();
    for _ in 0..10 {
        for script in &scripts {
            let _ = validate_script(script);
        }
    }
    let elapsed = start.elapsed();

    println!("  10 iterations (3 scripts each): {elapsed:?}");
    for script in &scripts {
        let r = validate_script(script);
        let status = if r.is_ok() { "PASS" } else { "FAIL" };
        println!("  '{script}': {status}");
    }
}

fn print_summary() {
    println!("\n--- Benchmark Summary ---");
    println!("  Operations benchmarked by Criterion.rs:");
    println!("  1. store_path_hash     — BLAKE3 composite path derivation");
    println!("  2. purity_classify     — 4-level purity classification");
    println!("  3. closure_hash        — Transitive dependency closure");
    println!("  4. repro_score         — Reproducibility scoring + grading");
    println!("  5. far_encode          — FAR archive binary encoding");
    println!("  6. far_decode          — FAR manifest streaming decode");
    println!("  7. chunk_bytes         — Fixed-size 64KB chunking");
    println!("  8. tree_hash           — Binary Merkle tree root");
    println!("  9. secret_scan         — 15-pattern regex detection");
    println!("  10. bash_validate      — bashrs I8 shell validation");
    println!();
    println!("  Run real benchmarks: cargo bench --bench store_bench");
    println!("  Run core benchmarks: cargo bench --bench core_bench");
}

fn sample_manifest() -> FarManifest {
    FarManifest {
        name: "bench-test".to_string(),
        version: "1.0.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:bench".to_string(),
        tree_hash: "blake3:tree".to_string(),
        file_count: 1,
        total_size: CHUNK_SIZE as u64,
        files: vec![FarFileEntry {
            path: "data.bin".to_string(),
            size: CHUNK_SIZE as u64,
            blake3: "blake3:data".to_string(),
        }],
        provenance: FarProvenance {
            origin_provider: "bench".to_string(),
            origin_ref: None,
            origin_hash: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            generator: "forjar-bench".to_string(),
        },
        kernel_contracts: None,
    }
}
