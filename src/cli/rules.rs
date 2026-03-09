//! FJ-3108: `forjar rules validate|coverage` CLI handler.
//!
//! Validates rulebook YAML files and reports event type coverage.

use crate::cli::commands::RulesCmd;
use crate::core::rules_engine::{
    event_type_coverage, validate_rulebook_file, IssueSeverity, ValidationSummary,
};
use crate::core::types::RulebookConfig;
use std::path::Path;

/// Dispatch rules subcommands.
pub fn dispatch_rules(cmd: RulesCmd) -> Result<(), String> {
    match cmd {
        RulesCmd::Validate { file, json } => cmd_rules_validate(&file, json),
        RulesCmd::Coverage { file, json } => cmd_rules_coverage(&file, json),
    }
}

/// Validate a rulebook YAML file.
fn cmd_rules_validate(file: &Path, json: bool) -> Result<(), String> {
    let content =
        std::fs::read_to_string(file).map_err(|e| format!("read {}: {e}", file.display()))?;

    // Check if file has rulebooks key
    let config: RulebookConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("YAML parse error: {e}"))?;

    let issues = validate_rulebook_file(file)?;
    let summary = ValidationSummary::new(config.rulebooks.len(), issues);

    if json {
        print_validate_json(&summary);
    } else {
        print_validate_table(&summary, file);
    }

    if !summary.passed() {
        return Err(format!("{} error(s) found", summary.error_count()));
    }

    Ok(())
}

fn print_validate_table(summary: &ValidationSummary, file: &Path) {
    println!("Validating rulebooks in {}", file.display());
    println!("{}", "-".repeat(60));
    println!(
        "{} rulebook(s), {} error(s), {} warning(s)",
        summary.rulebook_count,
        summary.error_count(),
        summary.warning_count()
    );

    for issue in &summary.issues {
        let level = match issue.severity {
            IssueSeverity::Error => "ERROR",
            IssueSeverity::Warning => "WARN ",
        };
        println!("  [{level}] {}: {}", issue.rulebook, issue.message);
    }

    if summary.passed() {
        println!("\nValidation passed.");
    } else {
        println!("\nValidation FAILED.");
    }
}

fn print_validate_json(summary: &ValidationSummary) {
    let issues: Vec<serde_json::Value> = summary
        .issues
        .iter()
        .map(|i| {
            serde_json::json!({
                "rulebook": i.rulebook,
                "severity": i.severity.to_string(),
                "message": i.message,
            })
        })
        .collect();

    let output = serde_json::json!({
        "rulebook_count": summary.rulebook_count,
        "errors": summary.error_count(),
        "warnings": summary.warning_count(),
        "passed": summary.passed(),
        "issues": issues,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

/// Show event type coverage across rulebooks.
fn cmd_rules_coverage(file: &Path, json: bool) -> Result<(), String> {
    let content =
        std::fs::read_to_string(file).map_err(|e| format!("read {}: {e}", file.display()))?;

    let config: RulebookConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("YAML parse error: {e}"))?;

    let coverage = event_type_coverage(&config);

    if json {
        let map: serde_json::Map<String, serde_json::Value> = coverage
            .iter()
            .map(|(et, count)| (et.to_string(), serde_json::json!(count)))
            .collect();
        println!("{}", serde_json::to_string_pretty(&map).unwrap());
    } else {
        println!("Event Type Coverage");
        println!("{}", "-".repeat(40));
        for (et, count) in &coverage {
            let bar = if *count > 0 { "+" } else { "-" };
            println!("  [{bar}] {et}: {count} rulebook(s)");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_valid_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.yaml");
        std::fs::write(
            &path,
            r#"
rulebooks:
  - name: test
    events: [{type: manual}]
    actions: [{script: "echo ok"}]
"#,
        )
        .unwrap();
        let result = cmd_rules_validate(&path, false);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_invalid_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.yaml");
        std::fs::write(
            &path,
            r#"
rulebooks:
  - name: bad
    events: []
    actions: []
"#,
        )
        .unwrap();
        let result = cmd_rules_validate(&path, false);
        assert!(result.is_err());
    }

    #[test]
    fn validate_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.yaml");
        std::fs::write(
            &path,
            r#"
rulebooks:
  - name: ok
    events: [{type: manual}]
    actions: [{script: "echo"}]
"#,
        )
        .unwrap();
        let result = cmd_rules_validate(&path, true);
        assert!(result.is_ok());
    }

    #[test]
    fn coverage_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.yaml");
        std::fs::write(
            &path,
            r#"
rulebooks:
  - name: r1
    events: [{type: file_changed}]
    actions: [{script: "echo"}]
"#,
        )
        .unwrap();
        let result = cmd_rules_coverage(&path, false);
        assert!(result.is_ok());
    }

    #[test]
    fn coverage_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.yaml");
        std::fs::write(
            &path,
            r#"
rulebooks:
  - name: r1
    events: [{type: cron_fired}]
    actions: [{script: "echo"}]
"#,
        )
        .unwrap();
        let result = cmd_rules_coverage(&path, true);
        assert!(result.is_ok());
    }

    #[test]
    fn dispatch_validate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.yaml");
        std::fs::write(
            &path,
            r#"
rulebooks:
  - name: ok
    events: [{type: manual}]
    actions: [{script: "echo ok"}]
"#,
        )
        .unwrap();
        let cmd = RulesCmd::Validate {
            file: path,
            json: false,
        };
        let result = dispatch_rules(cmd);
        assert!(result.is_ok());
    }
}
