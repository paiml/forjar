//! Demonstrate container transport: parse, validate, plan, and show lifecycle.
//!
//! This example shows how forjar handles container machines — parsing the
//! container config, validating it, resolving the DAG, and producing a plan.
//! No Docker required (doesn't actually execute).
//!
//! Usage: cargo run --example container_transport

use forjar::core::{parser, planner, resolver};
use std::collections::HashMap;

fn main() {
    let yaml = r#"
version: "1.0"
name: container-demo
description: "Container transport: parse → validate → plan"

machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      name: forjar-example
      ephemeral: true
      privileged: false
      init: true

resources:
  base-packages:
    type: package
    machine: test-box
    provider: apt
    packages: [curl, jq, tree]

  app-config:
    type: file
    machine: test-box
    path: /etc/forjar/demo.yaml
    content: |
      environment: example
      managed_by: forjar
    owner: root
    mode: "0644"
    depends_on: [base-packages]

  motd:
    type: file
    machine: test-box
    path: /etc/motd
    content: "Managed by forjar container transport"
    depends_on: [base-packages]

policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#;

    // 1. Parse
    let config = parser::parse_config(yaml).expect("YAML parse failed");
    println!("Parsed: {}", config.name);
    println!(
        "  {} machine(s), {} resource(s)",
        config.machines.len(),
        config.resources.len()
    );

    // 2. Show container machine details
    let machine = config.machines.get("test-box").expect("machine not found");
    println!("\nMachine: test-box");
    println!(
        "  is_container_transport: {}",
        machine.is_container_transport()
    );
    println!("  container_name: {}", machine.container_name());
    if let Some(ref c) = machine.container {
        println!("  runtime: {}", c.runtime);
        println!("  image: {:?}", c.image);
        println!("  ephemeral: {}", c.ephemeral);
        println!("  init: {}", c.init);
    }

    // 3. Validate
    let errors = parser::validate_config(&config);
    if errors.is_empty() {
        println!("\nValidation: OK");
    } else {
        for e in &errors {
            eprintln!("  ERROR: {e}");
        }
        std::process::exit(1);
    }

    // 4. Resolve DAG
    let order = resolver::build_execution_order(&config).expect("DAG resolution failed");
    println!("Execution order: {order:?}");

    // 5. Plan (no existing state → everything is Create)
    let locks = HashMap::new();
    let plan = planner::plan(&config, &order, &locks, None);
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

    // 6. Show lifecycle summary
    println!("\nContainer lifecycle (what `apply` would do):");
    println!("  1. ensure_container: docker run -d --name forjar-example --init ubuntu:22.04 sleep infinity");
    println!("  2. exec_container:   docker exec -i forjar-example bash <<< <script>");
    println!("  3. cleanup_container: docker rm -f forjar-example (ephemeral=true)");
}
