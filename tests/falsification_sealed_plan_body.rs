//! Refs #356 / #358 — the body of a saved plan must be load-bearing.
//!
//! `forjar plan --out p.json` wrote a `config_hash` and, underneath it,
//! `changes` / `execution_order` / three counters as plain JSON.
//! `apply --plan-file` checked the hash and trusted the rest. Two consequences,
//! both reproducible against the shipped binary:
//!
//! 1. Editing three integers to `0` — leaving `config_hash` byte-identical —
//!    made a requested apply print "Plan has no changes to apply." and exit 0
//!    having converged nothing. That is the #210 family the repo already has 24
//!    recorded instances of: a green that certifies the wrong thing.
//! 2. Nothing bound the plan to the LOCK it was planned against, so a plan
//!    whose create/update/destroy decisions had been invalidated by a lock
//!    rewritten underneath it was applied anyway. That is the TOCTOU the saved
//!    plan exists to prevent.
//!
//! These tests drive `CARGO_BIN_EXE_forjar` rather than the loader function,
//! because both defects lived in the wiring between the file and the apply — and
//! because the exit code an operator or CI job reads is the thing that was
//! wrong.
//!
//! NOTE ON WHAT DOES *NOT* FALSIFY: `a_byte_changed_in_the_config_is_rejected`
//! passes on unmodified `main`. `config_hash` has been checked since FJ-1250 and
//! canonical since GH-212. It is here as a labelled regression guard so nobody
//! reads its green tick as evidence that the state and diff legs work.

use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// One localhost file resource, so an apply is real but harmless.
fn project(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let managed = dir.join("managed.txt");
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\n\
             name: sealed-plan-body\n\
             machines:\n\
             \x20 localhost:\n\
             \x20   hostname: localhost\n\
             \x20   addr: 127.0.0.1\n\
             resources:\n\
             \x20 managed:\n\
             \x20   type: file\n\
             \x20   machine: localhost\n\
             \x20   path: {}\n\
             \x20   content: \"sealed\"\n",
            managed.display()
        ),
    )
    .expect("write config");
    (cfg, dir.join("state"), managed)
}

fn plan_out(cfg: &Path, state: &Path, out: &Path) -> std::process::Output {
    forjar()
        .args(["plan", "-f"])
        .arg(cfg)
        .arg("--state-dir")
        .arg(state)
        .arg("--out")
        .arg(out)
        .output()
        .expect("run plan")
}

fn apply_plan(cfg: &Path, state: &Path, plan: &Path) -> std::process::Output {
    forjar()
        .args(["apply", "-f"])
        .arg(cfg)
        .arg("--state-dir")
        .arg(state)
        .arg("--plan-file")
        .arg(plan)
        .arg("--yes")
        .output()
        .expect("run apply")
}

fn apply_all(cfg: &Path, state: &Path) -> std::process::Output {
    forjar()
        .args(["apply", "-f"])
        .arg(cfg)
        .arg("--state-dir")
        .arg(state)
        .arg("--yes")
        .output()
        .expect("run apply")
}

fn read_plan(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read plan")).expect("parse plan")
}

fn write_plan(path: &Path, doc: &serde_json::Value) {
    std::fs::write(path, serde_json::to_string_pretty(doc).expect("render")).expect("write plan");
}

fn combined(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// RED-1: the defect in #358 verbatim. Zero the three counters and empty the
/// change list, leave `config_hash` untouched, and ask for an apply.
///
/// Before the seal: exit 0, "Plan has no changes to apply.", nothing converged.
#[test]
fn a_hand_zeroed_plan_is_refused_rather_than_silently_obeyed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cfg, state, managed) = project(dir.path());
    let plan_path = dir.path().join("p.json");

    let planned = plan_out(&cfg, &state, &plan_path);
    assert!(planned.status.success(), "plan: {}", combined(&planned));

    let mut doc = read_plan(&plan_path);
    let hash_before = doc["config_hash"].clone();
    doc["to_create"] = serde_json::json!(0);
    doc["to_update"] = serde_json::json!(0);
    doc["to_destroy"] = serde_json::json!(0);
    doc["unchanged"] = serde_json::json!(0);
    doc["changes"] = serde_json::json!([]);
    write_plan(&plan_path, &doc);
    assert_eq!(
        read_plan(&plan_path)["config_hash"],
        hash_before,
        "the tamper must leave the config hash valid — that is the whole point"
    );

    let out = apply_plan(&cfg, &state, &plan_path);
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "a tampered plan must not produce a successful apply: {text}"
    );
    assert!(
        text.contains("PLAN_HASH_MISMATCH") || text.contains("PLAN_MALFORMED"),
        "the refusal must name the integrity failure: {text}"
    );
    assert!(
        !managed.exists(),
        "nothing should have been converged from a refused plan"
    );
}

