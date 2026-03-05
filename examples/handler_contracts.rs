//! Demonstrates FJ-2203 handler contract types: hash invariants, audit reports.

use forjar::core::types::{
    ContractAssertion, ContractKind, HandlerAuditReport, HandlerExemption, HashInvariantCheck,
    KaniHarness, ProofStatus,
};

fn main() {
    // Hash invariant checks
    println!("=== Hash Invariant Checks ===");
    let checks = vec![
        HashInvariantCheck::pass("nginx-pkg", "package", "blake3:abc123"),
        HashInvariantCheck::pass("nginx-conf", "file", "blake3:def456"),
        HashInvariantCheck::pass("nginx-svc", "service", "blake3:789abc"),
        HashInvariantCheck::fail(
            "cron-backup",
            "cron",
            "blake3:aaa111",
            "blake3:bbb222",
            "cron handler hashes schedule only, not full resource",
        ),
    ];
    for c in &checks {
        println!("  {c}");
    }

    // Full audit report
    println!("\n=== Handler Audit Report ===");
    let report = HandlerAuditReport {
        checks,
        exemptions: vec![HandlerExemption {
            handler: "task".into(),
            reason: "imperative by nature — no desired state hash".into(),
            approved_by: Some("FJ-2203 spec review".into()),
        }],
    };
    println!("{}", report.format_report());

    // Runtime contract assertions
    println!("=== Runtime Contracts ===");
    let assertions = vec![
        ContractAssertion {
            function: "determine_present_action".into(),
            module: "core::planner".into(),
            kind: ContractKind::Ensures,
            held: true,
            expression: Some("result is NoOp or Apply".into()),
        },
        ContractAssertion {
            function: "build_execution_order".into(),
            module: "core::planner".into(),
            kind: ContractKind::Requires,
            held: true,
            expression: Some("DAG is acyclic".into()),
        },
        ContractAssertion {
            function: "hash_desired_state".into(),
            module: "core::hasher".into(),
            kind: ContractKind::Ensures,
            held: true,
            expression: Some("same input -> same hash".into()),
        },
    ];
    for a in &assertions {
        let status = if a.held { "OK" } else { "VIOLATED" };
        println!(
            "  [{status}] {}::{} ({}: {})",
            a.module,
            a.function,
            a.kind,
            a.expression.as_deref().unwrap_or("?"),
        );
    }

    // Kani proof harnesses
    println!("\n=== Kani Proofs ===");
    let harnesses = vec![
        KaniHarness {
            name: "proof_blake3_idempotency".into(),
            property: "hashing is deterministic".into(),
            target_function: "blake3::hash".into(),
            status: ProofStatus::Verified,
            bound: Some(16),
        },
        KaniHarness {
            name: "proof_plan_determinism".into(),
            property: "same input produces same plan".into(),
            target_function: "planner::plan".into(),
            status: ProofStatus::Verified,
            bound: Some(8),
        },
        KaniHarness {
            name: "proof_layer_determinism".into(),
            property: "build_layer produces same hashes".into(),
            target_function: "oci::build_layer".into(),
            status: ProofStatus::Pending,
            bound: None,
        },
    ];
    for h in &harnesses {
        println!(
            "  {} -> {} [{}] (bound: {:?})",
            h.name, h.property, h.status, h.bound,
        );
    }
}
