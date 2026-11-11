//! Review findings on PR #899 (PMAT-890), each pinned by a test that fails
//! without its fix.
//!
//! * lane 1: `plan -t TAG` must select the same set as `apply -t TAG`, the
//!   tagged resources closed over `depends_on`, and an unknown tag must be an
//!   error as it is for `apply`.
//! * lane 3: `plan --param k=v --out p.json` must not write a sealed plan that
//!   `apply --plan-file` (which rebuilds the config without `--param`) can
//!   only reject with `PLAN_HASH_MISMATCH`; the pair is refused up front, as
//!   `--refresh --out` is.

use super::plan::{cmd_plan_with_pins, PlanVersionPins};
use super::plan_compute::plan_filtered;
use crate::core::plan_selectors::PlanSelectors;
use crate::core::types;
use std::collections::HashMap;

fn res(deps: &[&str], tags: &[&str]) -> types::Resource {
    types::Resource {
        resource_type: types::ResourceType::File,
        machine: types::MachineTarget::Multiple(vec!["local".to_string()]),
        depends_on: deps.iter().map(|d| (*d).to_string()).collect(),
        tags: tags.iter().map(|t| (*t).to_string()).collect(),
        ..Default::default()
    }
}

/// `b` (tagged `deploy`) depends on `a` (untagged); `c` is unrelated.
fn cfg() -> types::ForjarConfig {
    let mut config = types::ForjarConfig {
        version: "1.0".to_string(),
        name: "tag-closure".to_string(),
        ..Default::default()
    };
    config.machines.insert(
        "local".to_string(),
        serde_yaml_ng::from_str("hostname: h\naddr: localhost\n").expect("machine parses"),
    );
    config.resources.insert("a".to_string(), res(&[], &[]));
    config
        .resources
        .insert("b".to_string(), res(&["a"], &["deploy"]));
    config.resources.insert("c".to_string(), res(&[], &[]));
    config
}

#[test]
fn plan_dash_t_includes_the_depends_on_closure_like_apply() {
    let config = cfg();
    let sel = PlanSelectors::new(None, None, Some("deploy"), None);
    let plan = plan_filtered(&config, &HashMap::new(), &sel).expect("plans");
    let mut ids: Vec<&str> = plan
        .changes
        .iter()
        .map(|c| c.resource_id.as_str())
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, ["a", "b"], "a is b's prerequisite; c is unrelated");
    assert_eq!(plan.to_create, 2, "the summary counters follow the body");
}

#[test]
fn plan_dash_t_with_an_unknown_tag_is_an_error_like_apply() {
    let config = cfg();
    let sel = PlanSelectors::new(None, None, Some("nope"), None);
    let err = plan_filtered(&config, &HashMap::new(), &sel).unwrap_err();
    assert!(err.contains("--tag 'nope' matches no resource"), "{err}");
}

#[test]
fn plan_param_with_out_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("forjar.yaml");
    let state = dir.path().join("state");
    let out = dir.path().join("p.json");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(
        &cfg_path,
        "version: \"1.0\"\nname: t\nparams:\n  k: base\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  f:\n    type: file\n    machine: m1\n    path: /tmp/fj-plan-param-out\n    content: \"val={{params.k}}\"\n",
    )
    .unwrap();
    let err = cmd_plan_with_pins(
        &cfg_path,
        &state,
        None,
        None,
        None,
        false,
        false,
        None,
        None,
        None,
        true,
        None,
        false,
        &[],
        Some(out.as_path()),
        false,
        None,
        &["k=override".to_string()],
        false,
        PlanVersionPins::Skip,
    )
    .unwrap_err();
    assert!(err.contains("plan --param --out"), "{err}");
    assert!(!out.exists(), "no sealed plan may be written");
}