/// The same edit, but leaving `changes` populated so the counters contradict
/// their own list. That is refused structurally, whether or not the seal was
/// recomputed by whoever made the edit.
#[test]
fn counters_that_contradict_the_change_list_are_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cfg, state, _managed) = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&cfg, &state, &plan_path).status.success());

    let mut doc = read_plan(&plan_path);
    doc["to_create"] = serde_json::json!(0);
    write_plan(&plan_path, &doc);

    let out = apply_plan(&cfg, &state, &plan_path);
    let text = combined(&out);
    assert!(!out.status.success(), "{text}");
    assert!(
        text.contains("PLAN_MALFORMED") || text.contains("PLAN_HASH_MISMATCH"),
        "{text}"
    );
}

/// RED-2: the TOCTOU. A lock rewritten after the plan was sealed invalidates
/// every create-vs-update decision the plan made, and nothing used to notice —
/// measured, `apply --plan-file` printed "Plan has no changes to apply." and
/// exited 0 over a machine whose recorded state had just been corrupted.
///
/// The `.b3` sidecar is re-blessed with `forjar reseal --all` on purpose: the
/// point is that the STATE LEG catches this, not the sidecar check.
#[test]
fn a_lock_rewritten_after_the_plan_was_sealed_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cfg, state, _managed) = project(dir.path());

    let first = apply_all(&cfg, &state);
    assert!(first.status.success(), "seed apply: {}", combined(&first));

    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&cfg, &state, &plan_path).status.success());

    let lock = state.join("localhost").join("state.lock.yaml");
    let body = std::fs::read_to_string(&lock).expect("read lock");
    assert!(body.contains("hash"), "lock should record a hash: {body}");
    std::fs::write(&lock, body.replacen("hash:", "hash: tampered-", 1)).expect("rewrite lock");

    let resealed = forjar()
        .args(["reseal", "--all", "--state-dir"])
        .arg(&state)
        .output()
        .expect("run reseal");
    assert!(resealed.status.success(), "reseal: {}", combined(&resealed));

    let out = apply_plan(&cfg, &state, &plan_path);
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "a plan whose state moved underneath it must not apply: {text}"
    );
    assert!(text.contains("PLAN_HASH_MISMATCH"), "{text}");
    assert!(
        text.contains("state leg"),
        "the error must name the leg: {text}"
    );
}

/// RED-4: `sealed_at`/`ttl` live INSIDE the composition, so moving the expiry
/// is a hash mismatch rather than a successfully extended life.
#[test]
fn moving_the_expiry_is_a_mismatch_not_a_longer_life() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cfg, state, _managed) = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&cfg, &state, &plan_path).status.success());

    let mut doc = read_plan(&plan_path);
    doc["seal"]["ttl_secs"] = serde_json::json!(86_400);
    write_plan(&plan_path, &doc);

    let out = apply_plan(&cfg, &state, &plan_path);
    let text = combined(&out);
    assert!(!out.status.success(), "{text}");
    assert!(text.contains("PLAN_HASH_MISMATCH"), "{text}");
    assert!(text.contains("seal leg"), "{text}");
}

/// GREEN GUARD. Without this, `Err("PLAN_HASH_MISMATCH")` unconditionally would
/// pass every test above. Sealing must not become "refuse everything".
#[test]
fn an_untouched_sealed_plan_still_applies() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cfg, state, managed) = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&cfg, &state, &plan_path).status.success());

    let out = apply_plan(&cfg, &state, &plan_path);
    let text = combined(&out);
    assert!(out.status.success(), "sealed plan must apply: {text}");
    assert_eq!(
        std::fs::read_to_string(&managed).expect("managed file"),
        "sealed",
        "the reviewed change must actually have happened"
    );
}

/// LABELLED NON-FALSIFYING REGRESSION GUARD.
///
/// This passes on unmodified `main`: `check_plan_provenance` has rejected a
/// changed config since FJ-1250. It is kept so the config leg cannot silently
/// regress, and labelled so its green tick is never mistaken for evidence about
/// the state or diff legs.
#[test]
fn a_byte_changed_in_the_config_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (cfg, state, _managed) = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&cfg, &state, &plan_path).status.success());

    let yaml = std::fs::read_to_string(&cfg).expect("read config");
    std::fs::write(
        &cfg,
        yaml.replace("content: \"sealed\"", "content: \"edited\""),
    )
    .expect("edit");

    let out = apply_plan(&cfg, &state, &plan_path);
    assert!(!out.status.success(), "{}", combined(&out));
}
