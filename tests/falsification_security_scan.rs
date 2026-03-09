//! FJ-1390: Static IaC security scanner — 10 detection rules.
//!
//! Popperian rejection criteria for:
//! - SS-1: hardcoded secrets (password, token, api_key, secret, AWS_SECRET)
//! - SS-2: HTTP without TLS (http:// except localhost)
//! - SS-3: world-accessible permissions (mode last digit >= 4)
//! - SS-4: missing integrity check (external source, no check block)
//! - SS-5: privileged container (env privileged=true, root owner)
//! - SS-6: no resource limits (Docker without MEMORY/CPU limits)
//! - SS-7: weak crypto (md5, sha1, des, rc4, sslv3, tlsv1.0)
//! - SS-8: insecure protocol (telnet://, ftp://, rsh://)
//! - SS-9: unrestricted network (0.0.0.0, bind_address: *)
//! - SS-10: sensitive data (ssn=, credit_card=)
//!
//! Usage: cargo test --test falsification_security_scan

use forjar::core::security_scanner::{self, Severity};
use forjar::core::types::*;
use indexmap::IndexMap;

fn make_config(resources: Vec<(&str, Resource)>) -> ForjarConfig {
    let mut res = IndexMap::new();
    for (id, r) in resources {
        res.insert(id.to_string(), r);
    }
    ForjarConfig {
        version: "1.0".into(),
        name: "test".into(),
        resources: res,
        description: None,
        params: Default::default(),
        machines: Default::default(),
        policy: Default::default(),
        outputs: Default::default(),
        policies: Default::default(),
        data: Default::default(),
        includes: Default::default(),
        include_provenance: Default::default(),
        checks: Default::default(),
        moved: Default::default(),
        secrets: Default::default(),
        environments: Default::default(),
    }
}

fn make_resource(rtype: ResourceType) -> Resource {
    Resource {
        resource_type: rtype,
        ..Default::default()
    }
}

fn scan_resource(id: &str, r: Resource) -> Vec<security_scanner::SecurityFinding> {
    let config = make_config(vec![(id, r)]);
    security_scanner::scan(&config)
}

#[test]
fn ss1_hardcoded_secret() {
    let mut r = make_resource(ResourceType::File);
    r.content = Some("password=s3cret".into());
    let findings = scan_resource("cfg", r);
    assert!(findings.iter().any(|f| f.rule_id == "SS-1"));
    assert!(findings.iter().any(|f| f.severity == Severity::Critical));
}

#[test]
fn ss1_no_secret_clean() {
    let mut r = make_resource(ResourceType::File);
    r.content = Some("log_level=info".into());
    let findings = scan_resource("cfg", r);
    assert!(!findings.iter().any(|f| f.rule_id == "SS-1"));
}

#[test]
fn ss2_http_without_tls() {
    let mut r = make_resource(ResourceType::File);
    r.source = Some("http://example.com/file".into());
    let findings = scan_resource("dl", r);
    assert!(findings.iter().any(|f| f.rule_id == "SS-2"));
}

#[test]
fn ss2_localhost_exempt() {
    let mut r = make_resource(ResourceType::File);
    r.source = Some("http://localhost:8080/api".into());
    let findings = scan_resource("dl", r);
    assert!(!findings.iter().any(|f| f.rule_id == "SS-2"));
}

#[test]
fn ss3_world_accessible() {
    let mut r = make_resource(ResourceType::File);
    r.mode = Some("0644".into());
    let findings = scan_resource("f", r);
    assert!(findings.iter().any(|f| f.rule_id == "SS-3"));
}

#[test]
fn ss3_restricted_mode_clean() {
    let mut r = make_resource(ResourceType::File);
    r.mode = Some("0600".into());
    let findings = scan_resource("f", r);
    assert!(!findings.iter().any(|f| f.rule_id == "SS-3"));
}

#[test]
fn ss4_missing_integrity_check() {
    let mut r = make_resource(ResourceType::File);
    r.source = Some("https://example.com/bin".into());
    let findings = scan_resource("dl", r);
    assert!(findings.iter().any(|f| f.rule_id == "SS-4"));
}

#[test]
fn ss5_privileged_container() {
    let mut r = make_resource(ResourceType::Docker);
    r.environment = vec!["privileged=true".into()];
    let findings = scan_resource("app", r);
    assert!(findings.iter().any(|f| f.rule_id == "SS-5"));
}

#[test]
fn ss5_root_container() {
    let mut r = make_resource(ResourceType::Docker);
    r.owner = Some("root".into());
    let findings = scan_resource("app", r);
    assert!(findings
        .iter()
        .any(|f| f.rule_id == "SS-5" && f.severity == Severity::Medium));
}

#[test]
fn ss6_no_resource_limits() {
    let r = make_resource(ResourceType::Docker);
    let findings = scan_resource("app", r);
    assert!(findings.iter().any(|f| f.rule_id == "SS-6"));
}

#[test]
fn ss6_with_limits_clean() {
    let mut r = make_resource(ResourceType::Docker);
    r.environment = vec!["MEMORY_LIMIT=512m".into()];
    let findings = scan_resource("app", r);
    assert!(!findings.iter().any(|f| f.rule_id == "SS-6"));
}

#[test]
fn ss7_weak_crypto() {
    let mut r = make_resource(ResourceType::File);
    r.content = Some("hash_algorithm: md5".into());
    let findings = scan_resource("cfg", r);
    assert!(findings.iter().any(|f| f.rule_id == "SS-7"));
}

#[test]
fn ss8_insecure_protocol() {
    let mut r = make_resource(ResourceType::File);
    r.content = Some("url: ftp://files.example.com".into());
    let findings = scan_resource("cfg", r);
    assert!(findings.iter().any(|f| f.rule_id == "SS-8"));
}

#[test]
fn ss9_unrestricted_network() {
    let mut r = make_resource(ResourceType::File);
    r.content = Some("bind: 0.0.0.0:8080".into());
    let findings = scan_resource("cfg", r);
    assert!(findings.iter().any(|f| f.rule_id == "SS-9"));
}

#[test]
fn ss10_sensitive_data() {
    let mut r = make_resource(ResourceType::File);
    r.content = Some("ssn=123-45-6789".into());
    let findings = scan_resource("cfg", r);
    assert!(findings.iter().any(|f| f.rule_id == "SS-10"));
}

#[test]
fn severity_counts_mixed() {
    let mut r = make_resource(ResourceType::File);
    r.content = Some("password=x\nftp://a\n0.0.0.0".into());
    r.mode = Some("0777".into());
    let findings = scan_resource("cfg", r);
    let (c, h, m, l) = security_scanner::severity_counts(&findings);
    assert!(c >= 1, "should have at least 1 critical");
    assert!(h >= 1, "should have at least 1 high");
    assert!(c + h + m + l == findings.len());
}

#[test]
fn severity_counts_empty() {
    let (c, h, m, l) = security_scanner::severity_counts(&[]);
    assert_eq!((c, h, m, l), (0, 0, 0, 0));
}

#[test]
fn clean_resource_no_findings() {
    let mut r = make_resource(ResourceType::File);
    r.content = Some("log_level=info".into());
    r.mode = Some("0600".into());
    let findings = scan_resource("cfg", r);
    assert!(
        findings.is_empty(),
        "clean resource should have no findings"
    );
}
