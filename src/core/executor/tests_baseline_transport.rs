//! forjar#485, refiled with its mechanism: the baseline a lock records for a
//! file on a REMOTE machine is the CONTROLLER's copy of that path.
//!
//! `build_resource_details` reads through the transport for exactly one kind of
//! machine — `is_container_transport()`, which is `transport == "container" ||
//! addr == "container"`. Every SSH machine takes the other arm and hashes this
//! host's filesystem, so a fleet resource declaring `/home/<user>/.bashrc`
//! records the WORKSTATION's `.bashrc` as its baseline.
//!
//! THAT IS forjar#305'S ROOT CAUSE, FIXED ON THE READ SIDE AND LEFT ON THE
//! WRITE SIDE. `tripwire::drift::file::check_file_resource_drift` now asks the
//! machine, and its comment ends with the prescription this module needed:
//! "`exec_script` already dispatches pepita > container > local > SSH, so a
//! local machine still executes locally and nothing needs a special case."
//!
//! So drift's ACTUAL is right and its EXPECTED is a third file, and the gap is
//! permanent. The operator measured every half of that on the fleet: declared
//! and live byte-identical to each other, the stored expected matching neither,
//! a re-apply refreshing five fields and not this one, and the resources that
//! never converge being exactly those whose path also exists on the controller
//! — four of four, with the one resource lacking the field converging normally.
//!
//! NO SSH IS NEEDED. The branch is chosen from the machine's declaration alone,
//! before any connection is attempted.

use super::*;

fn remote_machine() -> Machine {
    Machine {
        hostname: "faraway".to_string(),
        // TEST-NET-1, unroutable by RFC 5737: nothing here may depend on
        // reaching it, and nothing here may hang trying.
        addr: "192.0.2.1".to_string(),
        user: "noah".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

fn file_on(machine: &str, path: &str, content: &str) -> Resource {
    Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single(machine.to_string()),
        path: Some(path.to_string()),
        content: Some(content.to_string()),
        ..Default::default()
    }
}

#[test]
fn a_remote_file_baseline_is_never_the_controllers_copy_of_that_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dot-bashrc");

    // The controller HAS a file here and it is not what the remote box holds.
    // That is the ordinary case for a dotfile path: it exists on the
    // workstation and on every machine in the fleet, differently.
    std::fs::write(&path, "# the WORKSTATION's copy\n").unwrap();
    let controllers_hash = crate::tripwire::hasher::hash_file(&path).unwrap();

    let resource = file_on(
        "faraway",
        path.to_str().unwrap(),
        "# the DECLARED content, which is what the remote box holds\n",
    );
    let details = build_resource_details(&resource, &remote_machine());
    let recorded = details
        .get("content_hash")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    assert_ne!(
        recorded.as_deref(),
        Some(controllers_hash.as_str()),
        "forjar#485: the lock baseline for a file on a REMOTE machine is a hash \
         of the CONTROLLER's file at the same path. drift then asks the machine \
         for the actual — correctly, since forjar#305 fixed the read side — and \
         compares it against a third file, so the resource can never be reported \
         converged again. Recorded {recorded:?}, which is this host's copy."
    );
}

#[test]
fn an_unreachable_target_records_no_baseline_rather_than_a_wrong_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dot-gitconfig");
    std::fs::write(&path, "# the workstation's copy\n").unwrap();

    let resource = file_on("faraway", path.to_str().unwrap(), "# declared\n");
    let details = build_resource_details(&resource, &remote_machine());

    assert!(
        !details.contains_key("content_hash"),
        "a baseline that could not be read is ABSENT, not guessed. Absent makes \
         `locked_file_target` return None and the file path decline to judge, \
         which is honest; a wrong one produces a confident verdict about a file \
         this host never saw. Recorded: {:?}",
        details.get("content_hash")
    );
}

/// AND THE LOCAL CASE MUST NOT REGRESS: a local target IS the controller, so it
/// still records the hash of the file it manages and local drift keeps working.
#[test]
fn a_local_machine_still_records_the_hash_of_the_file_it_manages() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("managed");
    std::fs::write(&path, "# managed here\n").unwrap();
    let local_hash = crate::tripwire::hasher::hash_file(&path).unwrap();

    let resource = file_on("test", path.to_str().unwrap(), "# managed here\n");
    let details = build_resource_details(&resource, &local_machine());

    assert_eq!(
        details.get("content_hash").and_then(|v| v.as_str()),
        Some(local_hash.as_str()),
        "a local machine must keep recording its own file, or this fix trades \
         one silent wrong answer for another"
    );
}

