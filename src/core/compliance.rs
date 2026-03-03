//! FJ-1387: Compliance testing framework — CIS, NIST 800-53, SOC2, HIPAA benchmarks.
//!
//! Structured benchmark definitions with composable rule evaluation.
//! Each rule returns findings (pass/fail) for a given resource set.

use super::types::*;

/// A compliance finding from evaluating a rule against a resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplianceFinding {
    pub rule_id: String,
    pub benchmark: String,
    pub severity: FindingSeverity,
    pub resource_id: String,
    pub message: String,
}

/// Severity of a compliance finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Evaluate all rules for a given benchmark against a config.
pub fn evaluate_benchmark(
    benchmark: &str,
    config: &ForjarConfig,
) -> Vec<ComplianceFinding> {
    match benchmark.to_lowercase().as_str() {
        "cis" => evaluate_cis(config),
        "nist" | "nist-800-53" => evaluate_nist_800_53(config),
        "soc2" => evaluate_soc2(config),
        "hipaa" => evaluate_hipaa(config),
        _ => vec![ComplianceFinding {
            rule_id: "UNKNOWN-1".to_string(),
            benchmark: benchmark.to_string(),
            severity: FindingSeverity::Info,
            resource_id: String::new(),
            message: format!("unknown benchmark: {benchmark}"),
        }],
    }
}

/// List all supported benchmark names.
pub fn supported_benchmarks() -> &'static [&'static str] {
    &["cis", "nist-800-53", "soc2", "hipaa"]
}

// ── CIS Benchmark ────────────────────────────────────────────────

fn evaluate_cis(config: &ForjarConfig) -> Vec<ComplianceFinding> {
    let mut findings = Vec::new();
    for (id, resource) in &config.resources {
        cis_world_writable(&mut findings, id, resource);
        cis_root_tmp(&mut findings, id, resource);
        cis_service_no_restart_policy(&mut findings, id, resource);
        cis_package_no_version_pin(&mut findings, id, resource);
    }
    findings
}

fn cis_world_writable(findings: &mut Vec<ComplianceFinding>, id: &str, r: &Resource) {
    if let Some(ref mode) = r.mode {
        if mode.ends_with('7') || mode.ends_with('6') {
            findings.push(ComplianceFinding {
                rule_id: "CIS-6.1.1".to_string(),
                benchmark: "cis".to_string(),
                severity: FindingSeverity::High,
                resource_id: id.to_string(),
                message: format!("world-writable mode {mode}"),
            });
        }
    }
}

fn cis_root_tmp(findings: &mut Vec<ComplianceFinding>, id: &str, r: &Resource) {
    if let Some(ref path) = r.path {
        if path.starts_with("/tmp") && r.owner.as_deref() == Some("root") {
            findings.push(ComplianceFinding {
                rule_id: "CIS-1.1.5".to_string(),
                benchmark: "cis".to_string(),
                severity: FindingSeverity::Medium,
                resource_id: id.to_string(),
                message: "root-owned file in /tmp".to_string(),
            });
        }
    }
}

fn cis_service_no_restart_policy(
    findings: &mut Vec<ComplianceFinding>,
    id: &str,
    r: &Resource,
) {
    if r.resource_type == ResourceType::Service && r.restart.is_none() {
        findings.push(ComplianceFinding {
            rule_id: "CIS-5.2.1".to_string(),
            benchmark: "cis".to_string(),
            severity: FindingSeverity::Low,
            resource_id: id.to_string(),
            message: "service has no restart policy".to_string(),
        });
    }
}

fn cis_package_no_version_pin(
    findings: &mut Vec<ComplianceFinding>,
    id: &str,
    r: &Resource,
) {
    if r.resource_type == ResourceType::Package && r.version.is_none() {
        findings.push(ComplianceFinding {
            rule_id: "CIS-6.2.1".to_string(),
            benchmark: "cis".to_string(),
            severity: FindingSeverity::Low,
            resource_id: id.to_string(),
            message: "package has no version pin".to_string(),
        });
    }
}

