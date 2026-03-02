//! Example: Reproducible package management — store, purity, closures, and scoring.
//!
//! Demonstrates the Nix-compatible reproducible package management features:
//! - Content-addressed store paths
//! - Purity classification (4 levels)
//! - Input closure tracking with deterministic hashing
//! - Lock file format for input pinning
//! - Reproducibility scoring (0-100)
//! - Reference scanning for GC
//! - Build sandbox configuration
//! - Binary cache substitution protocol
//! - Universal provider import
//! - Store derivations
//!
//! Usage: cargo run --example store_reproducibility

use forjar::core::store::cache::{
    build_inventory, resolve_substitution, CacheEntry, SubstitutionResult,
};
use forjar::core::store::closure::{all_closures, closure_hash, ResourceInputs};
use forjar::core::store::derivation::{
    derivation_closure_hash, derivation_purity, validate_dag, validate_derivation, Derivation,
    DerivationInput,
};
use forjar::core::store::lockfile::{check_completeness, check_staleness, LockFile, Pin};
use forjar::core::store::path::{store_entry_path, store_path};
use forjar::core::store::provider::{all_providers, capture_method, import_command, ImportConfig, ImportProvider};
use forjar::core::store::purity::{classify, level_label, recipe_purity, PurityLevel, PuritySignals};
use forjar::core::store::reference::is_valid_blake3_hash;
use forjar::core::store::repro_score::{compute_score, grade, ReproInput};
use forjar::core::store::sandbox::{blocks_network, preset_profile, validate_config};
use std::collections::BTreeMap;

fn main() {
    println!("=== Forjar Reproducible Package Management ===\n");

    demo_store_paths();
    demo_purity_classification();
    demo_input_closures();
    demo_lock_file();
    demo_reproducibility_score();
    demo_reference_scanning();
    demo_sandbox_config();
    demo_cache_substitution();
    demo_provider_import();
    demo_derivations();

    println!("\nDone — all store features demonstrated.");
}

fn demo_store_paths() {
    println!("--- 1. Content-Addressed Store Paths ---");
    let hash = store_path(
        "blake3:recipe_abc",
        &["blake3:input1", "blake3:input2"],
        "x86_64",
        "apt",
    );
    println!("Store hash: {hash}");
    println!("Store path: {}", store_entry_path(&hash));

    // Same inputs → same hash (deterministic)
    let hash2 = store_path(
        "blake3:recipe_abc",
        &["blake3:input2", "blake3:input1"], // different order
        "x86_64",
        "apt",
    );
    println!("Same inputs (reordered): {hash2}");
    println!("Deterministic: {}\n", hash == hash2);
}

fn demo_purity_classification() {
    println!("--- 2. Purity Classification (4 levels) ---");

    let cases = vec![
        ("nginx-pure", PuritySignals {
            has_version: true, has_store: true, has_sandbox: true,
            has_curl_pipe: false, dep_levels: vec![],
        }),
        ("nginx-pinned", PuritySignals {
            has_version: true, has_store: true, has_sandbox: false,
            has_curl_pipe: false, dep_levels: vec![],
        }),
        ("nginx-floating", PuritySignals {
            has_version: false, has_store: false, has_sandbox: false,
            has_curl_pipe: false, dep_levels: vec![],
        }),
        ("install-script", PuritySignals {
            has_version: true, has_store: true, has_sandbox: true,
            has_curl_pipe: true, dep_levels: vec![],
        }),
        ("derived-impure", PuritySignals {
            has_version: true, has_store: true, has_sandbox: true,
            has_curl_pipe: false, dep_levels: vec![PurityLevel::Impure],
        }),
    ];

    let mut levels = Vec::new();
    for (name, signals) in &cases {
        let result = classify(name, signals);
        println!("  {}: {} — {}", name, level_label(result.level), result.reasons.join("; "));
        levels.push(result.level);
    }
    println!("  Recipe purity: {}\n", level_label(recipe_purity(&levels)));
}

fn demo_input_closures() {
    println!("--- 3. Input Closure Tracking ---");
    let mut graph = BTreeMap::new();
    graph.insert("base-os".to_string(), ResourceInputs {
        input_hashes: vec!["blake3:ubuntu2404".to_string()],
        depends_on: vec![],
    });
    graph.insert("cuda-toolkit".to_string(), ResourceInputs {
        input_hashes: vec!["blake3:cuda126".to_string()],
        depends_on: vec!["base-os".to_string()],
    });
    graph.insert("ml-rootfs".to_string(), ResourceInputs {
        input_hashes: vec!["blake3:mlconfig".to_string()],
        depends_on: vec!["cuda-toolkit".to_string()],
    });

    let closures = all_closures(&graph);
    for (name, closure) in &closures {
        let hash = closure_hash(closure);
        println!("  {name}: {} inputs, closure hash: {}...",
            closure.len(), &hash[..20]);
    }
    println!();
}

fn demo_lock_file() {
    println!("--- 4. Lock File (Input Pinning) ---");
    let mut pins = BTreeMap::new();
    pins.insert("nginx".to_string(), Pin {
        provider: "apt".to_string(),
        version: Some("1.24.0-1ubuntu1".to_string()),
        hash: "blake3:abc123".to_string(),
        git_rev: None, pin_type: None,
    });
    pins.insert("ripgrep".to_string(), Pin {
        provider: "cargo".to_string(),
        version: Some("14.1.0".to_string()),
        hash: "blake3:def456".to_string(),
        git_rev: None, pin_type: None,
    });
    let lf = LockFile { schema: "1.0".to_string(), pins };

    // Staleness check
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:abc123".to_string());
    current.insert("ripgrep".to_string(), "blake3:UPDATED".to_string());
    let stale = check_staleness(&lf, &current);
    println!("  Stale pins: {}", stale.len());
    for s in &stale {
        println!("    {}: locked={} current={}", s.name, s.locked_hash, s.current_hash);
    }

    // Completeness check
    let inputs = vec!["nginx".to_string(), "ripgrep".to_string(), "python".to_string()];
    let missing = check_completeness(&lf, &inputs);
    println!("  Missing pins: {:?}\n", missing);
}

