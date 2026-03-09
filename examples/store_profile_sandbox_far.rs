//! FJ-1302/1315/1346: Profile generation, sandbox config, and FAR archive.
//!
//! Demonstrates:
//! - Profile generation with atomic rollback
//! - Sandbox config validation and presets
//! - FAR archive encode/decode roundtrip
//!
//! Usage: cargo run --example store_profile_sandbox_far

use forjar::core::store::far::{
    decode_far_manifest, encode_far, FarFileEntry, FarManifest, FarProvenance,
};
use forjar::core::store::profile::{
    create_generation, current_generation, list_generations, rollback,
};
use forjar::core::store::sandbox::{
    blocks_network, enforces_fs_isolation, preset_profile, validate_config, SandboxLevel,
};

fn main() {
    println!("Forjar Store: Profiles / Sandbox / FAR Archive");
    println!("{}", "=".repeat(55));

    // ── FJ-1302: Profile Generations ──
    println!("\n[FJ-1302] Profile Generations:");
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");

    let g0 = create_generation(&profiles, "/store/hash0/content").unwrap();
    let g1 = create_generation(&profiles, "/store/hash1/content").unwrap();
    let g2 = create_generation(&profiles, "/store/hash2/content").unwrap();
    println!("  Created generations: {g0}, {g1}, {g2}");
    println!("  Current: {:?}", current_generation(&profiles));

    let gens = list_generations(&profiles).unwrap();
    println!("  Listed: {} generations", gens.len());

    let rolled = rollback(&profiles).unwrap();
    println!("  Rolled back to: {rolled}");
    println!(
        "  Current after rollback: {:?}",
        current_generation(&profiles)
    );

    let profile_ok = current_generation(&profiles) == Some(1);
    println!(
        "  Rollback correct: {}",
        if profile_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(profile_ok);

    // ── FJ-1315: Sandbox Config ──
    println!("\n[FJ-1315] Sandbox Configuration:");
    for name in &["full", "network-only", "minimal", "gpu"] {
        let config = preset_profile(name).unwrap();
        let errors = validate_config(&config);
        println!(
            "  {}: level={:?}, mem={}MB, cpus={}, errors={}",
            name,
            config.level,
            config.memory_mb,
            config.cpus,
            errors.len()
        );
    }

    println!("  Network blocking:");
    for level in [
        SandboxLevel::Full,
        SandboxLevel::NetworkOnly,
        SandboxLevel::Minimal,
        SandboxLevel::None,
    ] {
        println!(
            "    {:?}: blocks_net={}, fs_iso={}",
            level,
            blocks_network(level),
            enforces_fs_isolation(level)
        );
    }

    let sandbox_ok = blocks_network(SandboxLevel::Full)
        && !blocks_network(SandboxLevel::None)
        && enforces_fs_isolation(SandboxLevel::Minimal);
    println!(
        "  Sandbox predicates: {}",
        if sandbox_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(sandbox_ok);

    // ── FJ-1346: FAR Archive ──
    println!("\n[FJ-1346] FAR Archive:");
    let manifest = FarManifest {
        name: "demo-pkg".to_string(),
        version: "1.0.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:abc123".to_string(),
        tree_hash: "blake3:def456".to_string(),
        file_count: 1,
        total_size: 1024,
        files: vec![FarFileEntry {
            path: "bin/demo".to_string(),
            size: 1024,
            blake3: "blake3:fileabc".to_string(),
        }],
        provenance: FarProvenance {
            origin_provider: "apt".to_string(),
            origin_ref: Some("demo=1.0.0".to_string()),
            origin_hash: None,
            created_at: "2026-03-09T00:00:00Z".to_string(),
            generator: "forjar 1.0".to_string(),
        },
        kernel_contracts: None,
    };

    let data = b"binary content for the chunk";
    let hash = blake3::hash(data);
    let chunks = vec![(*hash.as_bytes(), data.to_vec())];

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();
    println!("  Encoded: {} bytes ({} chunks)", buf.len(), chunks.len());

    let (decoded, entries) = decode_far_manifest(std::io::Cursor::new(&buf)).unwrap();
    let far_ok = decoded.name == "demo-pkg"
        && decoded.files.len() == 1
        && entries.len() == 1
        && entries[0].hash == *hash.as_bytes();
    println!(
        "  Decoded: name={}, files={}, chunks={}",
        decoded.name,
        decoded.files.len(),
        entries.len()
    );
    println!("  Roundtrip: {}", if far_ok { "✓" } else { "✗ FALSIFIED" });
    assert!(far_ok);

    println!("\n{}", "=".repeat(55));
    println!("All profile/sandbox/FAR criteria survived.");
}
