//! GH-211: a flag that parses and does nothing is refused instead.
//!
//! Family A of the 1.12.3 dogfood is 15 defects with one shape: a field is
//! declared on a clap args struct, appears in `--help` with an FJ- ticket
//! number, is accepted, exits 0 — and is read by nothing. `--pre-check` was a
//! gate that could never block. `--approval-webhook` (FJ-546, "POST for
//! approval before applying (GitOps gate)") made zero HTTP requests and applied
//! unconditionally, whether the approval endpoint denied with 500 or did not
//! exist. `--notify-file`, `--notify-log`, `--notify-exec` and `--log-file`
//! produced no file, no JSON and no subprocess while apply reported success.
//!
//! # Why refuse rather than warn
//!
//! An inert flag is worse than a missing one. `forjar apply --approval-webhook
//! ...` exiting 0 tells an operator that change control is enforced, and they
//! stop watching. A warning on stderr does not help: CI reads the exit code,
//! and every smoke test that runs the flag and checks `rc == 0` passes today.
//! The only signal a machine cannot ignore is a non-zero exit, so that is what
//! an unimplemented flag now produces, naming itself.
//!
//! This module is deliberately the ONLY place these fields are read. The guard
//! test `every_declared_flag_is_consumed_or_refused` excludes this file when it
//! looks for consumers, so a field read here and nowhere else counts as
//! REFUSED, a field read elsewhere counts as IMPLEMENTED, and a field read in
//! both is an error — that is the state where a flag was implemented and the
//! refusal was left behind to block it. There is no fourth state and no
//! hand-maintained list to drift out of date.
//!
//! Implementing one of these is a strict improvement: delete its line here,
//! drop the `[UNIMPLEMENTED]` marker from its doc comment in `apply_args.rs`,
//! and the guard flips it from refused to consumed on its own.

use super::commands::ApplyArgs;

