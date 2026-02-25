//! Demonstrate drift detection — detect unauthorized changes to managed files.
//!
//! Usage: cargo run --example drift_detection

use forjar::core::{state, types};
use forjar::tripwire::{drift, hasher};
use std::collections::HashMap;
use std::path::Path;

fn main() {
    let tmp = std::env::temp_dir().join("forjar-drift-example");
    let state_dir = tmp.join("state");
    std::fs::create_dir_all(state_dir.join("demo")).unwrap();

    // 1. Create a managed file
    let managed_file = tmp.join("config.yaml");
    let original_content = "database:\n  host: localhost\n  port: 5432\n";
    std::fs::write(&managed_file, original_content).unwrap();

    // 2. Record the state lock (simulates post-apply state)
    let content_hash = hasher::hash_file(&managed_file).unwrap();
    println!("1. Created managed file");
    println!("   Path: {}", managed_file.display());
    println!("   BLAKE3 hash: {}", content_hash);

    let mut resources = indexmap::IndexMap::new();
    resources.insert(
        "app-config".to_string(),
        types::ResourceLock {
            resource_type: types::ResourceType::File,
            status: types::ResourceStatus::Converged,
            applied_at: Some("2026-02-25T12:00:00Z".to_string()),
            duration_seconds: Some(0.01),
            hash: content_hash.clone(),
            details: {
                let mut d = HashMap::new();
                d.insert(
                    "path".to_string(),
                    serde_yaml_ng::Value::String(managed_file.to_string_lossy().to_string()),
                );
                d.insert(
                    "content_hash".to_string(),
                    serde_yaml_ng::Value::String(content_hash.clone()),
                );
                d
            },
        },
    );

    let lock = types::StateLock {
        schema: "forjar-lock-v1".to_string(),
        machine: "demo".to_string(),
        hostname: "localhost".to_string(),
        generated_at: "2026-02-25T12:00:00Z".to_string(),
        generator: "forjar-example".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };

    state::save_lock(&state_dir, &lock).unwrap();
    println!("   Lock saved to {}", state_dir.display());

    // 3. Check for drift — should find none
    let findings = drift::detect_drift(&lock);
    println!("\n2. Drift check (before tampering):");
    if findings.is_empty() {
        println!("   No drift detected ✓");
    }

    // 4. Tamper with the file (simulate unauthorized change)
    let tampered = "database:\n  host: evil.example.com\n  port: 5432\n";
    std::fs::write(&managed_file, tampered).unwrap();
    println!("\n3. Simulated unauthorized change:");
    println!("   host: localhost → evil.example.com");

    // 5. Re-check for drift — should detect the change
    let findings = drift::detect_drift(&lock);
    println!("\n4. Drift check (after tampering):");
    for f in &findings {
        println!("   DRIFT: {} ({})", f.resource_id, f.detail);
        println!("   Expected: {}", f.expected_hash);
        println!("   Actual:   {}", f.actual_hash);
    }
    assert!(!findings.is_empty(), "should detect tampered file");

    // 6. Restore the file (simulate remediation)
    std::fs::write(&managed_file, original_content).unwrap();
    let findings = drift::detect_drift(&lock);
    println!("\n5. After remediation:");
    if findings.is_empty() {
        println!("   No drift detected ✓ (file restored)");
    }

    // 7. Delete the file (simulate deletion)
    std::fs::remove_file(&managed_file).unwrap();
    let findings = drift::detect_drift(&lock);
    println!("\n6. After deletion:");
    for f in &findings {
        println!("   DRIFT: {} — {}", f.resource_id, f.detail);
    }

    // Cleanup
    let _ = std::fs::remove_dir_all(&tmp);
    println!("\nDrift detection example complete.");

    // Also demonstrate the state loading API
    println!("\n--- State API ---");
    let state_dir2 = Path::new("state");
    if state_dir2.exists() {
        match state::load_lock(state_dir2, "intel") {
            Ok(Some(lock)) => {
                println!(
                    "Loaded state for '{}' ({} resources)",
                    lock.machine,
                    lock.resources.len()
                );
                for (id, rl) in &lock.resources {
                    println!("  {:20} {:?} {:?}", id, rl.resource_type, rl.status);
                }
            }
            Ok(None) => println!("No state found for 'intel'"),
            Err(e) => println!("Error loading state: {}", e),
        }
    } else {
        println!("No state/ directory found (run forjar apply first)");
    }
}
