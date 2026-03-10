//! FJ-3206: Built-in compliance pack generators (NIST, SOC2, HIPAA).
//!
//! CIS Ubuntu pack lives in `cis_ubuntu_pack.rs`. This module provides
//! the remaining built-in packs and the dispatch logic.

use super::compliance_pack::{ComplianceCheck, CompliancePack, ComplianceRule};

/// Supported built-in pack names.
pub fn builtin_pack_names() -> &'static [&'static str] {
    &["cis-ubuntu-22", "nist-800-53", "soc2", "hipaa"]
}

/// Generate YAML for a built-in compliance pack by name.
///
/// Returns `Err` if the pack name is not recognized.
pub fn generate_builtin_pack_yaml(name: &str) -> Result<String, String> {
    let pack = generate_builtin_pack(name)?;
    serde_yaml_ng::to_string(&pack).map_err(|e| format!("serialize pack: {e}"))
}

/// Generate a built-in compliance pack by name.
pub fn generate_builtin_pack(name: &str) -> Result<CompliancePack, String> {
    match name {
        "cis-ubuntu-22" => Ok(super::cis_ubuntu_pack::cis_ubuntu_2204_pack()),
        "nist-800-53" => Ok(nist_800_53_pack()),
        "soc2" => Ok(soc2_pack()),
        "hipaa" => Ok(hipaa_pack()),
        _ => Err(format!(
            "unknown pack '{name}'. Available: {}",
            builtin_pack_names().join(", ")
        )),
    }
}

/// NIST 800-53 compliance pack.
fn nist_800_53_pack() -> CompliancePack {
    CompliancePack {
        name: "nist-800-53".into(),
        version: "1.0.0".into(),
        framework: "NIST".into(),
        description: Some("NIST SP 800-53 Rev.5 Security Controls".into()),
        rules: nist_rules(),
    }
}

fn nist_rules() -> Vec<ComplianceRule> {
    vec![
        pack_rule(
            "NIST-AC-3",
            "Access enforcement",
            "error",
            &["NIST AC-3"],
            ComplianceCheck::Require {
                resource_type: "file".into(),
                field: "owner".into(),
            },
        ),
        pack_rule(
            "NIST-AC-6",
            "Least privilege",
            "error",
            &["NIST AC-6"],
            ComplianceCheck::Deny {
                resource_type: "service".into(),
                field: "owner".into(),
                pattern: "root".into(),
            },
        ),
        pack_rule(
            "NIST-CM-6",
            "Configuration settings",
            "warning",
            &["NIST CM-6"],
            ComplianceCheck::Require {
                resource_type: "file".into(),
                field: "mode".into(),
            },
        ),
        pack_rule(
            "NIST-SC-28",
            "Protection at rest",
            "error",
            &["NIST SC-28"],
            ComplianceCheck::Deny {
                resource_type: "file".into(),
                field: "mode".into(),
                pattern: "777".into(),
            },
        ),
        pack_rule(
            "NIST-SI-7",
            "Integrity verification",
            "warning",
            &["NIST SI-7"],
            ComplianceCheck::Require {
                resource_type: "package".into(),
                field: "version".into(),
            },
        ),
        pack_rule(
            "NIST-AU-2",
            "Audit events",
            "warning",
            &["NIST AU-2"],
            ComplianceCheck::Require {
                resource_type: "service".into(),
                field: "enabled".into(),
            },
        ),
        pack_rule(
            "NIST-IA-5",
            "Authenticator management",
            "error",
            &["NIST IA-5"],
            ComplianceCheck::Deny {
                resource_type: "file".into(),
                field: "content".into(),
                pattern: "PasswordAuthentication yes".into(),
            },
        ),
        pack_rule(
            "NIST-SC-7",
            "Boundary protection",
            "warning",
            &["NIST SC-7"],
            ComplianceCheck::Require {
                resource_type: "service".into(),
                field: "firewall".into(),
            },
        ),
    ]
}

/// SOC2 compliance pack.
fn soc2_pack() -> CompliancePack {
    CompliancePack {
        name: "soc2".into(),
        version: "1.0.0".into(),
        framework: "SOC2".into(),
        description: Some("SOC 2 Type II Trust Services Criteria".into()),
        rules: soc2_rules(),
    }
}

