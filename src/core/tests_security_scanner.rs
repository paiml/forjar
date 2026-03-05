//! Tests for FJ-1390: Static IaC security scanner.

use super::security_scanner::*;
use super::types::*;
use indexmap::IndexMap;

fn minimal_config() -> ForjarConfig {
    ForjarConfig {
        version: "1".to_string(),
        name: "test".to_string(),
        description: None,
        params: std::collections::HashMap::new(),
        machines: IndexMap::new(),
        resources: IndexMap::new(),
        policy: Policy::default(),
        outputs: IndexMap::new(),
        policies: Vec::new(),
        data: IndexMap::new(),
        includes: Vec::new(),
        checks: IndexMap::new(),
        moved: Vec::new(),
    }
}

fn file_resource() -> Resource {
    Resource {
        resource_type: ResourceType::File,
        ..Default::default()
    }
}

fn docker_resource() -> Resource {
    Resource {
        resource_type: ResourceType::Docker,
        ..Default::default()
    }
}

// ── SS-1: Hard-coded secrets ─────────────────────────────────────

#[test]
fn ss1_detects_password_in_content() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.content = Some("db_password=hunter2".to_string());
    config.resources.insert("db-cfg".to_string(), r);
    let findings = scan(&config);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "SS-1");
    assert_eq!(findings[0].severity, Severity::Critical);
    assert_eq!(findings[0].category, "hard-coded-secret");
}

#[test]
fn ss1_detects_token() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.content = Some("auth_token=abc123".to_string());
    config.resources.insert("auth".to_string(), r);
    let findings = scan(&config);
    assert!(findings.iter().any(|f| f.rule_id == "SS-1"));
}

#[test]
fn ss1_case_insensitive() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.content = Some("API_KEY=secret".to_string());
    config.resources.insert("key".to_string(), r);
    let findings = scan(&config);
    assert!(findings.iter().any(|f| f.rule_id == "SS-1"));
}

#[test]
fn ss1_no_false_positive_on_clean_content() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.content = Some("listen_port=8080".to_string());
    config.resources.insert("safe".to_string(), r);
    let findings = scan(&config);
    assert!(!findings.iter().any(|f| f.rule_id == "SS-1"));
}

// ── SS-2: HTTP without TLS ──────────────────────────────────────

#[test]
fn ss2_detects_http_url_in_source() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.source = Some("http://example.com/file.tar.gz".to_string());
    config.resources.insert("dl".to_string(), r);
    let findings = scan(&config);
    assert!(findings.iter().any(|f| f.rule_id == "SS-2"));
    assert_eq!(
        findings
            .iter()
            .find(|f| f.rule_id == "SS-2")
            .unwrap()
            .severity,
        Severity::High
    );
}

#[test]
fn ss2_ignores_localhost() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.source = Some("http://localhost:8080/api".to_string());
    config.resources.insert("local".to_string(), r);
    let findings = scan(&config);
    assert!(!findings.iter().any(|f| f.rule_id == "SS-2"));
}

#[test]
fn ss2_passes_https() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.source = Some("https://example.com/file.tar.gz".to_string());
    config.resources.insert("safe".to_string(), r);
    let findings = scan(&config);
    assert!(!findings.iter().any(|f| f.rule_id == "SS-2"));
}

// ── SS-3: World-accessible permissions ──────────────────────────

#[test]
fn ss3_detects_world_readable() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.mode = Some("0644".to_string());
    config.resources.insert("pub".to_string(), r);
    let findings = scan(&config);
    assert!(findings.iter().any(|f| f.rule_id == "SS-3"));
}

#[test]
fn ss3_passes_private_mode() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.mode = Some("0600".to_string());
    config.resources.insert("priv".to_string(), r);
    let findings = scan(&config);
    assert!(!findings.iter().any(|f| f.rule_id == "SS-3"));
}

#[test]
fn ss3_detects_world_executable() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.mode = Some("0755".to_string());
    config.resources.insert("exec".to_string(), r);
    let findings = scan(&config);
    assert!(findings.iter().any(|f| f.rule_id == "SS-3"));
}

