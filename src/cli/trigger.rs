//! FJ-3107: `forjar trigger <rulebook>` — manual event trigger.
//!
//! Creates a Manual InfraEvent and evaluates it against the specified
//! rulebook via the rules runtime engine with cooldown tracking.

use crate::core::rules_engine;
use crate::core::rules_runtime;
use crate::core::types::{CooldownTracker, EventType, InfraEvent, RulebookConfig};
use std::collections::HashMap;
use std::path::Path;

/// Execute `forjar trigger <rulebook>`.
pub(crate) fn cmd_trigger(
    rulebook_name: &str,
    rules_file: &Path,
    payload: &[(String, String)],
    dry_run: bool,
    json: bool,
) -> Result<(), String> {
    // Load and validate rulebook config
    let config = load_rulebook_config(rules_file)?;

    // Verify the target rulebook exists
    let target = config
        .rulebooks
        .iter()
        .find(|rb| rb.name == rulebook_name)
        .ok_or_else(|| {
            let names: Vec<&str> = config.rulebooks.iter().map(|rb| rb.name.as_str()).collect();
            format!(
                "rulebook '{}' not found. Available: {}",
                rulebook_name,
                names.join(", ")
            )
        })?;

    // Build manual event
    let mut event_payload: HashMap<String, String> = payload.iter().cloned().collect();
    event_payload.insert("triggered_by".into(), "manual".into());
    event_payload.insert("rulebook".into(), rulebook_name.into());

    let event = InfraEvent {
        event_type: EventType::Manual,
        timestamp: crate::tripwire::eventlog::now_iso8601(),
        machine: None,
        payload: event_payload,
    };

    if dry_run {
        return print_dry_run(rulebook_name, target, &event, json);
    }

    // Evaluate with cooldown tracker
    let mut tracker = CooldownTracker::default();
    let results = rules_runtime::evaluate_event(&event, &config, &mut tracker);

    let fired: Vec<_> = results
        .iter()
        .filter(|r| !r.cooldown_blocked && !r.disabled && !r.actions.is_empty())
        .collect();

    if json {
        print_json_result(rulebook_name, &fired);
    } else {
        print_text_result(rulebook_name, &fired);
    }

    Ok(())
}

fn load_rulebook_config(path: &Path) -> Result<RulebookConfig, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;

    // Validate first
    let issues = rules_engine::validate_rulebook_yaml(&content)?;
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == rules_engine::IssueSeverity::Error)
        .collect();
    if !errors.is_empty() {
        let msgs: Vec<String> = errors
            .iter()
            .map(|i| format!("  {}: {}", i.rulebook, i.message))
            .collect();
        return Err(format!("rulebook validation failed:\n{}", msgs.join("\n")));
    }

    serde_yaml_ng::from_str(&content).map_err(|e| format!("parse rulebook: {e}"))
}

fn print_dry_run(
    name: &str,
    target: &crate::core::types::Rulebook,
    event: &InfraEvent,
    json: bool,
) -> Result<(), String> {
    if json {
        let output = serde_json::json!({
            "dry_run": true,
            "rulebook": name,
            "event_type": "manual",
            "actions": target.actions.len(),
            "cooldown_secs": target.cooldown_secs,
            "enabled": target.enabled,
            "payload": event.payload,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    } else {
        println!("Dry-run: trigger '{name}'");
        println!("  Actions: {}", target.actions.len());
        for (i, action) in target.actions.iter().enumerate() {
            println!("  [{i}] {}", action.action_type());
        }
        println!("  Cooldown: {}s", target.cooldown_secs);
        println!("  Enabled: {}", target.enabled);
    }
    Ok(())
}

fn print_json_result(name: &str, fired: &[&rules_runtime::EvalResult]) {
    let output = serde_json::json!({
        "rulebook": name,
        "fired": !fired.is_empty(),
        "actions_count": fired.iter().map(|r| r.actions.len()).sum::<usize>(),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_default()
    );
}

fn print_text_result(name: &str, fired: &[&rules_runtime::EvalResult]) {
    if fired.is_empty() {
        println!("Trigger '{name}': no actions fired (rulebook may not match Manual events)");
    } else {
        for r in fired {
            println!(
                "Trigger '{}': {} action(s) fired",
                r.rulebook,
                r.actions.len()
            );
            for (i, action) in r.actions.iter().enumerate() {
                println!("  [{i}] {}", action.action_type());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_rulebook(dir: &Path, content: &str) -> std::path::PathBuf {
        let path = dir.join("forjar-rules.yaml");
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn trigger_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let rules = write_rulebook(
            dir.path(),
            "rulebooks:\n  - name: test\n    events:\n      - type: manual\n    actions:\n      - script: echo hello\n    cooldown_secs: 0\n",
        );
        let result = cmd_trigger("test", &rules, &[], true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn trigger_dry_run_json() {
        let dir = tempfile::tempdir().unwrap();
        let rules = write_rulebook(
            dir.path(),
            "rulebooks:\n  - name: test\n    events:\n      - type: manual\n    actions:\n      - script: echo hello\n    cooldown_secs: 0\n",
        );
        let result = cmd_trigger("test", &rules, &[], true, true);
        assert!(result.is_ok());
    }

    #[test]
    fn trigger_fires() {
        let dir = tempfile::tempdir().unwrap();
        let rules = write_rulebook(
            dir.path(),
            "rulebooks:\n  - name: deploy\n    events:\n      - type: manual\n    actions:\n      - script: deploy.sh\n    cooldown_secs: 0\n",
        );
        let result = cmd_trigger("deploy", &rules, &[], false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn trigger_with_payload() {
        let dir = tempfile::tempdir().unwrap();
        let rules = write_rulebook(
            dir.path(),
            "rulebooks:\n  - name: deploy\n    events:\n      - type: manual\n    actions:\n      - script: deploy.sh\n    cooldown_secs: 0\n",
        );
        let payload = vec![("env".into(), "staging".into())];
        let result = cmd_trigger("deploy", &rules, &payload, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn trigger_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let rules = write_rulebook(
            dir.path(),
            "rulebooks:\n  - name: deploy\n    events:\n      - type: manual\n    actions:\n      - script: deploy.sh\n    cooldown_secs: 0\n",
        );
        let result = cmd_trigger("nonexistent", &rules, &[], false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn trigger_missing_file() {
        let result = cmd_trigger(
            "test",
            Path::new("/nonexistent/rules.yaml"),
            &[],
            false,
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn load_rulebook_validates() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_rulebook(
            dir.path(),
            "rulebooks:\n  - name: valid\n    events:\n      - type: manual\n    actions:\n      - script: echo ok\n",
        );
        let config = load_rulebook_config(&path).unwrap();
        assert_eq!(config.rulebooks.len(), 1);
    }

    #[test]
    fn load_rulebook_invalid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_rulebook(dir.path(), "not: valid: yaml: [");
        let result = load_rulebook_config(&path);
        assert!(result.is_err());
    }
}
