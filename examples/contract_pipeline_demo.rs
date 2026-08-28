//! Contract Pipeline Demo — end-to-end flow from YAML contracts
//! through build.rs env var emission to runtime assertion display.
//!
//! This example demonstrates:
//! 1. Loading contract YAML files from `contracts/`
//! 2. Extracting equations with their pre/post/invariant conditions
//! 3. Mapping build-time CONTRACT_* env vars to runtime checks
//!
//! Run with: `cargo run --example contract_pipeline_demo`

use std::collections::BTreeMap;

use forjar::core::types::{
    ContractAssertion, ContractCoverageReport, ContractEntry, ContractKind, HandlerInvariantStatus,
    VerificationTier,
};

/// Mirrors the build.rs struct for demonstration purposes.
#[derive(serde::Deserialize, Default)]
struct ContractYaml {
    #[serde(default)]
    metadata: Metadata,
    #[serde(default)]
    equations: BTreeMap<String, EquationYaml>,
}

#[derive(serde::Deserialize, Default)]
struct Metadata {
    #[serde(default)]
    description: String,
    #[serde(default)]
    version: String,
}

#[derive(serde::Deserialize, Default)]
#[allow(dead_code)]
struct EquationYaml {
    #[serde(default)]
    formula: String,
    #[serde(default)]
    domain: String,
    #[serde(default)]
    codomain: String,
    #[serde(default)]
    preconditions: Vec<String>,
    #[serde(default)]
    postconditions: Vec<String>,
    #[serde(default)]
    invariants: Vec<String>,
}

/// Everything phase 1 harvests from the `contracts/` directory.
#[derive(Default)]
struct LoadedContracts {
    total_equations: usize,
    total_invariants: usize,
    entries: Vec<ContractEntry>,
    assertions: Vec<ContractAssertion>,
}

/// The `.yaml` contract files under `contracts/`, sorted, with the
/// `binding.yaml` mapping file (which is not a contract) left out.
fn contract_yaml_paths(contracts_dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut paths: Vec<_> = std::fs::read_dir(contracts_dir)
        .expect("read contracts/")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yaml"))
        .filter(|p| p.file_stem().and_then(|s| s.to_str()) != Some("binding"))
        .collect();
    paths.sort();
    paths
}

/// Fold one equation into the running totals, the coverage entries and the
/// runtime assertions derived from its invariants.
fn absorb_equation(stem: &str, eq_name: &str, eq: &EquationYaml, out: &mut LoadedContracts) {
    let n_inv = eq.invariants.len();
    out.total_equations += 1;
    out.total_invariants += n_inv;
    println!("      {eq_name}: {n_inv} invariants");

    let module = format!("contracts::{}", stem.replace('-', "_"));
    out.entries.push(ContractEntry {
        function: eq_name.to_string(),
        module: module.clone(),
        contract_id: Some(format!("{stem}.yaml")),
        tier: VerificationTier::Bounded,
        verified_by: eq
            .invariants
            .iter()
            .map(|inv| format!("invariant: {inv}"))
            .collect(),
    });

    out.assertions
        .extend(eq.invariants.iter().map(|inv| ContractAssertion {
            function: eq_name.to_string(),
            module: module.clone(),
            kind: ContractKind::Invariant,
            held: true,
            expression: Some(inv.clone()),
        }));
}

/// Phase 1 — parse every contract file, echoing each one as it is absorbed.
fn load_contracts(contracts_dir: &std::path::Path) -> LoadedContracts {
    println!(
        "Phase 1: Loading contracts from {}\n",
        contracts_dir.display()
    );

    let mut loaded = LoadedContracts::default();
    for path in contract_yaml_paths(contracts_dir) {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let content = std::fs::read_to_string(&path).expect("read yaml");
        let contract: ContractYaml = serde_yaml_ng::from_str(&content).expect("parse yaml");

        println!(
            "  Contract: {} (v{})",
            contract.metadata.description, contract.metadata.version
        );
        println!("    File: {stem}.yaml");
        println!("    Equations: {}", contract.equations.len());

        for (eq_name, eq) in &contract.equations {
            absorb_equation(stem, eq_name, eq, &mut loaded);
        }
        println!();
    }
    loaded
}

