//! PMAT-721 review F2: `apply --plan-file` must only check and RECORD the
//! resources inside the reviewed plan's scope.
//!
//! `lockless_check::record` ran over the full, unpruned config, so a resource
//! the plan never named (here `outside`, whose completion_check passes) got a
//! host check and a `status: converged` lock entry written by an apply that
//! was reviewed for `inside` only.

use std::path::Path;
use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

fn write_cfg(dir: &Path) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: scoped-record\nmachines:\n  localhost:\n    hostname: localhost\n    addr: localhost\nresources:\n  inside:\n    type: task\n    machine: localhost\n    command: \"echo ran > {d}/inside.ran\"\n    completion_check: \"test -f {d}/inside.ran\"\n  outside:\n    type: task\n    machine: localhost\n    command: \"echo ran > {d}/outside.ran\"\n    completion_check: \"true\"\n",
            d = dir.display()
        ),
    )
    .unwrap();
    cfg
}

#[test]
fn plan_file_apply_records_no_lock_entry_for_an_out_of_scope_resource() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_cfg(dir.path());
    let state = dir.path().join("state");
    let plan = dir.path().join("plan.json");

    let out = forjar()
        .args(["plan", "-r", "inside", "-f"])
        .arg(&cfg)
        .arg("--state-dir")
        .arg(&state)
        .arg("--out")
        .arg(&plan)
        .output()
        .unwrap();
    assert!(out.status.success(), "plan failed: {out:?}");

    let out = forjar()
        .args(["apply", "--yes", "-f"])
        .arg(&cfg)
        .arg("--state-dir")
        .arg(&state)
        .arg("--plan-file")
        .arg(&plan)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "apply failed:\n{text}");
    assert!(
        dir.path().join("inside.ran").exists(),
        "in-scope resource did not run:\n{text}"
    );
    let lock = std::fs::read_to_string(state.join("localhost").join("state.lock.yaml"))
        .unwrap_or_else(|e| panic!("no lock written ({e}):\n{text}"));
    assert!(lock.contains("inside:"), "in-scope entry missing:\n{lock}");
    assert!(
        !lock.contains("outside:"),
        "out-of-scope resource got a lock entry from a plan-file apply:\n{lock}"
    );
}

// ---------------------------------------------------------------------------
// PMAT-721 quorum blocker: `-r` / `-g` on top of `--plan-file`.
//
// The plan below names BOTH resources. The operator then narrows the apply with
// `-r inside` (or `-g ga`). `lockless_check::record` ignored those filters, so
// `outside`'s completion_check still RAN on the host and a `converged` entry
// was written for it. The probe is a marker file: the check `touch`es it, so
// "the check did not run" is observed, not inferred from the lock.
// ---------------------------------------------------------------------------

fn write_two_group_cfg(dir: &Path) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: filtered-record\nmachines:\n  localhost:\n    hostname: localhost\n    addr: localhost\nresources:\n  inside:\n    type: task\n    machine: localhost\n    resource_group: ga\n    command: \"echo ran > {d}/inside.ran\"\n    completion_check: \"test -f {d}/inside.ran\"\n  outside:\n    type: task\n    machine: localhost\n    resource_group: gb\n    command: \"echo ran > {d}/outside.ran\"\n    completion_check: \"touch {d}/marker\"\n",
            d = dir.display()
        ),
    )
    .unwrap();
    cfg
}

fn filtered_plan_file_apply(filter: &[&str]) {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_two_group_cfg(dir.path());
    let state = dir.path().join("state");
    let plan = dir.path().join("plan.json");

    // Unfiltered plan: it names inside AND outside.
    let out = forjar()
        .args(["plan", "-f"])
        .arg(&cfg)
        .arg("--state-dir")
        .arg(&state)
        .arg("--out")
        .arg(&plan)
        .output()
        .unwrap();
    assert!(out.status.success(), "plan failed: {out:?}");
    let plan_text = std::fs::read_to_string(&plan).unwrap();
    assert!(
        plan_text.contains("outside"),
        "precondition: the plan must name outside:\n{plan_text}"
    );

    let out = forjar()
        .args(["apply", "--yes", "-f"])
        .arg(&cfg)
        .arg("--state-dir")
        .arg(&state)
        .arg("--plan-file")
        .arg(&plan)
        .args(filter)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "apply failed:\n{text}");
    assert!(
        dir.path().join("inside.ran").exists(),
        "in-scope resource did not run:\n{text}"
    );
    assert!(
        !dir.path().join("marker").exists(),
        "filtered-out resource's completion_check RAN on the host:\n{text}"
    );
    let lock = std::fs::read_to_string(state.join("localhost").join("state.lock.yaml"))
        .unwrap_or_else(|e| panic!("no lock written ({e}):\n{text}"));
    assert!(lock.contains("inside:"), "in-scope entry missing:\n{lock}");
    assert!(
        !lock.contains("outside:"),
        "filtered-out resource got a lock entry:\n{lock}"
    );
}

#[test]
fn plan_file_apply_with_r_filter_neither_checks_nor_records_the_excluded_resource() {
    filtered_plan_file_apply(&["-r", "inside"]);
}

#[test]
fn plan_file_apply_with_g_filter_neither_checks_nor_records_the_excluded_resource() {
    filtered_plan_file_apply(&["-g", "ga"]);
}
