//! GH-208 dogfood, partition P1 (apply / plan / make / dispatch).
//!
//! One test per root cause. Each was verified RED by reverting the fix and
//! watching it fail; the assertion text records the measurement from the
//! published 1.12.3 binary that motivated it.

use crate::core::types::{ExecutionPlan, ForjarConfig, PlanAction, PlannedChange, ResourceType};

fn cfg(yaml: &str) -> ForjarConfig {
    serde_yaml_ng::from_str(yaml).expect("fixture parses")
}

fn three_resource_config() -> ForjarConfig {
    cfg("version: \"1.0\"\n\
         name: repro\n\
         params:\n\
         \x20 sandbox: /tmp/fj-unit\n\
         machines:\n\
         \x20 local:\n\
         \x20   hostname: localhost\n\
         \x20   addr: 127.0.0.1\n\
         \x20   user: nobody\n\
         \x20   arch: x86_64\n\
         resources:\n\
         \x20 a-file:\n\
         \x20   type: file\n\
         \x20   machine: local\n\
         \x20   resource_group: alpha\n\
         \x20   path: \"{{params.sandbox}}/a.txt\"\n\
         \x20   content: \"aaa\\n\"\n\
         \x20 b-file:\n\
         \x20   type: file\n\
         \x20   machine: local\n\
         \x20   path: \"{{params.sandbox}}/b.txt\"\n\
         \x20   content: \"bbb\\n\"\n\
         \x20 c-file:\n\
         \x20   type: file\n\
         \x20   machine: local\n\
         \x20   path: \"{{params.sandbox}}/c.txt\"\n\
         \x20   content: \"ccc\\n\"\n")
}

fn change(id: &str, action: PlanAction) -> PlannedChange {
    PlannedChange {
        resource_id: id.to_string(),
        machine: "local".to_string(),
        resource_type: ResourceType::File,
        action,
        description: format!("{id}: create /tmp/fj-unit/{id}"),
    }
}

fn three_change_plan() -> ExecutionPlan {
    ExecutionPlan {
        name: "repro".to_string(),
        changes: vec![
            change("a-file", PlanAction::Create),
            change("b-file", PlanAction::Create),
            change("c-file", PlanAction::Create),
        ],
        execution_order: vec![
            "a-file".to_string(),
            "b-file".to_string(),
            "c-file".to_string(),
        ],
        to_create: 3,
        to_update: 0,
        to_destroy: 0,
        unchanged: 0,
    }
}

// ── GH-214: plan -r / -g were "not yet implemented … Flag ignored" ──

#[test]
fn plan_resource_filter_keeps_only_its_own_resource() {
    // 1.12.3: "Warning: --resource filter is not yet implemented for plan.
    // Flag ignored." followed by all three resources, while `apply -r` filtered.
    let mut plan = three_change_plan();
    super::plan_selector::apply_resource_filter(&mut plan, &three_resource_config(), Some("a-file"))
        .expect("a-file exists");
    assert_eq!(plan.changes.len(), 1, "-r must drop the other two");
    assert_eq!(plan.changes[0].resource_id, "a-file");
    assert_eq!(plan.to_create, 1, "the summary counters must agree with the body");
    assert_eq!(plan.execution_order, vec!["a-file".to_string()]);
}

#[test]
fn plan_resource_filter_with_no_match_is_an_error() {
    let mut plan = three_change_plan();
    let err =
        super::plan_selector::apply_resource_filter(&mut plan, &three_resource_config(), Some("nope"))
            .expect_err("a typo must not print an empty successful plan");
    assert!(err.contains("a-file"), "the error names what IS available: {err}");
}

#[test]
fn plan_without_a_resource_filter_keeps_everything() {
    // Non-regression: "filters" must not mean "always empties".
    let mut plan = three_change_plan();
    super::plan_selector::apply_resource_filter(&mut plan, &three_resource_config(), None)
        .expect("no filter");
    assert_eq!(plan.changes.len(), 3);
    assert_eq!(plan.to_create, 3);
}

#[test]
fn plan_group_filter_keeps_only_that_group() {
    let mut plan = three_change_plan();
    super::plan_selector::apply_group_filter(&mut plan, &three_resource_config(), Some("alpha"))
        .expect("group alpha exists");
    assert_eq!(plan.changes.len(), 1, "only a-file is in group alpha");
    assert_eq!(plan.changes[0].resource_id, "a-file");
    assert_eq!(plan.to_create, 1);
}

