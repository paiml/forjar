//! Demonstrates the store cache protocol: pull/push command generation,
//! substitution protocol execution, and cache verification.
//!
//! Run: `cargo run --example store_cache_protocol`

use forjar::core::store::cache::{build_inventory, CacheConfig, CacheEntry, CacheSource};
use forjar::core::store::cache_exec::{pull_command, push_command};
use forjar::core::store::substitution::{
    plan_substitution, requires_build, requires_pull, step_count, SubstitutionContext,
    SubstitutionOutcome,
};
use std::path::Path;

fn main() {
    println!("=== Forjar Cache Protocol Demo ===\n");
    demo_pull_push_commands();
    demo_substitution_local();
    demo_substitution_cache();
    demo_substitution_miss();
    println!("\n=== All cache protocol demos passed ===");
}

fn demo_pull_push_commands() {
    println!("--- 1. Pull/Push Command Generation ---");

    let ssh_source = CacheSource::Ssh {
        host: "cache.internal".to_string(),
        user: "forjar".to_string(),
        path: "/var/forjar/cache".to_string(),
        port: None,
    };

    let staging = Path::new("/tmp/forjar-staging/abc123");
    let pull = pull_command(&ssh_source, "blake3:abc123def456", staging);
    println!("  Pull: {pull}");
    assert!(pull.contains("rsync"));
    assert!(pull.contains("cache.internal"));

    let push = push_command(
        &ssh_source,
        "blake3:abc123def456",
        Path::new("/var/forjar/store"),
    );
    println!("  Push: {push}");
    assert!(push.contains("rsync"));

    // SSH with custom port
    let ssh_port = CacheSource::Ssh {
        host: "cache.example.com".to_string(),
        user: "deploy".to_string(),
        path: "/cache".to_string(),
        port: Some(2222),
    };
    let pull_port = pull_command(&ssh_port, "blake3:xyz789", staging);
    println!("  Pull (port 2222): {pull_port}");
    assert!(pull_port.contains("2222"));

    // Local source
    let local_source = CacheSource::Local {
        path: "/var/cache/forjar".to_string(),
    };
    let pull_local = pull_command(&local_source, "blake3:local123", staging);
    println!("  Pull (local): {pull_local}");
    assert!(pull_local.contains("cp -a") || pull_local.contains("rsync"));

    println!("  All pull/push commands generated correctly");
}

fn demo_substitution_local() {
    println!("\n--- 2. Substitution — Local Hit ---");

    let cc = CacheConfig {
        sources: vec![],
        auto_push: false,
        max_size_mb: 0,
    };
    let local = vec!["blake3:found_locally".to_string()];
    let ctx = SubstitutionContext {
        closure_hash: "blake3:found_locally",
        input_hashes: &[],
        local_entries: &local,
        cache_config: &cc,
        cache_inventories: &[],
        sandbox: None,
        store_dir: Path::new("/var/forjar/store"),
    };

    let plan = plan_substitution(&ctx);
    println!(
        "  Steps: {} | Build: {} | Pull: {}",
        step_count(&plan),
        requires_build(&plan),
        requires_pull(&plan)
    );

    if let SubstitutionOutcome::LocalHit { store_path } = &plan.outcome {
        println!("  Outcome: LOCAL HIT → {store_path}");
    }
    assert!(!requires_build(&plan));
    assert!(!requires_pull(&plan));
    println!("  Local hit: zero I/O required");
}

fn demo_substitution_cache() {
    println!("\n--- 3. Substitution — SSH Cache Hit ---");

    let cc = CacheConfig {
        sources: vec![CacheSource::Ssh {
            host: "cache.internal".to_string(),
            user: "forjar".to_string(),
            path: "/var/forjar/cache".to_string(),
            port: None,
        }],
        auto_push: false,
        max_size_mb: 0,
    };

    let inv = build_inventory(
        "cache.internal",
        vec![CacheEntry {
            store_hash: "blake3:in_cache".to_string(),
            size_bytes: 8192,
            created_at: "2026-01-20T10:00:00Z".to_string(),
            provider: "apt".to_string(),
            arch: "x86_64".to_string(),
        }],
    );
    let invs = [inv];

    let ctx = SubstitutionContext {
        closure_hash: "blake3:in_cache",
        input_hashes: &[],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &invs,
        sandbox: None,
        store_dir: Path::new("/var/forjar/store"),
    };

    let plan = plan_substitution(&ctx);
    assert!(requires_pull(&plan));
    assert!(!requires_build(&plan));

    if let SubstitutionOutcome::CacheHit { source, store_hash } = &plan.outcome {
        println!("  CACHE HIT: {store_hash} from {source}");
    }
    println!("  SSH cache hit: pull required, no build");
}

fn demo_substitution_miss() {
    println!("\n--- 4. Substitution — Cache Miss ---");

    let cc = CacheConfig {
        sources: vec![],
        auto_push: true,
        max_size_mb: 0,
    };

    let ctx = SubstitutionContext {
        closure_hash: "blake3:never_cached",
        input_hashes: &[],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[],
        sandbox: None,
        store_dir: Path::new("/var/forjar/store"),
    };

    let plan = plan_substitution(&ctx);
    assert!(requires_build(&plan));
    println!(
        "  CACHE MISS: build from scratch (auto_push={})",
        cc.auto_push
    );
    println!("  Steps: {}", step_count(&plan));
    println!("  Cache miss: full build required");
}
