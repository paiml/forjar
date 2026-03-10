//! FJ-3205: Compliance pack format and loader.
//!
//! Compliance packs are content-addressed bundles of policy rules
//! that map to frameworks like CIS, STIG, and SOC2.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A compliance pack — a named collection of policy rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompliancePack {
    /// Pack name (e.g., "cis-ubuntu-22.04").
    pub name: String,
    /// Pack version.
    pub version: String,
    /// Framework this pack implements.
    pub framework: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,
    /// Policy rules in this pack.
    #[serde(default)]
    pub rules: Vec<ComplianceRule>,
}

/// A single compliance rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRule {
    /// Rule ID (e.g., "CIS-1.1.1").
    pub id: String,
    /// Rule title.
    pub title: String,
    /// Rule description.
    #[serde(default)]
    pub description: Option<String>,
    /// Severity: error, warning, info.
    #[serde(default = "default_severity")]
    pub severity: String,
    /// Framework control mapping (e.g., "CIS 1.1.1", "SOC2 CC6.1").
    #[serde(default)]
    pub controls: Vec<String>,
    /// Assertion type and value.
    #[serde(flatten)]
    pub check: ComplianceCheck,
}

fn default_severity() -> String {
    "warning".to_string()
}

/// The actual check a compliance rule performs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ComplianceCheck {
    /// Assert a resource field has an expected value.
    #[serde(rename = "assert")]
    Assert {
        resource_type: String,
        field: String,
        expected: String,
    },
    /// Deny a resource field value.
    #[serde(rename = "deny")]
    Deny {
        resource_type: String,
        field: String,
        pattern: String,
    },
    /// Require a field to be present.
    #[serde(rename = "require")]
    Require {
        resource_type: String,
        field: String,
    },
    /// Require a tag on resources.
    #[serde(rename = "require_tag")]
    RequireTag { tag: String },
    /// Custom script check.
    #[serde(rename = "script")]
    Script { script: String },
}

/// Result of evaluating a compliance pack against a config.
#[derive(Debug, Clone)]
pub struct PackEvalResult {
    /// Pack name.
    pub pack_name: String,
    /// Individual rule results.
    pub results: Vec<RuleEvalResult>,
}

impl PackEvalResult {
    /// Number of passing rules.
    pub fn passed_count(&self) -> usize {
        self.results.iter().filter(|r| r.passed).count()
    }

    /// Number of failing rules.
    pub fn failed_count(&self) -> usize {
        self.results.iter().filter(|r| !r.passed).count()
    }

    /// Pass rate as a percentage.
    pub fn pass_rate(&self) -> f64 {
        if self.results.is_empty() {
            return 100.0;
        }
        (self.passed_count() as f64 / self.results.len() as f64) * 100.0
    }
}

/// Result of evaluating a single compliance rule.
#[derive(Debug, Clone)]
pub struct RuleEvalResult {
    /// Rule ID.
    pub rule_id: String,
    /// Whether the rule passed.
    pub passed: bool,
    /// Human-readable message.
    pub message: String,
    /// Controls this rule maps to.
    pub controls: Vec<String>,
}

/// Load a compliance pack from a YAML file.
pub fn load_pack(path: &Path) -> Result<CompliancePack, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    parse_pack(&content)
}

/// Parse a compliance pack from YAML content.
pub fn parse_pack(yaml: &str) -> Result<CompliancePack, String> {
    serde_yaml_ng::from_str(yaml).map_err(|e| format!("parse pack: {e}"))
}

/// List available compliance packs in a directory.
pub fn list_packs(dir: &Path) -> Vec<String> {
    let mut packs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
                if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                    packs.push(name.to_string());
                }
            }
        }
    }
    packs.sort();
    packs
}

// Re-export built-in pack functions from compliance_pack_builtin.
pub use super::compliance_pack_builtin::{
    builtin_pack_names, generate_builtin_pack, generate_builtin_pack_yaml,
};

/// Evaluate a compliance pack against config resources.
///
/// This is a simplified evaluator that checks resource metadata.
/// Full evaluation integrates with the policy engine.
pub fn evaluate_pack(
    pack: &CompliancePack,
    resources: &HashMap<String, HashMap<String, String>>,
) -> PackEvalResult {
    let results = pack
        .rules
        .iter()
        .map(|rule| evaluate_rule(rule, resources))
        .collect();

    PackEvalResult {
        pack_name: pack.name.clone(),
        results,
    }
}

