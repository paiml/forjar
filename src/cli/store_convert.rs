//! FJ-1328: `forjar convert --reproducible` — recipe conversion CLI.
//!
//! Automates steps 1-3 of the 5-step conversion ladder:
//! 1. Add version pins to all packages
//! 2. Add `store: true` to cacheable resources
//! 3. Generate `forjar.inputs.lock.yaml`

use crate::core::store::convert::{analyze_conversion, ConversionSignals};
use std::path::Path;

/// Convert recipe to reproducible format.
pub(crate) fn cmd_convert(file: &Path, reproducible: bool, json: bool) -> Result<(), String> {
    if !reproducible {
        return Err("use --reproducible flag to enable conversion".to_string());
    }

    let signals = extract_signals(file)?;
    let report = analyze_conversion(&signals);

    if json {
        let j = serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string());
        println!("{j}");
    } else {
        println!("Conversion report for {}:", file.display());
        println!(
            "  Current purity: {:?} → Projected: {:?}",
            report.current_purity, report.projected_purity
        );
        println!(
            "  Auto changes: {} | Manual changes: {}",
            report.auto_change_count, report.manual_change_count
        );
        println!();

        for res in &report.resources {
            println!("  {}:", res.name);
            println!("    {:?} → {:?}", res.current_purity, res.target_purity);
            for c in &res.auto_changes {
                println!("    [auto] {}", c.description);
            }
            for m in &res.manual_changes {
                println!("    [manual] {m}");
            }
        }
    }
    Ok(())
}

/// Extract conversion signals from a forjar.yaml config.
fn extract_signals(file: &Path) -> Result<Vec<ConversionSignals>, String> {
    let content =
        std::fs::read_to_string(file).map_err(|e| format!("read {}: {e}", file.display()))?;
    let doc: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("parse {}: {e}", file.display()))?;

    let resources = doc
        .get("resources")
        .and_then(|r| r.as_mapping())
        .ok_or_else(|| "no resources section found".to_string())?;

    let mut signals = Vec::new();
    for (key, val) in resources {
        let name = key.as_str().unwrap_or("").to_string();
        let provider = val
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("file")
            .to_string();
        let has_version = val.get("version").is_some();
        let has_store = val.get("store").and_then(|v| v.as_bool()).unwrap_or(false);
        let has_sandbox = val.get("sandbox").is_some();
        let has_curl_pipe = detect_curl_pipe(val);
        let current_version = val
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        signals.push(ConversionSignals {
            name,
            has_version,
            has_store,
            has_sandbox,
            has_curl_pipe,
            provider,
            current_version,
        });
    }
    signals.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(signals)
}

/// Detect curl|bash patterns in resource values.
fn detect_curl_pipe(val: &serde_yaml_ng::Value) -> bool {
    let s = serde_yaml_ng::to_string(val).unwrap_or_default();
    s.contains("curl") && s.contains("bash") || s.contains("wget") && s.contains("sh")
}
