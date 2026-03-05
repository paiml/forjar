//! Demonstrates FJ-2003 generation diff types and cross-generation comparison.

use forjar::core::types::{
    diff_resource_sets, DiffAction, GenerationDiff, ResourceDiff,
};

fn main() {
    // Build a diff manually
    println!("=== Manual Generation Diff ===");
    let diff = GenerationDiff {
        gen_from: 5,
        gen_to: 8,
        machine: "intel".into(),
        resources: vec![
            ResourceDiff::added("monitoring-agent", "package"),
            ResourceDiff::modified("bash-aliases", "file")
                .with_hashes(
                    Some("blake3:aaa111".into()),
                    Some("blake3:bbb222".into()),
                )
                .with_detail("content changed"),
            ResourceDiff::removed("legacy-cron", "service"),
            ResourceDiff::unchanged("nginx-pkg", "package"),
        ],
    };
    println!("{}", diff.format_summary());

    // Compute diff from resource sets
    println!("=== Computed Diff ===");
    let gen5_resources = vec![
        ("bash-aliases", "file", "blake3:aaa"),
        ("nginx-pkg", "package", "blake3:bbb"),
        ("nginx-conf", "file", "blake3:ccc"),
        ("legacy-cron", "service", "blake3:ddd"),
    ];
    let gen8_resources = vec![
        ("bash-aliases", "file", "blake3:aaa_new"),  // modified
        ("nginx-pkg", "package", "blake3:bbb"),       // unchanged
        ("nginx-conf", "file", "blake3:ccc"),          // unchanged
        ("monitoring-agent", "package", "blake3:eee"), // added
    ];

    let diffs = diff_resource_sets(&gen5_resources, &gen8_resources);
    let computed = GenerationDiff {
        gen_from: 5,
        gen_to: 8,
        machine: "intel".into(),
        resources: diffs,
    };
    println!("{}", computed.format_summary());

    // Diff action inspection
    println!("=== Diff Actions ===");
    for r in &computed.resources {
        if r.action != DiffAction::Unchanged {
            println!("  {} {} ({})", r.action, r.resource_id, r.resource_type);
        }
    }
}