fn evaluate_rule(
    rule: &ComplianceRule,
    resources: &HashMap<String, HashMap<String, String>>,
) -> RuleEvalResult {
    let (passed, message) = match &rule.check {
        ComplianceCheck::Assert {
            resource_type,
            field,
            expected,
        } => check_assert(resources, resource_type, field, expected),
        ComplianceCheck::Deny {
            resource_type,
            field,
            pattern,
        } => check_deny(resources, resource_type, field, pattern),
        ComplianceCheck::Require {
            resource_type,
            field,
        } => check_require(resources, resource_type, field),
        ComplianceCheck::RequireTag { tag } => check_require_tag(resources, tag),
        ComplianceCheck::Script { script } => check_script(script),
    };

    RuleEvalResult {
        rule_id: rule.id.clone(),
        passed,
        message,
        controls: rule.controls.clone(),
    }
}

fn check_assert(
    resources: &HashMap<String, HashMap<String, String>>,
    resource_type: &str,
    field: &str,
    expected: &str,
) -> (bool, String) {
    for (name, fields) in resources {
        if let Some(rtype) = fields.get("type") {
            if rtype == resource_type {
                if let Some(value) = fields.get(field) {
                    if value != expected {
                        return (
                            false,
                            format!("{name}: {field}={value}, expected {expected}"),
                        );
                    }
                }
            }
        }
    }
    (true, format!("all {resource_type} resources pass"))
}

fn check_deny(
    resources: &HashMap<String, HashMap<String, String>>,
    resource_type: &str,
    field: &str,
    pattern: &str,
) -> (bool, String) {
    for (name, fields) in resources {
        if let Some(rtype) = fields.get("type") {
            if rtype == resource_type {
                if let Some(value) = fields.get(field) {
                    if value.contains(pattern) {
                        return (
                            false,
                            format!("{name}: {field} contains denied pattern '{pattern}'"),
                        );
                    }
                }
            }
        }
    }
    (
        true,
        format!("no {resource_type} resources match denied pattern"),
    )
}

fn check_require(
    resources: &HashMap<String, HashMap<String, String>>,
    resource_type: &str,
    field: &str,
) -> (bool, String) {
    for (name, fields) in resources {
        if let Some(rtype) = fields.get("type") {
            if rtype == resource_type && !fields.contains_key(field) {
                return (false, format!("{name}: missing required field '{field}'"));
            }
        }
    }
    (
        true,
        format!("all {resource_type} resources have '{field}'"),
    )
}

fn check_require_tag(
    resources: &HashMap<String, HashMap<String, String>>,
    tag: &str,
) -> (bool, String) {
    for (name, fields) in resources {
        if let Some(tags) = fields.get("tags") {
            if !tags.contains(tag) {
                return (false, format!("{name}: missing required tag '{tag}'"));
            }
        } else {
            return (false, format!("{name}: no tags defined"));
        }
    }
    (true, format!("all resources have tag '{tag}'"))
}

