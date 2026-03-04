//! Demonstrate the state management system.
//!
//! Shows how forjar manages lock files, global locks, and tracks
//! resource state across apply runs.
//!
//! Usage: cargo run --example state_management

use forjar::core::state;
use forjar::core::types::{ResourceLock, ResourceStatus, ResourceType};
use std::collections::HashMap;

fn main() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let state_dir = dir.path();

    println!("=== State Management Example ===\n");
    println!("State dir: {}\n", state_dir.display());

    // 1. Create a new lock for a machine
    let mut lock = state::new_lock("web-server", "web-prod-01");
    println!(
        "Created lock: machine={}, hostname={}",
        lock.machine, lock.hostname
    );
    println!("  schema: {}", lock.schema);
    println!("  generator: {}", lock.generator);
    println!("  blake3_version: {}", lock.blake3_version);
    println!("  generated_at: {}", lock.generated_at);

    // 2. Add resource results (simulating an apply)
    lock.resources.insert(
        "base-packages".to_string(),
        ResourceLock {
            resource_type: ResourceType::Package,
            status: ResourceStatus::Converged,
            applied_at: Some(lock.generated_at.clone()),
            duration_seconds: Some(3.2),
            hash: "blake3:a7f2c9d4e5f6789012345678901234567890123456789012345678901234abcd"
                .to_string(),
            details: HashMap::new(),
        },
    );
    lock.resources.insert(
        "nginx-config".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: Some(lock.generated_at.clone()),
            duration_seconds: Some(0.1),
            hash: "blake3:b8e3da56f7a890123456789012345678901234567890123456789012345678ef"
                .to_string(),
            details: {
                let mut d = HashMap::new();
                d.insert(
                    "path".to_string(),
                    serde_yaml_ng::Value::String("/etc/nginx/nginx.conf".to_string()),
                );
                d.insert(
                    "content_hash".to_string(),
                    serde_yaml_ng::Value::String(
                        "blake3:c9f4eb67a8b901234567890123456789012345678901234567890123456789ab"
                            .to_string(),
                    ),
                );
                d
            },
        },
    );
    println!("\nAdded {} resources to lock", lock.resources.len());

    // 3. Save atomically
    state::save_lock(state_dir, &lock).expect("save lock");
    let lock_path = state::lock_file_path(state_dir, "web-server");
    println!("\nSaved lock to: {}", lock_path.display());
    println!("  (atomic: write tmp + rename)");

    // 4. Load it back
    let loaded = state::load_lock(state_dir, "web-server")
        .expect("load lock")
        .expect("lock should exist");
    println!("\nLoaded lock: {} resources", loaded.resources.len());
    for (id, rl) in &loaded.resources {
        println!(
            "  {} ({:?}): {:?}, {:.1}s",
            id,
            rl.resource_type,
            rl.status,
            rl.duration_seconds.unwrap_or(0.0)
        );
    }

    // 5. Check for non-existent machine
    let missing = state::load_lock(state_dir, "ghost-machine").expect("load ghost");
    println!("\nLoad non-existent machine: {missing:?}");

    // 6. Global lock — tracks all machines
    println!("\n--- Global Lock ---");
    let results = vec![
        ("web-server".to_string(), 2_usize, 2_usize, 0_usize),
        ("db-server".to_string(), 3_usize, 3_usize, 0_usize),
    ];
    state::update_global_lock(state_dir, "prod-infra", &results).expect("update global");

    let global = state::load_global_lock(state_dir)
        .expect("load global")
        .expect("global should exist");
    println!("Global lock: {}", global.name);
    println!("  last_apply: {}", global.last_apply);
    for (name, summary) in &global.machines {
        println!(
            "  {}: {} resources ({} converged, {} failed)",
            name, summary.resources, summary.converged, summary.failed
        );
    }

    // 7. Show the YAML content
    println!("\n--- Lock File Content ---");
    let content = std::fs::read_to_string(&lock_path).expect("read lock");
    for line in content.lines().take(15) {
        println!("  {line}");
    }
    println!("  ...");

    println!("\n=== State Management Example Complete ===");
}