/// THE PREDICATE MUST BE THE ONE DRIFT USES, not one that resembles it.
///
/// `machine_is_local` excludes a container but NOT a pepita namespace, while
/// drift's `reads_the_controller` excludes both. Using the first here left
/// apply hashing the controller for a pepita machine while drift asked the
/// namespace: the original defect surviving one transport over, with the two
/// sides now permanently disagreed rather than merely both wrong. Three review
/// lanes found it independently.
#[test]
fn a_pepita_namespace_is_not_the_controller_either() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inside-the-namespace");
    std::fs::write(&path, "# the CONTROLLER's copy\n").unwrap();
    let controllers_hash = crate::tripwire::hasher::hash_file(&path).unwrap();

    // A pepita machine commonly declares a loopback addr — its files live
    // inside the namespace, not at the controller's path of the same name.
    let mut m = local_machine();
    m.transport = Some("pepita".to_string());

    let resource = file_on("test", path.to_str().unwrap(), "# declared\n");
    let details = build_resource_details(&resource, &m);

    assert_ne!(
        details.get("content_hash").and_then(|v| v.as_str()),
        Some(controllers_hash.as_str()),
        "forjar#485: a pepita machine declaring a loopback addr still recorded \
         the CONTROLLER's file as its baseline, because the predicate excluded \
         containers and forgot namespaces. drift excludes both, so apply and \
         drift would disagree for ever."
    );
}

/// AND THE TWO SIDES MUST AGREE BY CONSTRUCTION, not by resemblance.
///
/// `tripwire::drift::file::reads_the_controller` is module-private, so this
/// asserts the property from the writer's side: the predicate the baseline uses
/// must differ from `machine_is_local` exactly where drift differs from it —
/// on a container and on a pepita namespace. The delegation itself is one line
/// in `file.rs` and is visible in the diff.
#[test]
fn apply_and_drift_share_one_definition_of_local() {
    let mut container = local_machine();
    container.transport = Some("container".to_string());
    let mut pepita = local_machine();
    pepita.transport = Some("pepita".to_string());
    let mut remote = local_machine();
    remote.addr = "192.0.2.1".to_string();

    // The two that MUST be excluded, and that `machine_is_local` does not both
    // exclude — which is the whole finding.
    for m in [container, pepita] {
        assert!(
            !crate::transport::controller_answers_for(&m),
            "a machine whose files live somewhere other than this host's \
             filesystem must never have its baseline read from here: {m:?}"
        );
    }
    assert!(
        crate::transport::controller_answers_for(&local_machine()),
        "and a genuinely local machine must still be read locally, or the fix \
         trades one wrong answer for another"
    );
    assert!(
        !crate::transport::controller_answers_for(&remote),
        "a routable address is not this host"
    );
}

/// ONE READER, NOT TWO THAT RESEMBLE EACH OTHER.
///
/// The writer used a plain `cat '{path}'` while drift used
/// `if [ -d ]; then echo __DIR__; else cat; fi` and digested `ls -la` on
/// seeing that marker. They disagreed on a directory and on a file whose entire
/// content is the literal `__DIR__` — a permanent mismatch on a converged
/// resource, found by a review lane. Both sides call `remote_path_digest` now.
///
/// A behavioural test would need a reachable non-local machine, which this
/// suite deliberately does not have, so the invariant is asserted where it
/// lives: the writer must not build a read script of its own.
#[test]
fn the_baseline_writer_does_not_grow_a_second_read_protocol() {
    let src = include_str!("helpers.rs");
    assert!(
        src.contains("remote_path_digest"),
        "the remote arm of build_resource_details must go through the ONE \
         reader drift uses; a second protocol is how the __DIR__ collision got \
         in"
    );
    assert!(
        !src.contains("cat '{path}'"),
        "the writer has grown its own `cat` script again. Drift answers \
         `__DIR__` for a directory and digests a listing instead; a plain `cat` \
         disagrees with that on a directory and on a file whose content IS the \
         literal __DIR__."
    );
}
