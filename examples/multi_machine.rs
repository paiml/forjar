//! Demonstrate multi-machine orchestration with dependencies, cost-based scheduling,
//! and cross-machine resource ordering.
//!
//! Usage: cargo run --example multi_machine

use forjar::core::{codegen, parser, planner, resolver};
use std::collections::HashMap;

fn main() {
    let yaml = r#"
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
  # NFS server setup (runs first, cheapest machine)
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

  # App server mounts NFS (depends on NFS service being up)
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

  # Monitoring server watches both (highest cost = runs last)
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
"#;

    println!("=== Multi-Machine Orchestration Example ===\n");

    // Parse and validate
    let config = parser::parse_config(yaml).expect("parse failed");
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

    // Show machines sorted by cost
    println!("\nMachines (sorted by cost):");
    let mut machines: Vec<_> = config.machines.iter().collect();
    machines.sort_by_key(|(_, m)| m.cost);
    for (name, machine) in &machines {
        println!(
            "  {} (addr: {}, cost: {})",
            name, machine.addr, machine.cost
        );
    }

    // Build execution order (DAG toposort)
    let order = resolver::build_execution_order(&config).expect("DAG failed");
    println!("\nExecution order (topological + alphabetical tie-break):");
    for (i, resource_id) in order.iter().enumerate() {
        let resource = &config.resources[resource_id];
        let machine = match &resource.machine {
            forjar::core::types::MachineTarget::Single(m) => m.clone(),
            forjar::core::types::MachineTarget::Multiple(ms) => ms.join(", "),
        };
        println!(
            "  {}. {} (type: {:?}, machine: {})",
            i + 1,
            resource_id,
            resource.resource_type,
            machine
        );
    }

    // Plan
    let plan = planner::plan(&config, &order, &HashMap::new(), None);
    println!(
        "\nPlan: {} create, {} update, {} destroy, {} unchanged",
        plan.to_create, plan.to_update, plan.to_destroy, plan.unchanged
    );

    // Show dependency graph
    println!("\nDependency edges:");
    for (name, resource) in &config.resources {
        if !resource.depends_on.is_empty() {
            println!("  {} depends_on: {:?}", name, resource.depends_on);
        }
    }

    // Template resolution demo
    println!("\n=== Template Resolution ===\n");
    let resource = &config.resources["app-config"];
    let resolved = resolver::resolve_resource_templates(resource, &config.params, &config.machines)
        .expect("resolve failed");
    println!("app-config content (resolved):");
    if let Some(ref content) = resolved.content {
        for line in content.lines() {
            println!("  {}", line);
        }
    }

    // Generate scripts for a sample resource
    println!("\n=== Generated Scripts (app-nfs-mount) ===\n");
    let mount_resource = &config.resources["app-nfs-mount"];
    let mount_resolved =
        resolver::resolve_resource_templates(mount_resource, &config.params, &config.machines)
            .expect("resolve failed");

    if let Ok(script) = codegen::apply_script(&mount_resolved) {
        println!("Apply script:");
        for line in script.lines() {
            println!("  {}", line);
        }
    }

    println!("\n=== Multi-Machine Example Complete ===");
}
