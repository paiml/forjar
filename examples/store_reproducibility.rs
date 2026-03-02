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
//! - Purity + reproducibility validation
//! - FAR archive encoding and decoding
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
use forjar::core::store::far::{
    decode_far_manifest, encode_far, FarFileEntry, FarManifest, FarProvenance,
};
use forjar::core::store::gc::{collect_roots, GcConfig};
use forjar::core::store::lockfile::{check_completeness, check_staleness, LockFile, Pin};
use forjar::core::store::path::{store_entry_path, store_path};
use forjar::core::store::provider::{
    all_providers, capture_method, import_command, ImportConfig, ImportProvider,
};
use forjar::core::store::purity::{
    classify, level_label, recipe_purity, PurityLevel, PuritySignals,
};
use forjar::core::store::reference::is_valid_blake3_hash;
use forjar::core::store::repro_score::{compute_score, grade, ReproInput};
use forjar::core::store::sandbox::{blocks_network, preset_profile, validate_config};
use forjar::core::store::validate::{
    format_purity_report, format_repro_report, validate_purity, validate_repro_score,
};
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
    demo_validation();
    demo_far_archive();
    demo_gc_roots();

    println!("\nDone — all store features demonstrated (13 sections).");
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

fn sigs(
    ver: bool,
    store: bool,
    sandbox: bool,
    curl: bool,
    deps: Vec<PurityLevel>,
) -> PuritySignals {
    PuritySignals {
        has_version: ver,
        has_store: store,
        has_sandbox: sandbox,
        has_curl_pipe: curl,
        dep_levels: deps,
    }
}

fn demo_purity_classification() {
    println!("--- 2. Purity Classification (4 levels) ---");

    let cases = vec![
        ("nginx-pure", sigs(true, true, true, false, vec![])),
        ("nginx-pinned", sigs(true, true, false, false, vec![])),
        ("nginx-floating", sigs(false, false, false, false, vec![])),
        ("install-script", sigs(true, true, true, true, vec![])),
        (
            "derived-impure",
            sigs(true, true, true, false, vec![PurityLevel::Impure]),
        ),
    ];

    let mut levels = Vec::new();
    for (name, signals) in &cases {
        let result = classify(name, signals);
        println!(
            "  {}: {} — {}",
            name,
            level_label(result.level),
            result.reasons.join("; ")
        );
        levels.push(result.level);
    }
    println!("  Recipe purity: {}\n", level_label(recipe_purity(&levels)));
}

fn ri(hashes: &[&str], deps: &[&str]) -> ResourceInputs {
    ResourceInputs {
        input_hashes: hashes.iter().map(|s| s.to_string()).collect(),
        depends_on: deps.iter().map(|s| s.to_string()).collect(),
    }
}

fn demo_input_closures() {
    println!("--- 3. Input Closure Tracking ---");
    let mut graph = BTreeMap::new();
    graph.insert("base-os".to_string(), ri(&["blake3:ubuntu2404"], &[]));
    graph.insert(
        "cuda-toolkit".to_string(),
        ri(&["blake3:cuda126"], &["base-os"]),
    );
    graph.insert(
        "ml-rootfs".to_string(),
        ri(&["blake3:mlconfig"], &["cuda-toolkit"]),
    );

    let closures = all_closures(&graph);
    for (name, closure) in &closures {
        let hash = closure_hash(closure);
        println!(
            "  {name}: {} inputs, closure hash: {}...",
            closure.len(),
            &hash[..20]
        );
    }
    println!();
}

fn pin(prov: &str, ver: &str, hash: &str) -> Pin {
    Pin {
        provider: prov.to_string(),
        version: Some(ver.to_string()),
        hash: hash.to_string(),
        git_rev: None,
        pin_type: None,
    }
}

fn demo_lock_file() {
    println!("--- 4. Lock File (Input Pinning) ---");
    let mut pins = BTreeMap::new();
    pins.insert(
        "nginx".to_string(),
        pin("apt", "1.24.0-1ubuntu1", "blake3:abc123"),
    );
    pins.insert(
        "ripgrep".to_string(),
        pin("cargo", "14.1.0", "blake3:def456"),
    );
    let lf = LockFile {
        schema: "1.0".to_string(),
        pins,
    };

    // Staleness check
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:abc123".to_string());
    current.insert("ripgrep".to_string(), "blake3:UPDATED".to_string());
    let stale = check_staleness(&lf, &current);
    println!("  Stale pins: {}", stale.len());
    for s in &stale {
        println!(
            "    {}: locked={} current={}",
            s.name, s.locked_hash, s.current_hash
        );
    }

    // Completeness check
    let inputs = vec![
        "nginx".to_string(),
        "ripgrep".to_string(),
        "python".to_string(),
    ];
    let missing = check_completeness(&lf, &inputs);
    println!("  Missing pins: {:?}\n", missing);
}

fn repro(name: &str, purity: PurityLevel, store: bool, lock: bool) -> ReproInput {
    ReproInput {
        name: name.to_string(),
        purity,
        has_store: store,
        has_lock_pin: lock,
    }
}

