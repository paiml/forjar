//! Refs #363 — a saved plan of a config that contains a `phony: true` resource
//! must still apply.
//!
//! `forjar plan --out` sealed the config leg over a config it had already
//! NARROWED: `strip_unrequested_phony` removes every unrequested phony resource
//! from `config.resources` before `save_plan_file` runs, and the canonical
//! config hash is taken over the whole `ForjarConfig`. `apply --plan-file`
//! rebuilds the config by parsing the file and stops after
//! `resolve_data_sources`, so it recomputes the leg over the UNNARROWED config
//! and the two hashes cannot agree. Measured on v1.24.0, with nothing changed
//! between the two commands:
//!
//! ```text
//!   $ forjar plan -f forjar.yaml --state-dir state --out p.json
//!   Plan saved to p.json                                            EXIT=0
//!   $ forjar apply -f forjar.yaml --state-dir state --plan-file p.json --yes
//!   error: PLAN_HASH_MISMATCH: the config changed since the plan was sealed
//!     (config leg: expected blake3:48529cd5…, got blake3:76887d58…)     EXIT=1
//! ```
//!
//! Sharper than "the hashes differ": the sealed document denotes a DIFFERENT
//! config from the one it was planned from. The plan file written from a config
//! holding `cleanup` applies cleanly against a config with `cleanup` physically
//! deleted, because the seal was taken after `cleanup` was stripped. Two configs
//! that differ only in their phony resources sealed identically.
//!
//! The fix seals the leg over the config as `apply --plan-file` reconstructs it
//! — the value at the point `prepare_config` stops — while the plan BODY stays
//! narrowed, which is what `apply_from_plan::replan` reproduces.

use std::path::Path;
use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

fn combined(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// `alpha` (an ordinary file) plus `cleanup`, a `phony: true` task whose only
/// observable is the marker it touches. The repo's shared plan-file fixture
/// (`tests/common/plan_project.rs`) has no phony resource, which is exactly why
/// this defect shipped.
fn write_config(dir: &Path, with_phony: bool) -> std::path::PathBuf {
    let alpha = dir.join("alpha.txt");
    let marker = dir.join("PHONY_RAN");
    let mut yaml = format!(
        "version: \"1.0\"\n\
         name: phonyplan\n\
         machines:\n\
         \x20 box:\n\
         \x20   hostname: localhost\n\
         \x20   addr: 127.0.0.1\n\
         resources:\n\
         \x20 alpha:\n\
         \x20   type: file\n\
         \x20   machine: box\n\
         \x20   path: {}\n\
         \x20   state: file\n\
         \x20   content: \"alpha\"\n",
        alpha.display()
    );
    if with_phony {
        yaml.push_str(&format!(
            "\x20 cleanup:\n\
             \x20   type: task\n\
             \x20   machine: box\n\
             \x20   phony: true\n\
             \x20   command: \"touch {}\"\n",
            marker.display()
        ));
    }
    let cfg = dir.join("forjar.yaml");
    std::fs::write(&cfg, yaml).expect("write config");
    cfg
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

/// RED-1, the load-bearing one: binary-only, so it is immune to how the fix is
/// implemented. Nothing changes between `plan --out` and `apply --plan-file`,
/// so the apply must run.
#[test]
fn a_saved_plan_of_a_config_with_a_phony_resource_applies() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), true);
    let state = dir.path().join("state");
    let plan = dir.path().join("p.json");

    let planned = plan_out(&cfg, &state, &plan);
    assert!(
        planned.status.success(),
        "plan --out must succeed: {}",
        combined(&planned)
    );

    let applied = apply_plan(&cfg, &state, &plan);
    let text = combined(&applied);
    assert!(
        !text.contains("PLAN_HASH_MISMATCH"),
        "the config did not change between plan and apply, so the seal must \
         still verify; got: {text}"
    );
    assert!(
        applied.status.success(),
        "apply --plan-file must succeed: {text}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("alpha.txt")).expect("alpha.txt"),
        "alpha",
        "the reviewed change must actually have been applied"
    );
}

