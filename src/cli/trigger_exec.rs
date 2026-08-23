//! FJ-3107: the executor `forjar trigger` never had.
//!
//! # Why this file exists
//!
//! `trigger <rulebook>` enumerated the rulebook's matched actions, counted
//! them, printed `1 action(s) fired` / `"fired": true` and exited 0 — having
//! dispatched nothing. No resource converged, no state directory appeared, and
//! `forjar apply` in the same directory did create the file the rulebook's
//! apply action named. The count was a count of DECLARATIONS, not of work.
//!
//! Everything here therefore returns a `Result` per action, and the banner
//! `trigger` prints is derived from those results. A rulebook whose action
//! fails makes `trigger` exit non-zero.
//!
//! # ⚠️ Payload templates are deliberately NOT expanded
//!
//! `rulebook_template::expand_action` substitutes event payload keys into
//! `RulebookAction.script` with `String::replace` and no shell quoting. That is
//! an injection whenever the payload is attacker-controlled, which is why
//! `cli::rules_serve` (the network receiver) refuses to execute at all. This
//! module is reached only from the local `forjar trigger` CLI, whose payload
//! comes from the operator's own `--payload` flags — and it still does not call
//! `expand_action`. The action text executed here is exactly the text in the
//! operator's own rulebook file, which is the same trust level as the
//! `command:` in their own `forjar.yaml`.

use crate::core::types::{ApplyAction, DestroyAction, RulebookAction};
use std::path::{Path, PathBuf};

/// What one action's execution actually did.
pub(crate) struct ActionOutcome {
    /// Position within the rulebook's action list.
    pub index: usize,
    /// `apply` / `destroy` / `script` / `notify`.
    pub kind: String,
    /// `Ok` only when the action ran and succeeded.
    pub result: Result<(), String>,
}

impl ActionOutcome {
    /// True when this action ran to a successful completion.
    pub(crate) fn succeeded(&self) -> bool {
        self.result.is_ok()
    }
}

/// The state directory a rulebook-driven apply/destroy writes to.
///
/// It is anchored to the config the action names rather than to the process
/// CWD, because a rulebook can be triggered from anywhere (and a daemon will
/// be). `forjar apply -f x/forjar.yaml` run from `x/` uses `x/state`, so for
/// the common case — the ledger's repro included — this is the same directory
/// the operator would get by hand.
fn action_state_dir(config_file: &Path) -> PathBuf {
    match config_file.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.join("state"),
        _ => PathBuf::from("state"),
    }
}

/// The one tag `cmd_apply` can filter on.
///
/// `passes_tag_filter` compares a single tag for equality, so a two-tag action
/// cannot be honoured. Applying the first one and ignoring the rest would widen
/// the blast radius silently, so this refuses instead.
fn single_tag(tags: &[String]) -> Result<Option<&str>, String> {
    match tags {
        [] => Ok(None),
        [one] => Ok(Some(one.as_str())),
        many => Err(format!(
            "apply action lists {} tags ({}); forjar can filter on exactly one — \
             split it into one action per tag",
            many.len(),
            many.join(", ")
        )),
    }
}

