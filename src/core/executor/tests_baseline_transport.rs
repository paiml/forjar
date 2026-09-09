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

use super::test_fixtures::*;
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
