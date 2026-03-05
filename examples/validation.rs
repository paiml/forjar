//! Demonstrate config validation — showing how errors are collected and reported.
//!
//! Usage: cargo run --example validation

use forjar::core::parser;

fn main() {
    println!("=== Valid Config ===\n");
    let valid = r#"
version: "1.0"
name: valid-demo
machines:
  web:
    hostname: web.example.com
    addr: 203.0.113.10
resources:
  nginx:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
  config:
    type: file
    machine: web
    path: /etc/nginx/nginx.conf
    content: "events { worker_connections 1024; }"
    mode: "0644"
    depends_on: [nginx]
"#;
    let config = parser::parse_config(valid).expect("parse failed");
    let errors = parser::validate_config(&config);
    println!("Errors: {} (expected 0)", errors.len());
    assert!(errors.is_empty());

    println!("\n=== Invalid Config (multiple errors) ===\n");
    let invalid = r#"
version: "2.0"
name: ""
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
    arch: sparc64
resources:
  bad-pkg:
    type: package
    machine: ghost-machine
  bad-file:
    type: file
    machine: m1
    content: inline
    source: ./also-a-file.txt
  bad-svc:
    type: service
    machine: m1
    state: restarted
  self-dep:
    type: file
    machine: m1
    path: /etc/loopy
    depends_on: [self-dep]
"#;
    let config = parser::parse_config(invalid).expect("parse failed");
    let errors = parser::validate_config(&config);
    println!("Found {} validation errors:", errors.len());
    for (i, e) in errors.iter().enumerate() {
        println!("  {}. {}", i + 1, e);
    }
    assert!(errors.len() >= 6, "expected at least 6 errors");

    println!("\n=== Container Transport Validation ===\n");
    let container = r#"
version: "1.0"
name: container-demo
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
resources:
  pkg:
    type: package
    machine: test-box
    provider: apt
    packages: [curl]
"#;
    let config = parser::parse_config(container).expect("parse failed");
    let errors = parser::validate_config(&config);
    println!("Container config errors: {} (expected 0)", errors.len());
    assert!(errors.is_empty());

    println!("\n=== Ephemeral Container Without Image ===\n");
    let bad_container = r#"
version: "1.0"
name: bad-container
machines:
  ephemeral:
    hostname: ephemeral
    addr: container
    transport: container
    container:
      runtime: docker
      ephemeral: true
resources: {}
"#;
    let config = parser::parse_config(bad_container).expect("parse failed");
    let errors = parser::validate_config(&config);
    println!("Errors: {}", errors.len());
    for e in &errors {
        println!("  - {e}");
    }
    assert!(errors.iter().any(|e| e.message.contains("ephemeral")));

    println!("\nAll validation demos passed.");
}
