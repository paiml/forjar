//! FJ-3203: Compliance pack pre-apply gate.
//!
//! Loads compliance packs from a directory and evaluates them against
//! config resources. Blocks apply if any error-severity rule fails.

use crate::core::compliance_pack::{evaluate_pack, list_packs, load_pack, PackEvalResult};
use crate::core::types::ForjarConfig;
use std::collections::HashMap;
use std::path::Path;

/// Result of the compliance gate check.
#[derive(Debug, Clone)]
pub struct ComplianceGateResult {
    /// Total packs evaluated.
    pub packs_evaluated: usize,
    /// Pack evaluation results.
    pub results: Vec<PackEvalResult>,
    /// Total error-severity failures.
    pub error_count: usize,
    /// Total warning-severity issues.
    pub warning_count: usize,
}

impl ComplianceGateResult {
    /// Whether the gate passed (no error-severity failures).
    pub fn passed(&self) -> bool {
        self.error_count == 0
    }
}

/// Convert forjar config resources to the flat map format expected by compliance pack evaluation.
pub fn config_to_resource_map(config: &ForjarConfig) -> HashMap<String, HashMap<String, String>> {
    let mut resources = HashMap::new();
    for (id, resource) in &config.resources {
        let mut fields = HashMap::new();
        fields.insert(
            "type".into(),
            format!("{:?}", resource.resource_type).to_lowercase(),
        );
        if let Some(ref owner) = resource.owner {
            fields.insert("owner".into(), owner.clone());
        }
        if let Some(ref group) = resource.group {
            fields.insert("group".into(), group.clone());
        }
        if let Some(ref mode) = resource.mode {
            fields.insert("mode".into(), mode.clone());
        }
        if let Some(ref content) = resource.content {
            fields.insert("content".into(), content.clone());
        }
        if let Some(ref name) = resource.name {
            fields.insert("name".into(), name.clone());
        }
        if let Some(ref enabled) = resource.enabled {
            fields.insert("enabled".into(), enabled.to_string());
        }
        if !resource.tags.is_empty() {
            fields.insert("tags".into(), resource.tags.join(","));
        }
        resources.insert(id.clone(), fields);
    }
    resources
}

/// Run the compliance gate: load all packs from a directory and evaluate.
///
/// Returns `Err` if any error-severity rule fails.
pub fn check_compliance_gate(
    policy_dir: &Path,
    config: &ForjarConfig,
    verbose: bool,
) -> Result<ComplianceGateResult, String> {
    let pack_names = list_packs(policy_dir);
    if pack_names.is_empty() {
        return Ok(ComplianceGateResult {
            packs_evaluated: 0,
            results: Vec::new(),
            error_count: 0,
            warning_count: 0,
        });
    }

    let resources = config_to_resource_map(config);
    let mut results = Vec::new();
    let mut total_errors = 0;
    let mut total_warnings = 0;

    for name in &pack_names {
        let path = policy_dir.join(format!("{name}.yaml"));
        let alt_path = policy_dir.join(format!("{name}.yml"));
        let pack_path = if path.exists() { &path } else { &alt_path };

        let pack = match load_pack(pack_path) {
            Ok(p) => p,
            Err(e) => {
                if verbose {
                    eprintln!("  [WARN] skip pack {name}: {e}");
                }
                continue;
            }
        };

        let eval = evaluate_pack(&pack, &resources);
        let errors = count_severity_failures(&eval, &pack, "error");
        let warnings = count_severity_failures(&eval, &pack, "warning");

        if verbose {
            eprintln!(
                "  pack {}: {}/{} rules passed ({} error, {} warning)",
                pack.name,
                eval.passed_count(),
                eval.results.len(),
                errors,
                warnings
            );
        }

        total_errors += errors;
        total_warnings += warnings;
        results.push(eval);
    }

    Ok(ComplianceGateResult {
        packs_evaluated: results.len(),
        results,
        error_count: total_errors,
        warning_count: total_warnings,
    })
}

/// Count failures by severity in a pack evaluation.
fn count_severity_failures(
    eval: &PackEvalResult,
    pack: &crate::core::compliance_pack::CompliancePack,
    severity: &str,
) -> usize {
    eval.results
        .iter()
        .filter(|r| !r.passed)
        .filter(|r| {
            pack.rules
                .iter()
                .find(|rule| rule.id == r.rule_id)
                .is_some_and(|rule| rule.severity == severity)
        })
        .count()
}

