//! Every script forjar GENERATES must survive the gate forjar puts in front of
//! executing it.
//!
//! forjar#345 shipped a `_fj_crates_ok` probe whose cleanup was `rm -rf "$_vh"`.
//! bashrs rates SEC011 ("missing validation before rm -rf") at Error severity,
//! and `validate_before_exec` refuses any script carrying an Error diagnostic —
//! so every `provider: cargo` apply script became unrunnable. `cargo test`,
//! `cargo clippy` and that change's own falsification test all passed while the
//! resource could not execute at all, because NOTHING in this repo pushed a
//! generated script through `purifier::validate_script`.
//!
//! That is the gap this file closes. It is deliberately generic: it walks a
//! table of resources and asserts the property for all three generated scripts,
//! so a future provider that emits shell bashrs rejects is a red test here
//! rather than a transport error on somebody's machine.
//!
//! It calls `validate_before_exec` — not `validate_script` — because the thing
//! that must hold is the composition: `strip_data_payloads` runs first, and
//! #345's regression was precisely that the strip's whitelist did not cover the
//! probe's temp variable.

use super::validate_before_exec;
use crate::core::codegen::{apply_script, check_script, state_query_script};
use crate::core::types::{MachineTarget, Resource, ResourceType};

/// A resource with only the fields a generator needs, everything else default.
fn res(resource_type: ResourceType, state: Option<&str>, f: impl FnOnce(&mut Resource)) -> Resource {
    let mut r = Resource {
        resource_type,
        machine: MachineTarget::Single("m1".to_string()),
        state: state.map(str::to_string),
        ..Default::default()
    };
    f(&mut r);
    r
}

/// The table. Each entry is (label, resource). Add a row when you add a
/// provider that emits shell.
fn corpus() -> Vec<(String, Resource)> {
    let mut out = Vec::new();

    // THE #345 REGRESSION. `provider: cargo, state: present` is the exact shape
    // that became unrunnable, so it is first and it is named.
    for state in ["present", "absent"] {
        out.push((
            format!("package/cargo/{state}"),
            res(ResourceType::Package, Some(state), |r| {
                r.provider = Some("cargo".to_string());
                r.packages = vec!["ripgrep".to_string()];
            }),
        ));
    }

    for provider in ["apt", "pip", "npm", "brew", "conda"] {
        out.push((
            format!("package/{provider}/present"),
            res(ResourceType::Package, Some("present"), |r| {
                r.provider = Some(provider.to_string());
                r.packages = vec!["jq".to_string()];
            }),
        ));
    }

    out.push((
        "file/present".to_string(),
        res(ResourceType::File, Some("present"), |r| {
            r.path = Some("/etc/forjar-lint-probe.conf".to_string());
            r.content = Some("key = value\n".to_string());
        }),
    ));

    out.push((
        "service/running".to_string(),
        res(ResourceType::Service, Some("running"), |r| {
            r.name = Some("nginx".to_string());
        }),
    ));

    out.push((
        "cron/present".to_string(),
        res(ResourceType::Cron, Some("present"), |r| {
            r.name = Some("nightly".to_string());
            r.command = Some("/usr/bin/true".to_string());
            r.schedule = Some("0 3 * * *".to_string());
        }),
    ));

    out.push((
        "task/plain".to_string(),
        res(ResourceType::Task, None, |r| {
            r.command = Some("echo hello".to_string());
        }),
    ));

    out
}

/// Run one generated script through the gate, or report which one failed.
fn assert_gate_accepts(label: &str, which: &str, script: &str) {
    if let Err(e) = validate_before_exec(script) {
        panic!(
            "the gate forjar puts in front of execution REJECTS the {which} script \
             forjar generates for {label}. This resource cannot run.\n\n{e}"
        );
    }
}

#[test]
fn every_generated_apply_script_survives_the_i8_gate() {
    for (label, r) in corpus() {
        if let Ok(s) = apply_script(&r) {
            assert_gate_accepts(&label, "apply", &s);
        }
    }
}

#[test]
fn every_generated_check_script_survives_the_i8_gate() {
    for (label, r) in corpus() {
        if let Ok(s) = check_script(&r) {
            assert_gate_accepts(&label, "check", &s);
        }
    }
}

#[test]
fn every_generated_state_query_script_survives_the_i8_gate() {
    for (label, r) in corpus() {
        if let Ok(s) = state_query_script(&r) {
            assert_gate_accepts(&label, "state_query", &s);
        }
    }
}

/// The denominator, so an empty or silently-shrinking corpus cannot read as a
/// pass. Same reason `ledger-replay.sh` prints how many entries it replayed.
#[test]
fn the_corpus_is_not_empty_and_covers_cargo() {
    let c = corpus();
    assert!(c.len() >= 10, "corpus shrank to {} rows", c.len());
    assert!(
        c.iter().any(|(l, _)| l == "package/cargo/present"),
        "the row for the #345 regression is gone"
    );
}