/// Run a rulebook `apply:` action.
///
/// The nested apply prints its own human summary even when `trigger --json` was
/// asked for, so a `--json` consumer sees that text ahead of the JSON document.
/// That is the same trade-off `drift`'s auto-remediation already makes
/// (`run_drift_remediation` passes `json=false` "remediation output is text"),
/// and it is stated here rather than hidden: an apply that ran silently would
/// be harder to debug than one whose output is in the wrong shape.
fn run_apply(action: &ApplyAction) -> Result<(), String> {
    let file = Path::new(&action.file);
    if !file.exists() {
        return Err(format!(
            "apply action names a config that does not exist: {}",
            file.display()
        ));
    }
    let tag = single_tag(&action.tags)?;
    let state_dir = action_state_dir(file);
    std::fs::create_dir_all(&state_dir)
        .map_err(|e| format!("create state dir {}: {e}", state_dir.display()))?;

    super::apply::cmd_apply(
        file,
        &state_dir,
        action.machine.as_deref(),
        None,  // resource filter — `subset` is expressed as goals below
        tag,   // tag filter
        None,  // group filter
        false, // not forced: a rulebook apply converges, it does not re-apply
        false, // not dry-run — that is `trigger --dry-run`
        false, // tripwire on
        &[],   // no param overrides
        false, // no auto-commit
        None,  // no timeout
        false, // text output; the trigger banner is the summary
        false, // not verbose
        None,  // no env_file
        None,  // no workspace
        false, // no report
        false, // no force_unlock
        None,  // no output mode
        false, // no progress
        false, // no timing
        0,     // no retry
        true,  // yes: the operator already confirmed by typing `trigger`
        false, // not parallel
        None,  // no resource_timeout
        false, // no rollback_on_failure
        None,  // no max_parallel
        None,  // no notify
        None,  // no subset glob
        false, // confirm_destructive
        None,  // no exclude
        false, // sequential (ignored)
        None,  // no telemetry endpoint
        false, // no refresh
        None,  // no force_tag
        &action.subset,
    )
}

/// Run a rulebook `destroy:` action.
///
/// `cmd_destroy` has no resource-subset parameter: it tears down everything in
/// the config. An action naming three of ten resources would therefore destroy
/// all ten, so a scoped destroy is REFUSED rather than silently widened. That
/// is the same reasoning as [`single_tag`] — for a destructive verb, "does
/// more than asked" is the one failure mode that must not be reachable.
fn run_destroy(action: &DestroyAction) -> Result<(), String> {
    if !action.resources.is_empty() {
        return Err(format!(
            "destroy action names {} resource(s) ({}), but `forjar destroy` cannot \
             scope to a subset — it would destroy the whole config. Refusing.",
            action.resources.len(),
            action.resources.join(", ")
        ));
    }
    let file = Path::new(&action.file);
    if !file.exists() {
        return Err(format!(
            "destroy action names a config that does not exist: {}",
            file.display()
        ));
    }
    let state_dir = action_state_dir(file);
    super::destroy::cmd_destroy(file, &state_dir, None, true, false)
}

/// Run a rulebook `script:` action on the local machine.
fn run_script(script: &str) -> Result<(), String> {
    let machine = super::check::localhost_machine();
    let mut body = String::from("set -euo pipefail\n");
    body.push_str(script);
    if !body.ends_with('\n') {
        body.push('\n');
    }

    let out = crate::transport::exec_script(&machine, &body)
        .map_err(|e| format!("script action execution error: {e}"))?;

    if !out.stdout.is_empty() {
        print!("{}", out.stdout);
    }
    if !out.stderr.is_empty() {
        eprint!("{}", out.stderr);
    }
    if out.success() {
        Ok(())
    } else {
        Err(format!("script action failed with exit {}", out.exit_code))
    }
}

/// Execute one action.
///
/// `notify` is the one action kind with no implementation behind it: forjar has
/// no outbound notification transport (the webhook code in `core` is a
/// RECEIVER). Reporting it as fired would be the very defect this module was
/// written to remove, so it fails loudly and says what to use instead.
fn execute_action(action: &RulebookAction) -> Result<(), String> {
    if let Some(apply) = &action.apply {
        return run_apply(apply);
    }
    if let Some(destroy) = &action.destroy {
        return run_destroy(destroy);
    }
    if let Some(script) = &action.script {
        return run_script(script);
    }
    if let Some(notify) = &action.notify {
        return Err(format!(
            "notify action (channel '{}') cannot be executed: forjar has no outbound \
             notification transport. Use a `script:` action that calls your notifier.",
            notify.channel
        ));
    }
    Err("action declares none of apply/destroy/script/notify".to_string())
}

