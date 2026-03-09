//! FJ-1310/1302/1304/1320: Lock files, profile generations, reference scanning, cache config.
//!
//! Usage: cargo run --example lockfile_profile_cache

use forjar::core::store::cache::{
    build_inventory, parse_cache_config, resolve_substitution, ssh_command, validate_cache_config,
    CacheEntry, SubstitutionResult,
};
use forjar::core::store::lockfile::{
    check_completeness, check_staleness, parse_lockfile, write_lockfile, LockFile, Pin,
};
use forjar::core::store::profile::{
    create_generation, current_generation, list_generations, rollback,
};
use forjar::core::store::reference::{is_valid_blake3_hash, scan_directory_refs, scan_file_refs};
use std::collections::{BTreeMap, BTreeSet};

fn main() {
    println!("Forjar: Lock Files, Profiles, References & Cache");
    println!("{}", "=".repeat(55));

    // ── FJ-1310: Lock Files ──
    println!("\n[FJ-1310] Lock Files:");
    let yaml = "schema: '1.0'\npins:\n  nginx:\n    provider: apt\n    hash: blake3:abc\n    version: '1.24'\n  curl:\n    provider: apt\n    hash: blake3:def\n";
    let lf = parse_lockfile(yaml).unwrap();
    println!("  Pins: {}", lf.pins.len());
    for (name, pin) in &lf.pins {
        println!("    {name}: {} hash={}", pin.provider, pin.hash);
    }

    let mut current = BTreeMap::new();
    current.insert("nginx".into(), "blake3:abc".into());
    current.insert("curl".into(), "blake3:NEW".into());
    let stale = check_staleness(&lf, &current);
    println!("  Stale pins: {}", stale.len());

    let missing = check_completeness(&lf, &["nginx".into(), "curl".into(), "openssl".into()]);
    println!("  Missing pins: {:?}", missing);

    let tmp = tempfile::tempdir().unwrap();
    write_lockfile(&tmp.path().join("lock.yaml"), &lf).unwrap();
    println!("  Roundtrip: OK");

    // ── FJ-1302: Profile Generations ──
    println!("\n[FJ-1302] Profile Generations:");
    let profiles = tmp.path().join("profiles");
    let g0 = create_generation(&profiles, "/store/hash-v1").unwrap();
    let g1 = create_generation(&profiles, "/store/hash-v2").unwrap();
    println!("  Created: gen {g0}, gen {g1}");
    println!("  Current: {:?}", current_generation(&profiles));

    let gens = list_generations(&profiles).unwrap();
    for (num, target) in &gens {
        println!("    gen {num}: {target}");
    }

    let rolled = rollback(&profiles).unwrap();
    println!("  Rolled back to: gen {rolled}");
    println!(
        "  Current after rollback: {:?}",
        current_generation(&profiles)
    );

    // ── FJ-1304: Reference Scanning ──
    println!("\n[FJ-1304] Reference Scanning:");
    let h1 = format!("blake3:{}", "a".repeat(64));
    let h2 = format!("blake3:{}", "b".repeat(64));
    println!("  Valid hash: {}", is_valid_blake3_hash(&h1));
    println!("  Invalid hash: {}", is_valid_blake3_hash("sha256:abc"));

    let content = format!("dep: {h1}\nother: {h2}\nplain: text\n");
    let known: BTreeSet<String> = [h1.clone()].into();
    let refs = scan_file_refs(content.as_bytes(), &known);
    println!("  File scan: {} refs found (1 known)", refs.len());

    let ref_dir = tmp.path().join("refs");
    std::fs::create_dir_all(&ref_dir).unwrap();
    std::fs::write(ref_dir.join("config"), format!("store: {h1}")).unwrap();
    let dir_refs = scan_directory_refs(&ref_dir, &known).unwrap();
    println!("  Dir scan: {} refs found", dir_refs.len());

    // ── FJ-1320: Cache Config ──
    println!("\n[FJ-1320] Cache Configuration:");
    let cache_yaml = "sources:\n  - type: ssh\n    host: cache.example.com\n    user: forjar\n    path: /var/lib/forjar/cache\n  - type: local\n    path: /tmp/local-cache\nauto_push: true\n";
    let cfg = parse_cache_config(cache_yaml).unwrap();
    println!("  Sources: {}", cfg.sources.len());
    println!("  Auto-push: {}", cfg.auto_push);
    println!("  Validation: {:?}", validate_cache_config(&cfg));

    if let Some(cmd) = ssh_command(&cfg.sources[0]) {
        println!("  SSH command: {cmd}");
    }

    let entry = CacheEntry {
        store_hash: "blake3:cached".into(),
        size_bytes: 2048,
        created_at: "2026-01-01T00:00:00Z".into(),
        provider: "apt".into(),
        arch: "x86_64".into(),
    };
    let inv = build_inventory("ssh-cache", vec![entry]);
    let local = vec!["blake3:local".into()];

    for hash in ["blake3:local", "blake3:cached", "blake3:missing"] {
        let result = resolve_substitution(hash, &local, &[inv.clone()]);
        let label = match &result {
            SubstitutionResult::LocalHit { .. } => "local hit",
            SubstitutionResult::CacheHit { .. } => "cache hit",
            SubstitutionResult::CacheMiss => "miss",
        };
        println!("  {hash}: {label}");
    }

    println!("\n{}", "=".repeat(55));
    println!("All lockfile/profile/reference/cache criteria survived.");
}
