#![allow(unused_imports)]
use super::container::*;
use crate::core::types::{ContainerConfig, Machine};

#[test]
fn test_fj021_ensure_with_volumes() {
    // Docker socket mount pattern for DinD / observability stacks
    let machine = Machine {
        hostname: "dind-box".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "/bin/echo".to_string(),
            image: Some("forjar-test-target".to_string()),
            name: Some("forjar-vol-test".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: None,
            devices: vec![],
            group_add: vec![],
            env: std::collections::HashMap::new(),
            volumes: vec![
                "/var/run/docker.sock:/var/run/docker.sock".to_string(),
                "/data:/container-data:ro".to_string(),
            ],
        }),
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    };
    let result = ensure_container(&machine);
    assert!(
        result.is_ok(),
        "ensure with volumes should succeed: {result:?}"
    );
}
