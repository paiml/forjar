//! FJ-1305/1329/1307/1304/1345: Store purity, reproducibility, closure,
//! reference scanning, and diff falsification.
//!
//! Demonstrates Popperian rejection criteria for:
//! - Purity classification (Pure/Pinned/Constrained/Impure)
//! - Reproducibility scoring with grade thresholds
//! - Input closure tracking with transitive dependencies
//! - Reference scanning for store path hashes
//! - Store diff and sync plan computation
//!
//! Usage: cargo run --example store_purity_falsification

use forjar::core::store::closure::{all_closures, closure_hash, input_closure, ResourceInputs};
use forjar::core::store::meta::{new_meta, read_meta, write_meta, Provenance};
use forjar::core::store::purity::{
    classify, level_label, recipe_purity, PurityLevel, PuritySignals,
};
use forjar::core::store::reference::{is_valid_blake3_hash, scan_file_refs};
use forjar::core::store::repro_score::{compute_score, grade, ReproInput};
use forjar::core::store::store_diff::{build_sync_plan, compute_diff};
use std::collections::{BTreeMap, BTreeSet};

fn main() {
    println!("Forjar Store Purity / Repro / Closure / Ref Falsification");
    println!("{}", "=".repeat(60));

    // ── FJ-1305: Purity classification ──
    println!("\n[FJ-1305] Purity Classification:");
    let levels = vec![
        (
            "sandbox-pkg",
            PuritySignals {
                has_version: true,
                has_store: true,
                has_sandbox: true,
                has_curl_pipe: false,
                dep_levels: vec![],
            },
        ),
        (
            "pinned-pkg",
            PuritySignals {
                has_version: true,
                has_store: true,
                has_sandbox: false,
                has_curl_pipe: false,
                dep_levels: vec![],
            },
        ),
        (
            "floating-pkg",
            PuritySignals {
                has_version: false,
                has_store: false,
                has_sandbox: false,
                has_curl_pipe: false,
                dep_levels: vec![],
            },
        ),
        (
            "curl-pkg",
            PuritySignals {
                has_version: true,
                has_store: true,
                has_sandbox: true,
                has_curl_pipe: true,
                dep_levels: vec![],
            },
        ),
    ];

    for (name, signals) in &levels {
        let result = classify(name, signals);
        println!("  {}: {}", name, level_label(result.level));
    }

    let recipe_levels: Vec<PurityLevel> =
        levels.iter().map(|(n, s)| classify(n, s).level).collect();
    let recipe = recipe_purity(&recipe_levels);
    let purity_ok = recipe == PurityLevel::Impure;
    println!(
        "  Recipe purity: {} {}",
        level_label(recipe),
        if purity_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(purity_ok);

    // ── FJ-1329: Reproducibility scoring ──
    println!("\n[FJ-1329] Reproducibility Scoring:");
    let inputs = vec![
        ReproInput {
            name: "pure-pkg".into(),
            purity: PurityLevel::Pure,
            has_store: true,
            has_lock_pin: true,
        },
        ReproInput {
            name: "pinned-pkg".into(),
            purity: PurityLevel::Pinned,
            has_store: true,
            has_lock_pin: false,
        },
    ];
    let score = compute_score(&inputs);
    let repro_ok = score.composite > 0.0 && grade(score.composite) != "F";
    println!(
        "  Score: {:.1}/100 (Grade {}) {}",
        score.composite,
        grade(score.composite),
        if repro_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(repro_ok);

    // ── FJ-1307: Input closure ──
    println!("\n[FJ-1307] Input Closure:");
    let mut graph = BTreeMap::new();
    graph.insert(
        "base".into(),
        ResourceInputs {
            input_hashes: vec!["h-base".into()],
            depends_on: vec![],
        },
    );
    graph.insert(
        "lib".into(),
        ResourceInputs {
            input_hashes: vec!["h-lib".into()],
            depends_on: vec!["base".into()],
        },
    );
    graph.insert(
        "app".into(),
        ResourceInputs {
            input_hashes: vec!["h-app".into()],
            depends_on: vec!["lib".into()],
        },
    );

    let closure = input_closure("app", &graph);
    let closure_ok = closure.len() == 3;
    println!(
        "  app closure: {} inputs {}",
        closure.len(),
        if closure_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(closure_ok);

    let hash = closure_hash(&closure);
    let hash_ok = !hash.is_empty() && closure_hash(&closure) == hash;
    println!(
        "  Closure hash deterministic: {} {}",
        if hash_ok { "yes" } else { "no" },
        if hash_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(hash_ok);

    let all = all_closures(&graph);
    println!("  All closures computed: {} resources", all.len());

    // ── FJ-1304: Reference scanning ──
    println!("\n[FJ-1304] Reference Scanning:");
    let store_hash = format!("blake3:{}", "a".repeat(64));
    let valid_ok = is_valid_blake3_hash(&store_hash);
    println!(
        "  Valid hash detection: {} {}",
        if valid_ok { "yes" } else { "no" },
        if valid_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(valid_ok);

    let content = format!("dep: {store_hash}");
    let mut known = BTreeSet::new();
    known.insert(store_hash.clone());
    let refs = scan_file_refs(content.as_bytes(), &known);
    let ref_ok = refs.contains(&store_hash);
    println!(
        "  Reference found in content: {} {}",
        if ref_ok { "yes" } else { "no" },
        if ref_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(ref_ok);

    // ── FJ-1301: Store metadata ──
    println!("\n[FJ-1301] Store Metadata:");
    let dir = tempfile::tempdir().unwrap();
    let entry_dir = dir.path().join("store/entry");
    let meta = new_meta("sh1", "rh1", &["h1".into()], "x86_64", "apt");
    write_meta(&entry_dir, &meta).unwrap();
    let loaded = read_meta(&entry_dir).unwrap();
    let meta_ok = loaded.store_hash == "sh1" && loaded.provider == "apt";
    println!(
        "  Write/read roundtrip: {} {}",
        if meta_ok { "pass" } else { "fail" },
        if meta_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(meta_ok);

    // ── FJ-1345: Store diff ──
    println!("\n[FJ-1345] Store Diff:");
    let mut meta_diff = new_meta("sh2", "rh2", &[], "x86_64", "apt");
    meta_diff.provenance = Some(Provenance {
        origin_provider: "apt".into(),
        origin_ref: Some("nginx".into()),
        origin_hash: Some("h-old".into()),
        derived_from: None,
        derivation_depth: 0,
    });
    let diff = compute_diff(&meta_diff, Some("h-new"));
    let diff_ok = diff.upstream_changed;
    println!(
        "  Upstream change detected: {} {}",
        if diff_ok { "yes" } else { "no" },
        if diff_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(diff_ok);

    let plan = build_sync_plan(&[(meta_diff, Some("h-new".into()))]);
    let plan_ok = plan.total_steps == 1 && plan.re_imports.len() == 1;
    println!(
        "  Sync plan: {} re-imports {}",
        plan.re_imports.len(),
        if plan_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(plan_ok);

    println!("\n{}", "=".repeat(60));
    println!("All store purity/repro/closure/ref/diff criteria survived.");
}
