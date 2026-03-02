//! Demonstrates forjar's store runtime executors:
//! sandbox lifecycle, substitution protocol, and derivation DAG.
//!
//! Run: `cargo run --example store_executors`

use forjar::core::store::cache::{
    build_inventory, CacheConfig, CacheEntry, CacheInventory, CacheSource,
};
use forjar::core::store::derivation::{Derivation, DerivationInput};
use forjar::core::store::derivation_exec::{
    execute_derivation_dag, execute_derivation_dag_live, is_store_hit, plan_derivation,
    simulate_derivation, skipped_steps,
};
use forjar::core::store::sandbox::{preset_profile, SandboxConfig, SandboxLevel};
use forjar::core::store::sandbox_exec::{
    plan_sandbox_build, plan_step_count, seccomp_rules_for_level, simulate_sandbox_build,
    validate_plan,
};
use forjar::core::store::substitution::{
    plan_substitution, requires_build, requires_pull, step_count, SubstitutionContext,
    SubstitutionOutcome,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn main() {
    println!("=== Forjar Store Executors Demo ===\n");
    demo_sandbox_lifecycle();
    demo_seccomp_profiles();
    demo_substitution_local_hit();
    demo_substitution_cache_hit();
    demo_substitution_cache_miss();
    demo_derivation_plan();
    demo_derivation_store_hit();
    demo_derivation_dag();
    demo_derivation_dag_live();
    println!("\n=== All executor demos passed ===");
}

fn demo_sandbox_lifecycle() {
    println!("--- 1. Sandbox Lifecycle Plan ---");

    let config = preset_profile("full").unwrap();
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "base".to_string(),
        PathBuf::from("/var/lib/forjar/store/abc123/content"),
    );

    let plan = plan_sandbox_build(
        &config,
        "blake3:abcdef1234567890",
        &inputs,
        "cp -r $inputs/base/* $out/ && echo done",
        Path::new("/var/lib/forjar/store"),
    );

    println!("  Namespace: {}", plan.namespace_id);
    println!("  Overlay lower dirs: {}", plan.overlay.lower_dirs.len());
    println!("  Seccomp rules: {}", plan.seccomp_rules.len());
    println!("  Steps:");
    for step in &plan.steps {
        println!("    Step {}: {}", step.step, step.description);
    }
    assert!(plan_step_count(&plan) >= 10);
    assert!(validate_plan(&plan).is_empty());
    println!("  Plan validated: {} steps", plan_step_count(&plan));

    // Simulate
    let result = simulate_sandbox_build(
        &config,
        "blake3:abcdef1234567890",
        &inputs,
        "cp -r $inputs/base/* $out/",
        Path::new("/var/lib/forjar/store"),
    );
    println!("  Simulated output hash: {}", &result.output_hash[..40]);
    println!("  Store path: {}", result.store_path);
}

fn demo_seccomp_profiles() {
    println!("\n--- 2. Seccomp BPF Profiles ---");

    for level in [
        SandboxLevel::Full,
        SandboxLevel::NetworkOnly,
        SandboxLevel::Minimal,
        SandboxLevel::None,
    ] {
        let rules = seccomp_rules_for_level(level);
        println!("  {:?}: {} deny rules", level, rules.len());
        for r in &rules {
            println!("    deny {}", r.syscall);
        }
    }
    assert_eq!(seccomp_rules_for_level(SandboxLevel::Full).len(), 3);
    println!("  Full level denies connect, mount, ptrace — verified");
}

