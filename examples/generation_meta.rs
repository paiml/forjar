//! Example: GenerationMeta usage
//!
//! Demonstrates the extended generation metadata with config tracking,
//! git ref, operator identity, and per-machine deltas.

use forjar::core::types::{GenerationMeta, MachineDelta};

fn main() {
    // Create a generation metadata with enriched fields
    let mut meta = GenerationMeta::new(5, "2026-03-05T14:30:00Z".into());
    meta = meta
        .with_config_hash("blake3:abc123def456".into())
        .with_git_ref("dc6a765".into());
    meta.forjar_version = Some("1.1.1".into());
    meta.operator = Some("deploy-bot@build-server".into());

    // Record per-machine deltas
    meta.record_machine(
        "web-01",
        MachineDelta {
            created: vec!["nginx-config".into(), "ssl-cert".into()],
            updated: vec!["app-binary".into()],
            destroyed: vec![],
            unchanged: 3,
        },
    );
    meta.record_machine(
        "db-01",
        MachineDelta {
            created: vec![],
            updated: vec!["pg-config".into()],
            destroyed: vec![],
            unchanged: 5,
        },
    );

    println!("Generation: {}", meta.generation);
    println!("Total changes: {}", meta.total_changes());
    println!("Is undo: {}", meta.is_undo());

    // Serialize to YAML
    let yaml = meta.to_yaml().unwrap();
    println!("\n--- .generation.yaml ---");
    println!("{yaml}");

    // Round-trip: deserialize back
    let parsed = GenerationMeta::from_yaml(&yaml).unwrap();
    assert_eq!(parsed.generation, 5);
    assert_eq!(parsed.total_changes(), 4);
    println!("Round-trip OK");
}
