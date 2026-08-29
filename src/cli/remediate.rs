//! `forjar remediate` — compute the corrections a config's own policies
//! determine, and print the corrected document (paiml/forjar#356).
//!
//! It never writes. `forjar remediate > forjar.yaml.new` is the write, and the
//! operator performs it after reading the diff — which is short, because the
//! edit is anchored at the byte range of each corrected value and every other
//! byte of the document is copied through unchanged.

use crate::cli::commands::RemediateArgs;
use crate::core::remediate::{self, Report};

pub(crate) fn cmd_remediate(args: &RemediateArgs) -> Result<(), String> {
    let source = std::fs::read_to_string(&args.file)
        .map_err(|e| format!("cannot read {}: {}", args.file.display(), e))?;
    let config = crate::core::parser::parse_and_validate(&args.file)?;
    let report = remediate::remediate(&source, &config, Some(&args.policy_id))?;
    if args.json {
        println!("{}", to_json(&report)?);
    } else {
        print!("{}", report.updated_yaml);
        report_to_stderr(&report);
    }
    Ok(())
}

/// The human summary goes to stderr so the corrected document on stdout stays
/// redirectable.
fn report_to_stderr(report: &Report) {
    for fix in &report.applied {
        eprintln!(
            "fixed: {}.{} {} -> {} (line {}, per policy {})",
            fix.resource_id,
            fix.field,
            fix.from.as_deref().unwrap_or("<unset>"),
            fix.to,
            fix.line,
            fix.policy_id
        );
    }
    for v in &report.remaining {
        eprintln!(
            "not fixed: {} on {} [{}] — {}",
            v.policy_id, v.resource_id, v.rule_type, v.reason
        );
    }
    if let Some(note) = &report.scope_note {
        eprintln!("note: {note}");
    }
}

fn to_json(report: &Report) -> Result<String, String> {
    let applied: Vec<serde_json::Value> = report
        .applied
        .iter()
        .map(|f| {
            serde_json::json!({
                "policy_id": f.policy_id,
                "resource_id": f.resource_id,
                "field": f.field,
                "from": f.from,
                "to": f.to,
                "line": f.line,
            })
        })
        .collect();
    let remaining: Vec<serde_json::Value> = report
        .remaining
        .iter()
        .map(|v| {
            serde_json::json!({
                "policy_id": v.policy_id,
                "resource_id": v.resource_id,
                "message": v.message,
                "severity": v.severity,
                "rule_type": v.rule_type,
                "remediation_hint": v.remediation_hint,
                "reason": v.reason,
            })
        })
        .collect();
    serde_json::to_string_pretty(&serde_json::json!({
        "remediations_applied": applied,
        "updated_yaml_content": report.updated_yaml,
        "remaining_violations": remaining,
        "changed": report.changed,
        "config_hash_before": report.hash_before,
        "config_hash_after": report.hash_after,
        "scope_note": report.scope_note,
    }))
    .map_err(|e| format!("JSON error: {e}"))
}
