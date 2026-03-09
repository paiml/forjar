//! FJ-3506: `forjar promote` CLI handler.
//!
//! Evaluates promotion gates for a target environment and reports results.

use crate::core::promotion::{evaluate_gates, GateResult};
use std::path::Path;

/// Run promote command: parse config, find promotion config, evaluate gates.
pub fn cmd_promote(
    file: &Path,
    target: &str,
    _yes: bool,
    dry_run: bool,
    json: bool,
) -> Result<(), String> {
    let config = crate::core::parser::parse_and_validate(file)?;

    // Find the target environment's promotion config
    if config.environments.is_empty() {
        return Err("no environments defined in config".to_string());
    }

    let env = config
        .environments
        .get(target)
        .ok_or_else(|| format!("environment '{target}' not found in config"))?;

    let promotion = env
        .promotion
        .as_ref()
        .ok_or_else(|| format!("environment '{target}' has no promotion config"))?;

    if dry_run {
        println!("Dry-run: evaluating gates for promotion to '{target}'");
    }

    let result = evaluate_gates(file, target, promotion);

    if json {
        print_json(
            &result.from,
            target,
            &result.gates,
            result.all_passed,
            result.auto_approve,
        );
    } else {
        print_table(
            &result.from,
            target,
            &result.gates,
            result.all_passed,
            result.auto_approve,
            dry_run,
        );
    }

    if !result.all_passed {
        return Err(format!(
            "promotion blocked: {} gate(s) failed",
            result.failed_count()
        ));
    }

    if dry_run {
        println!("\nDry-run complete. Use without --dry-run to apply promotion.");
    }

    Ok(())
}

fn print_table(
    from: &str,
    to: &str,
    gates: &[GateResult],
    all_passed: bool,
    auto_approve: bool,
    dry_run: bool,
) {
    println!("Promotion: {} -> {}", from, to);
    println!("{}", "-".repeat(60));
    for gate in gates {
        let icon = if gate.passed { "PASS" } else { "FAIL" };
        println!("  [{icon}] {}: {}", gate.gate_type, gate.message);
    }
    println!("{}", "-".repeat(60));
    let status = if all_passed { "APPROVED" } else { "BLOCKED" };
    let mode = if dry_run { " (dry-run)" } else { "" };
    println!("Result: {status}{mode} (auto-approve: {auto_approve})");
}

fn print_json(from: &str, to: &str, gates: &[GateResult], all_passed: bool, auto_approve: bool) {
    let gates_json: Vec<serde_json::Value> = gates
        .iter()
        .map(|g| {
            serde_json::json!({
                "gate_type": g.gate_type,
                "passed": g.passed,
                "message": g.message,
            })
        })
        .collect();

    let output = serde_json::json!({
        "from": from,
        "to": to,
        "gates": gates_json,
        "all_passed": all_passed,
        "auto_approve": auto_approve,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::environment::PromotionConfig;

    #[test]
    fn parse_promotion_config_roundtrip() {
        let yaml = r#"
from: dev
gates:
  - validate: { deep: true }
  - script: "echo ok"
auto_approve: true
"#;
        let config: PromotionConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.from, "dev");
        assert_eq!(config.gates.len(), 2);
        assert!(config.auto_approve);
    }

    #[test]
    fn promote_no_environments() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(
            &cfg,
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        let result = cmd_promote(&cfg, "staging", false, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no environments"));
    }

    #[test]
    fn promote_env_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(
            &cfg,
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
environments:
  dev:
    description: "Development"
"#,
        )
        .unwrap();
        let result = cmd_promote(&cfg, "production", false, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn promote_no_promotion_config() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(
            &cfg,
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
environments:
  staging:
    description: "Staging"
"#,
        )
        .unwrap();
        let result = cmd_promote(&cfg, "staging", false, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no promotion config"));
    }
}
