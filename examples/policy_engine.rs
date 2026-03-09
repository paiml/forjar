//! FJ-3200: Policy-as-Code engine example.
//!
//! Demonstrates assert, deny, require, limit, and warn policy types
//! with compliance mappings, severity overrides, and JSON output.

fn main() {
    let yaml = r#"
version: "1.0"
name: policy-demo
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
resources:
  nginx-conf:
    type: file
    machine: web
    path: /etc/nginx/nginx.conf
    owner: root
    mode: "0644"
    tags: [web, config]
  big-packages:
    type: package
    machine: web
    provider: apt
    packages: [curl, wget, jq, htop, vim, tmux, git, make, gcc, cmake]
  app-conf:
    type: file
    machine: web
    path: /etc/app.conf
policies:
  # Assert: owner must be root for config files
  - type: assert
    id: SEC-001
    message: "config files must be owned by root"
    resource_type: file
    tag: config
    condition_field: owner
    condition_value: root
    severity: error
    remediation: "Set owner: root on the resource"
    compliance:
      - framework: cis
        control: "6.1.2"

  # Deny: no files without mode
  - type: require
    id: SEC-002
    message: "all files must have explicit mode"
    resource_type: file
    field: mode
    remediation: "Add mode field (e.g., mode: '0644')"

  # Limit: packages under 5 per resource
  - type: limit
    id: PERF-001
    message: "package resources should have fewer than 5 packages"
    resource_type: package
    field: packages
    max_count: 5
    severity: warning
    remediation: "Split into multiple package resources for parallel install"

  # Warn: all resources should have tags
  - type: limit
    id: OPS-001
    message: "all resources must have at least 1 tag"
    field: tags
    min_count: 1
    severity: info
    remediation: "Add tags for selective filtering"
"#;

    let config: forjar::core::types::ForjarConfig =
        serde_yaml_ng::from_str(yaml).expect("valid YAML");

    // Run full policy check
    let result = forjar::core::parser::evaluate_policies_full(&config);

    println!("=== Policy Check Result ===\n");
    println!(
        "Rules evaluated: {}  |  Resources checked: {}",
        result.rules_evaluated, result.resources_checked
    );
    println!(
        "Errors: {}  |  Warnings: {}  |  Info: {}",
        result.error_count(),
        result.warning_count(),
        result.info_count()
    );
    println!("Blocking: {}\n", result.has_blocking_violations());

    for v in &result.violations {
        let sev = format!("{:?}", v.severity).to_uppercase();
        let id = v.policy_id.as_deref().unwrap_or("-");
        println!("[{sev}] [{id}] {}: {}", v.resource_id, v.rule_message);
        if let Some(ref rem) = v.remediation {
            println!("  fix: {rem}");
        }
        for c in &v.compliance {
            println!("  compliance: {} {}", c.framework, c.control);
        }
    }

    // JSON output
    println!("\n=== JSON Output ===\n");
    println!("{}", forjar::core::parser::policy_check_to_json(&result));
}
