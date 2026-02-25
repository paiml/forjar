//! Demonstrate a realistic full-stack deployment: packages, files, users,
//! services, cron, firewall, and Docker — all composed with dependencies.
//!
//! This example shows how forjar handles a multi-resource deployment with
//! dependency ordering, template resolution, and plan generation.
//!
//! Usage: cargo run --example full_stack_deploy

use forjar::core::{codegen, parser, planner, resolver};
use std::collections::HashMap;

fn main() {
    let yaml = r#"
version: "1.0"
name: full-stack-demo
description: "Complete app deployment: packages → users → files → services → cron → firewall → docker"

params:
  app_port: "8080"
  env: staging

machines:
  app-server:
    hostname: app-server
    addr: 127.0.0.1

resources:
  # Layer 1: Base packages
  base-packages:
    type: package
    machine: app-server
    provider: apt
    packages: [curl, jq, htop, ufw]

  # Layer 2: Application user
  app-user:
    type: user
    machine: app-server
    name: appservice
    shell: /usr/sbin/nologin
    system_user: true
    depends_on: [base-packages]

  # Layer 3: Application directory + config
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

  # Layer 4: Docker container for the app
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

  # Layer 5: Service monitoring
  health-cron:
    type: cron
    machine: app-server
    name: app-health-check
    owner: root
    schedule: "*/5 * * * *"
    command: "curl -sf http://localhost:{{params.app_port}}/health || logger 'app health check failed'"
    depends_on: [app-container]

  # Layer 6: Firewall rules
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
"#;

    // 1. Parse + validate
    let config = parser::parse_config(yaml).expect("parse failed");
    let errors = parser::validate_config(&config);
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("  ERROR: {}", e);
        }
        std::process::exit(1);
    }
    println!("Parsed: {} ({} resources)", config.name, config.resources.len());

    // 2. Resolve templates per-resource
    let params = config.params.clone();
    println!("\nTemplate resolution:");
    let mut resolved = config.clone();
    for (id, resource) in &config.resources {
        let r = resolver::resolve_resource_templates(resource, &params, &config.machines)
            .expect("resolution failed");
        if let Some(ref content) = r.content {
            if content.contains("staging") || content.contains("8080") {
                println!("  {}: templates resolved", id);
            }
        }
        resolved.resources.insert(id.clone(), r);
    }

    // 3. DAG execution order
    let order = resolver::build_execution_order(&resolved).expect("DAG failed");
    println!("\nExecution order (topological):");
    for (i, id) in order.iter().enumerate() {
        println!("  {}. {}", i + 1, id);
    }

    // 4. Plan
    let locks = HashMap::new();
    let plan = planner::plan(&resolved, &order, &locks, None);
    println!("\nPlan: {} resources", plan.changes.len());
    for change in &plan.changes {
        let sym = match change.action {
            forjar::core::types::PlanAction::Create => "+",
            forjar::core::types::PlanAction::Update => "~",
            forjar::core::types::PlanAction::Destroy => "-",
            forjar::core::types::PlanAction::NoOp => " ",
        };
        println!("  {} {}", sym, change.description);
    }

    // 5. Show generated scripts for key resources
    println!("\n--- Generated Scripts ---");
    for id in &["app-container", "health-cron", "allow-app-port"] {
        if let Some(resource) = resolved.resources.get(*id) {
            let apply = codegen::apply_script(resource).expect("codegen failed");
            println!("\n[{}] apply script:", id);
            for line in apply.lines().take(5) {
                println!("  {}", line);
            }
            let total_lines = apply.lines().count();
            if total_lines > 5 {
                println!("  ... ({} lines total)", total_lines);
            }
        }
    }

    println!("\n=== Full Stack Deploy Example Complete ===");
}
