//! FJ-1322/1348: Substitution protocol, conda index parsing.
//!
//! Usage: cargo run --example substitution_conda

use forjar::core::store::cache::{CacheConfig, CacheEntry, CacheInventory, CacheSource};
use forjar::core::store::conda::parse_conda_index;
use forjar::core::store::substitution::{
    plan_substitution, requires_build, requires_pull, step_count, SubstitutionContext,
    SubstitutionOutcome,
};
use std::collections::BTreeMap;
use std::path::Path;

fn main() {
    println!("Forjar: Substitution Protocol & Conda Index");
    println!("{}", "=".repeat(55));

    let store_dir = Path::new("/forjar/store");
    let ssh = CacheSource::Ssh {
        host: "cache.example.com".into(),
        user: "forjar".into(),
        path: "/cache".into(),
        port: Some(2222),
    };

    // ── Local Hit ──
    println!("\n[FJ-1322] Substitution — Local Hit:");
    let hash = "blake3:abc123def456";
    let cc = CacheConfig {
        sources: vec![ssh.clone()],
        auto_push: true,
        max_size_mb: 1024,
    };
    let ctx = SubstitutionContext {
        closure_hash: hash,
        input_hashes: &["blake3:in1".into()],
        local_entries: &[hash.into()],
        cache_config: &cc,
        cache_inventories: &[],
        sandbox: None,
        store_dir,
    };
    let plan = plan_substitution(&ctx);
    println!(
        "  Steps: {} | Build: {} | Pull: {}",
        step_count(&plan),
        requires_build(&plan),
        requires_pull(&plan)
    );
    if let SubstitutionOutcome::LocalHit { store_path } = &plan.outcome {
        println!("  Store path: {store_path}");
    }

    // ── Cache Hit ──
    println!("\n[FJ-1322] Substitution — Cache Hit:");
    let hash2 = "blake3:remote_only";
    let mut entries = BTreeMap::new();
    entries.insert(
        hash2.to_string(),
        CacheEntry {
            store_hash: hash2.into(),
            size_bytes: 4096,
            created_at: "2026-01-01T00:00:00Z".into(),
            provider: "apt".into(),
            arch: "x86_64".into(),
        },
    );
    let inv = CacheInventory {
        source_name: "forjar@cache.example.com".into(),
        entries,
    };
    let ctx2 = SubstitutionContext {
        closure_hash: hash2,
        input_hashes: &[],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[inv],
        sandbox: None,
        store_dir,
    };
    let plan2 = plan_substitution(&ctx2);
    println!(
        "  Steps: {} | Build: {} | Pull: {}",
        step_count(&plan2),
        requires_build(&plan2),
        requires_pull(&plan2)
    );
    if let SubstitutionOutcome::CacheHit { source, .. } = &plan2.outcome {
        println!("  Source: {source}");
    }

    // ── Cache Miss with Auto-Push ──
    println!("\n[FJ-1322] Substitution — Cache Miss (auto-push):");
    let hash3 = "blake3:build_from_scratch";
    let ctx3 = SubstitutionContext {
        closure_hash: hash3,
        input_hashes: &["blake3:in1".into(), "blake3:in2".into()],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[],
        sandbox: None,
        store_dir,
    };
    let plan3 = plan_substitution(&ctx3);
    println!(
        "  Steps: {} | Build: {} | Pull: {}",
        step_count(&plan3),
        requires_build(&plan3),
        requires_pull(&plan3)
    );
    for step in &plan3.steps {
        println!("    {:?}", step);
    }

    // ── Conda Index ──
    println!("\n[FJ-1348] Conda Index Parsing:");
    let json = r#"{"name": "numpy", "version": "1.26.4", "build": "py312h",
        "arch": "x86_64", "subdir": "linux-64"}"#;
    let info = parse_conda_index(json).unwrap();
    println!(
        "  {}-{} (build={}, arch={}, subdir={})",
        info.name, info.version, info.build, info.arch, info.subdir
    );

    let minimal = r#"{"name": "pip", "version": "24.0"}"#;
    let info2 = parse_conda_index(minimal).unwrap();
    println!(
        "  {}-{} (arch={}, subdir={})",
        info2.name, info2.version, info2.arch, info2.subdir
    );

    println!("\n{}", "=".repeat(55));
    println!("All substitution/conda criteria survived.");
}
