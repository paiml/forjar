//! Tests: ssh retry policy of `transport::exec_script_retry`, driven with an
//! injected transport so no verdict depends on the host's ssh or network.

use crate::core::types;
use crate::transport;

fn make_local_machine() -> types::Machine {
    types::Machine {
        hostname: "localhost".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
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

fn make_ssh_machine() -> types::Machine {
    types::Machine {
        hostname: "remote".to_string(),
        addr: "10.99.99.99".to_string(),
        user: "deploy".to_string(),
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

#[test]
fn test_exec_script_retry_ssh_fails_immediately_non_transient() {
    // A non-transient transport error must be returned after exactly one
    // attempt. The transport is injected, so the verdict does not depend on
    // the host's ssh, network or known_hosts.
    let machine = make_ssh_machine();
    let mut calls = 0u32;
    let result = transport::exec_script_retry_with(&machine, 3, || {
        calls += 1;
        Err("ssh: Permission denied (publickey)".to_string())
    });
    assert!(result.is_err());
    assert_eq!(calls, 1, "non-transient error must not be retried");
}

#[test]
fn test_exec_script_retry_ssh_transient_is_retried() {
    let machine = make_ssh_machine();
    let mut calls = 0u32;
    let result = transport::exec_script_retry_with(&machine, 2, || {
        calls += 1;
        Err("ssh: connect: Connection refused".to_string())
    });
    assert!(result.is_err());
    assert_eq!(calls, 2, "transient error must use every attempt");
}

#[test]
fn test_exec_script_retry_local_wrapper_single_attempt_ok() {
    // The public wrapper on a local target: one attempt, no network.
    let machine = make_local_machine();
    let out = transport::exec_script_retry(&machine, "true", Some(10), 3)
        .expect("local exec should succeed");
    assert!(out.success());
}