// ── NIST 800-53 ──────────────────────────────────────────────────

fn evaluate_nist_800_53(config: &ForjarConfig) -> Vec<ComplianceFinding> {
    let mut findings = Vec::new();
    for (id, resource) in &config.resources {
        nist_ac3_access_enforcement(&mut findings, id, resource);
        nist_ac6_least_privilege(&mut findings, id, resource);
        nist_cm6_config_settings(&mut findings, id, resource);
        nist_sc28_protection_at_rest(&mut findings, id, resource);
        nist_si7_integrity_verification(&mut findings, id, resource, config);
    }
    findings
}

/// AC-3: Access Enforcement — files must have explicit owner/group/mode.
fn nist_ac3_access_enforcement(
    findings: &mut Vec<ComplianceFinding>,
    id: &str,
    r: &Resource,
) {
    if r.resource_type != ResourceType::File {
        return;
    }
    if r.owner.is_none() {
        findings.push(ComplianceFinding {
            rule_id: "NIST-AC-3.1".to_string(),
            benchmark: "nist-800-53".to_string(),
            severity: FindingSeverity::Medium,
            resource_id: id.to_string(),
            message: "file resource missing owner (AC-3 access enforcement)".to_string(),
        });
    }
    if r.mode.is_none() {
        findings.push(ComplianceFinding {
            rule_id: "NIST-AC-3.2".to_string(),
            benchmark: "nist-800-53".to_string(),
            severity: FindingSeverity::Medium,
            resource_id: id.to_string(),
            message: "file resource missing mode (AC-3 access enforcement)".to_string(),
        });
    }
}

/// AC-6: Least Privilege — services should not run as root.
fn nist_ac6_least_privilege(
    findings: &mut Vec<ComplianceFinding>,
    id: &str,
    r: &Resource,
) {
    if r.resource_type == ResourceType::Service && r.owner.as_deref() == Some("root") {
        findings.push(ComplianceFinding {
            rule_id: "NIST-AC-6".to_string(),
            benchmark: "nist-800-53".to_string(),
            severity: FindingSeverity::High,
            resource_id: id.to_string(),
            message: "service running as root violates least privilege (AC-6)".to_string(),
        });
    }
}

/// CM-6: Configuration Settings — Docker containers need explicit port bindings.
fn nist_cm6_config_settings(
    findings: &mut Vec<ComplianceFinding>,
    id: &str,
    r: &Resource,
) {
    if r.resource_type == ResourceType::Docker && r.ports.is_empty() && r.port.is_none() {
        findings.push(ComplianceFinding {
            rule_id: "NIST-CM-6".to_string(),
            benchmark: "nist-800-53".to_string(),
            severity: FindingSeverity::Low,
            resource_id: id.to_string(),
            message: "Docker resource has no explicit port bindings (CM-6)".to_string(),
        });
    }
}

/// SC-28: Protection of Information at Rest — sensitive paths should have restricted modes.
fn nist_sc28_protection_at_rest(
    findings: &mut Vec<ComplianceFinding>,
    id: &str,
    r: &Resource,
) {
    if r.resource_type != ResourceType::File {
        return;
    }
    let sensitive_paths = ["/etc/shadow", "/etc/ssh", "/etc/ssl", "/etc/pki"];
    if let Some(ref path) = r.path {
        if sensitive_paths.iter().any(|p| path.starts_with(p)) && r.mode.is_none() {
            findings.push(ComplianceFinding {
                rule_id: "NIST-SC-28".to_string(),
                benchmark: "nist-800-53".to_string(),
                severity: FindingSeverity::High,
                resource_id: id.to_string(),
                message: format!("sensitive path {path} missing explicit mode (SC-28)"),
            });
        }
    }
}