fn demo_reproducibility_score() {
    println!("--- 5. Reproducibility Score ---");
    let inputs = vec![
        repro("nginx", PurityLevel::Pure, true, true),
        repro("config", PurityLevel::Pinned, true, true),
        repro("script", PurityLevel::Constrained, false, false),
    ];
    let score = compute_score(&inputs);
    println!(
        "  Composite score: {:.1}/100 (Grade: {})",
        score.composite,
        grade(score.composite)
    );
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
            cfg.level,
            cfg.memory_mb,
            cfg.cpus,
            cfg.timeout,
            blocks_network(cfg.level),
            errors.is_empty()
        );
    }
    println!();
}

fn demo_cache_substitution() {
    println!("--- 8. Binary Cache Substitution ---");
    let local = vec!["blake3:local111".to_string()];
    let remote_entries = vec![CacheEntry {
        store_hash: "blake3:remote222".to_string(),
        size_bytes: 5_000_000,
        created_at: "2026-03-02T10:00:00Z".to_string(),
        provider: "apt".to_string(),
        arch: "x86_64".to_string(),
    }];
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

fn imp(prov: ImportProvider, reference: &str, ver: Option<&str>) -> ImportConfig {
    ImportConfig {
        provider: prov,
        reference: reference.to_string(),
        version: ver.map(|s| s.to_string()),
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    }
}

fn demo_provider_import() {
    println!("--- 9. Universal Provider Import ---");
    let configs = vec![
        imp(ImportProvider::Apt, "nginx", Some("1.24.0")),
        imp(ImportProvider::Docker, "ubuntu", Some("24.04")),
        imp(ImportProvider::Nix, "nixpkgs#ripgrep", None),
    ];
    for cfg in &configs {
        println!("  {:?}: {}", cfg.provider, import_command(cfg));
        println!("    capture: {}", capture_method(cfg.provider));
    }
    println!("  Total providers: {}\n", all_providers().len());
}

fn demo_derivations() {
    println!("--- 10. Store Derivations ---");
    let inputs = BTreeMap::from([
        (
            "base".to_string(),
            DerivationInput::Store {
                store: "blake3:aaa111".to_string(),
            },
        ),
        (
            "cuda".to_string(),
            DerivationInput::Store {
                store: "blake3:bbb222".to_string(),
            },
        ),
    ]);
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
    println!("  DAG order: {:?}\n", order);
}

fn demo_validation() {
    println!("--- 14. Purity & Reproducibility Validation ---");

    let pure_sig = sigs(true, true, true, false, vec![]);
    let pinned_sig = sigs(true, true, false, false, vec![]);
    let purity_result = validate_purity(
        &[("nginx", &pure_sig), ("redis", &pinned_sig)],
        Some(PurityLevel::Pinned),
    );
    println!("{}\n", format_purity_report(&purity_result));

    // Repro score validation
    let inputs = vec![
        repro("nginx", PurityLevel::Pure, true, true),
        repro("redis", PurityLevel::Pinned, true, true),
    ];
    let repro_result = validate_repro_score(&inputs, Some(75.0));
    println!("{}", format_repro_report(&repro_result));
}

fn demo_far_archive() {
    println!("\n--- 15. FAR Archive Encode/Decode ---");

    let manifest = FarManifest {
        name: "nginx".to_string(),
        version: "1.24.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:aabb1122".to_string(),
        tree_hash: "blake3:ccdd3344".to_string(),
        file_count: 1,
        total_size: 1024,
        files: vec![FarFileEntry {
            path: "usr/sbin/nginx".to_string(),
            size: 1024,
            blake3: "blake3:ff001122".to_string(),
        }],
        provenance: FarProvenance {
            origin_provider: "apt".to_string(),
            origin_ref: Some("nginx=1.24.0".to_string()),
            origin_hash: None,
            created_at: "2026-03-02T10:00:00Z".to_string(),
            generator: "forjar 1.0.0".to_string(),
        },
        kernel_contracts: None,
    };

    let data = b"fake binary content for demo";
    let hash = blake3::hash(data);
    let chunks = vec![(*hash.as_bytes(), data.to_vec())];

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();
    println!("  Encoded FAR: {} bytes, 1 chunk", buf.len());
    let cursor = std::io::Cursor::new(&buf);
    let (decoded, chunk_table) = decode_far_manifest(cursor).unwrap();
    println!(
        "  Decoded: {} v{} ({} chunks)",
        decoded.name,
        decoded.version,
        chunk_table.len()
    );
    assert_eq!(manifest.store_hash, decoded.store_hash);
    println!("  Roundtrip verified");
}

fn demo_gc_roots() {
    println!("\n--- 16. GC Roots Collection ---");

    let _config = GcConfig::default();
    let profiles = vec!["blake3:gen1".to_string(), "blake3:gen2".to_string()];
    let locks = vec!["blake3:nginx".to_string(), "blake3:curl".to_string()];

    let roots = collect_roots(&profiles, &locks, None);
    println!("  GC roots collected: {}", roots.len());
    for root in &roots {
        println!("    {root}");
    }
    assert!(
        roots.contains("blake3:gen1"),
        "profile gen should be a root"
    );
    assert!(roots.contains("blake3:nginx"), "lock pin should be a root");
    println!("  GC root invariant verified");
}
