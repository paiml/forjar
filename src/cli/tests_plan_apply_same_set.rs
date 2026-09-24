//! forjar#615: `plan` and `apply` compute the SAME resource set for `-r` / `-g`.
//!
//! `apply -r x` resolves its selection through `resolve_selection` (PMAT-160):
//! the matched resources closed over `depends_on`. `plan -r x` kept the exact id,
//! so on paiml/infra's lambda-labs `plan -r ollama-model-qwen35-4b` said
//! "1 to add" while `apply -r` of the same resource prompted "2 create" — the
//! second being `ollama-binary`, whose command removes a library tree and
//! restarts a shared daemon. The plan an operator gates on must describe the
//! apply. RED on the pre-fix `plan_selector`: every case with a dependency
//! loses it from the plan side.

use super::apply_selection::{resolve_selection, Selectors};
use crate::core::plan_selectors::PlanSelectors;
use crate::core::types;
use std::collections::{BTreeSet, HashMap};

fn res(deps: &[&str], group: Option<&str>) -> types::Resource {
    types::Resource {
        resource_type: types::ResourceType::File,
        machine: types::MachineTarget::Single("local".to_string()),
        path: Some("/tmp/fj-615-fixture".to_string()),
        content: Some("x\n".to_string()),
        depends_on: deps.iter().map(|d| (*d).to_string()).collect(),
        resource_group: group.map(str::to_string),
        ..Default::default()
    }
}

/// `model -> binary -> runtime`, `other -> binary`, `unrelated`. `model` is in
/// group `ml`; `binary` is not, so `-g ml` has a dependency outside its group.
fn cfg() -> types::ForjarConfig {
    let mut config = types::ForjarConfig {
        version: "1.0".to_string(),
        name: "same-set".to_string(),
        ..Default::default()
    };
    let machine: types::Machine =
        serde_yaml_ng::from_str("hostname: h\naddr: localhost\n").expect("machine parses");
    config.machines.insert("local".to_string(), machine);
    for (id, r) in [
        ("runtime", res(&[], None)),
        ("binary", res(&["runtime"], None)),
        ("model", res(&["binary"], Some("ml"))),
        ("other", res(&["binary"], None)),
        ("unrelated", res(&[], None)),
    ] {
        config.resources.insert(id.to_string(), r);
    }
    config
}

/// The ids `plan` would list: every change the filtered plan carries.
fn plan_set(resource: Option<&str>, group: Option<&str>) -> BTreeSet<String> {
    let config = cfg();
    let selectors = PlanSelectors::new(None, resource, None, group);
    let plan = super::plan_compute::plan_filtered(&config, &HashMap::new(), &selectors)
        .expect("plan filters");
    plan.changes.into_iter().map(|c| c.resource_id).collect()
}

/// The ids `apply` would execute: what `resolve_selection` leaves in the config.
fn apply_set(resource: Option<&str>, group: Option<&str>) -> BTreeSet<String> {
    let mut config = cfg();
    let sel = Selectors {
        resource,
        group,
        ..Default::default()
    };
    resolve_selection(&mut config, &sel, false).expect("apply resolves");
    config.resources.keys().cloned().collect()
}

fn set(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn plan_r_and_apply_r_select_the_same_set_over_a_depends_on_chain() {
    // The lambda-labs shape: the target plus its whole prerequisite chain.
    let want = set(&["binary", "model", "runtime"]);
    assert_eq!(
        apply_set(Some("model"), None),
        want,
        "apply's own set (the premise)"
    );
    assert_eq!(
        plan_set(Some("model"), None),
        apply_set(Some("model"), None),
        "`plan -r model` must list what `apply -r model` would execute"
    );
}

#[test]
fn plan_and_apply_agree_for_every_selector_shape() {
    for (r, g) in [
        (Some("model"), None),
        (Some("binary"), None),
        (Some("runtime"), None),
        (Some("unrelated"), None),
        (None, Some("ml")),
        (Some("model"), Some("ml")),
    ] {
        assert_eq!(plan_set(r, g), apply_set(r, g), "-r {r:?} -g {g:?}");
    }
}

#[test]
fn a_leaf_selection_still_keeps_only_itself() {
    // Non-regression: the closure is of DEPENDENCIES, never dependents.
    assert_eq!(plan_set(Some("runtime"), None), set(&["runtime"]));
    assert_eq!(plan_set(Some("unrelated"), None), set(&["unrelated"]));
}

#[test]
fn an_empty_intersection_is_an_error_on_both_sides() {
    // `-r unrelated -g ml`: each selector matches something; together, nothing.
    let config = cfg();
    let selectors = PlanSelectors::new(None, Some("unrelated"), None, Some("ml"));
    assert!(super::plan_compute::plan_filtered(&config, &HashMap::new(), &selectors).is_err());
    let mut config = cfg();
    let sel = Selectors {
        resource: Some("unrelated"),
        group: Some("ml"),
        ..Default::default()
    };
    assert!(resolve_selection(&mut config, &sel, false).is_err());
}
