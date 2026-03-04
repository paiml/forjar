//! Demonstrate a realistic full-stack deployment: packages, files, users,
//! services, cron, firewall, and Docker — all composed with dependencies.
//!
//! This example shows how forjar handles a multi-resource deployment with
//! dependency ordering, template resolution, and plan generation.
//!
//! Usage: cargo run --example full_stack_deploy

use forjar::core::{codegen, parser, planner, resolver, types};
use std::collections::HashMap;

fn main() {
    let config = parse_demo_config();
    let resolved = resolve_templates(&config);
    show_execution_order(&resolved);
    show_plan(&resolved);
    show_scripts(&resolved);
    println!("\n=== Full Stack Deploy Example Complete ===");
}

fn demo_yaml() -> &'static str {
    r#"
version: "1.0"
name: full-stack-demo
description: "Complete app deployment: packages > users > files > services > cron > firewall > docker"

params:
  app_port: "8080"
  env: staging

machines:
  app-server:
    hostname: app-server
    addr: 127.0.0.1

resources:
  base-packages:
    type: package
    machine: app-server
    provider: apt
    packages: [curl, jq, htop, ufw]

  app-user:
    type: user
    machine: app-server
    name: appservice
    shell: /usr/sbin/nologin
    system_user: true
    depends_on: [base-packages]

  app-dir:
    type: file
    machine: app-server
    state: directory
    path: /opt/app
    owner: appservice
    mode: "0755"
    depends_on: [app-user]

  app-config:
    type: file
    machine: app-server
    path: /opt/app/config.yaml
    content: |
      port: {{params.app_port}}
      environment: {{params.env}}
      log_level: info
    owner: appservice
    mode: "0644"
    depends_on: [app-dir]

  app-container:
    type: docker
    machine: app-server
    name: myapp
    image: myapp:latest
    state: running
    ports:
      - "{{params.app_port}}:8080"
    environment:
      - "APP_ENV={{params.env}}"
    volumes:
      - "/opt/app/config.yaml:/etc/app/config.yaml:ro"
    restart: unless-stopped
    depends_on: [app-config]

  health-cron:
    type: cron
    machine: app-server
    name: app-health-check
    owner: root
    schedule: "*/5 * * * *"
    command: "curl -sf http://localhost:{{params.app_port}}/health || logger 'app health check failed'"
    depends_on: [app-container]

  allow-app-port:
    type: network
    machine: app-server
    port: "{{params.app_port}}"
    protocol: tcp
    action: allow
    name: app-http
    depends_on: [app-container]

  allow-ssh:
    type: network
    machine: app-server
    port: "22"
    protocol: tcp
    action: allow
    from: "10.0.0.0/8"
    name: ssh-internal

policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#
}

fn parse_demo_config() -> types::ForjarConfig {
    let config = parser::parse_config(demo_yaml()).expect("parse failed");
    let errors = parser::validate_config(&config);
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("  ERROR: {e}");
        }
        std::process::exit(1);
    }
    println!("Parsed: {} ({} resources)", config.name, config.resources.len());
    config
}

fn resolve_templates(config: &types::ForjarConfig) -> types::ForjarConfig {
    let params = config.params.clone();
    println!("\nTemplate resolution:");
    let mut resolved = config.clone();
    for (id, resource) in &config.resources {
        let r = resolver::resolve_resource_templates(resource, &params, &config.machines)
            .expect("resolution failed");
        if r.content.as_ref().is_some_and(|c| c.contains("staging") || c.contains("8080")) {
            println!("  {id}: templates resolved");
        }
        resolved.resources.insert(id.clone(), r);
    }
    resolved
}

fn show_execution_order(resolved: &types::ForjarConfig) {
    let order = resolver::build_execution_order(resolved).expect("DAG failed");
    println!("\nExecution order (topological):");
    for (i, id) in order.iter().enumerate() {
        println!("  {}. {id}", i + 1);
    }
}

fn show_plan(resolved: &types::ForjarConfig) {
    let order = resolver::build_execution_order(resolved).expect("DAG failed");
    let locks = HashMap::new();
    let plan = planner::plan(resolved, &order, &locks, None);
    println!("\nPlan: {} resources", plan.changes.len());
    for change in &plan.changes {
        let sym = match change.action {
            types::PlanAction::Create => "+",
            types::PlanAction::Update => "~",
            types::PlanAction::Destroy => "-",
            types::PlanAction::NoOp => " ",
        };
        println!("  {sym} {}", change.description);
    }
}

fn show_scripts(resolved: &types::ForjarConfig) {
    println!("\n--- Generated Scripts ---");
    for id in &["app-container", "health-cron", "allow-app-port"] {
        if let Some(resource) = resolved.resources.get(*id) {
            let apply = codegen::apply_script(resource).expect("codegen failed");
            println!("\n[{id}] apply script:");
            for line in apply.lines().take(5) {
                println!("  {line}");
            }
            let total_lines = apply.lines().count();
            if total_lines > 5 {
                println!("  ... ({total_lines} lines total)");
            }
        }
    }
}
