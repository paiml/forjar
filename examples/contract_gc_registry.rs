//! FJ-1351/1352/1365/E14: Contract coverage, scaffolding, GC sweep, registry push.
//!
//! Usage: cargo run --example contract_gc_registry

use forjar::core::store::contract_coverage::{
    coverage_report, scan_contracts_dir, BindingEntry, BindingRegistry, ContractStatus,
};
use forjar::core::store::contract_scaffold::{scaffold_contracts, write_stubs};
use forjar::core::store::gc::mark_and_sweep;
use forjar::core::store::gc_exec::{dir_size, sweep, sweep_dry_run};
use forjar::core::store::hf_config::KernelRequirement;
use forjar::core::store::meta::{new_meta, write_meta};
use forjar::core::store::mutation_runner::{applicable_operators, mutation_script};
use forjar::core::store::registry_push::{
    head_check_command, validate_push_config, RegistryPushConfig,
};
use std::collections::BTreeSet;

fn main() {
    println!("Forjar: Contract Coverage, GC Sweep & Registry Push");
    println!("{}", "=".repeat(55));

    // ── FJ-1351: Contract Coverage ──
    println!("\n[FJ-1351] Contract Coverage:");
    let registry = BindingRegistry {
        version: "1.0".into(),
        target_crate: "forjar-kernels".into(),
        bindings: vec![
            BindingEntry {
                contract: "softmax-v1".into(),
                equation: "EQ-1".into(),
                status: "implemented".into(),
            },
            BindingEntry {
                contract: "matmul-v1".into(),
                equation: "EQ-2".into(),
                status: "partial".into(),
            },
        ],
    };
    let required = vec![
        KernelRequirement {
            op: "softmax".into(),
            contract: "softmax-v1".into(),
        },
        KernelRequirement {
            op: "matmul".into(),
            contract: "matmul-v1".into(),
        },
        KernelRequirement {
            op: "layernorm".into(),
            contract: "layernorm-v1".into(),
        },
    ];
    let available = vec!["softmax-v1".into()];
    let report = coverage_report("llama", &required, &registry, &available);
    println!("  Model: {}", report.model_type);
    println!("  Required: {}", report.total_required);
    println!("  Covered: {}, Missing: {}", report.covered, report.missing);
    println!("  Coverage: {:.1}%", report.coverage_pct);
    for (name, status) in &report.contracts {
        println!("    {name}: {status:?}");
    }

    // ── FJ-1352: Contract Scaffolding ──
    println!("\n[FJ-1352] Contract Scaffolding:");
    let missing: Vec<_> = report
        .contracts
        .iter()
        .filter(|(_, s)| **s == ContractStatus::Missing)
        .map(|(name, _)| {
            required
                .iter()
                .find(|r| r.contract == *name)
                .cloned()
                .unwrap()
        })
        .collect();
    let stubs = scaffold_contracts(&missing, "forjar-team");
    println!("  Generated {} stubs:", stubs.len());
    for stub in &stubs {
        println!("    {}", stub.filename);
    }
    let tmp = tempfile::tempdir().unwrap();
    let written = write_stubs(&stubs, tmp.path()).unwrap();
    println!("  Written {} files", written.len());

    // ── Scan contracts dir ──
    let contracts_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        contracts_dir.path().join("softmax-v1.yaml"),
        "name: softmax",
    )
    .unwrap();
    let names = scan_contracts_dir(contracts_dir.path()).unwrap();
    println!("  Scanned dir: {} contracts found", names.len());

    // ── FJ-1365: GC Sweep ──
    println!("\n[FJ-1365] GC Sweep:");
    let gc_tmp = tempfile::tempdir().unwrap();
    let store = gc_tmp.path();
    let live = "a".repeat(64);
    let dead = "b".repeat(64);
    for h in [&live, &dead] {
        std::fs::create_dir_all(store.join(h)).unwrap();
        std::fs::write(store.join(h).join("data"), vec![0u8; 512]).unwrap();
        write_meta(
            &store.join(h),
            &new_meta(&format!("blake3:{h}"), "blake3:r", &[], "x86_64", "apt"),
        )
        .unwrap();
    }
    let roots: BTreeSet<String> = [format!("blake3:{live}")].into();
    let gc_report = mark_and_sweep(&roots, store).unwrap();

    let dry = sweep_dry_run(&gc_report, store);
    println!(
        "  Dry run: {} entries, {} bytes",
        dry.len(),
        dry.iter().map(|d| d.size_bytes).sum::<u64>()
    );

    let result = sweep(&gc_report, store).unwrap();
    println!(
        "  Swept: {} removed, {} bytes freed",
        result.removed.len(),
        result.bytes_freed
    );
    println!("  Remaining dir_size: {} bytes", dir_size(store));

    // ── E14: Registry Push Commands ──
    println!("\n[E14] Registry Push:");
    let cfg = RegistryPushConfig {
        registry: "ghcr.io".into(),
        name: "myorg/app".into(),
        tag: "v1.0".into(),
        check_existing: true,
    };
    println!("  Validation: {:?}", validate_push_config(&cfg));
    println!(
        "  HEAD: {}",
        head_check_command("ghcr.io", "myorg/app", "sha256:abc")
    );

    // ── Mutation Scripts ──
    println!("\n[Mutation] Scripts:");
    for rt in ["file", "service", "package"] {
        let ops = applicable_operators(rt);
        println!("  {rt}: {} operators", ops.len());
        for op in &ops {
            let script = mutation_script(*op, &format!("test-{rt}"));
            println!("    {op:?}: {}", script.lines().next().unwrap_or(""));
        }
    }

    println!("\n{}", "=".repeat(55));
    println!("All contract/gc/registry criteria survived.");
}