fn soc2_rules() -> Vec<ComplianceRule> {
    vec![
        pack_rule(
            "SOC2-CC6.1",
            "Logical access security",
            "error",
            &["SOC2 CC6.1"],
            ComplianceCheck::Require {
                resource_type: "file".into(),
                field: "owner".into(),
            },
        ),
        pack_rule(
            "SOC2-CC6.3",
            "Role-based access",
            "warning",
            &["SOC2 CC6.3"],
            ComplianceCheck::Require {
                resource_type: "file".into(),
                field: "group".into(),
            },
        ),
        pack_rule(
            "SOC2-CC7.2",
            "System monitoring",
            "warning",
            &["SOC2 CC7.2"],
            ComplianceCheck::Require {
                resource_type: "service".into(),
                field: "enabled".into(),
            },
        ),
        pack_rule(
            "SOC2-CC8.1",
            "Change management",
            "warning",
            &["SOC2 CC8.1"],
            ComplianceCheck::Require {
                resource_type: "package".into(),
                field: "version".into(),
            },
        ),
        pack_rule(
            "SOC2-CC6.6",
            "Boundary protection",
            "error",
            &["SOC2 CC6.6"],
            ComplianceCheck::Deny {
                resource_type: "file".into(),
                field: "mode".into(),
                pattern: "777".into(),
            },
        ),
        pack_rule(
            "SOC2-CC6.7",
            "Data integrity",
            "warning",
            &["SOC2 CC6.7"],
            ComplianceCheck::RequireTag {
                tag: "environment".into(),
            },
        ),
    ]
}

/// HIPAA compliance pack.
fn hipaa_pack() -> CompliancePack {
    CompliancePack {
        name: "hipaa".into(),
        version: "1.0.0".into(),
        framework: "HIPAA".into(),
        description: Some("HIPAA Security Rule (45 CFR 164.312)".into()),
        rules: hipaa_rules(),
    }
}

fn hipaa_rules() -> Vec<ComplianceRule> {
    vec![
        pack_rule(
            "HIPAA-164.312a",
            "Access control",
            "error",
            &["HIPAA 164.312(a)"],
            ComplianceCheck::Require {
                resource_type: "file".into(),
                field: "owner".into(),
            },
        ),
        pack_rule(
            "HIPAA-164.312b",
            "Audit controls",
            "error",
            &["HIPAA 164.312(b)"],
            ComplianceCheck::Require {
                resource_type: "service".into(),
                field: "enabled".into(),
            },
        ),
        pack_rule(
            "HIPAA-164.312c",
            "Integrity",
            "error",
            &["HIPAA 164.312(c)"],
            ComplianceCheck::Require {
                resource_type: "file".into(),
                field: "mode".into(),
            },
        ),
        pack_rule(
            "HIPAA-164.312d",
            "Authentication",
            "error",
            &["HIPAA 164.312(d)"],
            ComplianceCheck::Deny {
                resource_type: "file".into(),
                field: "content".into(),
                pattern: "PermitRootLogin yes".into(),
            },
        ),
        pack_rule(
            "HIPAA-164.312e",
            "Transmission security",
            "error",
            &["HIPAA 164.312(e)"],
            ComplianceCheck::Deny {
                resource_type: "file".into(),
                field: "mode".into(),
                pattern: "777".into(),
            },
        ),
        pack_rule(
            "HIPAA-164.308a",
            "Security management",
            "warning",
            &["HIPAA 164.308(a)"],
            ComplianceCheck::RequireTag {
                tag: "environment".into(),
            },
        ),
    ]
}

