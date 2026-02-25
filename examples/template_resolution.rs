//! Demonstrate template resolution: params, machine refs, and resource templates.
//!
//! Usage: cargo run --example template_resolution

use forjar::core::{parser, resolver};

fn main() {
    let yaml = r#"
version: "1.0"
name: template-demo
params:
  env: staging
  port: "8080"
  log_level: info

machines:
  web:
    hostname: web-01
    addr: 10.0.0.5
    user: deploy
    arch: x86_64
  db:
    hostname: db-01
    addr: 10.0.0.10
    user: admin
    arch: aarch64

resources:
  app-config:
    type: file
    machine: web
    path: /etc/myapp/config.yaml
    content: |
      environment: {{params.env}}
      listen_port: {{params.port}}
      log_level: {{params.log_level}}
      db_host: {{machine.db.addr}}
      db_user: {{machine.db.user}}
      hostname: {{machine.web.hostname}}
    owner: deploy
    mode: "0640"

  db-config:
    type: file
    machine: db
    path: /etc/mydb/access.conf
    content: |
      allow_host={{machine.web.addr}}
      arch={{machine.db.arch}}
"#;

    let config = parser::parse_config(yaml).expect("parse failed");

    println!("=== Template Resolution Demo ===\n");

    // Resolve templates for each resource
    let params = config.params.clone();

    for (id, resource) in &config.resources {
        let resolved = resolver::resolve_resource_templates(resource, &params, &config.machines)
            .expect("resolution failed");

        println!("Resource: {}", id);
        if let Some(ref path) = resolved.path {
            println!("  path: {}", path);
        }
        if let Some(ref content) = resolved.content {
            println!("  content:");
            for line in content.lines() {
                println!("    {}", line);
            }
        }
        println!();
    }

    // Demonstrate raw template resolution
    println!("=== Direct Template Resolution ===\n");
    let templates = [
        "http://{{machine.web.addr}}:{{params.port}}",
        "{{machine.db.hostname}} ({{machine.db.arch}})",
        "env={{params.env}}",
    ];
    for t in &templates {
        let result = resolver::resolve_template(t, &params, &config.machines).unwrap();
        println!("  {} → {}", t, result);
    }

    println!("\n=== Template Resolution Complete ===");
}
