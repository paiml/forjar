//! Demonstrate cross-architecture filtering in planning.
//!
//! Resources with `arch:` filters are only planned for machines with matching
//! architectures. This example shows how a multi-arch fleet plan skips
//! mismatched resources.
//!
//! Usage: cargo run --example arch_filtering

use forjar::core::{parser, planner, resolver};
use std::collections::HashMap;

fn main() {
    let yaml = r#"
version: "1.0"
name: multi-arch-fleet
description: "Mixed x86_64 and aarch64 fleet"

machines:
  x86-server:
    hostname: x86-server
    addr: 10.0.0.1
    arch: x86_64
  arm-server:
    hostname: arm-server
    addr: 10.0.0.2
    arch: aarch64

resources:
  # Runs on all machines (no arch filter)
  base-packages:
    type: package
    machine: [x86-server, arm-server]
    provider: apt
    packages: [curl, htop, jq]

  # Only runs on x86_64 machines
  intel-driver:
    type: package
    machine: [x86-server, arm-server]
    provider: apt
    packages: [intel-microcode]
    arch: [x86_64]
    depends_on: [base-packages]

  # Only runs on aarch64 machines
  arm-firmware:
    type: package
    machine: [x86-server, arm-server]
    provider: apt
    packages: [linux-firmware-raspi]
    arch: [aarch64]
    depends_on: [base-packages]

  # Runs on both — uses arch to target specific builds
  monitoring-agent:
    type: file
    machine: [x86-server, arm-server]
    path: /usr/local/bin/node_exporter
    source: ./binaries/node_exporter
    mode: "0755"
    depends_on: [base-packages]
"#;

    // Parse and validate
    let config = parser::parse_config(yaml).expect("parse failed");
    let errors = parser::validate_config(&config);
    assert!(errors.is_empty(), "validation errors: {errors:?}");
    println!(
        "Config: {} ({} machines, {} resources)",
        config.name,
        config.machines.len(),
        config.resources.len()
    );

    // Show machine architectures
    println!("\nMachines:");
    for (key, machine) in &config.machines {
        println!("  {} → {} ({})", key, machine.addr, machine.arch);
    }

    // Show resource arch filters
    println!("\nResource arch filters:");
    for (id, resource) in &config.resources {
        if resource.arch.is_empty() {
            println!("  {id} → all architectures");
        } else {
            println!("  {} → {:?}", id, resource.arch);
        }
    }

    // Build plan — arch filtering happens here
    let order = resolver::build_execution_order(&config).expect("DAG failed");
    println!("\nExecution order: {order:?}");

    let locks = HashMap::new();
    let plan = planner::plan(&config, &order, &locks, None);

    println!("\nPlan: {} changes", plan.changes.len());
    for change in &plan.changes {
        let symbol = match change.action {
            forjar::core::types::PlanAction::Create => "+",
            forjar::core::types::PlanAction::Update => "~",
            forjar::core::types::PlanAction::Destroy => "-",
            forjar::core::types::PlanAction::NoOp => " ",
        };
        println!("  {} {}", symbol, change.description);
    }

    // The plan should have:
    // - base-packages on both machines (2 changes)
    // - intel-driver on x86 only (1 change, skipped on arm)
    // - arm-firmware on arm only (1 change, skipped on x86)
    // - monitoring-agent on both (2 changes)
    // Total: 6 changes
    println!(
        "\nSummary: {} create, {} update, {} destroy, {} unchanged",
        plan.to_create, plan.to_update, plan.to_destroy, plan.unchanged
    );
    assert_eq!(
        plan.to_create, 6,
        "expected 6 creates (arch filtering skips 2)"
    );

    println!("\nArch filtering works correctly.");
}