/// Helper to build a `ComplianceRule` for built-in packs.
fn pack_rule(
    id: &str,
    title: &str,
    severity: &str,
    controls: &[&str],
    check: ComplianceCheck,
) -> ComplianceRule {
    ComplianceRule {
        id: id.into(),
        title: title.into(),
        description: None,
        severity: severity.into(),
        controls: controls.iter().map(|s| s.to_string()).collect(),
        check,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::compliance_pack::evaluate_pack;
    use std::collections::HashMap;

    #[test]
    fn builtin_names_has_four() {
        assert_eq!(builtin_pack_names().len(), 4);
    }

    #[test]
    fn generate_cis_ubuntu() {
        let pack = generate_builtin_pack("cis-ubuntu-22").unwrap();
        assert_eq!(pack.name, "cis-ubuntu-22.04");
        assert!(!pack.rules.is_empty());
    }

    #[test]
    fn generate_nist() {
        let pack = generate_builtin_pack("nist-800-53").unwrap();
        assert_eq!(pack.name, "nist-800-53");
        assert_eq!(pack.framework, "NIST");
        assert_eq!(pack.rules.len(), 8);
    }

    #[test]
    fn generate_soc2() {
        let pack = generate_builtin_pack("soc2").unwrap();
        assert_eq!(pack.name, "soc2");
        assert_eq!(pack.framework, "SOC2");
        assert_eq!(pack.rules.len(), 6);
    }

    #[test]
    fn generate_hipaa() {
        let pack = generate_builtin_pack("hipaa").unwrap();
        assert_eq!(pack.name, "hipaa");
        assert_eq!(pack.framework, "HIPAA");
        assert_eq!(pack.rules.len(), 6);
    }

    #[test]
    fn generate_unknown_fails() {
        let result = generate_builtin_pack("unknown");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown pack"));
    }

    #[test]
    fn yaml_roundtrip_nist() {
        let yaml = generate_builtin_pack_yaml("nist-800-53").unwrap();
        let pack: CompliancePack = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(pack.name, "nist-800-53");
        assert_eq!(pack.rules.len(), 8);
    }

    #[test]
    fn yaml_roundtrip_soc2() {
        let yaml = generate_builtin_pack_yaml("soc2").unwrap();
        let pack: CompliancePack = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(pack.name, "soc2");
    }

    #[test]
    fn yaml_roundtrip_hipaa() {
        let yaml = generate_builtin_pack_yaml("hipaa").unwrap();
        let pack: CompliancePack = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(pack.name, "hipaa");
    }

    #[test]
    fn nist_rule_ids_unique() {
        let pack = nist_800_53_pack();
        let mut seen = std::collections::HashSet::new();
        for rule in &pack.rules {
            assert!(seen.insert(&rule.id), "duplicate: {}", rule.id);
        }
    }

    #[test]
    fn soc2_rule_ids_unique() {
        let pack = soc2_pack();
        let mut seen = std::collections::HashSet::new();
        for rule in &pack.rules {
            assert!(seen.insert(&rule.id), "duplicate: {}", rule.id);
        }
    }

    #[test]
    fn hipaa_rule_ids_unique() {
        let pack = hipaa_pack();
        let mut seen = std::collections::HashSet::new();
        for rule in &pack.rules {
            assert!(seen.insert(&rule.id), "duplicate: {}", rule.id);
        }
    }

    #[test]
    fn all_packs_have_descriptions() {
        for name in builtin_pack_names() {
            let pack = generate_builtin_pack(name).unwrap();
            assert!(pack.description.is_some(), "pack {name} has no description");
        }
    }

    #[test]
    fn all_rules_have_controls() {
        for name in builtin_pack_names() {
            let pack = generate_builtin_pack(name).unwrap();
            for rule in &pack.rules {
                assert!(
                    !rule.controls.is_empty(),
                    "pack {name} rule {} has no controls",
                    rule.id
                );
            }
        }
    }

    #[test]
    fn nist_evaluate_passing() {
        let pack = nist_800_53_pack();
        let mut resources = HashMap::new();
        let mut fields = HashMap::new();
        fields.insert("type".into(), "file".into());
        fields.insert("owner".into(), "root".into());
        fields.insert("mode".into(), "0644".into());
        resources.insert("cfg".into(), fields);

        let result = evaluate_pack(&pack, &resources);
        assert!(result.passed_count() > 0);
    }

    #[test]
    fn soc2_evaluate_passing() {
        let pack = soc2_pack();
        let mut resources = HashMap::new();
        let mut fields = HashMap::new();
        fields.insert("type".into(), "file".into());
        fields.insert("owner".into(), "app".into());
        fields.insert("group".into(), "app".into());
        fields.insert("tags".into(), "environment".into());
        fields.insert("mode".into(), "0644".into());
        resources.insert("cfg".into(), fields);

        let result = evaluate_pack(&pack, &resources);
        assert!(result.passed_count() > 0);
    }

    #[test]
    fn hipaa_evaluate_deny_triggered() {
        let pack = hipaa_pack();
        let mut resources = HashMap::new();
        let mut fields = HashMap::new();
        fields.insert("type".into(), "file".into());
        fields.insert("mode".into(), "777".into());
        resources.insert("bad-file".into(), fields);

        let result = evaluate_pack(&pack, &resources);
        let failed_ids: Vec<_> = result
            .results
            .iter()
            .filter(|r| !r.passed)
            .map(|r| r.rule_id.as_str())
            .collect();
        assert!(
            failed_ids.contains(&"HIPAA-164.312e"),
            "expected HIPAA-164.312e to fail for mode 777"
        );
    }
}
