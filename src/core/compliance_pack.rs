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
    /// The severity the rule declared, carried here so a caller need not
    /// search `pack.rules` to level a failure.
    pub severity: String,
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
///
/// # Why this returns a `Result`
///
/// It used to answer `Vec::new()` for a directory it could not read, which no
/// caller could tell from "there are no packs here". `chmod 000 policies/` made
/// every pack inside it vanish: zero packs, zero findings, gate PASSED, while
/// the same directory readable produced a blocking error-severity failure. The
/// `FJQ-CMP-000` arm in `quality_gate::checks::check_compliance` — written
/// under the comment "a gate that cannot evaluate its packs must not silently
/// pass" — was unreachable for exactly as long as this swallowed the failure.
///
/// # Why "does not exist" is `Ok(vec![])` and not an error
///
/// That is the one case that genuinely means "no packs are declared here"
/// rather than "there are packs here I cannot see". `forjar apply
/// --policy-check` defaults `--policy-dir` to `policies`, which most projects
/// do not have, so there is nothing to be blind to. Every other failure —
/// permission denied, a path that is a file, an I/O error partway through the
/// listing — leaves packs that may exist unexamined, and is reported.
pub fn list_packs(dir: &Path) -> Result<Vec<String>, String> {
    let Some(entries) = open_pack_dir(dir)? else {
        return Ok(Vec::new());
    };
    let mut packs: Vec<String> = Vec::new();
    for entry in entries {
        // A per-entry error is the same blindness one level down: the iterator
        // yields it INSTEAD of a name that may be a pack, so `.flatten()` here
        // dropped the very files it could not see.
        let entry = entry.map_err(|e| format!("list {}: {e}", dir.display()))?;
        packs.extend(pack_name(&entry.path()));
    }
    packs.sort();
    Ok(packs)
}

/// The directory's entries; `None` when it does not exist.
///
/// The three-way split is the whole point, and collapsing any two of them is
/// the bug: `Some` means "these are the packs", `None` means "no packs are
/// declared here", and `Err` means "there may be packs here and I cannot see
/// them". The last two used to be the same answer.
fn open_pack_dir(dir: &Path) -> Result<Option<std::fs::ReadDir>, String> {
    match std::fs::read_dir(dir) {
        Ok(entries) => Ok(Some(entries)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("list {}: {e}", dir.display())),
    }
}

/// The pack name a path contributes, or `None` if it is not a `*.yaml`/`*.yml`.
fn pack_name(path: &Path) -> Option<String> {
    path.extension().filter(|e| *e == "yaml" || *e == "yml")?;
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(String::from)
}

// Re-export built-in pack functions from compliance_pack_builtin.
pub use super::compliance_pack_builtin::{
    builtin_pack_names, generate_builtin_pack, generate_builtin_pack_yaml,
};

/// Evaluate a compliance pack against config resources. A simplified
/// evaluator over resource metadata; full evaluation is the policy engine.
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
        severity: rule.severity.clone(),
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
#[path = "tests_compliance_pack.rs"]
mod tests_compliance_pack;
