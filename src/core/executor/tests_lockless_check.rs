//! forjar#615: a lockless resource asks its `completion_check` before `create`.
//!
//! Each case runs a REAL check against `localhost`, so it measures what the
//! planner will be handed, not a claim about it.

use super::{record, RecordFilters};
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

#[test]
fn record_leaves_a_failing_check_unrecorded() {
    let config = cfg(Some("false"));
    let mut locks = HashMap::new();
    record(
        &config,
        None,
        None,
        None,
        RecordFilters::default(),
        &mut locks,
    );
    assert!(!locks.contains_key("local"));
}

#[test]
fn record_honours_non_default_resource_and_group_selectors() {
    // A passing check that `-r` / `-g` exclude is neither asked nor recorded;
    // the matching selector still records it (the other lane only passed defaults).
    let config = cfg(Some("true"));
    let run = |resource, group| {
        let mut locks = HashMap::new();
        record(
            &config,
            None,
            None,
            None,
            RecordFilters { resource, group },
            &mut locks,
        );
        locks.contains_key("local")
    };
    assert!(run(Some("guarded"), None), "matching -r records");
    assert!(!run(Some("other"), None), "non-matching -r is excluded");
    assert!(
        !run(None, Some("no-such-group")),
        "non-matching -g is excluded"
    );
}
