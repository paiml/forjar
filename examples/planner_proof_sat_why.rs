//! FJ-1385/1382/045/1379: Planner proof obligation, reversibility, SAT deps, why.
//!
//! Demonstrates:
//! - Proof obligation taxonomy (classify, label, is_safe)
//! - Reversibility classification (classify, count/warn irreversible)
//! - SAT dependency resolution (build, solve)
//! - Why explanation (explain_why, format_why)
//!
//! Usage: cargo run --example planner_proof_sat_why

use forjar::core::planner::hash_desired_state;
use forjar::core::planner::proof_obligation::{self, ProofObligation};
use forjar::core::planner::reversibility;
use forjar::core::planner::sat_deps::{build_sat_problem, solve, SatResult};
use forjar::core::planner::why::{explain_why, format_why};
use forjar::core::types::*;
use indexmap::IndexMap;
use std::collections::HashMap;

fn main() {
    println!("Forjar: Planner Proof, SAT, Reversibility & Why");
    println!("{}", "=".repeat(50));

    // ── FJ-1385: Proof Obligation Taxonomy ──
    println!("\n[FJ-1385] Proof Obligation Taxonomy:");
    let cases: &[(ResourceType, PlanAction)] = &[
        (ResourceType::File, PlanAction::NoOp),
        (ResourceType::File, PlanAction::Create),
        (ResourceType::Service, PlanAction::Create),
        (ResourceType::Model, PlanAction::Create),
        (ResourceType::File, PlanAction::Update),
        (ResourceType::File, PlanAction::Destroy),
        (ResourceType::Service, PlanAction::Destroy),
    ];
    for (rt, action) in cases {
        let po = proof_obligation::classify(rt, action);
        println!(
            "  {:10} {:8} → {} (safe: {})",
            format!("{rt:?}"),
            format!("{action:?}"),
            proof_obligation::label(&po),
            proof_obligation::is_safe(&po)
        );
    }
    // Verify key invariants
    assert_eq!(
        proof_obligation::classify(&ResourceType::File, &PlanAction::NoOp),
        ProofObligation::Idempotent
    );
    assert_eq!(
        proof_obligation::classify(&ResourceType::Model, &PlanAction::Create),
        ProofObligation::Monotonic
    );
    assert!(!proof_obligation::is_safe(&ProofObligation::Destructive));

    // ── FJ-1382: Reversibility ──
    println!("\n[FJ-1382] Reversibility Classification:");
    let file_with_content = Resource {
        resource_type: ResourceType::File,
        content: Some("data".into()),
        ..Default::default()
    };
    let bare_file = Resource {
        resource_type: ResourceType::File,
        ..Default::default()
    };
    let user = Resource {
        resource_type: ResourceType::User,
        ..Default::default()
    };
    let svc = Resource {
        resource_type: ResourceType::Service,
        ..Default::default()
    };

    for (name, res, action) in [
        ("file+content", &file_with_content, PlanAction::Destroy),
        ("bare file", &bare_file, PlanAction::Destroy),
        ("user", &user, PlanAction::Destroy),
        ("service", &svc, PlanAction::Destroy),
        ("file create", &bare_file, PlanAction::Create),
    ] {
        let rev = reversibility::classify(res, &action);
        println!("  {name:15} {:8} → {rev:?}", format!("{action:?}"));
    }

    // count_irreversible
    let mut cfg = ForjarConfig::default();
    cfg.resources.insert("f1".into(), bare_file.clone());
    cfg.resources.insert("u1".into(), user.clone());
    cfg.resources.insert("s1".into(), svc.clone());
    let plan = ExecutionPlan {
        name: "test".into(),
        changes: vec![
            PlannedChange {
                resource_id: "f1".into(),
                machine: "m1".into(),
                resource_type: ResourceType::File,
                action: PlanAction::Destroy,
                description: "test".into(),
            },
            PlannedChange {
                resource_id: "u1".into(),
                machine: "m1".into(),
                resource_type: ResourceType::User,
                action: PlanAction::Destroy,
                description: "test".into(),
            },
            PlannedChange {
                resource_id: "s1".into(),
                machine: "m1".into(),
                resource_type: ResourceType::Service,
                action: PlanAction::Destroy,
                description: "test".into(),
            },
        ],
        execution_order: vec![],
        to_create: 0,
        to_update: 0,
        to_destroy: 3,
        unchanged: 0,
    };
    let count = reversibility::count_irreversible(&cfg, &plan);
    let warnings = reversibility::warn_irreversible(&cfg, &plan);
    println!("  Irreversible: {count} of {} destroys", plan.changes.len());
    for w in &warnings {
        println!("    ⚠ {w}");
    }
    assert_eq!(count, 2);

    // ── FJ-045: SAT Dependency Resolution ──
    println!("\n[FJ-045] SAT Dependency Resolution:");
    let resources = vec![
        "nginx".into(),
        "certbot".into(),
        "webapp".into(),
        "database".into(),
    ];
    let deps = vec![
        ("webapp".into(), "database".into()),
        ("webapp".into(), "nginx".into()),
        ("certbot".into(), "nginx".into()),
    ];
    let problem = build_sat_problem(&resources, &deps);
    println!(
        "  Problem: {} vars, {} clauses",
        problem.num_vars,
        problem.clauses.len()
    );
    match solve(&problem) {
        SatResult::Satisfiable { assignment } => {
            println!("  ✓ Satisfiable:");
            for (name, val) in &assignment {
                println!("    {name} = {val}");
            }
            assert!(assignment.values().all(|&v| v));
        }
        SatResult::Unsatisfiable { conflict_clause } => {
            println!("  ✗ Unsatisfiable: {conflict_clause:?}");
            panic!("should be satisfiable");
        }
    }

    // ── FJ-1379: Why Explanation ──
    println!("\n[FJ-1379] Change Explanation:");
    let file_res = Resource {
        resource_type: ResourceType::File,
        content: Some("server { listen 80; }".into()),
        path: Some("/etc/nginx/nginx.conf".into()),
        ..Default::default()
    };

    // First apply — no lock
    let reason1 = explain_why("nginx-conf", &file_res, "web-01", &HashMap::new());
    println!("  First apply:");
    println!("  {}", format_why(&reason1));
    assert_eq!(reason1.action, PlanAction::Create);

    // Hash unchanged — converged
    let hash = hash_desired_state(&file_res);
    let mut lock_resources = IndexMap::new();
    lock_resources.insert(
        "nginx-conf".into(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: hash.clone(),
            details: HashMap::new(),
        },
    );
    let lock = StateLock {
        schema: "1".into(),
        machine: "web-01".into(),
        hostname: "web-01".into(),
        generated_at: "2026-03-09T00:00:00Z".into(),
        generator: "forjar".into(),
        blake3_version: "1".into(),
        resources: lock_resources,
    };
    let mut locks = HashMap::new();
    locks.insert("web-01".into(), lock);
    let reason2 = explain_why("nginx-conf", &file_res, "web-01", &locks);
    println!("\n  Unchanged:");
    println!("  {}", format_why(&reason2));
    assert_eq!(reason2.action, PlanAction::NoOp);

    println!("\n{}", "=".repeat(50));
    println!("All planner proof/SAT/reversibility/why criteria survived.");
}
