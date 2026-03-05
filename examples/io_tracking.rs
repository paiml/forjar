//! FJ-2701: Input/output tracking example.
//!
//! Demonstrates BLAKE3-based content hashing for task caching:
//! input pattern hashing, output artifact hashing, and cache skip logic.
//!
//! ```bash
//! cargo run --example io_tracking
//! ```

use forjar::core::task::{hash_inputs, hash_outputs, should_skip_cached};

fn main() {
    demo_input_hashing();
    demo_output_hashing();
    demo_cache_skip();
}

fn demo_input_hashing() {
    println!("=== FJ-2701: Input Hashing ===\n");

    let dir = std::env::current_dir().unwrap();
    let patterns = vec!["src/core/task/*.rs".to_string()];

    match hash_inputs(&patterns, &dir) {
        Ok(Some(hash)) => println!("Input hash (src/core/task/*.rs): {hash}"),
        Ok(None) => println!("No input files matched"),
        Err(e) => println!("Error: {e}"),
    }

    let empty: Vec<String> = vec![];
    let result = hash_inputs(&empty, &dir).unwrap();
    println!("Empty patterns: {:?}\n", result);
}

fn demo_output_hashing() {
    println!("=== FJ-2701: Output Artifact Hashing ===\n");

    let artifacts = vec!["Cargo.toml".to_string()];
    match hash_outputs(&artifacts) {
        Ok(Some(hash)) => println!("Cargo.toml hash: {hash}"),
        Ok(None) => println!("No artifacts found"),
        Err(e) => println!("Error: {e}"),
    }

    let missing = vec!["/tmp/nonexistent-artifact-xyz".to_string()];
    let result = hash_outputs(&missing).unwrap();
    println!("Missing artifact: {:?}\n", result);
}

fn demo_cache_skip() {
    println!("=== FJ-2701: Cache Skip Logic ===\n");

    let hash_a = "blake3:abc123";
    let hash_b = "blake3:def456";

    println!("cache=false, same hash: skip={}",
        should_skip_cached(false, Some(hash_a), Some(hash_a)));
    println!("cache=true, same hash:  skip={}",
        should_skip_cached(true, Some(hash_a), Some(hash_a)));
    println!("cache=true, diff hash:  skip={}",
        should_skip_cached(true, Some(hash_a), Some(hash_b)));
    println!("cache=true, no stored:  skip={}",
        should_skip_cached(true, Some(hash_a), None));
}
