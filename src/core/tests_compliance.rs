//! Tests: FJ-1387 compliance testing framework.

#[cfg(test)]
mod tests {
    use crate::core::compliance::*;
    use crate::core::types::*;
    use std::collections::HashMap;

    fn minimal_config() -> ForjarConfig {
        ForjarConfig {
            version: "1.0".to_string(),
            name: "compliance-test".to_string(),
            description: None,
            machines: indexmap::IndexMap::new(),
            resources: indexmap::IndexMap::new(),
            params: std::collections::HashMap::new(),
            outputs: indexmap::IndexMap::new(),
            policy: Policy::default(),
            policies: vec![],
            moved: vec![],
            secrets: Default::default(),
            includes: vec![],
            include_provenance: HashMap::new(),
            data: indexmap::IndexMap::new(),
            checks: indexmap::IndexMap::new(),
            environments: indexmap::IndexMap::new(),
            dist: None,
        }
    }

    fn file_resource(path: &str, owner: Option<&str>, mode: Option<&str>) -> Resource {
        Resource {
            resource_type: ResourceType::File,
            path: Some(path.to_string()),
            owner: owner.map(|s| s.to_string()),
            mode: mode.map(|s| s.to_string()),
            machine: MachineTarget::Single("localhost".to_string()),
            ..Resource::default()
        }
    }

    fn service_resource(name: &str, owner: Option<&str>) -> Resource {
        Resource {
            resource_type: ResourceType::Service,
            name: Some(name.to_string()),
            owner: owner.map(|s| s.to_string()),
            machine: MachineTarget::Single("localhost".to_string()),
            ..Resource::default()
        }
    }

    fn package_resource(pkgs: &[&str], version: Option<&str>) -> Resource {
        Resource {
            resource_type: ResourceType::Package,
            packages: pkgs.iter().map(|s| s.to_string()).collect(),
            version: version.map(|s| s.to_string()),
            machine: MachineTarget::Single("localhost".to_string()),
            ..Resource::default()
        }
    }

    // ── CIS Tests ────────────────────────────────────────────────

    #[test]
    fn test_cis_world_writable() {
        let mut config = minimal_config();
        config.resources.insert(
            "bad-file".to_string(),
            file_resource("/etc/config", None, Some("0777")),
        );
        let findings = evaluate_benchmark("cis", &config);
        assert!(findings.iter().any(|f| f.rule_id == "CIS-6.1.1"));
    }

    #[test]
    fn test_cis_safe_mode_passes() {
        let mut config = minimal_config();
        config.resources.insert(
            "safe-file".to_string(),
            file_resource("/etc/config", Some("root"), Some("0644")),
        );
        let findings = evaluate_benchmark("cis", &config);
        assert!(!findings.iter().any(|f| f.rule_id == "CIS-6.1.1"));
    }

    #[test]
    fn test_cis_root_tmp() {
        let mut config = minimal_config();
        config.resources.insert(
            "tmp-file".to_string(),
            file_resource("/tmp/danger", Some("root"), Some("0644")),
        );
        let findings = evaluate_benchmark("cis", &config);
        assert!(findings.iter().any(|f| f.rule_id == "CIS-1.1.5"));
    }

    #[test]
    fn test_cis_service_no_restart() {
        let mut config = minimal_config();
        config
            .resources
            .insert("svc".to_string(), service_resource("nginx", None));
        let findings = evaluate_benchmark("cis", &config);
        assert!(findings.iter().any(|f| f.rule_id == "CIS-5.2.1"));
    }

    #[test]
    fn test_cis_package_no_version() {
        let mut config = minimal_config();
        config
            .resources
            .insert("pkg".to_string(), package_resource(&["nginx"], None));
        let findings = evaluate_benchmark("cis", &config);
        assert!(findings.iter().any(|f| f.rule_id == "CIS-6.2.1"));
    }

    #[test]
    fn test_cis_package_with_version_passes() {
        let mut config = minimal_config();
        config.resources.insert(
            "pkg".to_string(),
            package_resource(&["nginx"], Some("1.24")),
        );
        let findings = evaluate_benchmark("cis", &config);
        assert!(!findings.iter().any(|f| f.rule_id == "CIS-6.2.1"));
    }

    // ── NIST 800-53 Tests ────────────────────────────────────────

    #[test]
    fn test_nist_ac3_missing_owner() {
        let mut config = minimal_config();
        config.resources.insert(
            "f1".to_string(),
            file_resource("/etc/app.conf", None, Some("0644")),
        );
        let findings = evaluate_benchmark("nist-800-53", &config);
        assert!(findings.iter().any(|f| f.rule_id == "NIST-AC-3.1"));
    }

    #[test]
    fn test_nist_ac3_missing_mode() {
        let mut config = minimal_config();
        config.resources.insert(
            "f1".to_string(),
            file_resource("/etc/app.conf", Some("root"), None),
        );
        let findings = evaluate_benchmark("nist-800-53", &config);
        assert!(findings.iter().any(|f| f.rule_id == "NIST-AC-3.2"));
    }

    #[test]
    fn test_nist_ac6_root_service() {
        let mut config = minimal_config();
        config
            .resources
            .insert("svc".to_string(), service_resource("myapp", Some("root")));
        let findings = evaluate_benchmark("nist-800-53", &config);
        assert!(findings.iter().any(|f| f.rule_id == "NIST-AC-6"));
    }

    #[test]
    fn test_nist_ac6_nonroot_passes() {
        let mut config = minimal_config();
        config.resources.insert(
            "svc".to_string(),
            service_resource("myapp", Some("appuser")),
        );
        let findings = evaluate_benchmark("nist-800-53", &config);
        assert!(!findings.iter().any(|f| f.rule_id == "NIST-AC-6"));
    }