/// RED-2, the denotation. Sharper than "the two hashes differ": the sealed
/// document named a DIFFERENT config from the one it was planned from.
///
/// Two configs identical but for a `phony: true` resource sealed to the same
/// `config_hash`, and the phony config's own plan file applied cleanly against
/// the control — so a saved plan could not tell the two apart. Both halves are
/// observed through the binary alone, so nothing here pins how the fix is
/// implemented.
///
/// A first draft of this test recomputed the leg in-process with
/// `parser::parse_and_validate` + `resolver::resolve_data_sources` +
/// `config_hash`. That is NOT the config apply rebuilds — `prepare_config` also
/// runs `inject_workspace_param`, which inserts `params.workspace = "default"`
/// — so it was red before AND after the fix, for the wrong reason. Recorded
/// here because it is exactly the trap this file exists to avoid.
#[test]
fn two_configs_that_differ_only_in_a_phony_resource_do_not_seal_alike() {
    let dir = tempfile::tempdir().expect("tempdir");
    let with_phony = write_config(dir.path(), true);
    let state = dir.path().join("state");
    let plan = dir.path().join("p.json");
    assert!(plan_out(&with_phony, &state, &plan).status.success());

    // The same config with `cleanup` physically deleted — same machine, same
    // `alpha` at the same path, so the plan BODIES are identical.
    let control = dir.path().join("control.yaml");
    let text = std::fs::read_to_string(&with_phony).expect("read config");
    let cut = text
        .find("  cleanup:")
        .expect("fixture has a phony resource");
    std::fs::write(&control, &text[..cut]).expect("write control");
    let control_plan = dir.path().join("c.json");
    assert!(plan_out(&control, &state, &control_plan).status.success());

    let leg = |p: &Path| -> String {
        let doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(p).expect("read plan")).expect("json");
        let hash = doc["config_hash"]
            .as_str()
            .expect("config_hash")
            .to_string();
        assert_eq!(
            doc["seal"]["config_hash"].as_str().expect("seal leg"),
            hash,
            "the seal's own copy of the leg must agree with the document's"
        );
        hash
    };
    assert_ne!(
        leg(&plan),
        leg(&control_plan),
        "a plan sealed over a config holding a phony resource must not denote \
         the config with that resource deleted"
    );

    // And the consequence: the phony config's plan must not apply against the
    // control config. It did, at exit 0, printing `+ alpha on box`.
    let crossed = apply_plan(&control, &state, &plan);
    assert!(
        !crossed.status.success(),
        "a plan sealed over one config applied against another: {}",
        combined(&crossed)
    );
}

/// GREEN GUARD: making the seal see the phony resource must not make the phony
/// resource RUN. Goal-only semantics are the whole point of `phony: true`.
#[test]
fn the_phony_resource_still_does_not_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), true);
    let state = dir.path().join("state");
    let plan = dir.path().join("p.json");
    assert!(plan_out(&cfg, &state, &plan).status.success());

    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&plan).expect("read plan")).expect("json");
    let named: Vec<&str> = doc["changes"]
        .as_array()
        .expect("changes")
        .iter()
        .map(|c| c["resource_id"].as_str().expect("resource_id"))
        .collect();
    assert_eq!(
        named,
        vec!["alpha"],
        "the plan BODY must stay narrowed — a phony resource is goal-only"
    );

    let applied = apply_plan(&cfg, &state, &plan);
    assert!(applied.status.success(), "apply: {}", combined(&applied));
    assert!(
        !dir.path().join("PHONY_RAN").exists(),
        "the phony task must not have executed"
    );

    // And the stack is a fixed point afterwards: an unrequested phony resource
    // must not reappear as a perpetual change.
    let again = forjar()
        .args(["plan", "-f"])
        .arg(&cfg)
        .arg("--state-dir")
        .arg(&state)
        .output()
        .expect("run plan");
    let text = combined(&again);
    assert!(
        text.contains("0 to add, 0 to change, 0 to destroy"),
        "re-planning a converged stack must report no work: {text}"
    );
}

/// GREEN GUARD / control: the same shape with no phony resource anywhere passed
/// before this change and must keep passing.
#[test]
fn a_config_with_no_phony_resource_is_the_control() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), false);
    let state = dir.path().join("state");
    let plan = dir.path().join("p.json");
    assert!(plan_out(&cfg, &state, &plan).status.success());

    let applied = apply_plan(&cfg, &state, &plan);
    assert!(
        applied.status.success(),
        "control must apply: {}",
        combined(&applied)
    );
}
