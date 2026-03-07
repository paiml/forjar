//! Runtime contracts example — demonstrates the 6 critical-path
//! functions with `debug_assert!` postconditions.
//!
//! Run with: `cargo run --example runtime_contracts`

use forjar::core::types::{
    ContractCoverageReport, ContractEntry, HandlerInvariantStatus, VerificationTier,
};

fn main() {
    println!("Runtime Contracts — Critical-Path Function Coverage\n");

    // Build the actual contract registry for forjar's 6 contracted functions
    let entries = vec![
        ContractEntry {
            function: "determine_present_action".into(),
            module: "core::planner".into(),
            contract_id: Some("idempotency-v1".into()),
            tier: VerificationTier::Runtime,
            verified_by: vec![
                "debug_assert!(converged + hash match => NoOp)".into(),
                "verus::proof_idempotency_conditional".into(),
            ],
        },
        ContractEntry {
            function: "hash_desired_state".into(),
            module: "core::planner".into(),
            contract_id: Some("blake3-state-v1".into()),
            tier: VerificationTier::Bounded,
            verified_by: vec![
                "debug_assert!(double-hash equality)".into(),
                "kani::proof_hash_determinism_bounded".into(),
            ],
        },
        ContractEntry {
            function: "save_lock".into(),
            module: "core::state".into(),
            contract_id: Some("execution-safety-v1".into()),
            tier: VerificationTier::Runtime,
            verified_by: vec!["debug_assert!(file exists, temp removed)".into()],
        },
        ContractEntry {
            function: "build_execution_order".into(),
            module: "core::resolver".into(),
            contract_id: Some("dag-ordering-v1".into()),
            tier: VerificationTier::Bounded,
            verified_by: vec![
                "debug_assert!(topological ordering)".into(),
                "kani::proof_dag_ordering_bounded".into(),
            ],
        },
        ContractEntry {
            function: "build_layer".into(),
            module: "core::store".into(),
            contract_id: Some("oci-layer-v1".into()),
            tier: VerificationTier::Runtime,
            verified_by: vec!["debug_assert!(same BLAKE3 on rebuild)".into()],
        },
        ContractEntry {
            function: "assemble_image".into(),
            module: "core::store".into(),
            contract_id: Some("oci-manifest-v1".into()),
            tier: VerificationTier::Runtime,
            verified_by: vec!["debug_assert!(OCI layout validity)".into()],
        },
    ];

    // Handler invariants per resource type
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
            tier: VerificationTier::Bounded,
            exempt: false,
            exemption_reason: None,
        },
        HandlerInvariantStatus {
            resource_type: "task".into(),
            tier: VerificationTier::Unlabeled,
            exempt: true,
            exemption_reason: Some("imperative resource — no idempotency guarantee".into()),
        },
    ];

    let report = ContractCoverageReport {
        total_functions: 6,
        entries,
        handler_invariants,
    };

    // Print the report
    print!("{}", report.format_summary());

    // Show tier statistics
    let hist = report.histogram();
    println!("\nTier Distribution:");
    println!("  Runtime (L2): {} functions", hist[2]);
    println!("  Bounded (L3): {} functions", hist[3]);
    println!(
        "  At or above Runtime: {}/{}",
        report.at_or_above(VerificationTier::Runtime),
        report.total_functions
    );

    // Show each entry with its verifiers
    println!("\nDetailed Contracts:");
    for entry in &report.entries {
        println!(
            "  {} [{}]",
            entry,
            entry.contract_id.as_deref().unwrap_or("-")
        );
        for v in &entry.verified_by {
            println!("    verified: {v}");
        }
    }
}