/// Execute a rulebook's actions in declaration order, stopping at the first
/// failure.
///
/// Fail-fast matters here: rulebook actions are ordered and a later one
/// routinely assumes the earlier one converged. Actions after a failure are
/// reported as not run rather than attempted.
pub(crate) fn execute_actions(actions: &[RulebookAction]) -> Vec<ActionOutcome> {
    let mut outcomes = Vec::with_capacity(actions.len());
    let mut halted = false;

    for (index, action) in actions.iter().enumerate() {
        let kind = action.action_type().to_string();
        if halted {
            outcomes.push(ActionOutcome {
                index,
                kind,
                result: Err("not run: an earlier action in this rulebook failed".to_string()),
            });
            continue;
        }
        let result = execute_action(action);
        halted = result.is_err();
        outcomes.push(ActionOutcome {
            index,
            kind,
            result,
        });
    }

    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::NotifyAction;

    fn script_action(script: &str) -> RulebookAction {
        RulebookAction {
            apply: None,
            destroy: None,
            script: Some(script.to_string()),
            notify: None,
        }
    }

    #[test]
    fn state_dir_is_beside_the_config() {
        assert_eq!(
            action_state_dir(Path::new("/a/b/forjar.yaml")),
            PathBuf::from("/a/b/state")
        );
    }

    #[test]
    fn state_dir_falls_back_to_cwd_relative() {
        assert_eq!(
            action_state_dir(Path::new("forjar.yaml")),
            PathBuf::from("state")
        );
    }

    #[test]
    fn single_tag_accepts_none_and_one() {
        assert_eq!(single_tag(&[]).unwrap(), None);
        assert_eq!(single_tag(&["web".to_string()]).unwrap(), Some("web"));
    }

    #[test]
    fn single_tag_refuses_two() {
        let err = single_tag(&["a".to_string(), "b".to_string()]).unwrap_err();
        assert!(err.contains("exactly one"), "{err}");
    }

    #[test]
    fn script_action_produces_its_effect() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("m.txt");
        let outcomes = execute_actions(&[script_action(&format!("echo x > {}", marker.display()))]);
        assert_eq!(outcomes.len(), 1);
        assert!(outcomes[0].succeeded(), "{:?}", outcomes[0].result);
        assert!(marker.exists());
    }

    #[test]
    fn a_failure_halts_the_remaining_actions() {
        let dir = tempfile::tempdir().unwrap();
        let never = dir.path().join("never.txt");
        let outcomes = execute_actions(&[
            script_action("exit 7"),
            script_action(&format!("echo x > {}", never.display())),
        ]);
        assert!(!outcomes[0].succeeded());
        assert!(!outcomes[1].succeeded());
        assert!(
            !never.exists(),
            "an action after a failed one was executed anyway"
        );
    }

    #[test]
    fn notify_is_refused_not_reported_as_fired() {
        let outcomes = execute_actions(&[RulebookAction {
            apply: None,
            destroy: None,
            script: None,
            notify: Some(NotifyAction {
                channel: "slack".into(),
                message: "hi".into(),
            }),
        }]);
        let err = outcomes[0].result.as_ref().unwrap_err();
        assert!(err.contains("no outbound"), "{err}");
    }

    #[test]
    fn empty_action_is_an_error() {
        let outcomes = execute_actions(&[RulebookAction {
            apply: None,
            destroy: None,
            script: None,
            notify: None,
        }]);
        assert!(!outcomes[0].succeeded());
    }

    #[test]
    fn apply_action_with_a_missing_config_fails() {
        let outcomes = execute_actions(&[RulebookAction {
            apply: Some(ApplyAction {
                file: "/nonexistent/forjar.yaml".into(),
                subset: Vec::new(),
                tags: Vec::new(),
                machine: None,
            }),
            destroy: None,
            script: None,
            notify: None,
        }]);
        let err = outcomes[0].result.as_ref().unwrap_err();
        assert!(err.contains("does not exist"), "{err}");
    }

    #[test]
    fn scoped_destroy_is_refused() {
        let outcomes = execute_actions(&[RulebookAction {
            apply: None,
            destroy: Some(DestroyAction {
                file: "forjar.yaml".into(),
                resources: vec!["a".into()],
            }),
            script: None,
            notify: None,
        }]);
        let err = outcomes[0].result.as_ref().unwrap_err();
        assert!(err.contains("Refusing"), "{err}");
    }
}
