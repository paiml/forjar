//! FJ-3205: Compliance pack evaluation example.
//!
//! Demonstrates loading and evaluating compliance packs (CIS, SOC2)
//! against infrastructure resources.

use forjar::core::compliance_pack::*;
use std::collections::HashMap;

fn main() {
    println!("=== FJ-3205: Compliance Pack Evaluation ===\n");

    // 1. Parse a CIS-style compliance pack
    println!("--- CIS Ubuntu Pack ---");
    let cis_yaml = r#"
name: cis-ubuntu-22.04
version: "1.0.0"
framework: CIS
description: "CIS Benchmark for Ubuntu 22.04 LTS"
rules:
  - id: "CIS-1.1.1"
    title: "Ensure config files are root-owned"
    severity: error
    controls: ["CIS 1.1.1", "SOC2 CC6.1"]
    type: assert
    resource_type: file
    field: owner
    expected: root
  - id: "CIS-1.1.2"
    title: "No world-writable config files"
    severity: error
    controls: ["CIS 1.1.2"]
    type: deny
    resource_type: file
    field: mode
    pattern: "777"
  - id: "CIS-1.2.1"
    title: "All resources must have tags"
    severity: warning
    controls: ["CIS 1.2.1"]
    type: require_tag
    tag: managed
"#;

    let pack = parse_pack(cis_yaml).unwrap();
    println!("  Pack: {} v{}", pack.name, pack.version);
    println!("  Framework: {}", pack.framework);
    println!("  Rules: {}", pack.rules.len());

    // 2. Build test resources
    let mut resources = HashMap::new();

    let mut nginx_conf = HashMap::new();
    nginx_conf.insert("type".into(), "file".into());
    nginx_conf.insert("owner".into(), "root".into());
    nginx_conf.insert("mode".into(), "644".into());
    nginx_conf.insert("tags".into(), "managed,config".into());
    resources.insert("nginx-conf".into(), nginx_conf);

    let mut app_script = HashMap::new();
    app_script.insert("type".into(), "file".into());
    app_script.insert("owner".into(), "deploy".into());
    app_script.insert("mode".into(), "755".into());
    app_script.insert("tags".into(), "managed".into());
    resources.insert("app-script".into(), app_script);

    // 3. Evaluate pack against resources
    println!("\n--- Evaluation Results ---");
    let result = evaluate_pack(&pack, &resources);
    for r in &result.results {
        let icon = if r.passed { "PASS" } else { "FAIL" };
        let controls = if r.controls.is_empty() {
            String::new()
        } else {
            format!(" [{}]", r.controls.join(", "))
        };
        println!("  [{icon}] {}: {}{controls}", r.rule_id, r.message);
    }

    println!(
        "\n  Summary: {}/{} passed ({:.0}% compliance)",
        result.passed_count(),
        result.results.len(),
        result.pass_rate()
    );

    // 4. List packs in a directory
    println!("\n--- Pack Discovery ---");
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("cis-ubuntu.yaml"), cis_yaml).unwrap();
    std::fs::write(
        dir.path().join("soc2-basic.yaml"),
        "name: soc2-basic\nversion: '1.0'\nframework: SOC2\nrules: []",
    )
    .unwrap();

    let packs = list_packs(dir.path());
    for name in &packs {
        println!("  Found pack: {name}");
    }

    println!("\n--- Summary ---");
    println!("Frameworks: CIS, STIG, SOC2");
    println!("Check types: assert, deny, require, require_tag, script");
    println!("Controls: maps to framework-specific IDs");
}
