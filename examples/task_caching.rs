//! FJ-2701: Task input caching demonstration.
//!
//! Shows how forjar's task framework uses BLAKE3 hashing of input files
//! to skip re-execution when inputs haven't changed.
//!
//! ```bash
//! cargo run --example task_caching
//! ```

use forjar::core::task::{hash_inputs, hash_outputs, should_skip_cached};
use std::io::Write;

fn main() {
    println!("=== Task Input Caching (FJ-2701) ===\n");

    // Create temp files to demonstrate hashing
    let dir = tempfile::tempdir().unwrap();
    let file_a = dir.path().join("src").join("main.rs");
    let file_b = dir.path().join("Cargo.toml");
    std::fs::create_dir_all(file_a.parent().unwrap()).unwrap();
    std::fs::File::create(&file_a)
        .unwrap()
        .write_all(b"fn main() {}")
        .unwrap();
    std::fs::File::create(&file_b)
        .unwrap()
        .write_all(b"[package]\nname = \"demo\"")
        .unwrap();

    // Hash inputs
    let patterns = vec!["src/**/*.rs".to_string(), "Cargo.toml".to_string()];
    let hash1 = hash_inputs(&patterns, dir.path()).unwrap();
    println!("Input patterns: {:?}", patterns);
    println!("Input hash: {:?}\n", hash1);

    // Same inputs → same hash (cache hit)
    let hash2 = hash_inputs(&patterns, dir.path()).unwrap();
    let skip = should_skip_cached(true, hash2.as_deref(), hash1.as_deref());
    println!("Cache enabled, same inputs: skip={skip}");
    assert!(skip, "same inputs should skip");

    // Modify a file → different hash (cache miss)
    std::fs::File::create(&file_a)
        .unwrap()
        .write_all(b"fn main() { println!(\"changed\"); }")
        .unwrap();
    let hash3 = hash_inputs(&patterns, dir.path()).unwrap();
    let skip2 = should_skip_cached(true, hash3.as_deref(), hash1.as_deref());
    println!("Cache enabled, changed inputs: skip={skip2}");
    assert!(!skip2, "changed inputs should not skip");

    // Output artifact hashing
    let output_file = dir.path().join("target").join("output.bin");
    std::fs::create_dir_all(output_file.parent().unwrap()).unwrap();
    std::fs::write(&output_file, b"binary data").unwrap();
    let out_hash = hash_outputs(&[output_file.to_string_lossy().to_string()]).unwrap();
    println!("\nOutput hash: {:?}", out_hash);

    // Cache disabled → never skip
    let skip3 = should_skip_cached(false, hash1.as_deref(), hash1.as_deref());
    println!("Cache disabled: skip={skip3}");
    assert!(!skip3, "cache disabled should never skip");

    println!("\n=== YAML Config ===\n");
    println!("resources:");
    println!("  build-app:");
    println!("    type: task");
    println!("    task_mode: batch");
    println!("    cache: true");
    println!("    task_inputs:");
    println!("      - \"src/**/*.rs\"");
    println!("      - Cargo.toml");
    println!("    output_artifacts:");
    println!("      - target/release/app");
    println!("    command: \"cargo build --release\"");
}
