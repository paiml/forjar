//! Container transport integration tests.
//!
//! Gated behind `--features container-test` since they require Docker.
//! Run: cargo test --features container-test

#![cfg(feature = "container-test")]

use forjar::core::types::*;
use forjar::transport;
use forjar::transport::container;

fn test_machine() -> Machine {
    Machine {
        hostname: "integration-test".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "docker".to_string(),
            image: Some("forjar-test-target".to_string()),
            name: Some("forjar-integration-test".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
        }),
    }
}

#[test]
fn test_container_lifecycle() {
    let machine = test_machine();

    // Ensure container starts
    container::ensure_container(&machine).expect("ensure_container failed");

    // Execute a simple script
    let out = container::exec_container(&machine, "echo hello-from-container")
        .expect("exec_container failed");
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "hello-from-container");

    // Cleanup
    container::cleanup_container(&machine).expect("cleanup_container failed");
}

#[test]
fn test_container_exec_dispatch() {
    let machine = test_machine();

    container::ensure_container(&machine).expect("ensure_container failed");

    // Test via the transport dispatch layer
    let out = transport::exec_script(&machine, "whoami").expect("exec_script failed");
    assert!(out.success());

    container::cleanup_container(&machine).expect("cleanup_container failed");
}

#[test]
fn test_container_file_resource() {
    let machine = test_machine();

    container::ensure_container(&machine).expect("ensure_container failed");

    // Simulate a file resource apply script
    let script = r#"
set -euo pipefail
cat > /tmp/forjar-test.txt << 'FORJAR_EOF'
hello from forjar container test
FORJAR_EOF
test -f /tmp/forjar-test.txt
cat /tmp/forjar-test.txt
"#;
    let out = transport::exec_script(&machine, script).expect("exec_script failed");
    assert!(out.success());
    assert!(out.stdout.contains("hello from forjar container test"));

    container::cleanup_container(&machine).expect("cleanup_container failed");
}

#[test]
fn test_container_idempotent_ensure() {
    let machine = test_machine();

    // First ensure
    container::ensure_container(&machine).expect("first ensure failed");

    // Second ensure should be a no-op (container already running)
    container::ensure_container(&machine).expect("second ensure failed");

    // Still works
    let out = container::exec_container(&machine, "echo ok").expect("exec failed");
    assert!(out.success());

    container::cleanup_container(&machine).expect("cleanup failed");
}