#[test]
fn plan_group_filter_with_no_match_is_an_error() {
    let mut plan = three_change_plan();
    assert!(
        super::plan_selector::apply_group_filter(&mut plan, &three_resource_config(), Some("ghost"))
            .is_err(),
        "1.12.3 printed the WHOLE plan for a group that does not exist"
    );
}

// ── GH-212: plan named the unresolved template as the target path ──

#[test]
fn plan_descriptions_name_the_path_apply_will_write() {
    // 1.12.3: "+ a-file: create {{params.sandbox}}/a.txt" in text, --json and
    // --why, while `show` and the apply itself used /tmp/fj-unit/a.txt.
    let config = three_resource_config();
    let order = crate::core::resolver::build_execution_order(&config).expect("order");
    let plan = crate::core::planner::plan(&config, &order, &Default::default(), None);
    let desc = &plan
        .changes
        .iter()
        .find(|c| c.resource_id == "a-file")
        .expect("a-file planned")
        .description;
    assert!(
        !desc.contains("{{"),
        "plan is the pre-flight review surface; it must not show a template: {desc}"
    );
    assert!(desc.contains("/tmp/fj-unit/a.txt"), "{desc}");
}

// ── GH-210: --force reported 0 actual changes while restoring a tampered file ──

#[test]
fn a_drifted_resource_is_not_a_forced_noop() {
    // 1.12.3: "note: --force re-ran 3 resource(s) ... (0 actual change(s), 3
    // forced no-op(s))" for a run that demonstrably rewrote a tampered file.
    let changes = vec![
        change("a-file", PlanAction::NoOp),
        change("b-file", PlanAction::NoOp),
        change("c-file", PlanAction::NoOp),
    ];
    let drifted: std::collections::HashSet<String> = ["a-file".to_string()].into_iter().collect();
    assert_eq!(
        crate::core::executor::count_forced_noops(&changes, &drifted),
        2,
        "the resource the machine disagrees about was NOT a no-op"
    );
}

#[test]
fn a_fully_converged_stack_is_all_forced_noops() {
    // Non-regression: the C3 signal the count exists for must survive.
    let changes = vec![
        change("a-file", PlanAction::NoOp),
        change("b-file", PlanAction::NoOp),
    ];
    assert_eq!(
        crate::core::executor::count_forced_noops(&changes, &Default::default()),
        2
    );
}

#[test]
fn a_created_resource_is_never_a_forced_noop() {
    let changes = vec![change("a-file", PlanAction::Create)];
    assert_eq!(
        crate::core::executor::count_forced_noops(&changes, &Default::default()),
        0,
        "a creation is an actual change"
    );
}

// ── GH-210: --dry-run / make -n printed one line and no actions ──

#[test]
fn dry_run_shows_what_would_run() {
    // 1.12.3 printed exactly "Dry run — no changes applied." for both
    // `apply --dry-run` and `make -n`, though both help strings promise the
    // actions.
    let out = super::apply_dry_run::render_dry_run_actions(&three_change_plan());
    for id in ["a-file", "b-file", "c-file"] {
        assert!(out.contains(id), "dry run must name {id}: {out}");
    }
    assert!(out.contains("No changes applied."), "{out}");
}

#[test]
fn an_empty_dry_run_says_so_rather_than_looking_broken() {
    let empty = ExecutionPlan {
        changes: vec![],
        execution_order: vec![],
        to_create: 0,
        ..three_change_plan()
    };
    let out = super::apply_dry_run::render_dry_run_actions(&empty);
    assert!(out.contains("nothing selected"), "{out}");
}

// ── GH-211: --notify-webhook-headers was parsed and dropped ──

#[test]
fn a_malformed_header_json_is_refused_before_the_apply() {
    let args = super::commands::ApplyArgs {
        notify_webhook_headers: Some("X-Auth: nope".to_string()),
        ..Default::default()
    };
    let err = super::dispatch_apply_b::validate_notify_headers(&args)
        .expect_err("a header value that cannot be sent must not reach a live apply");
    assert!(err.contains("--notify-webhook-headers"), "{err}");
}

#[test]
fn a_well_formed_header_json_is_accepted() {
    // Non-regression: the validation must not reject the documented form.
    let args = super::commands::ApplyArgs {
        notify_webhook_headers: Some(r#"{"X-Auth":"SECRET123"}"#.to_string()),
        ..Default::default()
    };
    assert!(super::dispatch_apply_b::validate_notify_headers(&args).is_ok());
    assert!(
        super::dispatch_apply_b::validate_notify_headers(&super::commands::ApplyArgs::default())
            .is_ok()
    );
}
