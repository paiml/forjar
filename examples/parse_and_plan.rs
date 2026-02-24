//! Parse a forjar.yaml config, resolve the DAG, and produce an execution plan.
//!
//! Usage: cargo run --example parse_and_plan

use forjar::core::{parser, planner, resolver};
use std::collections::HashMap;

fn main() {
    let yaml = r#"
version: "1.0"
name: demo
description: "Example: parse → resolve → plan"

params:
  env: production

machines:
  local:
    hostname: localhost
    addr: 127.0.0.1

resources:
  base-packages:
    type: package
    machine: local
    provider: apt
    packages: [curl, htop, jq]

  app-config:
    type: file
    machine: local
    path: /tmp/forjar-example-config.yaml
    content: |
      environment: {{params.env}}
      log_level: info
    owner: root
    mode: "0644"
    depends_on: [base-packages]

  app-service:
    type: service
    machine: local
    name: example-app
    state: running
    enabled: true
    depends_on: [app-config]

policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#;

    // 1. Parse
    let config = parser::parse_config(yaml).expect("YAML parse failed");
    println!(
        "Parsed: {} ({} machines, {} resources)",
        config.name,
        config.machines.len(),
        config.resources.len()
    );

    // 2. Validate
    let errors = parser::validate_config(&config);
    if errors.is_empty() {
        println!("Validation: OK");
    } else {
        for e in &errors {
            eprintln!("  ERROR: {}", e);
        }
        std::process::exit(1);
    }

    // 3. Resolve DAG
    let order = resolver::build_execution_order(&config).expect("DAG resolution failed");
    println!("Execution order: {:?}", order);

    // 4. Plan (no existing state → everything is Create)
    let locks = HashMap::new();
    let plan = planner::plan(&config, &order, &locks);
    println!("\nPlan: {}", plan.name);
    for change in &plan.changes {
        let symbol = match change.action {
            forjar::core::types::PlanAction::Create => "+",
            forjar::core::types::PlanAction::Update => "~",
            forjar::core::types::PlanAction::Destroy => "-",
            forjar::core::types::PlanAction::NoOp => " ",
        };
        println!("  {} {}", symbol, change.description);
    }
    println!(
        "\n{} to create, {} to update, {} to destroy, {} unchanged",
        plan.to_create, plan.to_update, plan.to_destroy, plan.unchanged
    );
}
