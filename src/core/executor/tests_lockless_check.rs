//! forjar#615: a lockless resource asks its `completion_check` before `create`.
//!
//! Each case runs a REAL check against `localhost`, so it measures what the
//! planner will be handed, not a claim about it.

use super::{record, seeded};
use crate::core::planner;
use crate::core::resolver;
use crate::core::types::*;
use std::collections::HashMap;

fn cfg(check: Option<&str>) -> ForjarConfig {
    let check = match check {
        Some(c) => format!("    completion_check: \"{c}\"\n"),
        None => String::new(),
    };
    serde_yaml_ng::from_str(&format!(
        "version: \"1.0\"\n\
         name: lockless\n\
         machines:\n\
         \x20 local:\n\
         \x20   hostname: localhost\n\
         \x20   addr: localhost\n\
         resources:\n\
         \x20 guarded:\n\
         \x20   type: task\n\
         \x20   machine: local\n\
         \x20   command: \"rm -rf /nonexistent/fj-615\"\n\
         {check}"
    ))
    .expect("fixture parses")
}

fn action(config: &ForjarConfig, locks: &HashMap<String, StateLock>) -> PlanAction {
    let order = resolver::build_execution_order(config).expect("orders");
    let plan = planner::plan(config, &order, locks, None);
    plan.changes
        .iter()
        .find(|c| c.resource_id == "guarded")
        .map(|c| c.action.clone())
        .expect("guarded is planned")
}

#[test]
fn a_lockless_task_whose_check_passes_plans_noop() {
    // THE DEFECT. No lock, check exits 0: before the fix the planner saw no
    // entry, planned Create, and the executor ran the command.
    let config = cfg(Some("true"));
    assert_eq!(
        action(&config, &HashMap::new()),
        PlanAction::Create,
        "premise"
    );
    let locks = seeded(&config, None, None, &HashMap::new());
    assert_eq!(action(&config, &locks), PlanAction::NoOp);
    let entry = &locks["local"].resources["guarded"];
    assert_eq!(entry.status, ResourceStatus::Converged);
    assert!(
        entry.applied_at.is_none(),
        "nothing was applied; nothing is dated"
    );
}

#[test]
fn a_lockless_task_whose_check_fails_still_plans_create() {
    let config = cfg(Some("false"));
    let locks = seeded(&config, None, None, &HashMap::new());
    assert_eq!(action(&config, &locks), PlanAction::Create);
}

#[test]
fn a_task_with_no_completion_check_is_not_asked() {
    // No declared "already done" test: the default path stays lock-relative.
    let config = cfg(None);
    let locks = seeded(&config, None, None, &HashMap::new());
    assert!(locks.get("local").is_none_or(|l| l.resources.is_empty()));
    assert_eq!(action(&config, &locks), PlanAction::Create);
}

#[test]
fn an_existing_lock_entry_keeps_the_locks_word() {
    // A resource the lock already knows about is not re-checked here — that is
    // what --refresh is for. Seed once from a passing check, then plan against a
    // config whose check now FAILS: the entry must survive untouched.
    let passing = cfg(Some("true"));
    let locks = seeded(&passing, None, None, &HashMap::new());
    let failing = cfg(Some("false"));
    let again = seeded(&failing, None, None, &locks);
    assert_eq!(
        again["local"].resources["guarded"].status,
        ResourceStatus::Converged
    );
}

#[test]
fn filters_narrow_who_is_asked() {
    let config = cfg(Some("true"));
    // A different machine selected: nothing on `local` is asked.
    let locks = seeded(&config, Some("elsewhere"), None, &HashMap::new());
    assert!(!locks.contains_key("local"));
    // A tag the resource does not carry: not asked.
    let locks = seeded(&config, None, Some("nope"), &HashMap::new());
    assert!(!locks.contains_key("local"));
}

#[test]
fn an_unreachable_machine_is_not_a_pass() {
    // "Could not look" is not evidence of convergence.
    let mut config = cfg(Some("true"));
    config.machines.clear();
    let locks = seeded(&config, None, None, &HashMap::new());
    assert!(!locks.contains_key("local"));
}

#[test]
fn record_writes_what_seeded_previews_into_the_locks_the_apply_saves() {
    // A skipped command must not also skip the record: without the entry the
    // lock never learns the guard exists and drift declines it for ever.
    let config = cfg(Some("true"));
    let mut locks = HashMap::new();
    let preview = seeded(&config, None, None, &locks);
    assert!(locks.is_empty(), "the preview must not write");
    let view = record(&config, None, None, &mut locks);
    let ids =
        |l: &HashMap<String, StateLock>| l["local"].resources.keys().cloned().collect::<Vec<_>>();
    assert_eq!(ids(&view), ids(&preview), "record and seeded must agree");
    let lock = &locks["local"];
    assert_eq!(lock.resources["guarded"].status, ResourceStatus::Converged);
    // The header a real apply writes (state::new_lock), not a synthetic one.
    assert_eq!(lock.schema, "1.0");
    assert_eq!(lock.hostname, "localhost");
    assert!(lock.generator.starts_with("forjar "), "{}", lock.generator);
}

#[test]
fn record_leaves_a_failing_check_unrecorded() {
    let config = cfg(Some("false"));
    let mut locks = HashMap::new();
    record(&config, None, None, &mut locks);
    assert!(!locks.contains_key("local"));
}
