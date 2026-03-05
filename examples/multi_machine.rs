//! Demonstrate multi-machine orchestration with dependencies, cost-based scheduling,
//! and cross-machine resource ordering.
//!
//! Usage: cargo run --example multi_machine

use forjar::core::{codegen, parser, planner, resolver, types};
use std::collections::HashMap;

fn main() {
    println!("=== Multi-Machine Orchestration Example ===\n");
    let config = parse_demo_config();
    show_machines(&config);
    show_execution_order(&config);
    show_plan(&config);
    show_dependency_edges(&config);
    show_template_resolution(&config);
    show_mount_script(&config);
    println!("\n=== Multi-Machine Example Complete ===");
}

fn demo_yaml() -> &'static str {
    r#"
version: "1.0"
name: multi-machine-demo
description: "3-machine infrastructure with cross-machine dependencies"

params:
  env: staging
  nfs_server: 192.168.50.10

machines:
  nfs-server:
    hostname: nfs-srv
    addr: 192.168.50.10
    cost: 1
  app-server:
    hostname: app-srv
    addr: 192.168.50.20
    cost: 10
  monitor:
    hostname: mon-srv
    addr: 192.168.50.30
    cost: 5

resources:
  nfs-packages:
    type: package
    machine: nfs-server
    provider: apt
    packages: [nfs-kernel-server]

  nfs-exports:
    type: file
    machine: nfs-server
    path: /etc/exports
    content: "/srv/data *(rw,sync,no_subtree_check)\n"
    owner: root
    mode: "0644"
    depends_on: [nfs-packages]

  nfs-service:
    type: service
    machine: nfs-server
    name: nfs-kernel-server
    state: running
    enabled: true
    depends_on: [nfs-exports]
    restart_on: [nfs-exports]

  app-packages:
    type: package
    machine: app-server
    provider: apt
    packages: [nfs-common, nginx]

  app-nfs-mount:
    type: mount
    machine: app-server
    source: "{{params.nfs_server}}:/srv/data"
    path: /mnt/data
    fs_type: nfs
    options: "ro,hard,intr"
    state: mounted
    depends_on: [nfs-service, app-packages]

  app-config:
    type: file
    machine: app-server
    path: /etc/nginx/sites-available/app.conf
    content: |
      server {
          listen 80;
          server_name app.{{params.env}}.internal;
          root /mnt/data/www;
      }
    depends_on: [app-nfs-mount]

  app-service:
    type: service
    machine: app-server
    name: nginx
    state: running
    enabled: true
    depends_on: [app-config]
    restart_on: [app-config]

  monitor-packages:
    type: package
    machine: monitor
    provider: apt
    packages: [prometheus-node-exporter]

  monitor-config:
    type: file
    machine: monitor
    path: /etc/prometheus/targets.yml
    content: |
      - targets:
          - {{params.nfs_server}}:9100
          - 192.168.50.20:9100
    depends_on: [monitor-packages]

  firewall-prometheus:
    type: network
    machine: monitor
    port: "9090"
    protocol: tcp
    action: allow
    from_addr: 192.168.50.0/24
    depends_on: [monitor-packages]
"#
}

fn parse_demo_config() -> types::ForjarConfig {
    let config = parser::parse_config(demo_yaml()).expect("parse failed");
    let errors = parser::validate_config(&config);
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("  validation error: {}", e.message);
        }
        std::process::exit(1);
    }
    println!(
        "Config: {} machines, {} resources",
        config.machines.len(),
        config.resources.len()
    );
    config
}

fn show_machines(config: &types::ForjarConfig) {
    println!("\nMachines (sorted by cost):");
    let mut machines: Vec<_> = config.machines.iter().collect();
    machines.sort_by_key(|(_, m)| m.cost);
    for (name, machine) in &machines {
        println!("  {name} (addr: {}, cost: {})", machine.addr, machine.cost);
    }
}

fn show_execution_order(config: &types::ForjarConfig) {
    let order = resolver::build_execution_order(config).expect("DAG failed");
    println!("\nExecution order (topological + alphabetical tie-break):");
    for (i, resource_id) in order.iter().enumerate() {
        let resource = &config.resources[resource_id];
        let machine = match &resource.machine {
            types::MachineTarget::Single(m) => m.clone(),
            types::MachineTarget::Multiple(ms) => ms.join(", "),
        };
        println!(
            "  {}. {resource_id} (type: {:?}, machine: {machine})",
            i + 1,
            resource.resource_type
        );
    }
}

fn show_plan(config: &types::ForjarConfig) {
    let order = resolver::build_execution_order(config).expect("DAG failed");
    let plan = planner::plan(config, &order, &HashMap::new(), None);
    println!(
        "\nPlan: {} create, {} update, {} destroy, {} unchanged",
        plan.to_create, plan.to_update, plan.to_destroy, plan.unchanged
    );
}

fn show_dependency_edges(config: &types::ForjarConfig) {
    println!("\nDependency edges:");
    for (name, resource) in &config.resources {
        if !resource.depends_on.is_empty() {
            println!("  {name} depends_on: {:?}", resource.depends_on);
        }
    }
}

fn show_template_resolution(config: &types::ForjarConfig) {
    println!("\n=== Template Resolution ===\n");
    let resource = &config.resources["app-config"];
    let resolved = resolver::resolve_resource_templates(resource, &config.params, &config.machines)
        .expect("resolve failed");
    println!("app-config content (resolved):");
    if let Some(ref content) = resolved.content {
        for line in content.lines() {
            println!("  {line}");
        }
    }
}

fn show_mount_script(config: &types::ForjarConfig) {
    println!("\n=== Generated Scripts (app-nfs-mount) ===\n");
    let mount_resource = &config.resources["app-nfs-mount"];
    let mount_resolved =
        resolver::resolve_resource_templates(mount_resource, &config.params, &config.machines)
            .expect("resolve failed");
    if let Ok(script) = codegen::apply_script(&mount_resolved) {
        println!("Apply script:");
        for line in script.lines() {
            println!("  {line}");
        }
    }
}
