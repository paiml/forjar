//! forjar#549: a resource whose query never reached its target is UNMEASURED,
//! not DRIFTED.
//!
//! WHAT WAS OBSERVABLY WRONG. On paiml/infra, `forjar drift` printed
//! `DRIFTED: paiml-implement-skill on gx10 (transport error: transport timeout:
//! script on 'gx10-a5b5' exceeded 60s limit)` with `Actual: ERROR` — a verdict
//! about a resource nothing had measured, in the same word as a real difference,
//! counted in `drift_count`. The resource was converged: every clause of its
//! `completion_check` passed when run by hand; it only took longer than the limit.
//! The remediation that verdict invites is allowlisting the resource, which blinds
//! the tripwire to it for good.
//!
//! WHY THESE ASSERTIONS. The machine is at a TEST-NET-3 address nothing routes —
//! the fixture forjar#407 already uses — so no answer is possible. The BINARY is
//! driven, because an exit code and a JSON document are what a tripwire consumes.
//!   1. `--tripwire` exits with the Connection class (4): not success, and not a
//!      drift verdict.
//!   2. The resource is listed in `unmeasured`, naming the address it could not
//!      reach, and is absent from `findings`.
//!   3. `drift_count == findings.len()` still holds — paiml/infra's
//!      drift-tripwire.sh cross-checks exactly that and fails a report that
//!      breaks it.
//!   4. Text output never prints `No drift detected.` over a run that measured
//!      nothing, and never prints the resource as DRIFTED.

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

fn blake3_of(path: &Path) -> String {
    let bytes = std::fs::read(path).expect("read bait");
    format!("blake3:{}", blake3::hash(&bytes).to_hex())
}

/// A file that exists on the controller with the locked hash, over an
/// unroutable machine (copied from forjar#407's falsifier).
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

fn run_drift(cfg: &Path, state_dir: &Path, extra: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(forjar())
        .arg("drift")
        .arg("-f")
        .arg(cfg)
        .arg("--state-dir")
        .arg(state_dir)
        .args(extra)
        .output()
        .expect("spawn forjar");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The first JSON document on stdout; anything after it is ignored.
fn report_of(stdout: &str, stderr: &str) -> serde_json::Value {
    let start = stdout
        .find('{')
        .unwrap_or_else(|| panic!("no JSON on stdout.\nstdout: {stdout}\nstderr: {stderr}"));
    serde_json::Deserializer::from_str(&stdout[start..])
        .into_iter::<serde_json::Value>()
        .next()
        .and_then(Result::ok)
        .unwrap_or_else(|| panic!("stdout is not JSON.\nstdout: {stdout}\nstderr: {stderr}"))
}

/// FALSIFY-549-001 — could-not-measure is its own verdict, with its own exit class.
#[test]
fn an_unreachable_resource_is_unmeasured_not_drifted() {
    let d = tempfile::tempdir().expect("tempdir");
    let (cfg, state_dir) = unreachable_fleet(d.path());
    let (code, stdout, stderr) = run_drift(&cfg, &state_dir, &["--json", "--tripwire"]);
    let report = report_of(&stdout, &stderr);

    let findings = report["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("no `findings` array: {report}"));
    assert_eq!(
        report["drift_count"].as_u64(),
        Some(findings.len() as u64),
        "drift_count must equal findings.len() — paiml/infra's tripwire rejects a report that breaks it: {report}"
    );
    assert!(
        findings.iter().all(|f| f["resource"] != "conf"),
        "forjar#549: a resource nothing could reach was reported as a drift FINDING: {report}"
    );

    let unmeasured = report["unmeasured"]
        .as_array()
        .unwrap_or_else(|| panic!("forjar#549: no `unmeasured` array: {report}"));
    let conf = unmeasured
        .iter()
        .find(|u| u["resource"] == "conf")
        .unwrap_or_else(|| panic!("forjar#549: `conf` is not reported as unmeasured: {report}"));
    assert_eq!(conf["machine"], "web", "{report}");
    let detail = conf["detail"].as_str().unwrap_or("");
    assert!(
        detail.contains("203.0.113.9"),
        "the unmeasured verdict must name what could not be reached: {detail}"
    );
    assert_eq!(
        report["unmeasured_count"].as_u64(),
        Some(unmeasured.len() as u64),
        "{report}"
    );

    assert_eq!(
        code,
        Some(4),
        "--tripwire over a resource it could not reach must exit with the Connection class (4).\nstderr: {stderr}"
    );
}

/// FALSIFY-549-002 — the text surface cannot call an unmeasured run clean, or drifted.
#[test]
fn text_output_names_unmeasured_and_never_calls_it_clean_or_drifted() {
    let d = tempfile::tempdir().expect("tempdir");
    let (cfg, state_dir) = unreachable_fleet(d.path());
    let (_code, stdout, stderr) = run_drift(&cfg, &state_dir, &[]);
    assert!(
        !stdout.contains("No drift detected."),
        "a run that measured nothing printed a clean verdict:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("UNMEASURED"),
        "the resource that could not be measured is not named:\n{stdout}\n{stderr}"
    );
    assert!(
        !stdout.contains("DRIFTED"),
        "an unreachable resource was printed as DRIFTED:\n{stdout}\n{stderr}"
    );
}