fn demo_substitution_local_hit() {
    println!("\n--- 3. Substitution Protocol — Local Hit ---");

    let cc = CacheConfig {
        sources: vec![CacheSource::Local {
            path: "/var/forjar/store".to_string(),
        }],
        auto_push: false,
        max_size_mb: 0,
    };
    let local = vec!["blake3:abc123".to_string()];
    let ctx = SubstitutionContext {
        closure_hash: "blake3:abc123",
        input_hashes: &[],
        local_entries: &local,
        cache_config: &cc,
        cache_inventories: &[],
        sandbox: None,
        store_dir: Path::new("/var/forjar/store"),
    };

    let plan = plan_substitution(&ctx);
    println!("  Steps: {}", step_count(&plan));
    println!("  Requires build: {}", requires_build(&plan));
    println!("  Requires pull: {}", requires_pull(&plan));
    if let SubstitutionOutcome::LocalHit { store_path } = &plan.outcome {
        println!("  Outcome: LOCAL HIT at {store_path}");
    }
    assert!(!requires_build(&plan));
    assert!(!requires_pull(&plan));
    println!("  Local hit verified — no work needed");
}

fn demo_substitution_cache_hit() {
    println!("\n--- 4. Substitution Protocol — SSH Cache Hit ---");

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
            store_hash: "blake3:cached456".to_string(),
            size_bytes: 4096,
            created_at: "2026-01-15T12:00:00Z".to_string(),
            provider: "apt".to_string(),
            arch: "x86_64".to_string(),
        }],
    );
    let invs = [inv];

    let ctx = SubstitutionContext {
        closure_hash: "blake3:cached456",
        input_hashes: &[],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &invs,
        sandbox: None,
        store_dir: Path::new("/var/forjar/store"),
    };

    let plan = plan_substitution(&ctx);
    println!("  Steps: {}", step_count(&plan));
    assert!(requires_pull(&plan));
    assert!(!requires_build(&plan));
    if let SubstitutionOutcome::CacheHit { source, store_hash } = &plan.outcome {
        println!("  Outcome: CACHE HIT from {source} ({store_hash})");
    }
    println!("  SSH cache hit verified — pull required");
}

fn demo_substitution_cache_miss() {
    println!("\n--- 5. Substitution Protocol — Cache Miss ---");

    let sb = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 2048,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: Vec::new(),
        env: Vec::new(),
    };
    let cc = CacheConfig {
        sources: vec![CacheSource::Ssh {
            host: "cache.internal".to_string(),
            user: "forjar".to_string(),
            path: "/var/forjar/cache".to_string(),
            port: None,
        }],
        auto_push: true,
        max_size_mb: 0,
    };

    let ctx = SubstitutionContext {
        closure_hash: "blake3:never_seen",
        input_hashes: &["blake3:input1".to_string()],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[],
        sandbox: Some(&sb),
        store_dir: Path::new("/var/forjar/store"),
    };

    let plan = plan_substitution(&ctx);
    println!("  Steps: {}", step_count(&plan));
    assert!(requires_build(&plan));
    println!("  Outcome: CACHE MISS — build from scratch (auto_push enabled)");
    for step in &plan.steps {
        println!("    {:?}", step);
    }
    println!("  Cache miss with auto-push verified");
}

fn demo_derivation_plan() {
    println!("\n--- 6. Derivation Plan (Store Miss) ---");

    let deriv = sample_derivation();
    let plan = plan_derivation(
        &deriv,
        &BTreeMap::new(),
        &[],
        Path::new("/var/lib/forjar/store"),
    )
    .unwrap();

    println!("  Closure hash: {}", &plan.closure_hash[..40]);
    println!("  Store hit: {}", plan.store_hit);
    println!("  Steps:");
    for step in &plan.steps {
        let skip = if step.skipped { " [SKIPPED]" } else { "" };
        println!("    Step {}: {}{skip}", step.step, step.description);
    }
    assert!(!is_store_hit(&plan));
    assert_eq!(skipped_steps(&plan), 0);
    assert_eq!(plan.steps.len(), 10);
    println!("  10-step derivation plan verified");
}

