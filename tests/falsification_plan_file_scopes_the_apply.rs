//! Refs #358 — `apply --plan-file` must execute the plan, not the config.
//!
//! `cmd_apply_from_plan` loaded the plan, summed three integers out of it, and
//! then built an `ApplyConfig` in which every selector was `None`:
//!
//! ```text
//! // Execute as a normal apply using the plan's resource list   <- said the
//! let cfg = executor::ApplyConfig {                             //  opposite
//!     machine_filter: None, resource_filter: None, ...           //  of what
//! };                                                             //  it did
//! ```
//!
//! `plan.changes` and `plan.execution_order` were not referenced anywhere after
//! the sum, so `apply --plan-file p.json` converged the WHOLE current config.
//! Terraform's `-out=.tfplan`, cited in #356's own competitive research,
//! "guarantees that only the exact reviewed delta is executed". forjar wrote the
//! artifact, verified its provenance, and then ignored it.
//!
//! A saved plan can legitimately be narrower than the config — `plan -r` and
//! `plan -m` filter the plan body (GH-214) while `config_hash` still covers the
//! whole file — so these tests need no drift and no tampering. They ask for a
//! plan over one resource, apply that plan, and look at what happened to the
//! other one.

use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

struct Project {
    cfg: PathBuf,
    state: PathBuf,
    alpha: PathBuf,
    bravo: PathBuf,
}

/// Two resources on two (locally-executed) machines, so both a resource scope
/// and a machine scope have something to exclude.
fn project(dir: &Path) -> Project {
    let alpha = dir.join("alpha.txt");
    let bravo = dir.join("bravo.txt");
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\n\
             name: plan-scope\n\
             machines:\n\
             \x20 web:\n\
             \x20   hostname: localhost\n\
             \x20   addr: 127.0.0.1\n\
             \x20 db:\n\
             \x20   hostname: localhost\n\
             \x20   addr: 127.0.0.1\n\
             resources:\n\
             \x20 alpha:\n\
             \x20   type: file\n\
             \x20   machine: web\n\
             \x20   path: {}\n\
             \x20   content: \"alpha\"\n\
             \x20 bravo:\n\
             \x20   type: file\n\
             \x20   machine: db\n\
             \x20   path: {}\n\
             \x20   content: \"bravo\"\n",
            alpha.display(),
            bravo.display()
        ),
    )
    .expect("write config");
    Project {
        cfg,
        state: dir.join("state"),
        alpha,
        bravo,
    }
}

fn combined(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn plan_out(p: &Project, extra: &[&str], out: &Path) -> std::process::Output {
    let mut cmd = forjar();
    cmd.args(["plan", "-f"])
        .arg(&p.cfg)
        .arg("--state-dir")
        .arg(&p.state);
    cmd.args(extra);
    cmd.arg("--out").arg(out).output().expect("run plan")
}

fn apply_plan(p: &Project, plan: &Path) -> std::process::Output {
    forjar()
        .args(["apply", "-f"])
        .arg(&p.cfg)
        .arg("--state-dir")
        .arg(&p.state)
        .arg("--plan-file")
        .arg(plan)
        .arg("--yes")
        .output()
        .expect("run apply")
}

/// The #358 falsification: a plan that names ONE of two resources must converge
/// exactly that one.
#[test]
fn a_resource_scoped_plan_converges_only_what_it_named() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let plan_path = dir.path().join("p.json");

    let planned = plan_out(&p, &["-r", "alpha"], &plan_path);
    assert!(planned.status.success(), "plan: {}", combined(&planned));

    let out = apply_plan(&p, &plan_path);
    let text = combined(&out);
    assert!(out.status.success(), "apply: {text}");

    assert_eq!(
        std::fs::read_to_string(&p.alpha).expect("alpha must be converged"),
        "alpha"
    );
    assert!(
        !p.bravo.exists(),
        "'bravo' was not in the reviewed plan and must not have been applied: {text}"
    );
}

/// The same claim on the machine axis: a plan restricted to one machine must
/// not reach the other one.
#[test]
fn a_machine_scoped_plan_does_not_touch_the_other_machine() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let plan_path = dir.path().join("p.json");

    let planned = plan_out(&p, &["-m", "db"], &plan_path);
    assert!(planned.status.success(), "plan: {}", combined(&planned));

    let out = apply_plan(&p, &plan_path);
    let text = combined(&out);
    assert!(out.status.success(), "apply: {text}");

    assert_eq!(
        std::fs::read_to_string(&p.bravo).expect("bravo must be converged"),
        "bravo"
    );
    assert!(
        !p.alpha.exists(),
        "machine 'web' was not in the reviewed plan: {text}"
    );
    assert!(
        !p.state.join("web").exists(),
        "a machine outside the plan must not even get a lock written: {text}"
    );
}

/// Honouring the body is not the seal's job — an unsealed v1 plan is scoped
/// too. This one is hand-written, exactly as an older forjar would have
/// produced it, so the scope is proven independent of `forjar-plan-v2`.
#[test]
fn a_v1_plan_is_scoped_as_well() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());

    // Borrow the canonical config hash from a real plan file rather than
    // re-deriving it here — GH-212 exists because a second expression for
    // "the hash of this config" drifted from the first.
    let donor = dir.path().join("donor.json");
    assert!(plan_out(&p, &[], &donor).status.success());
    let config_hash = serde_json::from_str::<serde_json::Value>(
        &std::fs::read_to_string(&donor).expect("read donor"),
    )
    .expect("parse donor")["config_hash"]
        .clone();

    let v1 = serde_json::json!({
        "format": "forjar-plan-v1",
        "config_hash": config_hash,
        "name": "plan-scope",
        "to_create": 1, "to_update": 0, "to_destroy": 0, "unchanged": 0,
        "execution_order": ["alpha"],
        "changes": [{
            "resource_id": "alpha",
            "machine": "web",
            "resource_type": "file",
            "action": "create",
            "description": "alpha: create",
        }],
    });
    let plan_path = dir.path().join("v1.json");
    std::fs::write(
        &plan_path,
        serde_json::to_string_pretty(&v1).expect("render"),
    )
    .expect("write v1 plan");

    let out = apply_plan(&p, &plan_path);
    let text = combined(&out);
    assert!(out.status.success(), "apply: {text}");
    assert!(
        p.alpha.exists(),
        "the named resource must be applied: {text}"
    );
    assert!(
        !p.bravo.exists(),
        "a v1 plan's body is still the executed set: {text}"
    );
}

/// GREEN GUARD: scoping must not turn `apply --plan-file` into "apply nothing".
/// An unfiltered plan still converges everything it named.
#[test]
fn an_unfiltered_plan_still_converges_the_whole_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&p, &[], &plan_path).status.success());

    let out = apply_plan(&p, &plan_path);
    let text = combined(&out);
    assert!(out.status.success(), "apply: {text}");
    assert_eq!(std::fs::read_to_string(&p.alpha).expect("alpha"), "alpha");
    assert_eq!(std::fs::read_to_string(&p.bravo).expect("bravo"), "bravo");
}
