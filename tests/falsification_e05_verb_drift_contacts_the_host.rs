//! forjar#407 (CRUX audit E05): the `drift` verb — the one reachable over
//! MCP, HTTP and `forjar verb call` — hashed the CONTROLLER's filesystem and
//! reported it as the remote machine's state, and dropped the census.
//!
//! WHAT WAS OBSERVABLY WRONG. `DriftHandler` held `config.machines[name]` and
//! then called `drift::detect_drift(&lock)`, which is
//! `detect_drift_reported(lock, None)`. With no machine, the file detector
//! hashes THIS host's copy of the path; every non-file entry is census-skipped
//! as "no config loaded" three lines after the config was loaded; and
//! `DriftOutput` had no census at all, so `drifted: false` over six inspected
//! resources was byte-identical to `drifted: false` over zero. Then the first
//! fix's own review found the verb running a config-declared
//! `completion_check` on the controller — a `touch` here, `curl | sh` in the
//! same slot — under `readOnlyHint: true`.
//!
//! WHY THESE ASSERTIONS. The verb is driven through the BINARY
//! (`forjar verb call drift`), because "what does an agent get back" is a
//! property of the process, not of a handler struct, and because the unit
//! suite in `mcp::tests_drift_e05` cannot be the falsifier the quorum gate
//! binds to. The machine is at a TEST-NET-3 address that nothing routes, so
//! the only honest answers are "could not reach it" or "did not check it";
//! a file that exists on the controller with the locked hash is the bait.

use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

/// A lock naming resources, written the way a successful apply would.
fn write_lock(state_dir: &Path, machine: &str, resources_yaml: &str) {
    let md = state_dir.join(machine);
    std::fs::create_dir_all(&md).expect("state dir");
    std::fs::write(
        md.join("state.lock.yaml"),
        format!(
            "schema: \"1.0\"\nmachine: {machine}\nhostname: {machine}\ngenerated_at: now\n\
             generator: test\nblake3_version: \"1\"\nresources:\n{resources_yaml}"
        ),
    )
    .expect("write lock");
}

/// `forjar verb call drift --json {params}` → the verb's JSON answer.
fn call_drift(cfg: &Path, state_dir: &Path) -> serde_json::Value {
    let params = serde_json::json!({
        "path": cfg.display().to_string(),
        "state_dir": state_dir.display().to_string(),
    })
    .to_string();
    let out = Command::new(forjar())
        .args(["verb", "call", "drift", "--json", &params])
        .output()
        .expect("spawn forjar");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let start = stdout
        .find('{')
        .unwrap_or_else(|| panic!("no JSON on stdout.\nstdout: {stdout}\nstderr: {stderr}"));
    serde_json::from_str(&stdout[start..]).unwrap_or_else(|e| {
        panic!("verb output is not JSON ({e}).\nstdout: {stdout}\nstderr: {stderr}")
    })
}

fn blake3_of(path: &Path) -> String {
    let bytes = std::fs::read(path).expect("read bait");
    format!("blake3:{}", blake3::hash(&bytes).to_hex())
}

/// A file that exists on the controller with the locked hash, over an
/// unroutable machine.
fn unreachable_fleet(dir: &Path) -> (PathBuf, PathBuf) {
    let bait = dir.join("on-controller.txt");
    std::fs::write(&bait, "controller copy").expect("write bait");
    let content_hash = blake3_of(&bait);
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: e05\nmachines:\n  web:\n    hostname: web\n\
             \x20   addr: 203.0.113.9\n    user: root\nresources:\n  conf:\n    type: file\n\
             \x20   machine: web\n    path: {}\n    content: \"controller copy\"\n",
            bait.display()
        ),
    )
    .expect("write config");
    let state_dir = dir.join("state");
    write_lock(
        &state_dir,
        "web",
        &format!(
            "  conf:\n    type: file\n    status: converged\n    hash: \"h\"\n\
             \x20   details:\n      path: \"{}\"\n      content_hash: \"{content_hash}\"\n",
            bait.display()
        ),
    );
    (cfg, state_dir)
}

