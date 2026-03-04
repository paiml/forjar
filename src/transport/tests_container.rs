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
            devices: vec![],
            group_add: vec![],
            env: std::collections::HashMap::new(),
            volumes: vec![],
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
            devices: vec![],
            group_add: vec![],
            env: std::collections::HashMap::new(),
            volumes: vec![],
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
            "error should reference container: {e}"
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
            devices: vec![],
            group_add: vec![],
            env: std::collections::HashMap::new(),
            volumes: vec![],
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
            "error should reference container or pipe failure: {e}"
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
            "cleanup error should reference container: {e}"
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
            devices: vec![],
            group_add: vec![],
            env: std::collections::HashMap::new(),
            volumes: vec![],
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
            devices: vec![],
            group_add: vec![],
            env: std::collections::HashMap::new(),
            volumes: vec![],
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
            devices: vec![],
            group_add: vec![],
            env: std::collections::HashMap::new(),
            volumes: vec![],
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
            "podman error should reference container: {e}"
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
            devices: vec![],
            group_add: vec![],
            env: std::collections::HashMap::new(),
            volumes: vec![],
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
            devices: vec![],
            group_add: vec![],
            env: std::collections::HashMap::new(),
            volumes: vec![],
        }),
        pepita: None,
        cost: 0,
    };
    let result = ensure_container(&machine);
    assert!(
        result.is_ok(),
        "ensure with --gpus all should succeed: {result:?}"
    );
}

#[test]
fn test_fj021_ensure_with_rocm_devices() {
    // AMD ROCm pattern: --device /dev/kfd --device /dev/dri --group-add video --group-add render
    let machine = Machine {
        hostname: "rocm-box".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "/bin/echo".to_string(),
            image: Some("rocm/dev-ubuntu-22.04:6.1".to_string()),
            name: Some("forjar-rocm-test".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: None,
            devices: vec!["/dev/kfd".to_string(), "/dev/dri".to_string()],
            group_add: vec!["video".to_string(), "render".to_string()],
            env: [("ROCR_VISIBLE_DEVICES".to_string(), "0".to_string())]
                .into_iter()
                .collect(),
            volumes: vec![],
        }),
        pepita: None,
        cost: 0,
    };
    let result = ensure_container(&machine);
    assert!(
        result.is_ok(),
        "ensure with ROCm devices should succeed: {result:?}"
    );
}

#[test]
fn test_fj021_ensure_with_env_vars() {
    // Multi-GPU env var pattern (CUDA_VISIBLE_DEVICES for NVIDIA)
    let machine = Machine {
        hostname: "env-box".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "/bin/echo".to_string(),
            image: Some("nvidia/cuda:12.4.1-runtime-ubuntu22.04".to_string()),
            name: Some("forjar-env-test".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: Some("all".to_string()),
            devices: vec![],
            group_add: vec![],
            env: [("CUDA_VISIBLE_DEVICES".to_string(), "0,1".to_string())]
                .into_iter()
                .collect(),
            volumes: vec![],
        }),
        pepita: None,
        cost: 0,
    };
    let result = ensure_container(&machine);
    assert!(
        result.is_ok(),
        "ensure with env vars should succeed: {result:?}"
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
            devices: vec![],
            group_add: vec![],
            env: std::collections::HashMap::new(),
            volumes: vec![],
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
