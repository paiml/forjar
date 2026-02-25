//! Demonstrate user resource management: creation, SSH keys, groups, and shell config.
//!
//! Usage: cargo run --example user_management

use forjar::core::{codegen, parser, planner, resolver};
use std::collections::HashMap;

fn main() {
    let yaml = r#"
version: "1.0"
name: user-management-demo
description: "User resource lifecycle: create, configure, remove"

machines:
  local:
    hostname: localhost
    addr: 127.0.0.1

resources:
  deploy-user:
    type: user
    machine: local
    name: deploy
    uid: 2000
    shell: /bin/bash
    home: /home/deploy
    groups: [sudo, docker]
    ssh_authorized_keys:
      - "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExample1 deploy@laptop"
      - "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExample2 deploy@ci"

  app-user:
    type: user
    machine: local
    name: app
    uid: 3000
    shell: /usr/sbin/nologin
    home: /opt/app
    system_user: true

  removed-user:
    type: user
    machine: local
    name: old-admin
    state: absent
"#;

    println!("=== User Management Example ===\n");

    // Parse config
    let config = parser::parse_config(yaml).expect("parse failed");
    println!("Parsed {} resources:", config.resources.len());
    for (name, resource) in &config.resources {
        println!(
            "  {} (type: {:?}, state: {})",
            name,
            resource.resource_type,
            resource.state.as_deref().unwrap_or("present")
        );
    }

    // Validate
    let errors = parser::validate_config(&config);
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("  validation error: {}", e.message);
        }
        std::process::exit(1);
    }
    println!("\nValidation: OK");

    // Resolve and plan
    let order = resolver::build_execution_order(&config).expect("DAG failed");
    println!("\nExecution order: {:?}", order);

    let plan = planner::plan(&config, &order, &HashMap::new(), None);
    println!("\nPlan summary:");
    println!("  create: {}", plan.to_create);
    println!("  update: {}", plan.to_update);
    println!("  destroy: {}", plan.to_destroy);
    println!("  unchanged: {}", plan.unchanged);

    // Show generated scripts for each user resource
    println!("\n=== Generated Scripts ===\n");
    for (name, resource) in &config.resources {
        let resolved =
            resolver::resolve_resource_templates(resource, &config.params, &config.machines)
                .expect("resolve failed");

        println!("--- {} ({}) ---", name, resource.state.as_deref().unwrap_or("present"));

        match codegen::check_script(&resolved) {
            Ok(script) => println!("Check:\n{}\n", indent(&script)),
            Err(e) => println!("Check: {}\n", e),
        }

        match codegen::apply_script(&resolved) {
            Ok(script) => println!("Apply:\n{}\n", indent(&script)),
            Err(e) => println!("Apply: {}\n", e),
        }

        match codegen::state_query_script(&resolved) {
            Ok(script) => println!("State Query:\n{}\n", indent(&script)),
            Err(e) => println!("State Query: {}\n", e),
        }
    }

    println!("=== User Management Example Complete ===");
}

fn indent(s: &str) -> String {
    s.lines().map(|l| format!("  {}", l)).collect::<Vec<_>>().join("\n")
}