fn check_script(script: &str) -> (bool, String) {
    // FJ-3204: Validate script through bashrs before execution
    if let Err(e) = crate::core::purifier::validate_script(script) {
        return (false, format!("bashrs lint failed: {e}"));
    }
    // FJ-3307: Check for secret leakage patterns
    if let Err(e) = crate::core::script_secret_lint::validate_no_leaks(script) {
        return (false, format!("secret leak detected: {e}"));
    }

    match std::process::Command::new("sh")
        .args(["-c", script])
        .output()
    {
        Ok(output) if output.status.success() => (true, "script passed".into()),
        Ok(output) => (false, format!("script failed (exit {})", output.status)),
        Err(e) => (false, format!("script error: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_compliance_pack() {
        let yaml = r#"
name: test-pack
version: "1.0.0"
framework: CIS
description: "Test compliance pack"
rules:
  - id: "CIS-1.1"
    title: "Ensure root login disabled"
    severity: error
    controls: ["CIS 1.1.1"]
    type: assert
    resource_type: file
    field: owner
    expected: root
"#;
        let pack = parse_pack(yaml).unwrap();
        assert_eq!(pack.name, "test-pack");
        assert_eq!(pack.framework, "CIS");
        assert_eq!(pack.rules.len(), 1);
        assert_eq!(pack.rules[0].id, "CIS-1.1");
    }

    #[test]
    fn parse_pack_deny_rule() {
        let yaml = r#"
name: deny-test
version: "1.0.0"
framework: SOC2
rules:
  - id: "SOC2-1"
    title: "No world-writable files"
    type: deny
    resource_type: file
    field: mode
    pattern: "777"
"#;
        let pack = parse_pack(yaml).unwrap();
        assert_eq!(pack.rules[0].id, "SOC2-1");
    }

    #[test]
    fn evaluate_assert_passing() {
        let mut resources = HashMap::new();
        let mut fields = HashMap::new();
        fields.insert("type".into(), "file".into());
        fields.insert("owner".into(), "root".into());
        resources.insert("nginx-conf".into(), fields);

        let pack = CompliancePack {
            name: "test".into(),
            version: "1.0".into(),
            framework: "CIS".into(),
            description: None,
            rules: vec![ComplianceRule {
                id: "R1".into(),
                title: "Root owner".into(),
                description: None,
                severity: "error".into(),
                controls: vec!["CIS 1.1".into()],
                check: ComplianceCheck::Assert {
                    resource_type: "file".into(),
                    field: "owner".into(),
                    expected: "root".into(),
                },
            }],
        };

        let result = evaluate_pack(&pack, &resources);
        assert_eq!(result.passed_count(), 1);
        assert_eq!(result.failed_count(), 0);
        assert!((result.pass_rate() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn evaluate_assert_failing() {
        let mut resources = HashMap::new();
        let mut fields = HashMap::new();
        fields.insert("type".into(), "file".into());
        fields.insert("owner".into(), "nobody".into());
        resources.insert("bad-file".into(), fields);

        let (passed, _msg) = check_assert(&resources, "file", "owner", "root");
        assert!(!passed);
    }

    #[test]
    fn evaluate_deny() {
        let mut resources = HashMap::new();
        let mut fields = HashMap::new();
        fields.insert("type".into(), "file".into());
        fields.insert("mode".into(), "777".into());
        resources.insert("bad-file".into(), fields);

        let (passed, _msg) = check_deny(&resources, "file", "mode", "777");
        assert!(!passed);
    }

    #[test]
    fn evaluate_require() {
        let mut resources = HashMap::new();
        let mut fields = HashMap::new();
        fields.insert("type".into(), "file".into());
        resources.insert("no-owner".into(), fields);

        let (passed, _msg) = check_require(&resources, "file", "owner");
        assert!(!passed);
    }

    #[test]
    fn evaluate_require_tag() {
        let mut resources = HashMap::new();
        let mut fields = HashMap::new();
        fields.insert("tags".into(), "config,web".into());
        resources.insert("r1".into(), fields);

        let (passed, _) = check_require_tag(&resources, "config");
        assert!(passed);

        let (passed, _) = check_require_tag(&resources, "security");
        assert!(!passed);
    }

    #[test]
    fn list_packs_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let packs = list_packs(dir.path());
        assert!(packs.is_empty());
    }

    #[test]
    fn list_packs_with_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("cis.yaml"), "name: cis").unwrap();
        std::fs::write(dir.path().join("stig.yml"), "name: stig").unwrap();
        std::fs::write(dir.path().join("readme.txt"), "not a pack").unwrap();
        let packs = list_packs(dir.path());
        assert_eq!(packs, vec!["cis", "stig"]);
    }

    #[test]
    fn load_pack_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pack.yaml");
        std::fs::write(
            &path,
            r#"
name: file-pack
version: "1.0"
framework: STIG
rules: []
"#,
        )
        .unwrap();
        let pack = load_pack(&path).unwrap();
        assert_eq!(pack.name, "file-pack");
    }

    #[test]
    fn pack_eval_empty() {
        let result = PackEvalResult {
            pack_name: "empty".into(),
            results: vec![],
        };
        assert_eq!(result.passed_count(), 0);
        assert_eq!(result.failed_count(), 0);
        assert!((result.pass_rate() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn script_check_passes() {
        let (passed, _) = check_script("true");
        assert!(passed);
    }

    #[test]
    fn script_check_fails() {
        let (passed, _) = check_script("false");
        assert!(!passed);
    }
}