// ── SS-4: Missing integrity check ───────────────────────────────

#[test]
fn ss4_detects_external_file_without_check() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.source = Some("https://releases.example.com/v1.tar.gz".to_string());
    config.resources.insert("ext".to_string(), r);
    let findings = scan(&config);
    assert!(findings.iter().any(|f| f.rule_id == "SS-4"));
    assert_eq!(
        findings
            .iter()
            .find(|f| f.rule_id == "SS-4")
            .unwrap()
            .severity,
        Severity::Medium
    );
}

#[test]
fn ss4_passes_with_check() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.source = Some("https://releases.example.com/v1.tar.gz".to_string());
    config.resources.insert("ext".to_string(), r);
    config.checks.insert(
        "ext".to_string(),
        CheckBlock {
            machine: "web".to_string(),
            command: "sha256sum --check".to_string(),
            expect_exit: Some(0),
            description: None,
        },
    );
    let findings = scan(&config);
    assert!(!findings.iter().any(|f| f.rule_id == "SS-4"));
}

#[test]
fn ss4_ignores_non_file_resources() {
    let mut config = minimal_config();
    let r = Resource {
        resource_type: ResourceType::Service,
        source: Some("https://example.com/service".to_string()),
        ..Default::default()
    };
    config.resources.insert("svc".to_string(), r);
    let findings = scan(&config);
    assert!(!findings.iter().any(|f| f.rule_id == "SS-4"));
}

// ── SS-5: Privileged container ──────────────────────────────────

#[test]
fn ss5_detects_privileged_docker() {
    let mut config = minimal_config();
    let mut r = docker_resource();
    r.environment = vec!["privileged=true".to_string()];
    config.resources.insert("dkr".to_string(), r);
    let findings = scan(&config);
    assert!(findings
        .iter()
        .any(|f| f.rule_id == "SS-5" && f.severity == Severity::Critical));
}

#[test]
fn ss5_detects_root_owner() {
    let mut config = minimal_config();
    let mut r = docker_resource();
    r.owner = Some("root".to_string());
    config.resources.insert("dkr".to_string(), r);
    let findings = scan(&config);
    assert!(findings
        .iter()
        .any(|f| f.rule_id == "SS-5" && f.severity == Severity::Medium));
}

#[test]
fn ss5_ignores_non_docker() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.environment = vec!["privileged=true".to_string()];
    config.resources.insert("f".to_string(), r);
    let findings = scan(&config);
    assert!(!findings.iter().any(|f| f.rule_id == "SS-5"));
}

// ── SS-6: No resource limits ────────────────────────────────────

#[test]
fn ss6_detects_no_limits() {
    let mut config = minimal_config();
    let r = docker_resource();
    config.resources.insert("dkr".to_string(), r);
    let findings = scan(&config);
    assert!(findings.iter().any(|f| f.rule_id == "SS-6"));
    assert_eq!(
        findings
            .iter()
            .find(|f| f.rule_id == "SS-6")
            .unwrap()
            .severity,
        Severity::Low
    );
}

#[test]
fn ss6_passes_with_memory_limit() {
    let mut config = minimal_config();
    let mut r = docker_resource();
    r.environment = vec!["MEMORY_LIMIT=512m".to_string()];
    config.resources.insert("dkr".to_string(), r);
    let findings = scan(&config);
    assert!(!findings.iter().any(|f| f.rule_id == "SS-6"));
}

// ── SS-7: Weak cryptography ────────────────────────────────────

#[test]
fn ss7_detects_md5() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.content = Some("hash_algorithm: md5".to_string());
    config.resources.insert("cfg".to_string(), r);
    let findings = scan(&config);
    assert!(findings.iter().any(|f| f.rule_id == "SS-7"));
}

#[test]
fn ss7_detects_sha1() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.content = Some("checksum: sha1:abc123".to_string());
    config.resources.insert("cfg".to_string(), r);
    let findings = scan(&config);
    assert!(findings.iter().any(|f| f.rule_id == "SS-7"));
}

