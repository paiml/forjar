//! FJ-1390: CLI command for static IaC security scanning.

use crate::core::parser;
use crate::core::security_scanner::{self, Severity};
use std::path::Path;

pub(crate) fn cmd_security_scan(
    file: &Path,
    json: bool,
    fail_on: Option<&str>,
) -> Result<(), String> {
    let config = parser::parse_and_validate(file)?;
    let findings = security_scanner::scan(&config);

    if findings.is_empty() {
        if json {
            println!("{{\"findings\":[],\"summary\":{{\"total\":0}}}}");
        } else {
            println!("No security findings.");
        }
        return Ok(());
    }

    let (crit, high, med, low) = security_scanner::severity_counts(&findings);

    if json {
        print_json(&findings, crit, high, med, low);
    } else {
        print_text(&findings, crit, high, med, low);
    }

    // Check fail_on threshold
    if let Some(threshold) = fail_on {
        let should_fail = match threshold.to_lowercase().as_str() {
            "critical" => crit > 0,
            "high" => crit + high > 0,
            "medium" => crit + high + med > 0,
            "low" => !findings.is_empty(),
            _ => return Err(format!("unknown severity threshold: {threshold}")),
        };
        if should_fail {
            return Err(format!(
                "security scan failed: {} findings at or above {threshold}",
                findings.len()
            ));
        }
    }

    Ok(())
}

fn severity_symbol(s: Severity) -> &'static str {
    match s {
        Severity::Critical => "CRIT",
        Severity::High => "HIGH",
        Severity::Medium => "MED ",
        Severity::Low => "LOW ",
    }
}

fn print_text(
    findings: &[security_scanner::SecurityFinding],
    crit: usize,
    high: usize,
    med: usize,
    low: usize,
) {
    println!("Security Scan Results");
    println!("{}", "-".repeat(60));
    for f in findings {
        println!(
            "  [{}] {} ({}) — {}",
            severity_symbol(f.severity),
            f.rule_id,
            f.resource_id,
            f.message,
        );
    }
    println!("{}", "-".repeat(60));
    println!(
        "Summary: {} critical, {} high, {} medium, {} low ({} total)",
        crit,
        high,
        med,
        low,
        findings.len()
    );
}

fn print_json(
    findings: &[security_scanner::SecurityFinding],
    crit: usize,
    high: usize,
    med: usize,
    low: usize,
) {
    let items: Vec<String> = findings
        .iter()
        .map(|f| {
            format!(
                "{{\"rule_id\":\"{}\",\"category\":\"{}\",\"severity\":\"{:?}\",\"resource_id\":\"{}\",\"message\":\"{}\"}}",
                f.rule_id, f.category, f.severity, f.resource_id,
                f.message.replace('\"', "\\\"")
            )
        })
        .collect();
    println!(
        "{{\"findings\":[{}],\"summary\":{{\"total\":{},\"critical\":{},\"high\":{},\"medium\":{},\"low\":{}}}}}",
        items.join(","),
        findings.len(),
        crit,
        high,
        med,
        low
    );
}
