//! FJ-3107: `forjar trigger <rulebook>` — manual event trigger.
//!
//! Creates a Manual InfraEvent and evaluates it against the specified
//! rulebook via the rules runtime engine with cooldown tracking, then EXECUTES
//! the matched actions.
//!
//! # The count used to be a count of declarations
//!
//! The non-dry-run path enumerated the matched actions, counted them, printed
//! `Trigger 'rb1': 1 action(s) fired` / `"fired": true` and exited 0 without
//! dispatching any of them: no resource converged, no state directory appeared.
//! `--help` calls the other mode "Dry-run: show what would fire without
//! executing", so both modes were the dry run and only one said so.
//!
//! Every number printed below is now derived from [`trigger_exec`] outcomes,
//! and a failing action makes the command exit non-zero.

use super::helpers::{green, red};
use super::trigger_exec::{self, ActionOutcome};
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

    let matched: Vec<_> = results
        .iter()
        .filter(|r| !r.cooldown_blocked && !r.disabled && !r.actions.is_empty())
        .collect();

    // EXECUTE. Everything printed after this point is a report of what these
    // outcomes say happened, never of what the rulebook declared.
    let fired: Vec<(String, Vec<ActionOutcome>)> = matched
        .iter()
        .map(|r| {
            (
                r.rulebook.clone(),
                trigger_exec::execute_actions(&r.actions),
            )
        })
        .collect();

    if json {
        print_json_result(rulebook_name, &fired);
    } else {
        print_text_result(rulebook_name, &fired);
    }

    failure_summary(&fired)
}

/// Collapse the per-action outcomes into the command's exit status.
///
/// An action that failed used to be indistinguishable from one that succeeded,
/// because neither ever ran.
fn failure_summary(fired: &[(String, Vec<ActionOutcome>)]) -> Result<(), String> {
    let failures: Vec<String> = fired
        .iter()
        .flat_map(|(rulebook, outcomes)| {
            outcomes.iter().filter_map(move |o| {
                o.result
                    .as_ref()
                    .err()
                    .map(|e| format!("{rulebook}[{}] {}: {e}", o.index, o.kind))
            })
        })
        .collect();

    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} action(s) failed:\n  {}",
            failures.len(),
            failures.join("\n  ")
        ))
    }
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

