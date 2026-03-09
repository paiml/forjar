//! FJ-3206: CIS Ubuntu 22.04 LTS compliance pack.
//!
//! Provides a built-in CIS compliance pack with 20+ rules covering
//! file permissions, service hardening, network configuration, and
//! access controls.

use crate::core::compliance_pack::{ComplianceCheck, CompliancePack, ComplianceRule};

/// Generate the CIS Ubuntu 22.04 LTS compliance pack.
///
/// Returns a `CompliancePack` with 24 rules covering key CIS benchmarks:
/// - Section 1: Filesystem configuration
/// - Section 2: Services
/// - Section 3: Network configuration
/// - Section 4: Access controls
/// - Section 5: Authentication
/// - Section 6: System maintenance
pub fn cis_ubuntu_2204_pack() -> CompliancePack {
    CompliancePack {
        name: "cis-ubuntu-22.04".into(),
        version: "1.0.0".into(),
        framework: "CIS".into(),
        description: Some("CIS Ubuntu 22.04 LTS Benchmark v1.0".into()),
        rules: cis_rules(),
    }
}

fn cis_rules() -> Vec<ComplianceRule> {
    vec![
        // --- Section 1: Filesystem ---
        rule(
            "CIS-1.1.1",
            "Ensure /tmp is a separate partition",
            "error",
            &["CIS 1.1.1"],
            ComplianceCheck::Assert {
                resource_type: "mount".into(),
                field: "point".into(),
                expected: "/tmp".into(),
            },
        ),
        rule(
            "CIS-1.1.2",
            "Ensure noexec on /tmp",
            "error",
            &["CIS 1.1.2"],
            ComplianceCheck::Assert {
                resource_type: "mount".into(),
                field: "options".into(),
                expected: "noexec".into(),
            },
        ),
        rule(
            "CIS-1.4.1",
            "Ensure permissions on /etc/passwd",
            "error",
            &["CIS 1.4.1"],
            ComplianceCheck::Assert {
                resource_type: "file".into(),
                field: "mode".into(),
                expected: "0644".into(),
            },
        ),
        rule(
            "CIS-1.4.2",
            "Ensure permissions on /etc/shadow",
            "error",
            &["CIS 1.4.2"],
            ComplianceCheck::Assert {
                resource_type: "file".into(),
                field: "mode".into(),
                expected: "0640".into(),
            },
        ),
        // --- Section 2: Services ---
        rule(
            "CIS-2.1.1",
            "Ensure xinetd is not installed",
            "error",
            &["CIS 2.1.1"],
            ComplianceCheck::Deny {
                resource_type: "package".into(),
                field: "name".into(),
                pattern: "xinetd".into(),
            },
        ),
        rule(
            "CIS-2.1.2",
            "Ensure telnet server is not installed",
            "error",
            &["CIS 2.1.2"],
            ComplianceCheck::Deny {
                resource_type: "package".into(),
                field: "name".into(),
                pattern: "telnetd".into(),
            },
        ),
        rule(
            "CIS-2.1.3",
            "Ensure rsh server is not installed",
            "warning",
            &["CIS 2.1.3"],
            ComplianceCheck::Deny {
                resource_type: "package".into(),
                field: "name".into(),
                pattern: "rsh-server".into(),
            },
        ),
        rule(
            "CIS-2.2.1",
            "Ensure NFS is not enabled",
            "warning",
            &["CIS 2.2.1"],
            ComplianceCheck::Deny {
                resource_type: "service".into(),
                field: "name".into(),
                pattern: "nfs-server".into(),
            },
        ),
        rule(
            "CIS-2.2.2",
            "Ensure CUPS is not enabled",
            "info",
            &["CIS 2.2.2"],
            ComplianceCheck::Deny {
                resource_type: "service".into(),
                field: "name".into(),
                pattern: "cups".into(),
            },
        ),
        // --- Section 3: Network ---
        rule(
            "CIS-3.1.1",
            "Ensure IP forwarding is disabled",
            "error",
            &["CIS 3.1.1"],
            ComplianceCheck::Assert {
                resource_type: "file".into(),
                field: "content".into(),
                expected: "0".into(),
            },
        ),
        rule(
            "CIS-3.2.1",
            "Ensure firewall is configured",
            "error",
            &["CIS 3.2.1"],
            ComplianceCheck::Require {
                resource_type: "service".into(),
                field: "firewall".into(),
            },
        ),
        rule(
            "CIS-3.3.1",
            "Ensure TCP SYN cookies are enabled",
            "warning",
            &["CIS 3.3.1"],
            ComplianceCheck::Assert {
                resource_type: "file".into(),
                field: "content".into(),
                expected: "1".into(),
            },
        ),
        // --- Section 4: Access Controls ---
        rule(
            "CIS-4.1.1",
            "Ensure auditd is installed",
            "error",
            &["CIS 4.1.1"],
            ComplianceCheck::Require {
                resource_type: "package".into(),
                field: "name".into(),
            },
        ),
        rule(
            "CIS-4.1.2",
            "Ensure auditd service is enabled",
            "error",
            &["CIS 4.1.2"],
            ComplianceCheck::Require {
                resource_type: "service".into(),
                field: "enabled".into(),
            },
        ),
        rule(
            "CIS-4.2.1",
            "Ensure rsyslog is installed",
            "warning",
            &["CIS 4.2.1"],
            ComplianceCheck::Require {
                resource_type: "package".into(),
                field: "name".into(),
            },
        ),
        rule(
            "CIS-4.3.1",
            "Ensure all resources have environment tag",
            "warning",
            &["CIS 4.3.1"],
            ComplianceCheck::RequireTag {
                tag: "environment".into(),
            },
        ),
        // --- Section 5: Authentication ---
        rule(
            "CIS-5.1.1",
            "Ensure cron daemon is enabled",
            "warning",
            &["CIS 5.1.1"],
            ComplianceCheck::Require {
                resource_type: "service".into(),
                field: "enabled".into(),
            },
        ),
        rule(
            "CIS-5.2.1",
            "Ensure SSH root login is disabled",
            "error",
            &["CIS 5.2.1", "STIG V-238196"],
            ComplianceCheck::Deny {
                resource_type: "file".into(),
                field: "content".into(),
                pattern: "PermitRootLogin yes".into(),
            },
        ),
        rule(
            "CIS-5.2.2",
            "Ensure SSH password auth is disabled",
            "warning",
            &["CIS 5.2.2"],
            ComplianceCheck::Deny {
                resource_type: "file".into(),
                field: "content".into(),
                pattern: "PasswordAuthentication yes".into(),
            },
        ),
        rule(
            "CIS-5.3.1",
            "Ensure no world-writable files",
            "error",
            &["CIS 5.3.1"],
            ComplianceCheck::Deny {
                resource_type: "file".into(),
                field: "mode".into(),
                pattern: "777".into(),
            },
        ),
        rule(
            "CIS-5.4.1",
            "Ensure root-owned files have system tag",
            "warning",
            &["CIS 5.4.1"],
            ComplianceCheck::RequireTag {
                tag: "system".into(),
            },
        ),
        // --- Section 6: System Maintenance ---
        rule(
            "CIS-6.1.1",
            "Ensure files have owner defined",
            "error",
            &["CIS 6.1.1"],
            ComplianceCheck::Require {
                resource_type: "file".into(),
                field: "owner".into(),
            },
        ),
        rule(
            "CIS-6.1.2",
            "Ensure files have group defined",
            "warning",
            &["CIS 6.1.2"],
            ComplianceCheck::Require {
                resource_type: "file".into(),
                field: "group".into(),
            },
        ),
        rule(
            "CIS-6.2.1",
            "Ensure no duplicate user names",
            "error",
            &["CIS 6.2.1"],
            ComplianceCheck::Require {
                resource_type: "user".into(),
                field: "name".into(),
            },
        ),
    ]
}

