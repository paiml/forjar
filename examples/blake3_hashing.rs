//! Demonstrate BLAKE3 content-addressed hashing.
//!
//! Usage: cargo run --example blake3_hashing

use forjar::tripwire::hasher;

fn main() {
    // Hash a string
    let hash1 = hasher::hash_string("hello forjar");
    let hash2 = hasher::hash_string("hello forjar");
    let hash3 = hasher::hash_string("hello forjar!");

    println!("String hashing:");
    println!("  \"hello forjar\"  → {hash1}");
    println!("  \"hello forjar\"  → {hash2}");
    println!("  \"hello forjar!\" → {hash3}");
    println!("  Deterministic: {}", hash1 == hash2);
    println!("  Different input = different hash: {}", hash1 != hash3);

    // Composite hash (order-sensitive)
    let composite_ab = hasher::composite_hash(&["alpha", "beta"]);
    let composite_ba = hasher::composite_hash(&["beta", "alpha"]);
    println!("\nComposite hashing (order-sensitive):");
    println!("  [alpha, beta] → {composite_ab}");
    println!("  [beta, alpha] → {composite_ba}");
    println!("  Order matters: {}", composite_ab != composite_ba);

    // Hash a file
    let tmp = std::env::temp_dir().join("forjar-hash-example.txt");
    std::fs::write(&tmp, "content-addressed state").unwrap();
    match hasher::hash_file(&tmp) {
        Ok(hash) => println!("\nFile hashing:\n  {} → {}", tmp.display(), hash),
        Err(e) => eprintln!("\nFile hash error: {e}"),
    }
    let _ = std::fs::remove_file(&tmp);
}
