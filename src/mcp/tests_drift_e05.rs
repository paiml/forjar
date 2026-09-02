//! forjar#407 (CRUX E05): the agent-facing drift verb answered about the
//! CONTROLLER, not the machine.
//!
//! `DriftHandler` had `config.machines[name]` in hand and called
//! `drift::detect_drift(&lock)` — which is `detect_drift_reported(lock, None)`.
//! With no machine, `check_file_resource_drift` hashes the controller's
//! filesystem and reports the answer as the target's state, and every non-file
//! resource is census-skipped as `NoConfigLoaded` even though the config was
//! parsed three lines earlier.
//!
//! Measured on 1.23.1 against a machine at 203.0.113.9 (TEST-NET-3,
//! unroutable): `{"drifted": false, "findings": []}` in 0.016s, nothing
//! contacted. That is forjar#305's false CLEAN, fixed in `file.rs` and still
//! live on every transport an agent can reach.

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

/// THE E05 REPRODUCTION. A file that exists on the controller, matching the
/// locked `content_hash`, over a machine at an unroutable TEST-NET address.
///
/// The only honest answers are "the host could not be reached" or "this machine
/// was not checked". `drifted: false` with an empty `unchecked` is a clean bill
/// of health for a host nothing ever spoke to.
#[tokio::test]
async fn drift_over_an_unreachable_machine_must_not_answer_clean() {
    let d = tempfile::tempdir().unwrap();
    let target = d.path().join("on-controller.txt");
    std::fs::write(&target, "controller copy").unwrap();
    let content_hash = crate::tripwire::hasher::hash_file(&target).unwrap();

    let cfg = d.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: e05\nmachines:\n  web:\n    hostname: web\n\
             \x20   addr: 203.0.113.9\n    user: root\nresources:\n  conf:\n    type: file\n\
             \x20   machine: web\n    path: {}\n    content: \"controller copy\"\n",
            target.display()
        ),
    )
    .unwrap();

    let state_dir = d.path().join("state");
    write_lock(
        &state_dir,
        "web",
        &format!(
            "  conf:\n    type: file\n    status: converged\n    hash: \"h\"\n\
             \x20   details:\n      path: \"{}\"\n      content_hash: \"{content_hash}\"\n",
            target.display()
        ),
    );

    let out = DriftHandler
        .handle(DriftInput {
            path: cfg.display().to_string(),
            state_dir: Some(state_dir.display().to_string()),
            machine: None,
        })
        .await
        .expect("drift runs");

    assert!(
        out.drifted || !out.unchecked.is_empty(),
        "forjar#407: the verb answered `drifted: false` about 203.0.113.9 \
         without contacting it — it hashed the CONTROLLER's copy of the file \
         and reported that as the remote host's state. findings={:?} \
         unchecked={:?}",
        out.findings,
        out.unchecked
    );

    let finding = out
        .findings
        .iter()
        .find(|f| f.resource == "conf")
        .unwrap_or_else(|| panic!("no verdict for `conf`: {:?}", out.findings));
    assert!(
        finding.actual_hash == "ERROR" || finding.actual_hash == "MISSING",
        "the target was never asked: an unroutable host cannot yield a real \
         content hash, so `{}` proves the controller's filesystem was read \
         instead. detail={}",
        finding.actual_hash,
        finding.detail
    );
}

/// The denominator. A package in the lock is not comparable by content hash,
/// and the verb said nothing about that at all — `DriftOutput` had no census
/// field, so an agent reading `drifted: false` could not tell six inspected
/// resources from zero.
///
/// The machine here is local and the package lock entry records no observed
/// state, so this test asks nothing of any host: it asserts only what the
/// output DISCLOSES about its own coverage.
#[tokio::test]
async fn drift_output_carries_the_denominator() {
    let d = tempfile::tempdir().unwrap();
    let target = d.path().join("managed.txt");
    std::fs::write(&target, "managed").unwrap();
    let content_hash = crate::tripwire::hasher::hash_file(&target).unwrap();

    let cfg = d.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: e05census\nmachines:\n  local:\n    hostname: localhost\n\
             \x20   addr: 127.0.0.1\nresources:\n  conf:\n    type: file\n    machine: local\n\
             \x20   path: {}\n    content: managed\n  pkg:\n    type: package\n\
             \x20   machine: local\n    provider: apt\n    packages: [curl]\n",
            target.display()
        ),
    )
    .unwrap();

    let state_dir = d.path().join("state");
    write_lock(
        &state_dir,
        "local",
        &format!(
            "  conf:\n    type: file\n    status: converged\n    hash: \"h\"\n\
             \x20   details:\n      path: \"{}\"\n      content_hash: \"{content_hash}\"\n\
             \x20 pkg:\n    type: package\n    status: converged\n    hash: \"h\"\n",
            target.display()
        ),
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
    let census = json.get("census").unwrap_or_else(|| {
        panic!(
            "forjar#407: DriftOutput carries no census, so `drifted: false` \
             stands over an unstated population — the CLI has printed the \
             denominator since forjar#380. got={json}"
        )
    });
    let m = &census[0];

    assert_eq!(m["in_scope"], 2, "both locked resources are in scope: {m}");
    assert_eq!(
        m["inspected"], 1,
        "the file was compared, the package was not: {m}"
    );
    assert!(
        m["skipped_by_reason"]
            .get("no config loaded (file hashes only)")
            .is_none(),
        "the config WAS loaded — the handler parsed it before loading the \
         lock. Reporting the package as unseen for want of a config is the \
         E05 census defect: {m}"
    );
    assert_eq!(
        m["skipped_by_reason"]["no observed state in the lock"], 1,
        "the real reason the package was not compared: {m}"
    );
    assert_eq!(json["resources_inspected"], 1, "{json}");
    assert_eq!(json["resources_skipped"], 1, "{json}");
}