fn demo_reproducibility_score() {
    println!("--- 5. Reproducibility Score ---");
    let inputs = vec![
        ReproInput { name: "nginx".to_string(), purity: PurityLevel::Pure, has_store: true, has_lock_pin: true },
        ReproInput { name: "config".to_string(), purity: PurityLevel::Pinned, has_store: true, has_lock_pin: true },
        ReproInput { name: "script".to_string(), purity: PurityLevel::Constrained, has_store: false, has_lock_pin: false },
    ];
    let score = compute_score(&inputs);
    println!("  Composite score: {:.1}/100 (Grade: {})", score.composite, grade(score.composite));
    println!("  Purity:  {:.1}", score.purity_score);
    println!("  Store:   {:.1}", score.store_score);
    println!("  Lock:    {:.1}", score.lock_score);
    for r in &score.resources {
        println!("    {}: {:.1} ({:?})", r.name, r.score, r.purity);
    }
    println!();
}

fn demo_reference_scanning() {
    println!("--- 6. Reference Scanning ---");
    let hashes = [
        "blake3:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
        "blake3:short",
        "blake3:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        "sha256:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
    ];
    for h in &hashes {
        println!("  {}: valid={}", h, is_valid_blake3_hash(h));
    }
    println!();
}

fn demo_sandbox_config() {
    println!("--- 7. Build Sandbox Configuration ---");
    for name in &["full", "network-only", "minimal", "gpu"] {
        let cfg = preset_profile(name).unwrap();
        let errors = validate_config(&cfg);
        println!(
            "  {name}: level={:?}, mem={}MB, cpus={}, timeout={}s, net_blocked={}, valid={}",
            cfg.level, cfg.memory_mb, cfg.cpus, cfg.timeout,
            blocks_network(cfg.level), errors.is_empty()
        );
    }
    println!();
}

fn demo_cache_substitution() {
    println!("--- 8. Binary Cache Substitution ---");
    let local = vec!["blake3:local111".to_string()];
    let remote_entries = vec![
        CacheEntry {
            store_hash: "blake3:remote222".to_string(),
            size_bytes: 5_000_000,
            created_at: "2026-03-02T10:00:00Z".to_string(),
            provider: "apt".to_string(),
            arch: "x86_64".to_string(),
        },
    ];
    let inv = build_inventory("cache.internal", remote_entries);

    for hash in &["blake3:local111", "blake3:remote222", "blake3:missing"] {
        let result = resolve_substitution(hash, &local, &[inv.clone()]);
        let label = match &result {
            SubstitutionResult::LocalHit { .. } => "LOCAL HIT",
            SubstitutionResult::CacheHit { .. } => "CACHE HIT",
            SubstitutionResult::CacheMiss => "MISS (build needed)",
        };
        println!("  {hash}: {label}");
    }
    println!();
}

fn demo_provider_import() {
    println!("--- 9. Universal Provider Import ---");
    let configs = vec![
        ImportConfig {
            provider: ImportProvider::Apt,
            reference: "nginx".to_string(),
            version: Some("1.24.0".to_string()),
            arch: "x86_64".to_string(),
            options: BTreeMap::new(),
        },
        ImportConfig {
            provider: ImportProvider::Docker,
            reference: "ubuntu".to_string(),
            version: Some("24.04".to_string()),
            arch: "x86_64".to_string(),
            options: BTreeMap::new(),
        },
        ImportConfig {
            provider: ImportProvider::Nix,
            reference: "nixpkgs#ripgrep".to_string(),
            version: None,
            arch: "x86_64".to_string(),
            options: BTreeMap::new(),
        },
    ];
    for cfg in &configs {
        println!("  {:?}: {}", cfg.provider, import_command(cfg));
        println!("    capture: {}", capture_method(cfg.provider));
    }
    println!("  Total providers: {}\n", all_providers().len());
}

fn demo_derivations() {
    println!("--- 10. Store Derivations ---");
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "base".to_string(),
        DerivationInput::Store { store: "blake3:aaa111".to_string() },
    );
    inputs.insert(
        "cuda".to_string(),
        DerivationInput::Store { store: "blake3:bbb222".to_string() },
    );
    let d = Derivation {
        inputs,
        script: "cp -r $inputs/base/* $out/\ncp -r $inputs/cuda/* $out/usr/local/".to_string(),
        sandbox: Some(preset_profile("full").unwrap()),
        arch: "x86_64".to_string(),
        out_var: "$out".to_string(),
    };
    let errors = validate_derivation(&d);
    println!("  Valid: {} (errors: {})", errors.is_empty(), errors.len());
    println!("  Purity: {:?}", derivation_purity(&d));

    let mut hashes = BTreeMap::new();
    hashes.insert("base".to_string(), "blake3:aaa111".to_string());
    hashes.insert("cuda".to_string(), "blake3:bbb222".to_string());
    let ch = derivation_closure_hash(&d, &hashes);
    println!("  Closure hash: {}...", &ch[..20]);

    // DAG validation
    let mut dag = BTreeMap::new();
    dag.insert("base-os".to_string(), vec![]);
    dag.insert("cuda-toolkit".to_string(), vec!["base-os".to_string()]);
    dag.insert("ml-rootfs".to_string(), vec!["cuda-toolkit".to_string()]);
    let order = validate_dag(&dag).unwrap();
    println!("  DAG order: {:?}", order);
}