/// SI-7: Software, Firmware, and Information Integrity — resources should have integrity checks.
fn nist_si7_integrity_verification(
    findings: &mut Vec<ComplianceFinding>,
    id: &str,
    r: &Resource,
    config: &ForjarConfig,
) {
    if r.resource_type != ResourceType::File || r.source.is_none() {
        return;
    }
    // Files sourced from external locations should have a check block
    let has_check = config.checks.contains_key(id);
    if !has_check {
        findings.push(ComplianceFinding {
            rule_id: "NIST-SI-7".to_string(),
            benchmark: "nist-800-53".to_string(),
            severity: FindingSeverity::Medium,
            resource_id: id.to_string(),
            message: "externally-sourced file has no integrity check block (SI-7)".to_string(),
        });
    }
}

// ── SOC2 ─────────────────────────────────────────────────────────

fn evaluate_soc2(config: &ForjarConfig) -> Vec<ComplianceFinding> {
    let mut findings = Vec::new();
    for (id, resource) in &config.resources {
        soc2_file_ownership(&mut findings, id, resource);
        soc2_service_monitoring(&mut findings, id, resource);
    }
    findings
}

fn soc2_file_ownership(findings: &mut Vec<ComplianceFinding>, id: &str, r: &Resource) {
    if r.resource_type == ResourceType::File && r.owner.is_none() {
        findings.push(ComplianceFinding {
            rule_id: "SOC2-CC6.1".to_string(),
            benchmark: "soc2".to_string(),
            severity: FindingSeverity::Medium,
            resource_id: id.to_string(),
            message: "file resource missing owner (CC6.1 logical access)".to_string(),
        });
    }
}

fn soc2_service_monitoring(findings: &mut Vec<ComplianceFinding>, id: &str, r: &Resource) {
    if r.resource_type == ResourceType::Service && r.restart_on.is_empty() {
        findings.push(ComplianceFinding {
            rule_id: "SOC2-CC7.2".to_string(),
            benchmark: "soc2".to_string(),
            severity: FindingSeverity::Low,
            resource_id: id.to_string(),
            message: "service has no restart_on triggers (CC7.2 monitoring)".to_string(),
        });
    }
}

// ── HIPAA ────────────────────────────────────────────────────────

fn evaluate_hipaa(config: &ForjarConfig) -> Vec<ComplianceFinding> {
    let mut findings = Vec::new();
    for (id, resource) in &config.resources {
        hipaa_file_permissions(&mut findings, id, resource);
        hipaa_network_encryption(&mut findings, id, resource);
    }
    findings
}

fn hipaa_file_permissions(findings: &mut Vec<ComplianceFinding>, id: &str, r: &Resource) {
    if let Some(ref mode) = r.mode {
        let last = mode.chars().last().unwrap_or('0');
        if last != '0' {
            findings.push(ComplianceFinding {
                rule_id: "HIPAA-164.312a".to_string(),
                benchmark: "hipaa".to_string(),
                severity: FindingSeverity::High,
                resource_id: id.to_string(),
                message: format!("mode {mode} allows other access (164.312(a) access control)"),
            });
        }
    }
}

fn hipaa_network_encryption(findings: &mut Vec<ComplianceFinding>, id: &str, r: &Resource) {
    if r.resource_type == ResourceType::Network {
        if let Some(ref port) = r.port {
            if port == "80" || port == "8080" {
                findings.push(ComplianceFinding {
                    rule_id: "HIPAA-164.312e".to_string(),
                    benchmark: "hipaa".to_string(),
                    severity: FindingSeverity::Critical,
                    resource_id: id.to_string(),
                    message: format!(
                        "unencrypted port {port} (164.312(e) transmission security)"
                    ),
                });
            }
        }
    }
}

/// Count findings by severity.
pub fn count_by_severity(findings: &[ComplianceFinding]) -> (usize, usize, usize, usize) {
    let mut critical = 0;
    let mut high = 0;
    let mut medium = 0;
    let mut low = 0;
    for f in findings {
        match f.severity {
            FindingSeverity::Critical => critical += 1,
            FindingSeverity::High => high += 1,
            FindingSeverity::Medium => medium += 1,
            FindingSeverity::Low | FindingSeverity::Info => low += 1,
        }
    }
    (critical, high, medium, low)
}
