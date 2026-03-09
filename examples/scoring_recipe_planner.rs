//! FJ-2800/1385/1382: Scoring engine, recipe validation, and planner analysis.
//!
//! Demonstrates:
//! - ForjarScore v2 two-tier grading (static + runtime)
//! - Recipe input validation (type checking, bounds, enums)
//! - Proof obligation classification and reversibility analysis
//! - BLAKE3 desired-state hashing determinism
//!
//! Usage: cargo run --example scoring_recipe_planner

use forjar::core::planner::hash_desired_state;
use forjar::core::planner::proof_obligation;
use forjar::core::planner::reversibility;
use forjar::core::recipe::{validate_inputs, RecipeInput, RecipeMetadata};
use forjar::core::scoring::{compute, format_score_report, RuntimeData, ScoringInput};
use forjar::core::types::*;
use indexmap::IndexMap;
use std::collections::HashMap;

fn main() {
    println!("Forjar: Scoring, Recipe Validation & Planner Analysis");
    println!("{}", "=".repeat(55));

    // ── FJ-2800: Scoring Engine ──
    println!("\n[FJ-2800] Scoring Engine v2:");
    let config = ForjarConfig {
        name: "web-server".into(),
        version: "1.0".into(),
        ..Default::default()
    };

    // Static-only scoring
    let static_input = ScoringInput {
        status: "pending".into(),
        idempotency: "strong".into(),
        budget_ms: 5000,
        runtime: None,
        raw_yaml: Some("# Recipe: web-server\n# Tier: production\nname: web-server\n".into()),
    };
    let static_result = compute(&config, &static_input);
    println!(
        "  Static-only: grade={}, composite={}",
        static_result.grade, static_result.composite
    );
    assert!(static_result.grade.contains("pending"));

    // With runtime data
    let rt_input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 5000,
        runtime: Some(RuntimeData {
            validate_pass: true,
            plan_pass: true,
            first_apply_pass: true,
            second_apply_pass: true,
            zero_changes_on_reapply: true,
            hash_stable: true,
            all_resources_converged: true,
            state_lock_written: true,
            warning_count: 0,
            changed_on_reapply: 0,
            first_apply_ms: 1200,
            second_apply_ms: 80,
        }),
        raw_yaml: None,
    };
    let rt_result = compute(&config, &rt_input);
    println!(
        "  With runtime: grade={}, composite={}",
        rt_result.grade, rt_result.composite
    );
    let report = format_score_report(&rt_result);
    println!("{report}");

    // ── Recipe Validation ──
    println!("[Recipe] Input Validation:");
    let mut inputs = IndexMap::new();
    inputs.insert(
        "port".into(),
        RecipeInput {
            input_type: "int".into(),
            description: Some("HTTP port".into()),
            default: Some(serde_yaml_ng::Value::Number(8080.into())),
            min: Some(1),
            max: Some(65535),
            choices: vec![],
        },
    );
    inputs.insert(
        "env".into(),
        RecipeInput {
            input_type: "enum".into(),
            description: Some("Environment".into()),
            default: None,
            min: None,
            max: None,
            choices: vec!["dev".into(), "staging".into(), "prod".into()],
        },
    );
    let recipe = RecipeMetadata {
        name: "web-recipe".into(),
        version: Some("1.0".into()),
        description: Some("Web server recipe".into()),
        inputs,
        requires: vec![],
    };

    // Valid inputs
    let mut provided = HashMap::new();
    provided.insert("env".into(), serde_yaml_ng::Value::String("prod".into()));
    let resolved = validate_inputs(&recipe, &provided).unwrap();
    println!(
        "  port={} (default), env={}",
        resolved["port"], resolved["env"]
    );

    // Invalid enum
    let mut bad = HashMap::new();
    bad.insert("env".into(), serde_yaml_ng::Value::String("canary".into()));
    let err = validate_inputs(&recipe, &bad).unwrap_err();
    println!("  Invalid enum caught: {err}");

    // ── FJ-1385: Proof Obligations ──
    println!("\n[FJ-1385] Proof Obligation Taxonomy:");
    for (rtype, action, expected) in [
        (ResourceType::File, PlanAction::NoOp, "idempotent"),
        (ResourceType::Service, PlanAction::Create, "convergent"),
        (ResourceType::Model, PlanAction::Create, "monotonic"),
        (ResourceType::File, PlanAction::Destroy, "destructive"),
    ] {
        let po = proof_obligation::classify(&rtype, &action);
        let lbl = proof_obligation::label(&po);
        let safe = proof_obligation::is_safe(&po);
        println!("  {rtype:?}/{action:?}: {lbl} (safe={safe})");
        assert_eq!(lbl, expected);
    }

    // ── FJ-1382: Reversibility ──
    println!("\n[FJ-1382] Reversibility Classification:");
    let file_with_content = Resource {
        resource_type: ResourceType::File,
        content: Some("data".into()),
        ..Default::default()
    };
    let user = Resource {
        resource_type: ResourceType::User,
        ..Default::default()
    };
    for (name, resource, action) in [
        ("file+content", &file_with_content, PlanAction::Destroy),
        ("user", &user, PlanAction::Destroy),
    ] {
        let rev = reversibility::classify(resource, &action);
        println!("  {name} destroy: {rev:?}");
    }

    // ── FJ-004: Desired State Hashing ──
    println!("\n[FJ-004] Desired State Hash Determinism:");
    let r1 = Resource {
        resource_type: ResourceType::File,
        path: Some("/etc/app.conf".into()),
        content: Some("key=value".into()),
        mode: Some("0644".into()),
        ..Default::default()
    };
    let h1 = hash_desired_state(&r1);
    let h2 = hash_desired_state(&r1);
    let ok = h1 == h2 && h1.starts_with("blake3:");
    println!(
        "  Same resource → same hash: {}",
        if ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(ok);

    println!("\n{}", "=".repeat(55));
    println!("All scoring/recipe/planner criteria survived.");
}