    #[test]
    fn test_nist_sc28_sensitive_path_no_mode() {
        let mut config = minimal_config();
        config.resources.insert(
            "ssh-config".to_string(),
            file_resource("/etc/ssh/sshd_config", Some("root"), None),
        );
        let findings = evaluate_benchmark("nist", &config);
        assert!(findings.iter().any(|f| f.rule_id == "NIST-SC-28"));
    }

    #[test]
    fn test_nist_si7_external_source_no_check() {
        let mut config = minimal_config();
        let mut r = file_resource("/opt/app/bin", Some("root"), Some("0755"));
        r.source = Some("https://example.com/app.tar.gz".to_string());
        config.resources.insert("app-bin".to_string(), r);
        let findings = evaluate_benchmark("nist-800-53", &config);
        assert!(findings.iter().any(|f| f.rule_id == "NIST-SI-7"));
    }

    #[test]
    fn test_nist_si7_with_check_passes() {
        let mut config = minimal_config();
        let mut r = file_resource("/opt/app/bin", Some("root"), Some("0755"));
        r.source = Some("https://example.com/app.tar.gz".to_string());
        config.resources.insert("app-bin".to_string(), r);
        config.checks.insert(
            "app-bin".to_string(),
            CheckBlock {
                machine: "localhost".to_string(),
                command: "sha256sum /opt/app/bin".to_string(),
                expect_exit: None,
                description: None,
            },
        );
        let findings = evaluate_benchmark("nist-800-53", &config);
        assert!(!findings.iter().any(|f| f.rule_id == "NIST-SI-7"));
    }

    // ── SOC2 Tests ───────────────────────────────────────────────

    #[test]
    fn test_soc2_file_no_owner() {
        let mut config = minimal_config();
        config.resources.insert(
            "f1".to_string(),
            file_resource("/etc/app.conf", None, Some("0644")),
        );
        let findings = evaluate_benchmark("soc2", &config);
        assert!(findings.iter().any(|f| f.rule_id == "SOC2-CC6.1"));
    }

    #[test]
    fn test_soc2_service_no_monitoring() {
        let mut config = minimal_config();
        config
            .resources
            .insert("svc".to_string(), service_resource("myapp", None));
        let findings = evaluate_benchmark("soc2", &config);
        assert!(findings.iter().any(|f| f.rule_id == "SOC2-CC7.2"));
    }

    // ── HIPAA Tests ──────────────────────────────────────────────

    #[test]
    fn test_hipaa_other_permissions() {
        let mut config = minimal_config();
        config.resources.insert(
            "f1".to_string(),
            file_resource("/data/records.db", Some("app"), Some("0644")),
        );
        let findings = evaluate_benchmark("hipaa", &config);
        assert!(findings.iter().any(|f| f.rule_id == "HIPAA-164.312a"));
    }

    #[test]
    fn test_hipaa_restrictive_mode_passes() {
        let mut config = minimal_config();
        config.resources.insert(
            "f1".to_string(),
            file_resource("/data/records.db", Some("app"), Some("0640")),
        );
        let findings = evaluate_benchmark("hipaa", &config);
        assert!(!findings.iter().any(|f| f.rule_id == "HIPAA-164.312a"));
    }

    #[test]
    fn test_hipaa_unencrypted_port() {
        let mut config = minimal_config();
        config.resources.insert(
            "net".to_string(),
            Resource {
                resource_type: ResourceType::Network,
                port: Some("80".to_string()),
                machine: MachineTarget::Single("localhost".to_string()),
                ..Resource::default()
            },
        );
        let findings = evaluate_benchmark("hipaa", &config);
        assert!(findings.iter().any(|f| f.rule_id == "HIPAA-164.312e"));
    }

    // ── General Tests ────────────────────────────────────────────

    #[test]
    fn test_unknown_benchmark() {
        let config = minimal_config();
        let findings = evaluate_benchmark("unknown", &config);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("unknown benchmark"));
    }

    #[test]
    fn test_supported_benchmarks() {
        let benchmarks = supported_benchmarks();
        assert!(benchmarks.contains(&"cis"));
        assert!(benchmarks.contains(&"nist-800-53"));
        assert!(benchmarks.contains(&"soc2"));
        assert!(benchmarks.contains(&"hipaa"));
    }

    #[test]
    fn test_count_by_severity() {
        let findings = vec![
            ComplianceFinding {
                rule_id: "a".to_string(),
                benchmark: "cis".to_string(),
                severity: FindingSeverity::Critical,
                resource_id: "r1".to_string(),
                message: String::new(),
            },
            ComplianceFinding {
                rule_id: "b".to_string(),
                benchmark: "cis".to_string(),
                severity: FindingSeverity::High,
                resource_id: "r2".to_string(),
                message: String::new(),
            },
            ComplianceFinding {
                rule_id: "c".to_string(),
                benchmark: "cis".to_string(),
                severity: FindingSeverity::Medium,
                resource_id: "r3".to_string(),
                message: String::new(),
            },
        ];
        let (c, h, m, l) = count_by_severity(&findings);
        assert_eq!((c, h, m, l), (1, 1, 1, 0));
    }

    #[test]
    fn test_empty_config_all_benchmarks() {
        let config = minimal_config();
        for benchmark in supported_benchmarks() {
            let findings = evaluate_benchmark(benchmark, &config);
            assert!(
                findings.is_empty(),
                "expected no findings for empty config with {benchmark}"
            );
        }
    }
}
