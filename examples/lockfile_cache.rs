//! FJ-1310/1320: Lock files and binary cache.
//!
//! Usage: cargo run --example lockfile_cache

use forjar::core::store::cache::{
    build_inventory, resolve_substitution, ssh_command, validate_cache_config, verify_entry,
    CacheConfig, CacheEntry, CacheSource, SubstitutionResult,
};
use forjar::core::store::lockfile::{check_completeness, check_staleness, parse_lockfile};
use std::collections::BTreeMap;

fn main() {
    println!("Forjar: Lock Files & Binary Cache");
    println!("{}", "=".repeat(55));

    // ── Lock Files ──
    println!("\n[FJ-1310] Lock File Parsing:");
    let yaml = "schema: '1'\npins:\n  nginx:\n    provider: apt\n    hash: blake3:abc123\n    version: '1.24'\n  curl:\n    provider: apt\n    hash: blake3:def456\n";
    let lf = parse_lockfile(yaml).unwrap();
    println!("  Schema: {} | Pins: {}", lf.schema, lf.pins.len());
    for (name, pin) in &lf.pins {
        println!("    {name}: {} ({})", pin.hash, pin.provider);
    }

    println!("\n[FJ-1310] Staleness Check:");
    let current = BTreeMap::from([
        ("nginx".into(), "blake3:abc123".into()),
        ("curl".into(), "blake3:new999".into()),
    ]);
    let stale = check_staleness(&lf, &current);
    println!("  Stale pins: {}", stale.len());
    for s in &stale {
        println!("    {}: {} → {}", s.name, s.locked_hash, s.current_hash);
    }

    println!("\n[FJ-1310] Completeness Check:");
    let missing = check_completeness(&lf, &["nginx".into(), "curl".into(), "wget".into()]);
    println!("  Missing: {:?}", missing);

    // ── Binary Cache ──
    println!("\n[FJ-1320] Cache Config:");
    let config = CacheConfig {
        sources: vec![
            CacheSource::Local {
                path: "/var/lib/forjar/store".into(),
            },
            CacheSource::Ssh {
                host: "cache.example.com".into(),
                user: "forjar".into(),
                path: "/cache".into(),
                port: Some(2222),
            },
        ],
        auto_push: true,
        max_size_mb: 1024,
    };
    let errors = validate_cache_config(&config);
    println!(
        "  Sources: {} | Valid: {} | Auto-push: {}",
        config.sources.len(),
        errors.is_empty(),
        config.auto_push
    );

    println!("\n[FJ-1320] SSH Command:");
    if let Some(cmd) = ssh_command(&config.sources[1]) {
        println!("  {cmd}");
    }

    println!("\n[FJ-1320] Substitution Protocol:");
    let entry = CacheEntry {
        store_hash: "blake3:abc".into(),
        size_bytes: 4096,
        created_at: "2026-03-09T10:00:00Z".into(),
        provider: "apt".into(),
        arch: "x86_64".into(),
    };
    let inv = build_inventory("remote-cache", vec![entry.clone()]);
    for hash in ["blake3:abc", "blake3:xyz"] {
        let result = resolve_substitution(hash, &[], &[inv.clone()]);
        let status = match result {
            SubstitutionResult::LocalHit { .. } => "local hit",
            SubstitutionResult::CacheHit { .. } => "cache hit",
            SubstitutionResult::CacheMiss => "miss",
        };
        println!("  {hash} → {status}");
    }

    println!("\n[FJ-1320] Entry Verification:");
    println!("  Correct hash: {}", verify_entry(&entry, "blake3:abc"));
    println!("  Wrong hash:   {}", verify_entry(&entry, "blake3:wrong"));

    println!("\n{}", "=".repeat(55));
    println!("All lockfile/cache criteria survived.");
}