/// Phase 2 — the `CONTRACT_*` env vars build.rs derives from those contracts.
fn print_build_env_mapping(loaded: &LoadedContracts) {
    println!("Phase 2: Build-time CONTRACT_* env vars\n");
    println!("  build.rs emits CONTRACT_INV_*, CONTRACT_PRE_*, CONTRACT_POST_* env vars");
    println!("  Total equations: {}", loaded.total_equations);
    println!("  Total invariants: {}", loaded.total_invariants);
    println!();

    println!("  Sample env var keys:");
    let sample_keys = [
        "CONTRACT_INV_BLAKE3_STATE_V1_HASH_STRING_0",
        "CONTRACT_INV_DAG_ORDERING_V1_TOPOLOGICAL_SORT_0",
        "CONTRACT_INV_EXECUTION_SAFETY_V1_ATOMIC_WRITE_0",
        "CONTRACT_INV_RECIPE_DETERMINISM_V1_EXPAND_RECIPE_0",
    ];
    // Only the BLAKE3 key can be resolved here: `option_env!` needs a literal.
    let blake3_val = option_env!("CONTRACT_INV_BLAKE3_STATE_V1_HASH_STRING_0");
    for key in &sample_keys {
        match (key.contains("BLAKE3"), blake3_val) {
            (true, Some(v)) => println!("    {key} = \"{v}\""),
            (true, None) => println!("    {key} = (would be set by build.rs)"),
            (false, _) => println!("    {key} = (set by build.rs)"),
        }
    }
    println!();
}

/// Phase 3 — the invariants as they would be checked at runtime.
fn print_runtime_assertions(assertions: &[ContractAssertion]) {
    println!("Phase 3: Runtime Contract Assertions\n");
    for a in assertions {
        let status = if a.held { "HELD" } else { "VIOLATED" };
        println!(
            "  [{status}] {}::{} ({}: {})",
            a.module,
            a.function,
            a.kind,
            a.expression.as_deref().unwrap_or("?"),
        );
    }
    println!();
}

/// The per-resource-type verification tiers the demo report is built against.
fn demo_handler_invariants() -> Vec<HandlerInvariantStatus> {
    vec![
        HandlerInvariantStatus {
            resource_type: "file".into(),
            tier: VerificationTier::Bounded,
            exempt: false,
            exemption_reason: None,
        },
        HandlerInvariantStatus {
            resource_type: "package".into(),
            tier: VerificationTier::Bounded,
            exempt: false,
            exemption_reason: None,
        },
        HandlerInvariantStatus {
            resource_type: "service".into(),
            tier: VerificationTier::Runtime,
            exempt: false,
            exemption_reason: None,
        },
        HandlerInvariantStatus {
            resource_type: "task".into(),
            tier: VerificationTier::Unlabeled,
            exempt: true,
            exemption_reason: Some("imperative resource type".into()),
        },
    ]
}

/// Display name for a bucket of `ContractCoverageReport::histogram`.
fn tier_label(index: usize) -> &'static str {
    match index {
        0 => "Unlabeled (L0)",
        1 => "Labeled (L1)",
        2 => "Runtime (L2)",
        3 => "Bounded (L3)",
        4 => "Proved (L4)",
        5 => "Structural (L5)",
        _ => "Unknown",
    }
}

/// Phase 4 — summary, tier histogram and the at-or-above-Bounded count.
fn print_coverage_report(entries: Vec<ContractEntry>) {
    println!("Phase 4: Contract Coverage Report\n");
    let report = ContractCoverageReport {
        total_functions: entries.len(),
        entries,
        handler_invariants: demo_handler_invariants(),
    };

    print!("{}", report.format_summary());
    println!();

    println!("  Tier Distribution:");
    for (i, count) in report.histogram().iter().enumerate() {
        if *count > 0 {
            println!("    {}: {count}", tier_label(i));
        }
    }
    println!(
        "\n  At or above Bounded: {}/{}",
        report.at_or_above(VerificationTier::Bounded),
        report.total_functions
    );
}

fn main() {
    println!("=== Contract Pipeline Demo ===\n");

    let contracts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("contracts");
    let loaded = load_contracts(&contracts_dir);

    print_build_env_mapping(&loaded);
    print_runtime_assertions(&loaded.assertions);
    print_coverage_report(loaded.entries);

    println!("\n=== Pipeline Complete ===");
}