/// FALSIFY-E05-001 — the verb must not answer clean about a host it never
/// reached. RED on main: `drifted: false`, no `unchecked`, because the
/// controller's own copy of the file was hashed.
#[test]
fn drift_over_an_unreachable_machine_must_not_answer_clean() {
    let d = tempfile::tempdir().expect("tempdir");
    let (cfg, state_dir) = unreachable_fleet(d.path());

    let out = call_drift(&cfg, &state_dir);
    let drifted = out["drifted"].as_bool().unwrap_or(false);
    let unchecked = out["unchecked"].as_array().map(|a| a.len()).unwrap_or(0);
    assert!(
        drifted || unchecked > 0,
        "forjar#407: the verb answered clean about 203.0.113.9 without contacting it: {out}"
    );

    let conf = out["findings"]
        .as_array()
        .and_then(|f| f.iter().find(|f| f["resource"] == "conf"))
        .unwrap_or_else(|| panic!("no verdict for `conf`: {out}"));
    let actual = conf["actual_hash"].as_str().unwrap_or("");
    assert!(
        actual == "ERROR" || actual == "MISSING",
        "an unroutable host cannot yield a real content hash; `{actual}` proves the \
         controller's filesystem was read instead. detail={}",
        conf["detail"]
    );
}

/// FALSIFY-E05-002 — the answer carries its own denominator. RED on main:
/// `DriftOutput` had no `census`, `resources_inspected` or
/// `resources_skipped` key at all.
#[test]
fn the_verb_discloses_how_much_it_inspected() {
    let d = tempfile::tempdir().expect("tempdir");
    let (cfg, state_dir) = unreachable_fleet(d.path());

    let out = call_drift(&cfg, &state_dir);
    assert!(
        out.get("census").is_some_and(|c| c.is_array()),
        "no census in the verb's answer — an agent cannot tell six inspected resources \
         from zero: {out}"
    );
    assert!(
        out.get("resources_inspected").is_some() && out.get("resources_skipped").is_some(),
        "the inspected/skipped counts are missing: {out}"
    );
}

/// FALSIFY-E05-003 — a read-only verb must not execute a config-declared
/// `completion_check` on the controller, and must SAY it declined. RED on the
/// first cut of #407: the trap file appeared the moment the verb ran.
#[test]
fn the_readonly_verb_neither_runs_the_completion_check_nor_hides_that() {
    let d = tempfile::tempdir().expect("tempdir");
    let trap = d.path().join("COMPLETION_CHECK_FIRED");
    let cfg = d.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: e05trap\nmachines:\n  local:\n    hostname: localhost\n\
             \x20   addr: 127.0.0.1\nresources:\n  guard:\n    type: task\n    machine: local\n\
             \x20   command: \"true\"\n    completion_check: \"touch {}\"\n",
            trap.display()
        ),
    )
    .expect("write config");
    let state_dir = d.path().join("state");
    write_lock(
        &state_dir,
        "local",
        "  guard:\n    type: task\n    status: converged\n    hash: \"h\"\n",
    );

    let out = call_drift(&cfg, &state_dir);
    assert!(
        !trap.exists(),
        "forjar#372: a verb published with readOnlyHint: true executed the completion_check \
         a config declared, on the controller: {out}"
    );
    let census = &out["census"][0];
    assert_eq!(
        census["skipped_by_reason"]["--no-task-checks"],
        serde_json::json!(1),
        "the assertion was not executed and the census does not say so: {out}"
    );
    let disclosed = out["unattended_skipped"]
        .as_array()
        .map(|a| {
            a.iter()
                .any(|s| s.as_str().is_some_and(|s| s.contains("guard")))
        })
        .unwrap_or(false);
    assert!(
        disclosed,
        "declining to run the check must be NAMED, not silent: {out}"
    );
}
