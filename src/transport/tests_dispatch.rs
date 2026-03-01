use super::*;

#[test]
fn test_transport_local_detection() {
    assert!(is_local_addr("127.0.0.1"));
    assert!(is_local_addr("localhost"));
    assert!(is_local_addr("::1"));
    assert!(!is_local_addr("192.168.1.100"));
    assert!(!is_local_addr("10.0.0.1"));
}

/// BH-MUT-0001: Kill mutation of exec_script local dispatch.
/// Verify local execution works for 127.0.0.1 and localhost addresses.
#[test]
fn test_transport_exec_local_127() {
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script(&machine, "echo ok").unwrap();
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "ok");
}

/// BH-MUT-0001: Verify localhost also dispatches locally.
#[test]
fn test_transport_exec_local_localhost() {
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "localhost".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script(&machine, "echo local").unwrap();
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "local");
}

#[test]
fn test_transport_exec_output_success() {
    let ok = ExecOutput {
        exit_code: 0,
        stdout: "ok".into(),
        stderr: "".into(),
    };
    assert!(ok.success());
    let fail = ExecOutput {
        exit_code: 1,
        stdout: "".into(),
        stderr: "err".into(),
    };
    assert!(!fail.success());
    let sig = ExecOutput {
        exit_code: 137,
        stdout: "".into(),
        stderr: "killed".into(),
    };
    assert!(!sig.success());
}

#[test]
fn test_transport_query_delegates() {
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = query(&machine, "echo query-test").unwrap();
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "query-test");
}

#[test]
fn test_timeout_none_succeeds() {
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script_timeout(&machine, "echo ok", None).unwrap();
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "ok");
}

#[test]
fn test_timeout_generous_succeeds() {
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script_timeout(&machine, "echo fast", Some(10)).unwrap();
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "fast");
}

#[test]
fn test_timeout_exceeded_returns_error() {
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let result = exec_script_timeout(&machine, "sleep 10", Some(1));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("timeout"));
}

#[test]
fn test_transport_timeout_error_includes_hostname() {
    let machine = Machine {
        hostname: "slow-box".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let err = exec_script_timeout(&machine, "sleep 10", Some(1)).unwrap_err();
    assert!(
        err.contains("slow-box"),
        "timeout error should include hostname: {}",
        err
    );
}

#[test]
fn test_transport_container_dispatch_priority() {
    // Container transport takes priority even if addr is a valid IP
    use crate::core::types::ContainerConfig;
    let machine = Machine {
        hostname: "hybrid".to_string(),
        addr: "127.0.0.1".to_string(), // Would normally be local
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "/bin/echo".to_string(),
            image: Some("test:latest".to_string()),
            name: Some("forjar-dispatch-test".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: None,
            devices: vec![],
            group_add: vec![],
            env: std::collections::HashMap::new(),
            volumes: vec![],
        }),
        pepita: None,
        cost: 0,
    };
    // With container transport, exec_script dispatches to container, not local
    // /bin/echo as runtime won't run bash properly, so it will fail or produce empty output
    let result = exec_script(&machine, "echo should-not-reach-local");
    if let Ok(out) = result {
        // If /bin/echo handled it, stdout won't contain "should-not-reach-local"
        // because echo doesn't execute bash
        assert_ne!(
            out.stdout.trim(),
            "should-not-reach-local",
            "container transport should intercept before local dispatch"
        );
    }
    // Err is expected: /bin/echo can't exec bash
}

#[test]
fn test_transport_ipv6_loopback_is_local() {
    assert!(is_local_addr("::1"));
}

#[test]
fn test_transport_remote_addr_not_local() {
    assert!(!is_local_addr("8.8.8.8"));
    assert!(!is_local_addr("google.com"));
    assert!(!is_local_addr("192.168.1.1"));
}

#[test]
fn test_transport_exec_captures_both_streams() {
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script(&machine, "echo OUT; echo ERR >&2").unwrap();
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "OUT");
    assert!(out.stderr.contains("ERR"));
}

#[test]
fn test_transport_exec_multiline_script() {
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let script = "A=hello\nB=world\necho \"$A $B\"";
    let out = exec_script(&machine, script).unwrap();
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "hello world");
}

#[test]
fn test_transport_exec_nonzero_exit_code() {
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script(&machine, "exit 77").unwrap();
    assert!(!out.success());
    assert_eq!(out.exit_code, 77);
}

#[test]
fn test_transport_timeout_error_includes_seconds() {
    let machine = Machine {
        hostname: "slow".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let err = exec_script_timeout(&machine, "sleep 10", Some(1)).unwrap_err();
    assert!(
        err.contains("1s"),
        "error should include timeout value: {}",
        err
    );
}

#[test]
fn test_transport_exec_empty_script() {
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script(&machine, "").unwrap();
    assert!(out.success());
}

#[test]
fn test_transport_exec_output_debug() {
    // ExecOutput should derive Debug
    let out = ExecOutput {
        exit_code: 0,
        stdout: "test".to_string(),
        stderr: "".to_string(),
    };
    let debug = format!("{:?}", out);
    assert!(debug.contains("exit_code: 0"));
}

#[test]
fn test_transport_exec_output_clone() {
    // ExecOutput should derive Clone
    let out = ExecOutput {
        exit_code: 42,
        stdout: "test".to_string(),
        stderr: "err".to_string(),
    };
    let cloned = out.clone();
    assert_eq!(cloned.exit_code, 42);
    assert_eq!(cloned.stdout, "test");
    assert_eq!(cloned.stderr, "err");
}

#[test]
fn test_transport_query_is_readonly_alias() {
    // query() is just an alias for exec_script — verify same output
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let q = query(&machine, "echo q").unwrap();
    let e = exec_script(&machine, "echo q").unwrap();
    assert_eq!(q.stdout, e.stdout);
    assert_eq!(q.exit_code, e.exit_code);
}

// --- FJ-132: Transport dispatch edge cases ---

#[test]
fn test_fj132_exec_script_special_chars_in_output() {
    // Verify transport preserves special characters in stdout
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script(&machine, r#"printf 'tab\there\nnewline'"#).unwrap();
    assert!(out.success());
    assert!(out.stdout.contains("tab"));
}

