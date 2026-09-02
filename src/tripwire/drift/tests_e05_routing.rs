//! E05 (#407, agy lane): a container or pepita machine that declares a local
//! `addr` is NOT the controller. Its files live inside the namespace, so the
//! detector must go through the transport for it, exactly as it does for an
//! SSH machine — reading the controller's path of the same name answers about
//! the wrong host.

use super::file::reads_the_controller;
use crate::core::types::Machine;

fn machine(addr: &str, transport: Option<&str>) -> Machine {
    Machine {
        hostname: "h".to_string(),
        addr: addr.to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: transport.map(str::to_string),
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

#[test]
fn a_plain_local_machine_is_the_controller() {
    assert!(reads_the_controller(&machine("127.0.0.1", None)));
    assert!(reads_the_controller(&machine("localhost", None)));
}

#[test]
fn a_remote_machine_is_not() {
    assert!(!reads_the_controller(&machine("203.0.113.9", None)));
}

#[test]
fn a_container_at_a_local_addr_is_not_the_controller() {
    assert!(!reads_the_controller(&machine(
        "127.0.0.1",
        Some("container")
    )));
    assert!(!reads_the_controller(&machine("container", None)));
}

#[test]
fn a_pepita_machine_at_a_local_addr_is_not_the_controller() {
    assert!(!reads_the_controller(&machine("127.0.0.1", Some("pepita"))));
}
