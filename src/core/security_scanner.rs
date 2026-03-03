//! FJ-1390: Static IaC security scanner — detect security smells in configs.
//!
//! Implements a subset of the 62 IaC security smell categories from
//! [arXiv:2509.18761] "Security Smells in IaC: Taxonomy Update Beyond the Seven Sins".

use super::types::*;

/// A security finding from scanning a config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityFinding {
    pub rule_id: String,
    pub category: &'static str,
    pub severity: Severity,
    pub resource_id: String,
    pub message: String,
}

/// Finding severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

/// Scan a config for all security smells.
pub fn scan(config: &ForjarConfig) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    for (id, resource) in &config.resources {
        hardcoded_secrets(&mut findings, id, resource);
        http_without_tls(&mut findings, id, resource);
        world_accessible(&mut findings, id, resource);
        missing_integrity_check(&mut findings, id, resource, config);
        privileged_container(&mut findings, id, resource);
        no_resource_limits(&mut findings, id, resource);
        weak_crypto(&mut findings, id, resource);
        insecure_protocol(&mut findings, id, resource);
        unrestricted_network(&mut findings, id, resource);
        sensitive_data_exposure(&mut findings, id, resource);
    }
    findings
}

/// Count findings by severity.
pub fn severity_counts(findings: &[SecurityFinding]) -> (usize, usize, usize, usize) {
    let (mut c, mut h, mut m, mut l) = (0, 0, 0, 0);
    for f in findings {
        match f.severity {
            Severity::Critical => c += 1,
            Severity::High => h += 1,
            Severity::Medium => m += 1,
            Severity::Low => l += 1,
        }
    }
    (c, h, m, l)
}

// ── Rule implementations ─────────────────────────────────────────

/// SS-1: Hard-coded secrets — passwords, tokens, keys in plain text.
fn hardcoded_secrets(findings: &mut Vec<SecurityFinding>, id: &str, r: &Resource) {
    let patterns = ["password=", "token=", "api_key=", "secret=", "AWS_SECRET"];
    let content = r.content.as_deref().unwrap_or("");
    for pat in &patterns {
        if content.to_lowercase().contains(&pat.to_lowercase()) {
            findings.push(SecurityFinding {
                rule_id: "SS-1".to_string(),
                category: "hard-coded-secret",
                severity: Severity::Critical,
                resource_id: id.to_string(),
                message: format!("content contains potential secret pattern: {pat}"),
            });
            break;
        }
    }
}

/// SS-2: HTTP without TLS — unencrypted network communication.
fn http_without_tls(findings: &mut Vec<SecurityFinding>, id: &str, r: &Resource) {
    let fields = [
        r.content.as_deref(),
        r.source.as_deref(),
        r.target.as_deref(),
    ];
    for field in &fields {
        if let Some(val) = field {
            if val.starts_with("http://") && !val.starts_with("http://localhost") {
                findings.push(SecurityFinding {
                    rule_id: "SS-2".to_string(),
                    category: "http-without-tls",
                    severity: Severity::High,
                    resource_id: id.to_string(),
                    message: "unencrypted HTTP URL detected (use HTTPS)".to_string(),
                });
                break;
            }
        }
    }
}

/// SS-3: World-accessible permissions — mode allows other read/write/execute.
fn world_accessible(findings: &mut Vec<SecurityFinding>, id: &str, r: &Resource) {
    if let Some(ref mode) = r.mode {
        let last = mode.chars().last().unwrap_or('0');
        if last >= '4' {
            findings.push(SecurityFinding {
                rule_id: "SS-3".to_string(),
                category: "world-accessible",
                severity: Severity::High,
                resource_id: id.to_string(),
                message: format!("mode {mode} allows world access"),
            });
        }
    }
}

/// SS-4: Missing integrity check — externally-sourced files without verification.
fn missing_integrity_check(
    findings: &mut Vec<SecurityFinding>,
    id: &str,
    r: &Resource,
    config: &ForjarConfig,
) {
    if r.source.is_none() || r.resource_type != ResourceType::File {
        return;
    }
    if !config.checks.contains_key(id) {
        findings.push(SecurityFinding {
            rule_id: "SS-4".to_string(),
            category: "missing-integrity-check",
            severity: Severity::Medium,
            resource_id: id.to_string(),
            message: "externally-sourced file has no integrity check".to_string(),
        });
    }
}

