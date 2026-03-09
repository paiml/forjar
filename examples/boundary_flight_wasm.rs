//! FJ-3209/115/113/2402/2403: Policy boundary, flight-grade, Ferrocene, WASM, CI.
//!
//! Demonstrates:
//! - Compliance boundary config generation and testing
//! - Flight-grade compliance checking and topological sort
//! - Ferrocene source compliance checking
//! - WASM size budgets and bundle drift detection
//! - Reproducible build configs and feature matrix
//!
//! Usage: cargo run --example boundary_flight_wasm

use forjar::core::compliance_pack::*;
use forjar::core::ferrocene::*;
use forjar::core::flight_grade::*;
use forjar::core::policy_boundary::*;
use forjar::core::types::*;

fn main() {
    println!("Forjar: Boundary, Flight-Grade, WASM & CI");
    println!("{}", "=".repeat(50));

    // ── Policy Boundary ──
    println!("\n[FJ-3209] Policy Boundary Testing:");
    let pack = CompliancePack {
        name: "hardening".into(),
        version: "1.0".into(),
        framework: "cis".into(),
        description: Some("CIS hardening rules".into()),
        rules: vec![
            ComplianceRule {
                id: "H-001".into(),
                title: "File mode 0644".into(),
                description: None,
                severity: "error".into(),
                controls: vec!["CIS-6.1.1".into()],
                check: ComplianceCheck::Assert {
                    resource_type: "file".into(),
                    field: "mode".into(),
                    expected: "0644".into(),
                },
            },
            ComplianceRule {
                id: "H-002".into(),
                title: "No root password".into(),
                description: None,
                severity: "error".into(),
                controls: vec![],
                check: ComplianceCheck::Deny {
                    resource_type: "user".into(),
                    field: "password".into(),
                    pattern: "root".into(),
                },
            },
        ],
    };
    let configs = generate_boundary_configs(&pack);
    println!("  Generated {} boundary configs", configs.len());
    let result = test_boundaries(&pack);
    println!("{}", format_boundary_results(&result));

    // ── Flight-Grade ──
    println!("\n[FJ-115] Flight-Grade Compliance:");
    let report = check_compliance(100, 10);
    println!(
        "  Compliant: {} (max_resources={}, max_depth={})",
        report.compliant, report.max_resources, report.max_depth
    );

    let mut plan = FgPlan::empty();
    plan.resources[0].id = 0;
    plan.resources[1].id = 1;
    plan.resources[1].deps[0] = 0;
    plan.resources[1].dep_count = 1;
    plan.count = 2;
    fg_topo_sort(&mut plan).unwrap();
    println!(
        "  Topo sort: {} resources, order {:?}",
        plan.count,
        &plan.order[..plan.order_len]
    );

    // ── Ferrocene ──
    println!("\n[FJ-113] Ferrocene Source Compliance:");
    let clean = "fn main() { println!(\"safe\"); }";
    let violations = check_source_compliance(clean);
    println!("  Clean source: {} violations", violations.len());

    let ev = generate_evidence(SafetyStandard::Iso26262, "binhash", "srchash");
    println!("  Evidence: {:?}, compliant={}", ev.standard, ev.compliant);

    // ── WASM Types ──
    println!("\n[FJ-2402] WASM Types:");
    let budget = WasmSizeBudget::default();
    println!(
        "  Budget: core={}KB, app={}KB",
        budget.core_kb, budget.full_app_kb
    );

    let drift = BundleSizeDrift::check(&budget, 90 * 1024, Some(85 * 1024));
    println!("  Drift: {drift}");

    let drift_bad = BundleSizeDrift::check(&budget, 110 * 1024, Some(90 * 1024));
    println!("  Drift: {drift_bad}");

    for level in [
        WasmOptLevel::Fast,
        WasmOptLevel::Balanced,
        WasmOptLevel::MaxSpeed,
        WasmOptLevel::MinSize,
    ] {
        println!("  Opt {level}: {}", level.flag());
    }

    // ── CI Pipeline ──
    println!("\n[FJ-2403] CI Pipeline:");
    let repro = ReproBuildConfig::default();
    println!("  Reproducible: {}", repro.is_reproducible());
    println!("  Cargo args: {:?}", repro.cargo_args());

    let msrv = MsrvCheck::new("1.88.0");
    println!(
        "  MSRV 1.88.0 satisfies 1.89.0: {}",
        msrv.satisfies("1.89.0")
    );
    println!(
        "  MSRV 1.88.0 satisfies 1.87.0: {}",
        msrv.satisfies("1.87.0")
    );

    let matrix = FeatureMatrix::new(vec!["encryption", "container-test"]);
    println!(
        "  Feature matrix: {} combinations",
        matrix.combinations().len()
    );
    for cmd in matrix.cargo_commands() {
        println!("    {cmd}");
    }

    let integrity = ModelIntegrityCheck::check("llama3", "abc", "abc", 7_000_000_000);
    println!("  Model: {integrity}");

    println!("\n{}", "=".repeat(50));
    println!("All boundary/flight/WASM/CI criteria survived.");
}
