//! FJ-2203: Contract coverage — verification tier tracking.
//!
//! ```bash
//! cargo run --example contract_coverage
//! ```

use forjar::core::types::{
    ContractCoverageReport, ContractEntry, HandlerInvariantStatus, VerificationTier,
};

fn main() {
    // Verification tiers
    println!("=== Verification Tiers ===");
    for tier in [
        VerificationTier::Unlabeled,
        VerificationTier::Labeled,
        VerificationTier::Runtime,
        VerificationTier::Bounded,
        VerificationTier::Proved,
        VerificationTier::Structural,
    ] {
        println!("  {tier}");
    }
    println!();

    // Build a coverage report
    let report = ContractCoverageReport {
        total_functions: 24,
        entries: vec![
            ContractEntry {
                function: "execute_resource".into(),
                module: "core::executor".into(),
                contract_id: Some("handler-invariant-v1".into()),
                tier: VerificationTier::Structural,
                verified_by: vec!["trait ResourceHandler".into()],
            },
            ContractEntry {
                function: "reconcile".into(),
                module: "core::verus_spec".into(),
                contract_id: Some("convergence-v1".into()),
                tier: VerificationTier::Proved,
                verified_by: vec!["verus::proof_convergence".into()],
            },
            ContractEntry {
                function: "hash_desired_state".into(),
                module: "core::planner".into(),
                contract_id: Some("blake3-state-v1".into()),
                tier: VerificationTier::Bounded,
                verified_by: vec!["kani::proof_hash_determinism".into()],
            },
            ContractEntry {
                function: "determine_present_action".into(),
                module: "core::planner".into(),
                contract_id: Some("idempotency-v1".into()),
                tier: VerificationTier::Bounded,
                verified_by: vec!["kani::proof_planner_idempotency".into()],
            },
            ContractEntry {
                function: "build_execution_order".into(),
                module: "resolver::dag".into(),
                contract_id: Some("dag-ordering-v1".into()),
                tier: VerificationTier::Runtime,
                verified_by: vec!["debug_assert".into()],
            },
            ContractEntry {
                function: "save_lock".into(),
                module: "core::state".into(),
                contract_id: Some("execution-safety-v1".into()),
                tier: VerificationTier::Runtime,
                verified_by: vec!["debug_assert".into()],
            },
        ],
        handler_invariants: vec![
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
        ],
    };

    print!("{}", report.format_summary());
    println!();

    // Query the report
    println!("=== Coverage Stats ===");
    println!(
        "  At or above Runtime:    {}",
        report.at_or_above(VerificationTier::Runtime)
    );
    println!(
        "  At or above Bounded:    {}",
        report.at_or_above(VerificationTier::Bounded)
    );
    println!(
        "  At or above Proved:     {}",
        report.at_or_above(VerificationTier::Proved)
    );
    println!(
        "  At or above Structural: {}",
        report.at_or_above(VerificationTier::Structural)
    );
}