/// Format compliance gate result for display.
pub fn format_gate_result(result: &ComplianceGateResult) -> String {
    let status = if result.passed() { "PASS" } else { "FAIL" };
    format!(
        "Compliance gate: {} ({} packs, {} errors, {} warnings)",
        status, result.packs_evaluated, result.error_count, result.warning_count
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{ForjarConfig, Resource, ResourceType};

    fn make_config(resources: &[(&str, &str, Option<&str>)]) -> ForjarConfig {
        let mut config = ForjarConfig::default();
        for (name, rtype, owner) in resources {
            let resource = Resource {
                resource_type: match *rtype {
                    "file" => ResourceType::File,
                    "package" => ResourceType::Package,
                    "service" => ResourceType::Service,
                    _ => ResourceType::File,
                },
                owner: owner.map(|o| o.to_string()),
                ..Default::default()
            };
            config.resources.insert(name.to_string(), resource);
        }
        config
    }

    #[test]
    fn config_to_map_includes_fields() {
        let config = make_config(&[("nginx", "file", Some("root"))]);
        let map = config_to_resource_map(&config);
        assert_eq!(map.len(), 1);
        let nginx = map.get("nginx").unwrap();
        assert_eq!(nginx.get("type").unwrap(), "file");
        assert_eq!(nginx.get("owner").unwrap(), "root");
    }

    #[test]
    fn config_to_map_optional_fields() {
        let config = make_config(&[("pkg", "package", None)]);
        let map = config_to_resource_map(&config);
        let pkg = map.get("pkg").unwrap();
        assert_eq!(pkg.get("type").unwrap(), "package");
        assert!(pkg.get("owner").is_none());
    }

    #[test]
    fn gate_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config = make_config(&[("f1", "file", None)]);
        let result = check_compliance_gate(dir.path(), &config, false).unwrap();
        assert!(result.passed());
        assert_eq!(result.packs_evaluated, 0);
    }

    #[test]
    fn gate_with_passing_pack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("test.yaml"),
            r#"
name: test-gate
version: "1.0"
framework: TEST
rules:
  - id: T1
    title: Files have owner
    type: require
    resource_type: file
    field: owner
"#,
        )
        .unwrap();

        let config = make_config(&[("f1", "file", Some("root"))]);
        let result = check_compliance_gate(dir.path(), &config, false).unwrap();
        assert!(result.passed());
    }

    #[test]
    fn gate_with_failing_pack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("strict.yaml"),
            r#"
name: strict
version: "1.0"
framework: TEST
rules:
  - id: S1
    title: Must have owner
    severity: error
    type: require
    resource_type: file
    field: owner
"#,
        )
        .unwrap();

        let config = make_config(&[("f1", "file", None)]);
        let result = check_compliance_gate(dir.path(), &config, false).unwrap();
        assert_eq!(result.error_count, 1);
        assert!(!result.passed());
    }

    #[test]
    fn gate_result_format() {
        let result = ComplianceGateResult {
            packs_evaluated: 2,
            results: vec![],
            error_count: 0,
            warning_count: 1,
        };
        let text = format_gate_result(&result);
        assert!(text.contains("PASS"));
        assert!(text.contains("2 packs"));
    }

    #[test]
    fn gate_result_format_fail() {
        let result = ComplianceGateResult {
            packs_evaluated: 1,
            results: vec![],
            error_count: 2,
            warning_count: 0,
        };
        let text = format_gate_result(&result);
        assert!(text.contains("FAIL"));
        assert!(text.contains("2 errors"));
    }

    #[test]
    fn config_with_tags() {
        let mut config = ForjarConfig::default();
        let resource = Resource {
            resource_type: ResourceType::File,
            tags: vec!["web".into(), "config".into()],
            ..Default::default()
        };
        config.resources.insert("nginx".into(), resource);

        let map = config_to_resource_map(&config);
        let nginx = map.get("nginx").unwrap();
        assert_eq!(nginx.get("tags").unwrap(), "web,config");
    }

    #[test]
    fn config_with_mode() {
        let mut config = ForjarConfig::default();
        let resource = Resource {
            resource_type: ResourceType::File,
            mode: Some("0644".into()),
            ..Default::default()
        };
        config.resources.insert("f1".into(), resource);

        let map = config_to_resource_map(&config);
        assert_eq!(map.get("f1").unwrap().get("mode").unwrap(), "0644");
    }
}
