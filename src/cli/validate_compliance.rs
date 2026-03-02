//! Compliance validation.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

/// Compute drift risk score and reasons for a single resource.
fn compute_drift_risk(
    name: &str,
    res: &types::Resource,
    config: &types::ForjarConfig,
    state_dir: &Path,
) -> Option<(String, f64, String)> {
    let mut score: f64 = 0.0;
    let mut reasons: Vec<String> = Vec::new();

    if res.resource_type == types::ResourceType::File && res.content.is_some() {
        score += 0.3;
        reasons.push("mutable file content".to_string());
    }

    let dependent_count = config
        .resources
        .values()
        .filter(|r| r.depends_on.contains(&name.to_string()))
        .count();
    if dependent_count >= 3 {
        score += 0.2;
        reasons.push(format!("{} dependents", dependent_count));
    }

    if state_dir.exists() {
        score_from_event_history(name, state_dir, &mut score, &mut reasons);
    }

    if score > 0.0 {
        Some((name.to_string(), score.min(1.0), reasons.join(", ")))
    } else {
        None
    }
}

/// Score drift risk from event history files.
fn score_from_event_history(
    name: &str,
    state_dir: &Path,
    score: &mut f64,
    reasons: &mut Vec<String>,
) {
    let machines = discover_machines(state_dir);
    for m in &machines {
        let events_path = state_dir.join(format!("{}.events.jsonl", m));
        if !events_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&events_path).unwrap_or_default();
        let fail_count = content
            .lines()
            .filter(|line| {
                line.contains(name) && (line.contains("Failed") || line.contains("Drifted"))
            })
            .count();
        if fail_count > 0 {
            *score += 0.1 * fail_count.min(5) as f64;
            reasons.push(format!("{} past failures", fail_count));
        }
    }
}

/// Format drift risk output as JSON.
fn print_drift_risk_json(risk_scores: &[(String, f64, String)]) {
    let entries: Vec<String> = risk_scores
        .iter()
        .map(|(name, score, reason)| {
            format!(
                r#"{{"resource":"{}","risk_score":{:.2},"reasons":"{}"}}"#,
                name, score, reason
            )
        })
        .collect();
    println!("[{}]", entries.join(","));
}

/// Format drift risk output as text.
fn print_drift_risk_text(risk_scores: &[(String, f64, String)]) {
    if risk_scores.is_empty() {
        println!("{} No drift risk detected.", green("✓"));
    } else {
        println!("Drift risk assessment:\n");
        for (name, score, reason) in risk_scores {
            let level = if *score > 0.7 {
                red("HIGH")
            } else if *score > 0.3 {
                yellow("MEDIUM")
            } else {
                "LOW".to_string()
            };
            println!(
                "  [{}] {} ({:.0}%) — {}",
                level,
                name,
                score * 100.0,
                reason
            );
        }
    }
}

/// FJ-541: Check drift risk — score drift risk based on resource volatility.
pub(crate) fn cmd_validate_check_drift_risk(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let state_dir = std::path::Path::new("state");

    let mut risk_scores: Vec<(String, f64, String)> = Vec::new();

    for (name, res) in &config.resources {
        if let Some(entry) = compute_drift_risk(name, res, &config, state_dir) {
            risk_scores.push(entry);
        }
    }

    risk_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if json {
        print_drift_risk_json(&risk_scores);
    } else {
        print_drift_risk_text(&risk_scores);
    }
    Ok(())
}

/// Check CIS compliance for a single resource.
fn check_cis_compliance(name: &str, res: &types::Resource, violations: &mut Vec<(String, String)>) {
    if let Some(ref mode) = res.mode {
        if mode.ends_with('7') || mode.ends_with('6') {
            let last = mode.chars().last().unwrap_or('0');
            if last == '7' || last == '6' {
                violations.push((
                    name.to_string(),
                    format!("CIS: world-writable mode {}", mode),
                ));
            }
        }
    }
    if let Some(ref owner) = res.owner {
        if owner == "root" {
            if let Some(ref path) = res.path {
                if path.starts_with("/tmp") {
                    violations.push((name.to_string(), "CIS: root-owned file in /tmp".to_string()));
                }
            }
        }
    }
}

/// Check HIPAA compliance for a single resource.
fn check_hipaa_compliance(
    name: &str,
    res: &types::Resource,
    violations: &mut Vec<(String, String)>,
) {
    if let Some(ref mode) = res.mode {
        let chars: Vec<char> = mode.chars().collect();
        if chars.len() >= 4 {
            let other = chars[chars.len() - 1];
            if other != '0' {
                violations.push((
                    name.to_string(),
                    format!("HIPAA: other permissions not zero in mode {}", mode),
                ));
            }
        }
    }
}