#[test]
fn ss7_passes_sha256() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.content = Some("checksum: sha256:abc123".to_string());
    config.resources.insert("cfg".to_string(), r);
    let findings = scan(&config);
    assert!(!findings.iter().any(|f| f.rule_id == "SS-7"));
}

// ── SS-8: Insecure protocol ────────────────────────────────────

#[test]
fn ss8_detects_telnet() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.content = Some("endpoint: telnet://192.168.1.1".to_string());
    config.resources.insert("cfg".to_string(), r);
    let findings = scan(&config);
    assert!(findings.iter().any(|f| f.rule_id == "SS-8"));
}

#[test]
fn ss8_detects_ftp() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.content = Some("mirror: ftp://mirror.example.com".to_string());
    config.resources.insert("cfg".to_string(), r);
    let findings = scan(&config);
    assert!(findings.iter().any(|f| f.rule_id == "SS-8"));
}

// ── SS-9: Unrestricted network binding ─────────────────────────

#[test]
fn ss9_detects_bind_all() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.content = Some("listen: 0.0.0.0:8080".to_string());
    config.resources.insert("cfg".to_string(), r);
    let findings = scan(&config);
    assert!(findings.iter().any(|f| f.rule_id == "SS-9"));
}

#[test]
fn ss9_passes_localhost() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.content = Some("listen: 127.0.0.1:8080".to_string());
    config.resources.insert("cfg".to_string(), r);
    let findings = scan(&config);
    assert!(!findings.iter().any(|f| f.rule_id == "SS-9"));
}

// ── SS-10: Sensitive data exposure ─────────────────────────────

#[test]
fn ss10_detects_ssn() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.content = Some("ssn=123-45-6789".to_string());
    config.resources.insert("pii".to_string(), r);
    let findings = scan(&config);
    assert!(findings.iter().any(|f| f.rule_id == "SS-10"));
    assert_eq!(
        findings
            .iter()
            .find(|f| f.rule_id == "SS-10")
            .unwrap()
            .severity,
        Severity::Critical
    );
}

#[test]
fn ss10_detects_credit_card() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.content = Some("credit_card=4111111111111111".to_string());
    config.resources.insert("pii".to_string(), r);
    let findings = scan(&config);
    assert!(findings.iter().any(|f| f.rule_id == "SS-10"));
}

// ── severity_counts ────────────────────────────────────────────

#[test]
fn severity_counts_correct() {
    let findings = vec![
        SecurityFinding {
            rule_id: "SS-1".to_string(),
            category: "hard-coded-secret",
            severity: Severity::Critical,
            resource_id: "a".to_string(),
            message: "test".to_string(),
        },
        SecurityFinding {
            rule_id: "SS-2".to_string(),
            category: "http-without-tls",
            severity: Severity::High,
            resource_id: "b".to_string(),
            message: "test".to_string(),
        },
        SecurityFinding {
            rule_id: "SS-4".to_string(),
            category: "missing-integrity-check",
            severity: Severity::Medium,
            resource_id: "c".to_string(),
            message: "test".to_string(),
        },
        SecurityFinding {
            rule_id: "SS-6".to_string(),
            category: "no-resource-limits",
            severity: Severity::Low,
            resource_id: "d".to_string(),
            message: "test".to_string(),
        },
    ];
    let (c, h, m, l) = severity_counts(&findings);
    assert_eq!((c, h, m, l), (1, 1, 1, 1));
}

// ── Empty config = no findings ─────────────────────────────────

#[test]
fn empty_config_no_findings() {
    let config = minimal_config();
    let findings = scan(&config);
    assert!(findings.is_empty());
}

// ── Multiple findings from one resource ────────────────────────

#[test]
fn multiple_findings_per_resource() {
    let mut config = minimal_config();
    let mut r = file_resource();
    r.content = Some("password=secret\nlisten: 0.0.0.0:80\nchecksum: md5:abc".to_string());
    r.source = Some("http://example.com/file".to_string());
    config.resources.insert("multi".to_string(), r);
    let findings = scan(&config);
    // SS-1 (password), SS-2 (http), SS-4 (no check), SS-7 (md5), SS-9 (0.0.0.0)
    assert!(findings.len() >= 4);
}
