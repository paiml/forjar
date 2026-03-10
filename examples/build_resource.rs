//! FJ-33: Cross-compile build resource — codegen scripts.
//!
//! Demonstrates the `build` resource type that compiles on a powerful
//! build machine, then transfers the artifact to the deploy target.
//!
//! Usage: cargo run --example build_resource

use forjar::core::codegen;
use forjar::core::types::{MachineTarget, Resource, ResourceType};
use std::collections::HashMap;

fn base_build() -> Resource {
    Resource {
        resource_type: ResourceType::Build,
        machine: MachineTarget::Single("jetson".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: None,
        content: None,
        source: Some("/tmp/cross/release/apr".to_string()),
        target: Some("~/.cargo/bin/apr".to_string()),
        owner: None,
        group: None,
        mode: None,
        name: None,
        enabled: None,
        restart_on: vec![],
        triggers: vec![],
        fs_type: None,
        options: None,
        uid: None,
        shell: None,
        home: None,
        groups: vec![],
        ssh_authorized_keys: vec![],
        system_user: false,
        schedule: None,
        command: Some(
            "cargo build --release --target aarch64-unknown-linux-gnu -p apr-cli".to_string(),
        ),
        image: None,
        ports: vec![],
        environment: vec![],
        volumes: vec![],
        restart: None,
        protocol: None,
        port: None,
        action: None,
        from_addr: None,
        recipe: None,
        inputs: HashMap::new(),
        arch: vec![],
        tags: vec![],
        resource_group: None,
        when: None,
        count: None,
        for_each: None,
        chroot_dir: None,
        namespace_uid: None,
        namespace_gid: None,
        seccomp: false,
        netns: false,
        cpuset: None,
        memory_limit: None,
        overlay_lower: None,
        overlay_upper: None,
        overlay_work: None,
        overlay_merged: None,
        format: None,
        quantization: None,
        checksum: None,
        cache_dir: None,
        gpu_backend: None,
        driver_version: None,
        cuda_version: None,
        rocm_version: None,
        devices: vec![],
        persistence_mode: None,
        compute_mode: None,
        gpu_memory_limit_mb: None,
        output_artifacts: vec![],
        completion_check: Some("apr --version".to_string()),
        timeout: None,
        working_dir: Some("~/src/aprender".to_string()),
        task_mode: None,
        task_inputs: vec![],
        stages: vec![],
        cache: false,
        gpu_device: None,
        restart_delay: None,
        quality_gate: None,
        health_check: None,
        restart_policy: None,
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
        store: false,
        sudo: false,
        script: None,
        gather: vec![],
        scatter: vec![],
        build_machine: Some("intel".to_string()),
        repo: None,
        tag: None,
        asset_pattern: None,
        binary: None,
        install_dir: None,
        ..Default::default()
    }
}

fn main() {
    println!("=== FJ-33: Build Resource (Cross-Compile) ===\n");

    // Remote cross-compile: build on intel, deploy to jetson
    let build = base_build();

    println!("-- Check script (completion_check) --");
    println!("{}", codegen::check_script(&build).unwrap());

    println!("\n-- Apply script (SSH build → SCP transfer → verify) --");
    println!("{}", codegen::apply_script(&build).unwrap());

    println!("\n-- State query script (sha256sum of artifact) --");
    println!("{}", codegen::state_query_script(&build).unwrap());

    // Local build variant: build_machine = localhost
    println!("\n=== Build Resource (localhost, no SSH) ===\n");
    let local_build = Resource {
        build_machine: Some("localhost".to_string()),
        source: Some("/tmp/release/myapp".to_string()),
        target: Some("/usr/local/bin/myapp".to_string()),
        command: Some("cargo build --release -p myapp".to_string()),
        working_dir: Some("/home/user/src/myapp".to_string()),
        completion_check: Some("myapp --version".to_string()),
        ..base_build()
    };

    println!("-- Apply script (local cp, no SSH) --");
    println!("{}", codegen::apply_script(&local_build).unwrap());

    // Build without completion_check: fallback to test -x
    println!("\n=== Build Resource (no completion_check) ===\n");
    let no_check = Resource {
        completion_check: None,
        ..base_build()
    };

    println!("-- Check script (fallback: test -x target) --");
    println!("{}", codegen::check_script(&no_check).unwrap());
}