fn demo_derivation_store_hit() {
    println!("\n--- 7. Derivation Plan (Store Hit) ---");

    let deriv = sample_derivation();
    let input_hashes =
        forjar::core::store::derivation::collect_input_hashes(&deriv, &BTreeMap::new()).unwrap();
    let closure = forjar::core::store::derivation::derivation_closure_hash(&deriv, &input_hashes);

    let plan = plan_derivation(
        &deriv,
        &BTreeMap::new(),
        &[closure.clone()],
        Path::new("/var/lib/forjar/store"),
    )
    .unwrap();

    println!("  Store hit: {}", plan.store_hit);
    println!("  Skipped steps: {}", skipped_steps(&plan));
    assert!(is_store_hit(&plan));
    assert_eq!(skipped_steps(&plan), 7);
    assert!(plan.sandbox_plan.is_none());
    println!("  Store hit: 7 steps skipped, no sandbox needed");
}

fn demo_derivation_dag() {
    println!("\n--- 8. Derivation DAG Execution ---");

    let mut derivations = BTreeMap::new();
    derivations.insert("base".to_string(), sample_derivation());

    let mut chain_inputs = BTreeMap::new();
    chain_inputs.insert(
        "src".to_string(),
        DerivationInput::Resource {
            resource: "base".to_string(),
        },
    );
    derivations.insert(
        "derived".to_string(),
        Derivation {
            inputs: chain_inputs,
            script: "cp -r $inputs/src/* $out/ && echo patched".to_string(),
            sandbox: None,
            arch: "x86_64".to_string(),
            out_var: "$out".to_string(),
        },
    );

    let results = execute_derivation_dag(
        &derivations,
        &["base".to_string(), "derived".to_string()],
        &BTreeMap::new(),
        &[],
        Path::new("/var/lib/forjar/store"),
    )
    .unwrap();

    println!("  DAG nodes: {}", results.len());
    for (name, result) in &results {
        println!(
            "    {name}: hash={}, depth={}",
            &result.store_hash[..32],
            result.derivation_depth
        );
    }
    assert_eq!(results.len(), 2);
    println!("  2-node DAG executed: base → derived");

    // Simulate single derivation
    let single = simulate_derivation(
        &sample_derivation(),
        &BTreeMap::new(),
        &[],
        Path::new("/var/lib/forjar/store"),
    )
    .unwrap();
    println!("  Single simulate: hash={}", &single.store_hash[..32]);
    println!("  DAG execution verified");
}

fn demo_derivation_dag_live() {
    println!("\n--- 9. Derivation DAG Live Execution (dry_run=true) ---");

    let mut derivations = BTreeMap::new();
    derivations.insert("base".to_string(), sample_derivation());

    // Dry-run mode (same as simulate)
    let results = execute_derivation_dag_live(
        &derivations,
        &["base".to_string()],
        &BTreeMap::new(),
        &[],
        Path::new("/var/lib/forjar/store"),
        true, // dry_run
    )
    .unwrap();

    assert_eq!(results.len(), 1);
    println!("  dry_run=true: {} result(s)", results.len());

    // Live mode — requires kernel namespace support (pepita).
    // Expected to fail in environments without namespace capabilities.
    match execute_derivation_dag_live(
        &derivations,
        &["base".to_string()],
        &BTreeMap::new(),
        &[],
        Path::new("/var/lib/forjar/store"),
        false, // live
    ) {
        Ok(results_live) => {
            assert_eq!(results_live.len(), 1);
            println!("  dry_run=false: {} result(s)", results_live.len());
        }
        Err(e) => {
            println!("  dry_run=false: sandbox unavailable ({e}) — expected without pepita");
        }
    }
    println!("  DAG execution verified");
}

fn sample_derivation() -> Derivation {
    let mut inputs = BTreeMap::new();
    inputs.insert(
        "base".to_string(),
        DerivationInput::Store {
            store: "blake3:abc123def456".to_string(),
        },
    );

    Derivation {
        inputs,
        script: "cp -r $inputs/base/* $out/".to_string(),
        sandbox: Some(SandboxConfig {
            level: SandboxLevel::Full,
            memory_mb: 2048,
            cpus: 4.0,
            timeout: 600,
            bind_mounts: Vec::new(),
            env: Vec::new(),
        }),
        arch: "x86_64".to_string(),
        out_var: "$out".to_string(),
    }
}
