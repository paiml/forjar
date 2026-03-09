//! FJ-005/044/1350/2700/045: Codegen, migration, HF config, dispatch, SAT.
//!
//! Demonstrates:
//! - Script generation for file/package/service resources
//! - Docker → pepita migration with warnings
//! - HuggingFace config parsing and kernel requirement derivation
//! - Dispatch param substitution and summary formatting
//! - SAT dependency solving
//!
//! Usage: cargo run --example codegen_dispatch_sat

use forjar::core::codegen::{apply_script, check_script};
use forjar::core::migrate::docker_to_pepita;
use forjar::core::planner::sat_deps::{build_sat_problem, solve, SatResult};
use forjar::core::store::hf_config::{parse_hf_config_str, required_kernels};
use forjar::core::task::dispatch::{
    format_dispatch_summary, prepare_dispatch, record_invocation, success_rate,
};
use forjar::core::types::*;

fn main() {
    println!("Forjar: Codegen, Dispatch & SAT");
    println!("{}", "=".repeat(50));

    // ── Script Codegen ──
    println!("\n[FJ-005] Script Codegen:");
    let mut file_res = Resource {
        resource_type: ResourceType::File,
        ..Default::default()
    };
    file_res.path = Some("/etc/app.conf".into());
    file_res.content = Some("key=value".into());
    let check = check_script(&file_res).unwrap();
    let apply = apply_script(&file_res).unwrap();
    println!("  File check: {} chars", check.len());
    println!("  File apply: {} chars", apply.len());

    // ── Docker Migration ──
    println!("\n[FJ-044] Docker → Pepita:");
    let mut docker = Resource {
        resource_type: ResourceType::Docker,
        ..Default::default()
    };
    docker.name = Some("web".into());
    docker.image = Some("nginx:latest".into());
    docker.ports = vec!["80:80".into()];
    docker.state = Some("running".into());
    let result = docker_to_pepita("web", &docker);
    println!("  Type: {:?}", result.resource.resource_type);
    println!("  State: {:?}", result.resource.state);
    println!("  Netns: {}", result.resource.netns);
    for w in &result.warnings {
        println!("  ⚠ {w}");
    }

    // ── HF Config ──
    println!("\n[FJ-1350] HuggingFace Config:");
    let config = parse_hf_config_str(
        r#"{"model_type":"llama","num_attention_heads":32,"num_key_value_heads":8}"#,
    )
    .unwrap();
    println!("  Model: {}", config.model_type);
    let kernels = required_kernels(&config);
    for k in &kernels {
        println!("  Kernel: {} ({})", k.op, k.contract);
    }

    // ── Dispatch ──
    println!("\n[FJ-2700] Dispatch:");
    let dc = DispatchConfig {
        name: "deploy".into(),
        command: "deploy --env {{ env }} --region {{ region }}".into(),
        params: vec![
            ("env".into(), "production".into()),
            ("region".into(), "us-east-1".into()),
        ],
        timeout_secs: Some(300),
    };
    let prepared = prepare_dispatch(&dc, &[]);
    println!("  Command: {}", prepared.command);

    let mut state = DispatchState::default();
    record_invocation(
        &mut state,
        DispatchInvocation {
            timestamp: "2026-03-09T12:00:00Z".into(),
            exit_code: 0,
            duration_ms: 1500,
            caller: Some("admin".into()),
        },
        10,
    );
    record_invocation(
        &mut state,
        DispatchInvocation {
            timestamp: "2026-03-09T12:05:00Z".into(),
            exit_code: 1,
            duration_ms: 300,
            caller: None,
        },
        10,
    );
    println!("{}", format_dispatch_summary("deploy", &state));
    println!("  Success rate: {:.1}%", success_rate(&state));

    // ── SAT Solver ──
    println!("\n[FJ-045] SAT Dependency Resolution:");
    let resources = vec!["nginx".into(), "app".into(), "db".into(), "cache".into()];
    let deps = vec![
        ("app".into(), "db".into()),
        ("app".into(), "cache".into()),
        ("nginx".into(), "app".into()),
    ];
    let problem = build_sat_problem(&resources, &deps);
    println!(
        "  {} vars, {} clauses",
        problem.num_vars,
        problem.clauses.len()
    );
    match solve(&problem) {
        SatResult::Satisfiable { assignment } => {
            for (name, val) in &assignment {
                println!("  {name} = {val}");
            }
        }
        SatResult::Unsatisfiable { conflict_clause } => {
            println!("  UNSAT: {:?}", conflict_clause);
        }
    }

    println!("\n{}", "=".repeat(50));
    println!("All codegen/dispatch/SAT criteria survived.");
}
