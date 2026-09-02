//! Adversarial review of the forjar#407 fix.
//!
//! Two questions the E05 tests do not answer:
//!
//! 1. Does the verb now actually QUERY a non-file resource, or did it only
//!    stop mislabelling the skip? `tests_drift_e05` asserts the census reason
//!    changed from `no config loaded` to `no observed state` — both are skips.
//!    A fix that reached `detect_drift_full_reported` and still inspected
//!    nothing but files would pass it.
//! 2. What does the new execution do to forjar#372's promise? `sanitize_config`
//!    strips `ambient_inputs`, `sops`/`op` and `output_equivalence`, and the
//!    module's own header says this surface "never executes what a config
//!    declares". A `completion_check` is a config-declared shell command.

use super::types::*;
use crate::mcp::DriftHandler;
use pforge_runtime::Handler;

/// A lock naming resources, written the way a successful apply would.
fn write_lock(state_dir: &std::path::Path, machine: &str, resources_yaml: &str) {
    let md = state_dir.join(machine);
    std::fs::create_dir_all(&md).unwrap();
    std::fs::write(
        md.join("state.lock.yaml"),
        format!(
            "schema: \"1.0\"\nmachine: {machine}\nhostname: {machine}\ngenerated_at: now\n\
             generator: test\nblake3_version: \"1\"\nresources:\n{resources_yaml}"
        ),
    )
    .unwrap();
}

/// THE DENOMINATOR IS NOT ENOUGH: something must be INSPECTED.
///
/// A package whose lock entry records an observed hash is comparable, and the
/// only way to compare it is to run `state_query_script` on the machine. Before
/// the fix this resource was census-skipped as `NoConfigLoaded` and no query
/// ran; the census test in `tests_drift_e05` swaps that for `NoObservedState`,
/// which is still a skip. This one requires the query to have HAPPENED —
/// `inspected_by_type.package == 1` and a finding whose actual hash is the
/// digest of what the machine said.
#[tokio::test]
async fn a_locked_package_with_an_observed_hash_is_actually_queried() {
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        "version: \"1.0\"\nname: e05query\nmachines:\n  local:\n    hostname: localhost\n\
         \x20   addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: local\n\
         \x20   provider: apt\n    packages: [forjar-not-a-real-package]\n",
    )
    .unwrap();

    let state_dir = d.path().join("state");
    write_lock(
        &state_dir,
        "local",
        "  pkg:\n    type: package\n    status: converged\n    hash: \"h\"\n\
         \x20   observed: \"a-hash-the-machine-cannot-produce\"\n",
    );

    let out = DriftHandler
        .handle(DriftInput {
            path: cfg.display().to_string(),
            state_dir: Some(state_dir.display().to_string()),
            machine: None,
        })
        .await
        .expect("drift runs");

    let json = serde_json::to_value(&out).expect("DriftOutput serialises");
    let m = &json["census"][0];
    assert_eq!(
        m["inspected_by_type"]["package"], 1,
        "forjar#407: the package was never queried — a census that only \
         renames the skip reason is not the fix: {m}"
    );
    let f = out
        .findings
        .iter()
        .find(|f| f.resource == "pkg")
        .unwrap_or_else(|| panic!("no verdict for `pkg`: {:?}", out.findings));
    assert_ne!(
        f.actual_hash, "a-hash-the-machine-cannot-produce",
        "the actual hash must come from the target's stdout: {f:?}"
    );
    assert!(out.drifted, "{json}");
}

/// forjar#372 ON THE VERB THAT NOW EXECUTES.
///
/// `core::unattended`'s header: "strip everything a config can make a READ verb
/// EXECUTE", because `readOnlyHint: true` is the only signal an agent has
/// before calling a tool unattended against a repository it did not write.
/// `sanitize_config` strips three keys — and a `completion_check` is a fourth
/// command string a config author types, which `task_check::check_task_drift`
/// hands to `transport::exec_script_timeout`. For a `local` machine that
/// transport is the CONTROLLER's own shell.
///
/// The trap below is `touch`; the same slot takes `curl … | sh`. The state dir
/// is the one a checkout ships (`state/` beside the config), so nothing here
/// requires the caller to have applied anything.
#[tokio::test]
async fn the_readonly_verb_must_not_run_a_config_declared_completion_check() {
    let d = tempfile::tempdir().unwrap();
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
    .unwrap();

    let state_dir = d.path().join("state");
    write_lock(
        &state_dir,
        "local",
        "  guard:\n    type: task\n    status: converged\n    hash: \"h\"\n",
    );

    let _ = DriftHandler
        .handle(DriftInput {
            path: cfg.display().to_string(),
            state_dir: Some(state_dir.display().to_string()),
            machine: None,
        })
        .await
        .expect("drift runs");

    assert!(
        !trap.exists(),
        "forjar#372: `forjar_drift` publishes readOnlyHint: true and \
         `unattended::sanitize_config` promises this surface never executes \
         what a config declares — but the completion_check of a locked task \
         ran on the controller. An agent pointed at an untrusted checkout \
         carrying its own `state/` executes whatever that checkout declares."
    );
}

/// The disclosure that makes the skip above honest.
///
/// Not running the check is only defensible if the answer says so: an empty
/// findings list over an unexecuted assertion is exactly the false CLEAN
/// forjar#380 built the census against. `SkipReason::TaskChecksDisabled`
/// already exists for the CLI's `--no-task-checks`.
#[tokio::test]
async fn declining_to_run_the_check_is_reported_not_silent() {
    let d = tempfile::tempdir().unwrap();
    let trap = d.path().join("COMPLETION_CHECK_FIRED");
    let cfg = d.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: e05disclose\nmachines:\n  local:\n    hostname: localhost\n\
             \x20   addr: 127.0.0.1\nresources:\n  guard:\n    type: task\n    machine: local\n\
             \x20   command: \"true\"\n    completion_check: \"touch {}\"\n",
            trap.display()
        ),
    )
    .unwrap();

    let state_dir = d.path().join("state");
    write_lock(
        &state_dir,
        "local",
        "  guard:\n    type: task\n    status: converged\n    hash: \"h\"\n",
    );

    let out = DriftHandler
        .handle(DriftInput {
            path: cfg.display().to_string(),
            state_dir: Some(state_dir.display().to_string()),
            machine: None,
        })
        .await
        .expect("drift runs");

    let json = serde_json::to_value(&out).expect("DriftOutput serialises");
    let m = &json["census"][0];
    assert_eq!(
        m["skipped_by_reason"]["--no-task-checks"], 1,
        "the assertion was not executed and the census does not say so: {m}"
    );
    assert!(
        out.unattended_skipped.iter().any(|s| s.contains("guard")),
        "forjar#372's disclosure names what was skipped and why: {:?}",
        out.unattended_skipped
    );
}
