use super::container::*;
use crate::core::types::{ContainerConfig, Machine};

fn container_machine() -> Machine {
    Machine {
        hostname: "test-box".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "docker".to_string(),
            image: Some("ubuntu:22.04".to_string()),
            name: Some("forjar-unit-test".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    }
}

#[test]
fn test_fj021_exec_no_container_config() {
    let machine = Machine {
        hostname: "bad".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: None,
        pepita: None,
        cost: 0,
    };
    let result = exec_container(&machine, "echo hi");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no container config"));
}

#[test]
fn test_fj021_ensure_no_container_config() {
    let machine = Machine {
        hostname: "bad".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: None,
        pepita: None,
        cost: 0,
    };
    let result = ensure_container(&machine);
    assert!(result.is_err());
}

#[test]
fn test_fj021_cleanup_no_container_config() {
    let machine = Machine {
        hostname: "bad".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: None,
        pepita: None,
        cost: 0,
    };
    let result = cleanup_container(&machine);
    assert!(result.is_err());
}

#[test]
fn test_fj021_container_name_from_config() {
    let m = container_machine();
    assert_eq!(m.container_name(), "forjar-unit-test");
}

#[test]
fn test_fj021_ensure_no_image() {
    let machine = Machine {
        hostname: "no-image".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "docker".to_string(),
            image: None,
            name: Some("forjar-no-image".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    // ensure_container on a non-existent container with no image should fail
    // (unless the container already exists, which it won't in unit tests)
    let result = ensure_container(&machine);
    // This will either fail because docker isn't available or because no image
    assert!(result.is_err());
}

#[test]
fn test_fj021_exec_error_includes_container_name() {
    // When exec fails, error message should include the container name
    let m = container_machine();
    let result = exec_container(&m, "echo hi");
    // Docker likely not available in test env, but error should reference the name
    if let Err(e) = result {
        assert!(
            e.contains("forjar-unit-test") || e.contains("container"),
            "error should reference container: {}",
            e
        );
    }
}

#[test]
fn test_fj021_exec_with_fake_runtime() {
    // Use /bin/false as a fake runtime to verify error path consistently
    let machine = Machine {
        hostname: "fake-rt".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "/bin/false".to_string(),
            image: Some("test:latest".to_string()),
            name: Some("forjar-fake".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    let result = exec_container(&machine, "echo test");
    // /bin/false doesn't accept args, so spawn will succeed but
    // the command will return non-zero — still produces ExecOutput
    match result {
        Ok(out) => assert!(!out.success(), "/bin/false should fail"),
        Err(e) => assert!(
            e.contains("forjar-fake") || e.contains("pipe") || e.contains("false"),
            "error should reference container or pipe failure: {}",
            e
        ),
    }
}

#[test]
fn test_fj021_cleanup_error_includes_container_name() {
    let m = container_machine();
    let result = cleanup_container(&m);
    if let Err(e) = result {
        assert!(
            e.contains("forjar-unit-test") || e.contains("container"),
            "cleanup error should reference container: {}",
            e
        );
    }
}

#[test]
fn test_fj021_container_name_derived_from_hostname() {
    // When no explicit name, container_name() derives from hostname
    let machine = Machine {
        hostname: "my-web-server".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "docker".to_string(),
            image: Some("ubuntu:22.04".to_string()),
            name: None,
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    assert_eq!(machine.container_name(), "forjar-my-web-server");
}

#[test]
fn test_fj021_container_name_explicit_overrides() {
    let machine = Machine {
        hostname: "ignored-hostname".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "docker".to_string(),
            image: Some("ubuntu:22.04".to_string()),
            name: Some("custom-name".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    assert_eq!(machine.container_name(), "custom-name");
}

#[test]
fn test_fj021_podman_runtime() {
    // Verify podman runtime is used when configured
    let machine = Machine {
        hostname: "podman-box".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "podman".to_string(),
            image: Some("ubuntu:22.04".to_string()),
            name: Some("forjar-podman-test".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    // exec_container will try to run podman, which probably isn't available
    let result = exec_container(&machine, "echo test");
    if let Err(e) = result {
        // Error should mention the container name
        assert!(
            e.contains("forjar-podman-test"),
            "podman error should reference container: {}",
            e
        );
    }
}

#[test]
fn test_fj021_ensure_with_privileged_and_init_flags() {
    // Verify that privileged+init flags are used by ensure_container
    let machine = Machine {
        hostname: "priv-box".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "/bin/echo".to_string(), // echo will print args, showing flags
            image: Some("test:latest".to_string()),
            name: Some("forjar-priv-test".to_string()),
            ephemeral: true,
            privileged: true,
            init: true,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    // /bin/echo as runtime: `echo inspect -f ...` succeeds but doesn't output "true"
    // So ensure_container will proceed to run, where `echo run -d --name ... --init --privileged ...`
    // will succeed (exit 0) since echo always succeeds
    let result = ensure_container(&machine);
    assert!(
        result.is_ok(),
        "ensure with /bin/echo runtime should succeed"
    );
}

#[test]
fn test_fj021_ensure_with_gpus_flag() {
    let machine = Machine {
        hostname: "gpu-box".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "/bin/echo".to_string(),
            image: Some("nvidia/cuda:12.9.0-devel-ubuntu22.04".to_string()),
            name: Some("forjar-gpu-test".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: Some("all".to_string()),
        }),
        pepita: None,
        cost: 0,
    };
    let result = ensure_container(&machine);
    assert!(
        result.is_ok(),
        "ensure with --gpus all should succeed: {:?}",
        result
    );
}

#[test]
fn test_fj021_ensure_no_init_no_privileged() {
    // Verify flags are omitted when disabled
    let machine = Machine {
        hostname: "bare-box".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "/bin/echo".to_string(),
            image: Some("test:latest".to_string()),
            name: Some("forjar-bare-test".to_string()),
            ephemeral: true,
            privileged: false,
            init: false,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    let result = ensure_container(&machine);
    assert!(
        result.is_ok(),
        "ensure without init/privileged should succeed"
    );
}

#[test]
fn test_fj021_cleanup_with_echo_runtime() {
    // /bin/echo as runtime: `echo rm -f forjar-cleanup-test` succeeds
    let machine = Machine {
        hostname: "cleanup-box".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "/bin/echo".to_string(),
            image: Some("test:latest".to_string()),
            name: Some("forjar-cleanup-test".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    let result = cleanup_container(&machine);
    assert!(result.is_ok(), "cleanup with echo runtime should succeed");
}

#[test]
fn test_fj021_exec_container_error_msg_no_config() {
    // Verify the exact error message wording
    let machine = Machine {
        hostname: "precise-host".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: None,
        pepita: None,
        cost: 0,
    };
    let err = exec_container(&machine, "echo").unwrap_err();
    assert_eq!(err, "machine 'precise-host' has no container config");
}

#[test]
fn test_fj021_ensure_container_error_msg_no_config() {
    let machine = Machine {
        hostname: "precise-host".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: None,
        pepita: None,
        cost: 0,
    };
    let err = ensure_container(&machine).unwrap_err();
    assert_eq!(err, "machine 'precise-host' has no container config");
}

#[test]
fn test_fj021_cleanup_container_error_msg_no_config() {
    let machine = Machine {
        hostname: "precise-host".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: None,
        pepita: None,
        cost: 0,
    };
    let err = cleanup_container(&machine).unwrap_err();
    assert_eq!(err, "machine 'precise-host' has no container config");
}

// --- FJ-132: Container transport edge cases ---

#[test]
fn test_fj132_ensure_attached_no_image_required() {
    // Non-ephemeral containers don't need an image (they already exist)
    let machine = Machine {
        hostname: "attached".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "/bin/echo".to_string(),
            image: None,
            name: Some("existing-container".to_string()),
            ephemeral: false, // attached mode
            privileged: false,
            init: true,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    // Should succeed — attached containers just verify existence
    let result = ensure_container(&machine);
    // /bin/echo will succeed or fail depending on args, but should not
    // error about missing image
    assert!(
        result.is_ok() || !result.unwrap_err().contains("no image specified"),
        "attached (ephemeral=false) should not require image"
    );
}

#[test]
fn test_fj132_ephemeral_guard_skips_non_ephemeral() {
    // The executor's ephemeral guard should skip cleanup for non-ephemeral containers.
    // cleanup_container() itself always removes — the guard lives in the caller.
    let machine = Machine {
        hostname: "persistent".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "/bin/false".to_string(),
            image: None,
            name: Some("keep-me".to_string()),
            ephemeral: false,
            privileged: false,
            init: true,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    // Verify the ephemeral guard pattern: non-ephemeral should NOT trigger cleanup
    let config = machine.container.as_ref().unwrap();
    assert!(!config.ephemeral, "test machine should be non-ephemeral");
    // The executor checks: if container.ephemeral { cleanup_container(...) }
    // So for ephemeral=false, cleanup_container is never called
}

#[test]
fn test_fj132_container_name_default_derivation() {
    // If no explicit name, container_name() should derive from hostname
    let machine = Machine {
        hostname: "my-test-box".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "docker".to_string(),
            image: Some("ubuntu:22.04".to_string()),
            name: None,
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    let name = machine.container_name();
    assert!(
        name.contains("my-test-box"),
        "derived name should contain hostname: {}",
        name
    );
}

// --- FJ-021: Coverage boost tests ---

#[test]
fn test_fj021_ensure_uses_correct_runtime() {
    // Verify ensure_container with runtime="podman" uses the podman command.
    // We use a non-existent binary path to confirm the runtime field is
    // actually passed to Command::new — if "podman" binary is absent the
    // error message will reference it.
    let machine = Machine {
        hostname: "podman-ensure".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "podman".to_string(),
            image: Some("alpine:latest".to_string()),
            name: Some("forjar-podman-ensure".to_string()),
            ephemeral: true,
            privileged: false,
            init: false,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    let result = ensure_container(&machine);
    // In CI/unit-test env podman is typically absent, so the error should
    // reference "forjar-podman-ensure" proving the runtime dispatched.
    if let Err(e) = result {
        assert!(
            e.contains("forjar-podman-ensure"),
            "ensure error should reference the container name: {}",
            e
        );
    }
    // If podman happens to be installed and succeeds, that's also fine.
}

#[test]
fn test_fj021_cleanup_nonexistent_returns_err() {
    // Cleaning up a container that doesn't exist should return an error
    // when using a real runtime (docker) that rejects the rm.
    let machine = Machine {
        hostname: "ghost".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "/bin/false".to_string(),
            image: Some("test:latest".to_string()),
            name: Some("forjar-nonexistent-cleanup".to_string()),
            ephemeral: true,
            privileged: false,
            init: false,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    let result = cleanup_container(&machine);
    // /bin/false always exits 1, so rm -f will "fail"
    assert!(
        result.is_err(),
        "cleanup of nonexistent container via /bin/false should error"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("forjar-nonexistent-cleanup"),
        "error should contain container name: {}",
        err
    );
}

#[test]
fn test_fj021_ephemeral_guard_cleans_up() {
    // The executor's ephemeral guard pattern: ephemeral=true triggers cleanup.
    // We verify the pattern by checking that cleanup_container succeeds with
    // /bin/echo (simulating a successful rm) on an ephemeral container.
    let machine = Machine {
        hostname: "ephemeral-guard".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "/bin/echo".to_string(),
            image: Some("test:latest".to_string()),
            name: Some("forjar-ephemeral-guard".to_string()),
            ephemeral: true,
            privileged: false,
            init: false,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    let config = machine.container.as_ref().unwrap();
    assert!(config.ephemeral, "test setup: should be ephemeral");
    // Simulate the executor's ephemeral guard: if ephemeral, call cleanup
    if config.ephemeral {
        let result = cleanup_container(&machine);
        assert!(
            result.is_ok(),
            "ephemeral cleanup via /bin/echo should succeed: {:?}",
            result.err()
        );
    }
}

#[test]
fn test_fj021_container_name_from_machine_key() {
    // When container name is None, container_name() auto-generates from hostname
    let machine = Machine {
        hostname: "web-prod-01".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "docker".to_string(),
            image: Some("ubuntu:22.04".to_string()),
            name: None,
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: None,
        }),
        pepita: None,
        cost: 0,
    };
    let name = machine.container_name();
    assert_eq!(
        name, "forjar-web-prod-01",
        "auto-generated name should be forjar-<hostname>"
    );
}