/// `fired` / `actions_count` count actions that RAN and SUCCEEDED.
///
/// They used to count the actions the rulebook declared, which is a number the
/// command could produce without doing anything — and did.
fn print_json_result(name: &str, fired: &[(String, Vec<ActionOutcome>)]) {
    let executed: usize = fired
        .iter()
        .flat_map(|(_, o)| o.iter())
        .filter(|o| o.succeeded())
        .count();
    let actions: Vec<serde_json::Value> = fired
        .iter()
        .flat_map(|(rulebook, outcomes)| {
            outcomes.iter().map(move |o| {
                serde_json::json!({
                    "rulebook": rulebook,
                    "index": o.index,
                    "type": o.kind,
                    "status": if o.succeeded() { "ok" } else { "failed" },
                    "error": o.result.as_ref().err(),
                })
            })
        })
        .collect();
    let failed = actions.len() - executed;

    let output = serde_json::json!({
        "rulebook": name,
        "fired": executed > 0,
        "actions_count": executed,
        "actions_failed": failed,
        "actions": actions,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_default()
    );
}

fn print_text_result(name: &str, fired: &[(String, Vec<ActionOutcome>)]) {
    if fired.is_empty() {
        println!("Trigger '{name}': no actions fired (rulebook may not match Manual events)");
        return;
    }
    for (rulebook, outcomes) in fired {
        let ok = outcomes.iter().filter(|o| o.succeeded()).count();
        println!(
            "Trigger '{rulebook}': {ok} of {} action(s) fired",
            outcomes.len()
        );
        for o in outcomes {
            match &o.result {
                Ok(()) => println!("  [{}] {} {}", o.index, o.kind, green("ok")),
                Err(e) => println!("  [{}] {} {}: {e}", o.index, o.kind, red("FAILED")),
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

    // STRENGTHENED: both of these used to run `script: deploy.sh` — a file that
    // does not exist — and assert only `is_ok()`. They passed because nothing
    // ran, and they would have kept passing for exactly as long as nothing ran.
    // They now execute a script whose effect is checkable and check it.

    #[test]
    fn trigger_fires() {
        let dir = tempfile::tempdir().unwrap();
        let deployed = dir.path().join("deployed.txt");
        let rules = write_rulebook(
            dir.path(),
            &format!(
                "rulebooks:\n  - name: deploy\n    events:\n      - type: manual\n    actions:\n      - script: \"echo shipped > {}\"\n    cooldown_secs: 0\n",
                deployed.display()
            ),
        );
        let result = cmd_trigger("deploy", &rules, &[], false, false);
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(
            std::fs::read_to_string(&deployed).unwrap().trim(),
            "shipped"
        );
    }

    #[test]
    fn trigger_with_payload() {
        let dir = tempfile::tempdir().unwrap();
        let deployed = dir.path().join("deployed.txt");
        let rules = write_rulebook(
            dir.path(),
            &format!(
                "rulebooks:\n  - name: deploy\n    events:\n      - type: manual\n    actions:\n      - script: \"echo shipped > {}\"\n    cooldown_secs: 0\n",
                deployed.display()
            ),
        );
        let payload = vec![("env".into(), "staging".into())];
        let result = cmd_trigger("deploy", &rules, &payload, false, true);
        assert!(result.is_ok(), "{result:?}");
        assert!(deployed.exists(), "the payload path did not execute either");
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

    // ====================================================================
    // Ledger: trigger-claims-actions-fired-but-does-nothing
    //
    // Every assertion below is on a SIDE EFFECT ON DISK. "1 action(s) fired"
    // was the only thing that ever happened; asserting on that banner is the
    // defect, not the test.
    // ====================================================================

    #[test]
    fn trigger_actually_runs_a_script_action() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("marker.txt");
        let rules = write_rulebook(
            dir.path(),
            &format!(
                "rulebooks:\n  - name: rb1\n    events:\n      - type: manual\n    actions:\n      - script: \"echo fired > {}\"\n    cooldown_secs: 0\n",
                marker.display()
            ),
        );

        cmd_trigger("rb1", &rules, &[], false, false).unwrap();

        assert!(
            marker.exists(),
            "trigger reported actions fired and ran nothing"
        );
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap().trim(),
            "fired",
            "the script action did not produce its effect"
        );
    }

    #[test]
    fn trigger_actually_runs_an_apply_action() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("f1.txt");
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            format!(
                "version: \"1.0\"\nname: t\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  f1:\n    type: file\n    machine: local\n    path: {}\n    content: \"hi\"\n",
                target.display()
            ),
        )
        .unwrap();
        let rules = write_rulebook(
            dir.path(),
            &format!(
                "rulebooks:\n  - name: rb1\n    events:\n      - type: manual\n    actions:\n      - apply:\n          file: {}\n    cooldown_secs: 0\n",
                config.display()
            ),
        );

        cmd_trigger("rb1", &rules, &[], false, false).unwrap();

        assert!(
            target.exists(),
            "the rulebook's apply action never converged its resource"
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap().trim(), "hi");
    }

    #[test]
    fn trigger_propagates_a_failing_action() {
        let dir = tempfile::tempdir().unwrap();
        let rules = write_rulebook(
            dir.path(),
            "rulebooks:\n  - name: rb1\n    events:\n      - type: manual\n    actions:\n      - script: \"exit 3\"\n    cooldown_secs: 0\n",
        );

        let result = cmd_trigger("rb1", &rules, &[], false, false);

        assert!(
            result.is_err(),
            "a failing action must reach the exit code, not be reported as fired"
        );
    }

    #[test]
    fn trigger_dry_run_executes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("must-not-exist.txt");
        let rules = write_rulebook(
            dir.path(),
            &format!(
                "rulebooks:\n  - name: rb1\n    events:\n      - type: manual\n    actions:\n      - script: \"echo oops > {}\"\n    cooldown_secs: 0\n",
                marker.display()
            ),
        );

        cmd_trigger("rb1", &rules, &[], true, false).unwrap();

        assert!(!marker.exists(), "--dry-run executed the action");
    }
}
