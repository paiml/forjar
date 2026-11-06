//! `plan --param k=v` reads the same desired state as `apply --param k=v`
//! (fleetops-2). `apply` had the flag and `plan` did not, so a plan could not
//! preview an overridden run. Observed through `--output-dir`: the exported
//! script is rendered from the (overridden) params.

use super::plan::{cmd_plan_with_pins, PlanVersionPins};
use std::path::Path;

fn plan_scripts(params: &[String]) -> Result<String, String> {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("forjar.yaml");
    let state = dir.path().join("state");
    let out = dir.path().join("scripts");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(
        &cfg,
        "version: \"1.0\"\nname: t\nparams:\n  k: base\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  f:\n    type: file\n    machine: m1\n    path: /tmp/fj-plan-param\n    content: \"val={{params.k}}\"\n",
    )
    .unwrap();
    cmd_plan_with_pins(
        &cfg,
        &state,
        None,
        None,
        None,
        false,
        false,
        Some(out.as_path()),
        None,
        None,
        true,
        None,
        false,
        &[],
        None,
        false,
        None,
        params,
        false,
        PlanVersionPins::Skip,
    )?;
    Ok(read_all(&out))
}

fn read_all(dir: &Path) -> String {
    let mut s = String::new();
    for e in std::fs::read_dir(dir).unwrap() {
        s.push_str(&std::fs::read_to_string(e.unwrap().path()).unwrap_or_default());
    }
    s
}

#[test]
fn plan_param_overrides_the_declared_value() {
    let scripts = plan_scripts(&["k=override".to_string()]).expect("plan runs");
    assert!(scripts.contains("val=override"), "{scripts}");
    assert!(!scripts.contains("val=base"));
}

#[test]
fn plan_without_param_keeps_the_declared_value() {
    assert!(plan_scripts(&[]).unwrap().contains("val=base"));
}

#[test]
fn plan_param_rejects_a_pair_without_equals() {
    let err = plan_scripts(&["nokv".to_string()]).unwrap_err();
    assert!(err.contains("expected KEY=VALUE"), "{err}");
}
