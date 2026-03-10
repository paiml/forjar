#![allow(clippy::field_reassign_with_default)]
//! Golden hash demonstration — pinned BLAKE3 hash detects serialization changes.
//!
//! Run: `cargo run --example golden_hash`

use forjar::core::planner::hash_desired_state;
use forjar::core::types::{MachineTarget, Resource, ResourceType};

fn main() {
    println!("Golden Hash Test");
    println!("================\n");

    // Minimal Package resource — same as the pinned test in tests_hash.rs
    let r = Resource {
        resource_type: ResourceType::Package,
        machine: MachineTarget::Single("m1".to_string()),
        provider: Some("apt".to_string()),
        packages: vec!["curl".to_string()],
        ..Resource::default()
    };

    let hash = hash_desired_state(&r);
    let expected = "blake3:8106dfb610d17486462652c99c0ac5c8e582a34064b75acb22a84fab2efa7f0b";

    println!("Resource: Package(apt, [curl]) on m1");
    println!("Hash:     {hash}");
    println!("Expected: {expected}");

    if hash == expected {
        println!("\nGolden hash MATCHES — serialization order is stable.");
    } else {
        println!("\nGolden hash MISMATCH — serialization order changed!");
        println!("This means hash_desired_state output is no longer backward-compatible.");
        std::process::exit(1);
    }

    // Show that changing any field changes the hash
    let r2 = Resource {
        packages: vec!["wget".to_string()],
        ..r.clone()
    };
    let h2 = hash_desired_state(&r2);
    println!("\nField sensitivity:");
    println!("  curl → {hash}");
    println!("  wget → {h2}");
    assert_ne!(hash, h2, "different packages must produce different hashes");
    println!("  Hashes differ — field sensitivity confirmed.");
}