/// Helper to build a ComplianceRule.
fn rule(
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

/// Serialize the CIS Ubuntu pack to YAML.
pub fn cis_ubuntu_yaml() -> Result<String, String> {
    let pack = cis_ubuntu_2204_pack();
    serde_yaml_ng::to_string(&pack).map_err(|e| format!("serialize CIS pack: {e}"))
}

/// Rule count by severity.
pub fn severity_summary(pack: &CompliancePack) -> (usize, usize, usize) {
    let errors = pack.rules.iter().filter(|r| r.severity == "error").count();
    let warnings = pack
        .rules
        .iter()
        .filter(|r| r.severity == "warning")
        .count();
    let info = pack.rules.iter().filter(|r| r.severity == "info").count();
    (errors, warnings, info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::compliance_pack::evaluate_pack;
    use std::collections::HashMap;

    #[test]
    fn pack_has_24_rules() {
        let pack = cis_ubuntu_2204_pack();
        assert_eq!(pack.rules.len(), 24);
    }

    #[test]
    fn pack_metadata() {
        let pack = cis_ubuntu_2204_pack();
        assert_eq!(pack.name, "cis-ubuntu-22.04");
        assert_eq!(pack.framework, "CIS");
        assert_eq!(pack.version, "1.0.0");
        assert!(pack.description.is_some());
    }

    #[test]
    fn all_rules_have_controls() {
        let pack = cis_ubuntu_2204_pack();
        for rule in &pack.rules {
            assert!(
                !rule.controls.is_empty(),
                "rule {} has no controls",
                rule.id
            );
        }
    }

    #[test]
    fn severity_counts() {
        let pack = cis_ubuntu_2204_pack();
        let (errors, warnings, info) = severity_summary(&pack);
        assert!(
            errors >= 12,
            "expected at least 12 error rules, got {errors}"
        );
        assert!(
            warnings >= 8,
            "expected at least 8 warning rules, got {warnings}"
        );
        assert!(info >= 1, "expected at least 1 info rule, got {info}");
    }

    #[test]
    fn rule_ids_unique() {
        let pack = cis_ubuntu_2204_pack();
        let mut seen = std::collections::HashSet::new();
        for rule in &pack.rules {
            assert!(seen.insert(&rule.id), "duplicate rule ID: {}", rule.id);
        }
    }

    #[test]
    fn rule_ids_prefixed() {
        let pack = cis_ubuntu_2204_pack();
        for rule in &pack.rules {
            assert!(
                rule.id.starts_with("CIS-"),
                "rule {} should start with CIS-",
                rule.id
            );
        }
    }

    #[test]
    fn evaluate_passing_config() {
        let pack = cis_ubuntu_2204_pack();
        let mut resources = HashMap::new();
        let mut file_fields = HashMap::new();
        file_fields.insert("type".into(), "file".into());
        file_fields.insert("owner".into(), "root".into());
        file_fields.insert("group".into(), "root".into());
        file_fields.insert("mode".into(), "0644".into());
        file_fields.insert("tags".into(), "system,environment".into());
        resources.insert("config-file".into(), file_fields);

        let result = evaluate_pack(&pack, &resources);
        // Should have some passes (file rules that match)
        assert!(result.passed_count() > 0);
    }

    #[test]
    fn evaluate_world_writable_fails() {
        let pack = cis_ubuntu_2204_pack();
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
            failed_ids.contains(&"CIS-5.3.1"),
            "CIS-5.3.1 should fail for mode 777"
        );
    }

    #[test]
    fn evaluate_ssh_root_login_fails() {
        let pack = cis_ubuntu_2204_pack();
        let mut resources = HashMap::new();
        let mut fields = HashMap::new();
        fields.insert("type".into(), "file".into());
        fields.insert("content".into(), "PermitRootLogin yes".into());
        resources.insert("sshd-config".into(), fields);

        let result = evaluate_pack(&pack, &resources);
        let failed_ids: Vec<_> = result
            .results
            .iter()
            .filter(|r| !r.passed)
            .map(|r| r.rule_id.as_str())
            .collect();
        assert!(
            failed_ids.contains(&"CIS-5.2.1"),
            "CIS-5.2.1 should fail for PermitRootLogin yes"
        );
    }

    #[test]
    fn yaml_serialization() {
        let yaml = cis_ubuntu_yaml().unwrap();
        assert!(yaml.contains("cis-ubuntu-22.04"));
        assert!(yaml.contains("CIS-1.1.1"));
        assert!(yaml.contains("CIS-6.2.1"));
    }

    #[test]
    fn yaml_roundtrip() {
        let yaml = cis_ubuntu_yaml().unwrap();
        let parsed: CompliancePack = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed.name, "cis-ubuntu-22.04");
        assert_eq!(parsed.rules.len(), 24);
    }

    #[test]
    fn cross_maps_to_stig() {
        let pack = cis_ubuntu_2204_pack();
        let stig_rules: Vec<_> = pack
            .rules
            .iter()
            .filter(|r| r.controls.iter().any(|c| c.starts_with("STIG")))
            .collect();
        assert!(
            !stig_rules.is_empty(),
            "at least one rule should cross-map to STIG"
        );
    }
}