/// SS-5: Privileged container — Docker running with elevated permissions.
fn privileged_container(findings: &mut Vec<SecurityFinding>, id: &str, r: &Resource) {
    if r.resource_type != ResourceType::Docker {
        return;
    }
    for env in &r.environment {
        if env.contains("privileged=true") || env.contains("PRIVILEGED=true") {
            findings.push(SecurityFinding {
                rule_id: "SS-5".to_string(),
                category: "privileged-container",
                severity: Severity::Critical,
                resource_id: id.to_string(),
                message: "Docker container running in privileged mode".to_string(),
            });
        }
    }
    // Check if user is root (implicit privilege)
    if r.owner.as_deref() == Some("root") {
        findings.push(SecurityFinding {
            rule_id: "SS-5".to_string(),
            category: "privileged-container",
            severity: Severity::Medium,
            resource_id: id.to_string(),
            message: "Docker container running as root".to_string(),
        });
    }
}

/// SS-6: No resource limits — containers/services without CPU/memory bounds.
fn no_resource_limits(findings: &mut Vec<SecurityFinding>, id: &str, r: &Resource) {
    if r.resource_type != ResourceType::Docker {
        return;
    }
    let has_limits = r.environment.iter().any(|e| {
        e.contains("MEMORY_LIMIT") || e.contains("CPU_LIMIT") || e.contains("--memory")
    });
    if !has_limits {
        findings.push(SecurityFinding {
            rule_id: "SS-6".to_string(),
            category: "no-resource-limits",
            severity: Severity::Low,
            resource_id: id.to_string(),
            message: "Docker container has no resource limits".to_string(),
        });
    }
}

/// SS-7: Weak cryptographic configuration.
fn weak_crypto(findings: &mut Vec<SecurityFinding>, id: &str, r: &Resource) {
    let content = r.content.as_deref().unwrap_or("");
    let weak = ["md5", "sha1", "des", "rc4", "sslv3", "tlsv1.0"];
    for pat in &weak {
        if content.to_lowercase().contains(pat) {
            findings.push(SecurityFinding {
                rule_id: "SS-7".to_string(),
                category: "weak-crypto",
                severity: Severity::High,
                resource_id: id.to_string(),
                message: format!("content references weak cryptography: {pat}"),
            });
            break;
        }
    }
}

/// SS-8: Insecure protocol usage — telnet, ftp, rsh in configs.
fn insecure_protocol(findings: &mut Vec<SecurityFinding>, id: &str, r: &Resource) {
    let content = r.content.as_deref().unwrap_or("");
    let protos = ["telnet://", "ftp://", "rsh://"];
    for proto in &protos {
        if content.contains(proto) {
            findings.push(SecurityFinding {
                rule_id: "SS-8".to_string(),
                category: "insecure-protocol",
                severity: Severity::High,
                resource_id: id.to_string(),
                message: format!("insecure protocol: {proto}"),
            });
            break;
        }
    }
}

/// SS-9: Unrestricted network binding — binding to 0.0.0.0 or all interfaces.
fn unrestricted_network(findings: &mut Vec<SecurityFinding>, id: &str, r: &Resource) {
    let content = r.content.as_deref().unwrap_or("");
    if content.contains("0.0.0.0") || content.contains("bind_address: *") {
        findings.push(SecurityFinding {
            rule_id: "SS-9".to_string(),
            category: "unrestricted-network",
            severity: Severity::Medium,
            resource_id: id.to_string(),
            message: "service binding to all interfaces (0.0.0.0)".to_string(),
        });
    }
}

/// SS-10: Sensitive data in content — PII patterns, credit card numbers.
fn sensitive_data_exposure(findings: &mut Vec<SecurityFinding>, id: &str, r: &Resource) {
    let content = r.content.as_deref().unwrap_or("");
    if content.contains("ssn=") || content.contains("credit_card=") {
        findings.push(SecurityFinding {
            rule_id: "SS-10".to_string(),
            category: "sensitive-data-exposure",
            severity: Severity::Critical,
            resource_id: id.to_string(),
            message: "content may contain PII or sensitive data".to_string(),
        });
    }
}
