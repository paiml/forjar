#![allow(clippy::field_reassign_with_default)]
//! Example: Compliance pack pre-apply gate (FJ-3203)
//!
//! Demonstrates loading compliance packs from disk and evaluating
//! them against config resources as a pre-apply gate.
//!
//! ```bash
//! cargo run --example compliance_gate
//! ```

use forjar::core::compliance_gate::{
    check_compliance_gate, config_to_resource_map, format_gate_result,
};
use forjar::core::types::{ForjarConfig, Resource, ResourceType};
use tempfile::TempDir;

fn main() {
    println!("=== Compliance Pack Pre-Apply Gate (FJ-3203) ===\n");

    // 1. Create config with resources
    let mut config = ForjarConfig::default();

    let mut nginx = Resource::default();
    nginx.resource_type = ResourceType::File;
    nginx.owner = Some("root".into());
    nginx.mode = Some("0644".into());
    nginx.tags = vec!["web".into(), "config".into()];
    config.resources.insert("nginx-conf".into(), nginx);

    let mut sshd = Resource::default();
    sshd.resource_type = ResourceType::File;
    sshd.owner = Some("root".into());
    sshd.mode = Some("0600".into());
    config.resources.insert("sshd-config".into(), sshd);

    let mut docker = Resource::default();
    docker.resource_type = ResourceType::Package;
    config.resources.insert("docker-ce".into(), docker);

    println!("1. Resources:");
    let map = config_to_resource_map(&config);
    for (id, fields) in &map {
        println!("   {id}: {:?}", fields);
    }

    // 2. Create a policy directory with packs
    let dir = TempDir::new().unwrap();

    // Passing pack
    std::fs::write(
        dir.path().join("ownership.yaml"),
        r#"
name: ownership-check
version: "1.0"
framework: INTERNAL
rules:
  - id: OWN-001
    title: Files must have owner
    severity: error
    type: require
    resource_type: file
    field: owner
"#,
    )
    .unwrap();

    // Pack with expected failures
    std::fs::write(
        dir.path().join("hardening.yaml"),
        r#"
name: hardening
version: "1.0"
framework: CIS
rules:
  - id: HARD-001
    title: Packages must have name
    severity: warning
    type: require
    resource_type: package
    field: name
"#,
    )
    .unwrap();

    // 3. Run the gate
    println!("\n2. Evaluating compliance packs:");
    let result = check_compliance_gate(dir.path(), &config, true).unwrap();
    println!("\n3. {}", format_gate_result(&result));

    if result.passed() {
        println!("   Apply would proceed.");
    } else {
        println!("   Apply would be BLOCKED by compliance violations.");
    }

    println!("\nDone.");
}
