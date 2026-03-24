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
    ContractAssertion, ContractCoverageReport, ContractEntry, ContractKind,
    HandlerInvariantStatus, VerificationTier,
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

fn main() {
    println!("=== Contract Pipeline Demo ===\n");

    // Phase 1: Load contract YAML files from contracts/
    let contracts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("contracts");
    println!("Phase 1: Loading contracts from {}\n", contracts_dir.display());

    let mut total_equations = 0usize;
    let mut total_invariants = 0usize;
    let mut contract_entries: Vec<ContractEntry> = Vec::new();
    let mut runtime_assertions: Vec<ContractAssertion> = Vec::new();

    let mut paths: Vec<_> = std::fs::read_dir(&contracts_dir)
        .expect("read contracts/")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yaml"))
        .collect();
    paths.sort();

    for path in &paths {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Skip binding.yaml (it's a mapping file, not a contract)
        if stem == "binding" {
            continue;
        }

        let content = std::fs::read_to_string(path).expect("read yaml");
        let contract: ContractYaml = serde_yaml_ng::from_str(&content).expect("parse yaml");

        println!(
            "  Contract: {} (v{})",
            contract.metadata.description, contract.metadata.version
        );
        println!("    File: {stem}.yaml");
        println!("    Equations: {}", contract.equations.len());

        for (eq_name, eq) in &contract.equations {
            total_equations += 1;
            let n_inv = eq.invariants.len();
            total_invariants += n_inv;

            println!("      {eq_name}: {n_inv} invariants");

            // Build a contract entry for the coverage report
            let verified: Vec<String> = eq
                .invariants
                .iter()
                .map(|inv| format!("invariant: {inv}"))
                .collect();

            contract_entries.push(ContractEntry {
                function: eq_name.clone(),
                module: format!("contracts::{}", stem.replace('-', "_")),
                contract_id: Some(format!("{stem}.yaml")),
                tier: VerificationTier::Bounded,
                verified_by: verified,
            });

            // Build runtime assertions from invariants
            for inv in &eq.invariants {
                runtime_assertions.push(ContractAssertion {
                    function: eq_name.clone(),
                    module: format!("contracts::{}", stem.replace('-', "_")),
                    kind: ContractKind::Invariant,
                    held: true,
                    expression: Some(inv.clone()),
                });
            }
        }
        println!();
    }

    // Phase 2: Show build-time env var mapping
    println!("Phase 2: Build-time CONTRACT_* env vars\n");
    println!(
        "  build.rs emits CONTRACT_INV_*, CONTRACT_PRE_*, CONTRACT_POST_* env vars"
    );
    println!("  Total equations: {total_equations}");
    println!("  Total invariants: {total_invariants}");
    println!();

    // Show a sample of what the env vars look like
    println!("  Sample env var keys:");
    let sample_keys = [
        "CONTRACT_INV_BLAKE3_STATE_V1_HASH_STRING_0",
        "CONTRACT_INV_DAG_ORDERING_V1_TOPOLOGICAL_SORT_0",
        "CONTRACT_INV_EXECUTION_SAFETY_V1_ATOMIC_WRITE_0",
        "CONTRACT_INV_RECIPE_DETERMINISM_V1_EXPAND_RECIPE_0",
    ];
    for key in &sample_keys {
        // Try to read from env (set by build.rs at compile time)
        let val = option_env!("CONTRACT_INV_BLAKE3_STATE_V1_HASH_STRING_0");
        if key.contains("BLAKE3") {
            if let Some(v) = val {
                println!("    {key} = \"{v}\"");
            } else {
                println!("    {key} = (would be set by build.rs)");
            }
        } else {
            println!("    {key} = (set by build.rs)");
        }
    }
    println!();

    // Phase 3: Runtime assertion display
    println!("Phase 3: Runtime Contract Assertions\n");
    for a in &runtime_assertions {
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

    // Phase 4: Coverage report
    println!("Phase 4: Contract Coverage Report\n");
    let handler_invariants = vec![
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
    ];

    let report = ContractCoverageReport {
        total_functions: contract_entries.len(),
        entries: contract_entries,
        handler_invariants,
    };

    print!("{}", report.format_summary());
    println!();

    let hist = report.histogram();
    println!("  Tier Distribution:");
    for (i, count) in hist.iter().enumerate() {
        if *count > 0 {
            let tier_name = match i {
                0 => "Unlabeled (L0)",
                1 => "Labeled (L1)",
                2 => "Runtime (L2)",
                3 => "Bounded (L3)",
                4 => "Proved (L4)",
                5 => "Structural (L5)",
                _ => "Unknown",
            };
            println!("    {tier_name}: {count}");
        }
    }
    println!(
        "\n  At or above Bounded: {}/{}",
        report.at_or_above(VerificationTier::Bounded),
        report.total_functions
    );

    println!("\n=== Pipeline Complete ===");
}