/// FJ-551: Validate resources against compliance policy (CIS, SOC2, HIPAA).
pub(crate) fn cmd_validate_check_compliance(
    file: &Path,
    policy: &str,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut violations: Vec<(String, String)> = Vec::new();

    let policy_upper = policy.to_uppercase();

    for (name, res) in &config.resources {
        match policy_upper.as_str() {
            "CIS" => check_cis_compliance(name, res, &mut violations),
            "SOC2" => {
                if res.owner.is_none()
                    && res.resource_type == crate::core::types::ResourceType::File
                {
                    violations.push((
                        name.clone(),
                        "SOC2: file resource missing owner".to_string(),
                    ));
                }
            }
            "HIPAA" => check_hipaa_compliance(name, res, &mut violations),
            _ => {
                return Err(format!(
                    "Unknown compliance policy: {}. Supported: CIS, SOC2, HIPAA",
                    policy
                ));
            }
        }
    }

    if json {
        let items: Vec<String> = violations
            .iter()
            .map(|(n, v)| format!(r#"{{"resource":"{}","violation":"{}"}}"#, n, v))
            .collect();
        println!(
            r#"{{"policy":"{}","violations":[{}],"count":{}}}"#,
            policy,
            items.join(","),
            violations.len()
        );
    } else if violations.is_empty() {
        println!("Compliance check ({}) passed: no violations found", policy);
    } else {
        println!("Compliance violations ({}):", policy);
        for (name, violation) in &violations {
            println!("  {} — {}", name, violation);
        }
    }
    Ok(())
}

/// FJ-561: Check resources for platform-specific assumptions.
pub(crate) fn cmd_validate_check_portability(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut warnings: Vec<(String, String)> = Vec::new();

    for (name, res) in &config.resources {
        if let Some(ref path) = res.path {
            if path.starts_with("/proc") || path.starts_with("/sys") {
                warnings.push((name.clone(), format!("Linux-specific path: {}", path)));
            }
        }
        if let Some(ref provider) = res.provider {
            if provider == "apt" {
                warnings.push((
                    name.clone(),
                    "apt provider is Debian/Ubuntu-specific".to_string(),
                ));
            }
        }
        if res.resource_type == crate::core::types::ResourceType::Service {
            warnings.push((
                name.clone(),
                "service type assumes systemd (not portable to non-systemd)".to_string(),
            ));
        }
    }

    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|(n, w)| format!(r#"{{"resource":"{}","warning":"{}"}}"#, n, w))
            .collect();
        println!(
            r#"{{"portability_warnings":[{}],"count":{}}}"#,
            items.join(","),
            warnings.len()
        );
    } else if warnings.is_empty() {
        println!("Portability check passed: no platform-specific assumptions found");
    } else {
        println!("Portability warnings ({}):", warnings.len());
        for (name, warning) in &warnings {
            println!("  {} — {}", name, warning);
        }
    }
    Ok(())
}

/// FJ-611: Deep idempotency analysis — simulate re-apply to detect non-idempotent resources.
pub(crate) fn cmd_validate_check_idempotency_deep(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut suspects: Vec<(String, String)> = Vec::new();

    for (rname, resource) in &config.resources {
        if let Some(ref content) = resource.content {
            if content.contains("$(date")
                || content.contains("$(hostname")
                || content.contains("$RANDOM")
            {
                suspects.push((
                    rname.clone(),
                    "dynamic shell expansion in content".to_string(),
                ));
            }
        }
        if resource.resource_type == crate::core::types::ResourceType::File
            && resource.content.is_some()
            && resource.mode.is_none()
        {
            suspects.push((
                rname.clone(),
                "file content without explicit mode (may vary)".to_string(),
            ));
        }
    }

    if json {
        let items: Vec<String> = suspects
            .iter()
            .map(|(r, reason)| format!(r#"{{"resource":"{}","reason":"{}"}}"#, r, reason))
            .collect();
        println!(
            r#"{{"idempotency_suspects":[{}],"count":{}}}"#,
            items.join(","),
            suspects.len()
        );
    } else if suspects.is_empty() {
        println!("All resources appear idempotent");
    } else {
        println!("Idempotency suspects ({}):", suspects.len());
        for (r, reason) in &suspects {
            println!("  {} — {}", r, reason);
        }
    }
    Ok(())
}