/// Every `ApplyArgs` field with no dispatch site, paired with the flag that
/// sets it. The bool is "the operator supplied it".
fn inert_apply_flags(a: &ApplyArgs) -> [(&'static str, bool); FLAG_COUNT] {
    [
        ("--tag-filter", a.tag_filter.is_some()),
        ("--resume", a.resume),
        ("--confirm", a.confirm),
        ("--max-failures", a.max_failures.is_some()),
        ("--rate-limit", a.rate_limit.is_some()),
        ("--label", !a.labels.is_empty()),
        ("--concurrency", a.concurrency.is_some()),
        ("--rollback-snapshot", a.rollback_snapshot.is_some()),
        ("--retry-delay", a.retry_delay.is_some()),
        ("--tags", !a.tags.is_empty()),
        ("--log-file", a.log_file.is_some()),
        ("--comment", a.comment.is_some()),
        ("--only-changed", a.only_changed),
        ("--approval-required", a.approval_required),
        ("--canary-percent", a.canary_percent.is_some()),
        ("--schedule", a.schedule.is_some()),
        ("--batch-size", a.batch_size.is_some()),
        ("--rollback-on-threshold", a.rollback_on_threshold.is_some()),
        ("--metrics-port", a.metrics_port.is_some()),
        ("--circuit-breaker", a.circuit_breaker.is_some()),
        ("--require-approval", a.require_approval.is_some()),
        ("--change-window", a.change_window.is_some()),
        ("--max-duration", a.max_duration.is_some()),
        ("--rate-limit-resources", a.rate_limit_resources.is_some()),
        ("--checkpoint-interval", a.checkpoint_interval.is_some()),
        ("--blue-green", a.blue_green.is_some()),
        ("--progressive", a.progressive.is_some()),
        ("--approval-webhook", a.approval_webhook.is_some()),
        ("--sign-off", a.sign_off.is_some()),
        ("--runbook", a.runbook.is_some()),
        ("--fleet-strategy", a.fleet_strategy.is_some()),
        ("--pre-check", a.pre_check.is_some()),
        ("--post-check", a.post_check.is_some()),
        ("--max-retries", a.max_retries.is_some()),
        ("--rollback-window", a.rollback_window.is_some()),
        ("--approval-timeout", a.approval_timeout.is_some()),
        ("--checkpoint", a.checkpoint.is_some()),
        ("--gate", a.gate.is_some()),
        ("--explain", a.explain),
        ("--summary-only", a.summary_only),
        ("--pre-apply-hook", a.pre_apply_hook.is_some()),
        ("--post-apply-hook", a.post_apply_hook.is_some()),
        ("--canary-resource", a.canary_resource.is_some()),
        ("--timeout-per-resource", a.timeout_per_resource.is_some()),
        ("--skip-unchanged", a.skip_unchanged),
        ("--retry-backoff", a.retry_backoff.is_some()),
        ("--plan-output-file", a.plan_output_file.is_some()),
        ("--resource-priority", !a.resource_priority.is_empty()),
        ("--apply-window", a.apply_window.is_some()),
        ("--fail-fast-machine", a.fail_fast_machine),
        ("--cooldown", a.cooldown.is_some()),
        (
            "--notify-webhook-headers",
            a.notify_webhook_headers.is_some(),
        ),
        ("--notify-log", a.notify_log.is_some()),
        ("--notify-exec", a.notify_exec.is_some()),
        ("--notify-file", a.notify_file.is_some()),
        ("--notify-json", a.notify_json),
        ("--notify-slack-webhook", a.notify_slack_webhook.is_some()),
        ("--notify-telegram", a.notify_telegram.is_some()),
        ("--notify-webhook-v2", a.notify_webhook_v2.is_some()),
    ]
}

/// Number of refused `apply` flags. Kept as a named constant so the array
/// length and the table cannot silently disagree.
const FLAG_COUNT: usize = 59;

/// Refuse the run if any unimplemented flag was supplied.
///
/// Reports ALL of them at once rather than the first: an operator whose script
/// passes four inert flags should learn that in one run, not four.
pub(crate) fn reject_inert_apply_flags(a: &ApplyArgs) -> Result<(), String> {
    let supplied: Vec<&str> = inert_apply_flags(a)
        .into_iter()
        .filter_map(|(flag, on)| on.then_some(flag))
        .collect();
    if supplied.is_empty() {
        return Ok(());
    }
    Err(unimplemented_message(&supplied))
}

/// The refusal text. Names every offending flag and says what did NOT happen.
fn unimplemented_message(flags: &[&str]) -> String {
    format!(
        "{} not implemented in this build: {}.\n\
         Nothing was done. These flags parse but have no effect, so forjar \
         refuses rather than exiting 0 having ignored them (GH-211). Remove \
         them, or track the implementation at \
         https://github.com/paiml/forjar/issues/211",
        if flags.len() == 1 { "Flag" } else { "Flags" },
        flags.join(", ")
    )
}

/// GH-211: the same refusal for the three inert flags outside `apply`.
///
/// Two of them were already *known* inert and marked as such in the dispatcher
/// — `lint --rules` destructured to `rules: _rules`, `validate --schema-version`
/// to `schema_version: _schema_version`. An underscore binding silences rustc;
/// it does not tell the operator anything. The third, `status
/// --dependency-count`, was hidden behind a `..` rest pattern, which silences
/// rustc without even leaving a marker in the source — that is the shape the
/// guard test exists to find.
pub(crate) fn reject_inert_flag(flag: &str, supplied: bool) -> Result<(), String> {
    if supplied {
        return Err(unimplemented_message(&[flag]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clean_apply_is_not_refused() {
        assert!(reject_inert_apply_flags(&ApplyArgs::default()).is_ok());
    }

    #[test]
    fn an_inert_option_flag_is_refused_by_name() {
        let a = ApplyArgs {
            approval_webhook: Some("http://127.0.0.1:1/approve".to_string()),
            ..Default::default()
        };
        let err = reject_inert_apply_flags(&a).expect_err("a gate that cannot deny must not run");
        assert!(err.contains("--approval-webhook"), "{err}");
        assert!(err.contains("Nothing was done"), "{err}");
    }

    #[test]
    fn an_inert_bool_flag_is_refused_by_name() {
        let a = ApplyArgs {
            explain: true,
            ..Default::default()
        };
        let err = reject_inert_apply_flags(&a).expect_err("--explain explains nothing");
        assert!(err.contains("--explain"), "{err}");
    }

    #[test]
    fn an_inert_vec_flag_is_refused_only_when_supplied() {
        let a = ApplyArgs {
            labels: vec!["owner=alice".to_string()],
            ..Default::default()
        };
        let err = reject_inert_apply_flags(&a).expect_err("--label is recorded nowhere");
        assert!(err.contains("--label"), "{err}");
        assert!(reject_inert_apply_flags(&ApplyArgs::default()).is_ok());
    }

    #[test]
    fn every_supplied_inert_flag_is_reported_in_one_run() {
        let a = ApplyArgs {
            notify_json: true,
            pre_check: Some("true".to_string()),
            runbook: Some("http://rb/x".to_string()),
            ..Default::default()
        };
        let err = reject_inert_apply_flags(&a).unwrap_err();
        for f in ["--notify-json", "--pre-check", "--runbook"] {
            assert!(err.contains(f), "{f} missing from: {err}");
        }
        assert!(err.starts_with("Flags"), "plural form expected: {err}");
    }

    #[test]
    fn the_table_covers_the_declared_count() {
        assert_eq!(inert_apply_flags(&ApplyArgs::default()).len(), FLAG_COUNT);
    }

    #[test]
    fn a_non_apply_inert_flag_is_refused_by_name() {
        let err = reject_inert_flag("--rules", true).expect_err("lint --rules loads nothing");
        assert!(err.contains("--rules"), "{err}");
        assert!(err.starts_with("Flag "), "singular form expected: {err}");
        assert!(reject_inert_flag("--rules", false).is_ok());
    }
}
