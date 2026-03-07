//! FJ-2002: Extended generation model example.
//!
//! Demonstrates generation metadata with config tracking, git refs,
//! resource deltas, and undo chains.
//!
//! ```bash
//! cargo run --example generation_model
//! ```

use forjar::core::types::{
    get_git_ref, git_is_dirty, DestroyLogEntry, GenerationMeta, MachineDelta,
};

fn main() {
    demo_generation_meta();
    demo_undo_chain();
    demo_destroy_log();
    demo_git_integration();
}

fn demo_generation_meta() {
    println!("=== FJ-2002: Extended Generation Metadata ===\n");

    let mut meta = GenerationMeta::new(5, "2026-03-05T14:30:00Z".into());
    meta.config_hash = Some("blake3:abc123def456".into());
    meta.git_ref = Some("a1b2c3d".into());
    meta.operator = Some("noah@workstation".into());
    meta.forjar_version = Some("1.1.1".into());

    meta.record_machine(
        "intel",
        MachineDelta {
            created: vec!["zshrc".into()],
            updated: vec!["stack-tools".into(), "gitconfig".into()],
            destroyed: vec![],
            unchanged: 14,
        },
    );
    meta.record_machine(
        "jetson",
        MachineDelta {
            created: vec![],
            updated: vec![],
            destroyed: vec![],
            unchanged: 8,
        },
    );

    let yaml = meta.to_yaml().unwrap();
    println!("{yaml}");
    println!("  Total changes: {}", meta.total_changes());
    println!();
}

fn demo_undo_chain() {
    println!("=== FJ-2002: Undo Chain ===\n");

    let gen0 = GenerationMeta::new(0, "2026-03-01T10:00:00Z".into())
        .with_config_hash("blake3:aaa".into())
        .with_git_ref("abc1234".into());

    let gen1 = GenerationMeta::new(1, "2026-03-02T10:00:00Z".into())
        .with_config_hash("blake3:bbb".into())
        .with_git_ref("def5678".into());

    let gen2 = GenerationMeta::new_undo(2, "2026-03-03T10:00:00Z".into(), 0);

    for gen in [&gen0, &gen1, &gen2] {
        let action = if gen.is_undo() { "UNDO" } else { "APPLY" };
        let parent = gen
            .parent_generation
            .map(|p| format!(" (reverts to gen {p})"))
            .unwrap_or_default();
        println!(
            "  Gen {}: [{action}] {}{parent}",
            gen.generation, gen.created_at
        );
    }
    println!();
}

fn demo_destroy_log() {
    println!("=== FJ-2005: Destroy Log ===\n");

    let entries = vec![
        DestroyLogEntry {
            timestamp: "2026-03-05T14:30:00Z".into(),
            machine: "intel".into(),
            resource_id: "nginx-config".into(),
            resource_type: "file".into(),
            pre_hash: "blake3:file_content_hash".into(),
            generation: 5,
            config_fragment: Some("path: /etc/nginx/nginx.conf\ncontent: |  ...".into()),
            reliable_recreate: true,
        },
        DestroyLogEntry {
            timestamp: "2026-03-05T14:30:01Z".into(),
            machine: "intel".into(),
            resource_id: "nginx-pkg".into(),
            resource_type: "package".into(),
            pre_hash: "blake3:pkg_state_hash".into(),
            generation: 5,
            config_fragment: Some("name: nginx\nstate: present".into()),
            reliable_recreate: false,
        },
    ];

    for entry in &entries {
        let reliable = if entry.reliable_recreate {
            "reliable"
        } else {
            "best-effort"
        };
        println!(
            "  {} ({}) — {} [{reliable}]",
            entry.resource_id, entry.resource_type, entry.pre_hash
        );
    }

    println!("\n  JSONL format:");
    for entry in &entries {
        println!("  {}", entry.to_jsonl().unwrap());
    }
    println!();
}

fn demo_git_integration() {
    println!("=== FJ-2002: Git Integration ===\n");

    match get_git_ref() {
        Some(ref git_ref) => println!("  Current git ref: {git_ref}"),
        None => println!("  Not in a git repository"),
    }

    println!("  Working tree dirty: {}", git_is_dirty());
}
