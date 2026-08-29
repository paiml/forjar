//! The repo's ONE SARIF 2.1.0 emitter.
//!
//! Lifted out of `parser::policy::policy_check_to_sarif`, which is now a thin
//! projection onto [`GateFinding`] that calls in here. Two emitters would drift
//! — and the one that existed hardcoded `artifactLocation.uri` to the literal
//! `"forjar.yaml"` and emitted no `region` at all, so a SARIF consumer could
//! not navigate to the finding.

use super::GateFinding;
use serde_json::{json, Value};

/// Deduped `rules[]`: first finding per rule id supplies its description.
fn rules_for(findings: &[GateFinding]) -> Vec<Value> {
    let mut seen: Vec<&str> = Vec::new();
    let mut rules = Vec::new();
    for f in findings {
        if seen.contains(&f.rule_id.as_str()) {
            continue;
        }
        let mut rule = json!({
            "id": f.rule_id,
            "shortDescription": { "text": f.message },
        });
        if let Some(ref help) = f.remediation {
            rule["help"] = json!({ "text": help });
        }
        rules.push(rule);
        seen.push(f.rule_id.as_str());
    }
    rules
}

/// `physicalLocation`, carrying a `region` only when the line is known.
fn location_for(f: &GateFinding, artifact_uri: &str) -> Value {
    let mut physical = json!({ "artifactLocation": { "uri": artifact_uri } });
    if let Some(line) = f.yaml_line {
        physical["region"] = json!({ "startLine": line });
    }
    json!({ "physicalLocation": physical })
}

/// The message a consumer displays: resource-prefixed when there is a resource.
fn message_for(f: &GateFinding) -> String {
    if f.resource_id.is_empty() {
        f.message.clone()
    } else {
        format!("{}: {}", f.resource_id, f.message)
    }
}

/// Project findings onto a SARIF 2.1.0 log object.
pub fn findings_to_sarif(findings: &[GateFinding], artifact_uri: &str) -> Value {
    let results: Vec<Value> = findings
        .iter()
        .map(|f| {
            json!({
                "ruleId": f.rule_id,
                "level": f.level.sarif_level(),
                "message": { "text": message_for(f) },
                "locations": [location_for(f, artifact_uri)],
            })
        })
        .collect();

    json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "forjar",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/paiml/forjar",
                    "rules": rules_for(findings),
                }
            },
            "results": results,
        }]
    })
}
