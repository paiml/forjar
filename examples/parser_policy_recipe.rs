//! FJ-2501/2500/220/3200/004: Parser validation, policy, unknown fields, recipes.
//!
//! Demonstrates:
//! - Format validation (mode, port, cron, owner/group, deny_paths)
//! - Unknown field detection with Levenshtein suggestions
//! - Policy-as-Code evaluation (require, deny, assert, limit)
//! - Policy JSON and SARIF output
//! - Recipe parsing, input validation, and expansion
//!
//! Usage: cargo run --example parser_policy_recipe

use forjar::core::parser::{
    check_unknown_fields, evaluate_policies_full, parse_config, policy_check_to_json,
    validate_config,
};
use forjar::core::recipe::{expand_recipe, parse_recipe, recipe_terminal_id, validate_inputs};
use forjar::core::types::*;
use std::collections::HashMap;

fn main() {
    println!("Forjar: Parser, Policy & Recipe Validation");
    println!("{}", "=".repeat(50));

    // ── FJ-2501: Format Validation ──
    println!("\n[FJ-2501] Format Validation:");
    let valid_yaml = r#"
version: "1.0"
name: format-demo
resources:
  conf:
    type: file
    path: /etc/app.conf
    mode: "0644"
    owner: www-data
    group: www-data
"#;
    let cfg = parse_config(valid_yaml).unwrap();
    let errors = validate_config(&cfg);
    println!("  Valid config: {} errors", errors.len());
    assert!(errors.is_empty());

    let bad_mode_yaml = r#"
version: "1.0"
name: bad-mode
resources:
  conf:
    type: file
    path: /etc/test.conf
    mode: "999"
"#;
    let cfg = parse_config(bad_mode_yaml).unwrap();
    let errors = validate_config(&cfg);
    println!("  Invalid mode '999': {} errors", errors.len());
    assert!(errors.iter().any(|e| e.message.contains("mode")));

    // ── FJ-2500: Unknown Fields ──
    println!("\n[FJ-2500] Unknown Field Detection:");
    let typo_yaml = r#"
version: "1.0"
name: typo-demo
resources:
  pkg:
    type: package
    packges: [nginx]
"#;
    let warnings = check_unknown_fields(typo_yaml);
    for w in &warnings {
        println!("  Warning: {}", w.message);
    }
    assert!(!warnings.is_empty());
    assert!(warnings[0].message.contains("packages"));

    // ── FJ-220/3200: Policy Evaluation ──
    println!("\n[FJ-220/3200] Policy Evaluation:");
    let policy_yaml = r#"
version: "1.0"
name: policy-demo
resources:
  conf:
    type: file
    path: /etc/app.conf
  svc:
    type: service
    name: sshd
policies:
  - type: require
    message: "files must have owner"
    resource_type: file
    field: owner
  - type: deny
    message: "telnet is forbidden"
    resource_type: service
    condition_field: name
    condition_value: telnetd
"#;
    let cfg = parse_config(policy_yaml).unwrap();
    let result = evaluate_policies_full(&cfg);
    println!(
        "  Rules: {}, Resources: {}, Violations: {}",
        result.rules_evaluated,
        result.resources_checked,
        result.violations.len()
    );
    for v in &result.violations {
        println!(
            "    [{:?}] {}: {}",
            v.severity, v.resource_id, v.rule_message
        );
    }
    assert_eq!(result.violations.len(), 1); // file missing owner

    let json = policy_check_to_json(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    println!("  JSON passed: {}", parsed["passed"]);
    assert_eq!(parsed["passed"], false);

    // ── FJ-004: Recipe Parsing & Expansion ──
    println!("\n[FJ-004] Recipe Expansion:");
    let recipe_yaml = r#"
recipe:
  name: web-stack
  inputs:
    domain:
      type: string
    port:
      type: int
      default: 443
      min: 80
      max: 65535
resources:
  pkg:
    type: package
    packages: [nginx]
  conf:
    type: file
    path: "/etc/nginx/{{inputs.domain}}.conf"
    content: "listen {{inputs.port}}"
    depends_on: [pkg]
"#;
    let rf = parse_recipe(recipe_yaml).unwrap();
    println!(
        "  Recipe: {} ({} resources)",
        rf.recipe.name,
        rf.resources.len()
    );

    let mut inputs = HashMap::new();
    inputs.insert(
        "domain".into(),
        serde_yaml_ng::Value::String("example.com".into()),
    );
    let machine = MachineTarget::Single("web-01".into());
    let expanded = expand_recipe("web", &rf, &machine, &inputs, &[]).unwrap();
    for (id, r) in &expanded {
        println!(
            "  {id}: type={:?}, path={:?}",
            r.resource_type,
            r.path.as_deref().unwrap_or("-")
        );
    }
    assert!(expanded.contains_key("web/conf"));
    let conf = &expanded["web/conf"];
    assert!(conf.path.as_ref().unwrap().contains("example.com"));
    assert!(conf.depends_on.contains(&"web/pkg".to_string()));

    let tid = recipe_terminal_id("web", &rf);
    println!("  Terminal ID: {:?}", tid);
    assert_eq!(tid, Some("web/conf".into()));

    // Input validation
    let meta = &rf.recipe;
    let resolved = validate_inputs(meta, &inputs).unwrap();
    println!(
        "  Resolved inputs: domain={}, port={}",
        resolved["domain"], resolved["port"]
    );
    assert_eq!(resolved["domain"], "example.com");
    assert_eq!(resolved["port"], "443"); // default

    println!("\n{}", "=".repeat(50));
    println!("All parser/policy/recipe criteria survived.");
}
