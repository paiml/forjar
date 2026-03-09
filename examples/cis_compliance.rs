//! Example: CIS Ubuntu 22.04 compliance pack (FJ-3206)
//!
//! Demonstrates the built-in CIS compliance pack with 24 rules,
//! evaluating resources against CIS benchmarks.
//!
//! ```bash
//! cargo run --example cis_compliance
//! ```

use forjar::core::cis_ubuntu_pack::{cis_ubuntu_2204_pack, severity_summary};
use forjar::core::compliance_pack::evaluate_pack;
use std::collections::HashMap;

fn main() {
    println!("=== CIS Ubuntu 22.04 Compliance Pack (FJ-3206) ===\n");

    let pack = cis_ubuntu_2204_pack();

    // 1. Pack overview
    println!("1. Pack Overview:");
    println!("   Name:      {}", pack.name);
    println!("   Version:   {}", pack.version);
    println!("   Framework: {}", pack.framework);
    println!("   Rules:     {}", pack.rules.len());

    let (errors, warnings, info) = severity_summary(&pack);
    println!("   Severity:  {errors} error, {warnings} warning, {info} info");

    // 2. List all rules
    println!("\n2. Rule Summary:");
    for rule in &pack.rules {
        let controls = rule.controls.join(", ");
        println!(
            "   {:<12} [{:<7}] {} ({})",
            rule.id, rule.severity, rule.title, controls
        );
    }

    // 3. Evaluate against a compliant configuration
    println!("\n3. Evaluation — Compliant Config:");
    let mut resources = HashMap::new();
    let mut file_fields = HashMap::new();
    file_fields.insert("type".into(), "file".into());
    file_fields.insert("owner".into(), "root".into());
    file_fields.insert("group".into(), "root".into());
    file_fields.insert("mode".into(), "0644".into());
    file_fields.insert("content".into(), "0".into());
    file_fields.insert("tags".into(), "system,environment".into());
    resources.insert("etc-passwd".into(), file_fields);

    let mut svc_fields = HashMap::new();
    svc_fields.insert("type".into(), "service".into());
    svc_fields.insert("enabled".into(), "true".into());
    svc_fields.insert("firewall".into(), "ufw".into());
    svc_fields.insert("tags".into(), "system,environment".into());
    resources.insert("auditd".into(), svc_fields);

    let result = evaluate_pack(&pack, &resources);
    println!(
        "   Passed: {}/{}",
        result.passed_count(),
        result.results.len()
    );
    println!("   Failed: {}", result.failed_count());
    println!("   Rate:   {:.1}%", result.pass_rate());

    for r in &result.results {
        let icon = if r.passed { "PASS" } else { "FAIL" };
        println!("   [{icon}] {}: {}", r.rule_id, r.message);
    }

    // 4. Evaluate against a non-compliant configuration
    println!("\n4. Evaluation — Non-Compliant Config:");
    let mut bad_resources = HashMap::new();
    let mut bad_file = HashMap::new();
    bad_file.insert("type".into(), "file".into());
    bad_file.insert("mode".into(), "777".into());
    bad_file.insert("content".into(), "PermitRootLogin yes".into());
    bad_resources.insert("sshd-config".into(), bad_file);

    let mut bad_pkg = HashMap::new();
    bad_pkg.insert("type".into(), "package".into());
    bad_pkg.insert("name".into(), "telnetd".into());
    bad_resources.insert("legacy-telnet".into(), bad_pkg);

    let result = evaluate_pack(&pack, &bad_resources);
    println!(
        "   Passed: {}/{}",
        result.passed_count(),
        result.results.len()
    );
    println!("   Failed: {}", result.failed_count());

    let failures: Vec<_> = result.results.iter().filter(|r| !r.passed).collect();
    for r in &failures {
        let controls = r.controls.join(", ");
        println!("   [FAIL] {}: {} ({})", r.rule_id, r.message, controls);
    }

    println!("\nDone.");
}
