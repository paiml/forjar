use super::hasher::*;

// --- FJ-132: Hasher edge case tests ---

#[test]
fn test_fj132_hash_string_empty() {
    let h = hash_string("");
    assert!(h.starts_with("blake3:"));
    assert_eq!(h.len(), 71);
}

#[test]
fn test_fj132_hash_string_unicode() {
    let h = hash_string("Hello 世界 🌍");
    assert!(h.starts_with("blake3:"));
    assert_eq!(h.len(), 71);
}

#[test]
fn test_fj132_hash_file_large() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.bin");
    let content = "x".repeat(STREAM_BUF_SIZE * 3 + 42);
    std::fs::write(&path, &content).unwrap();
    let h = hash_file(&path).unwrap();
    assert!(h.starts_with("blake3:"));
    assert_eq!(h.len(), 71);
}

#[test]
fn test_fj132_hash_directory_with_multiple_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "alpha").unwrap();
    std::fs::write(dir.path().join("b.txt"), "beta").unwrap();
    std::fs::write(dir.path().join("c.txt"), "gamma").unwrap();
    let h = hash_directory(dir.path()).unwrap();
    assert!(h.starts_with("blake3:"));

    std::fs::write(dir.path().join("b.txt"), "beta-changed").unwrap();
    let h2 = hash_directory(dir.path()).unwrap();
    assert_ne!(h, h2, "modifying a file should change directory hash");
}

#[test]
fn test_fj132_composite_hash_empty() {
    let h: String = composite_hash(&[]);
    assert!(h.starts_with("blake3:"));
    assert_eq!(h.len(), 71);
}

#[test]
fn test_fj132_composite_hash_single_element() {
    let h = composite_hash(&["only"]);
    assert!(h.starts_with("blake3:"));
    let h_str = hash_string("only");
    assert_ne!(h, h_str, "composite(x) != hash_string(x) due to separator");
}

// --- FJ-036: Hasher determinism and coverage tests ---

#[test]
fn test_fj036_hash_desired_state_deterministic() {
    use crate::core::planner::hash_desired_state;
    use crate::core::types::{MachineTarget, Resource, ResourceType};
    use std::collections::HashMap;

    let r = Resource {
        resource_type: ResourceType::Package,
        machine: MachineTarget::Single("m1".to_string()),
        state: Some("present".to_string()),
        depends_on: vec![],
        provider: Some("apt".to_string()),
        packages: vec!["curl".to_string()],
        version: None,
        path: None,
        content: None,
        source: None,
        target: None,
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
        command: None,
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
        completion_check: None,
        timeout: None,
        working_dir: None,
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
        build_machine: None,
    };
    let h1 = hash_desired_state(&r);
    let h2 = hash_desired_state(&r);
    assert_eq!(h1, h2, "hash_desired_state must be deterministic");
    assert!(h1.starts_with("blake3:"));
}

#[test]
fn test_fj036_hash_desired_state_changes_on_content() {
    use crate::core::planner::hash_desired_state;
    use crate::core::types::{MachineTarget, Resource, ResourceType};
    use std::collections::HashMap;

    let r1 = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("m1".to_string()),
        state: Some("present".to_string()),
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: Some("/etc/app.conf".to_string()),
        content: Some("original content".to_string()),
        source: None,
        target: None,
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
        command: None,
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
        completion_check: None,
        timeout: None,
        working_dir: None,
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
        build_machine: None,
    };
    let r2 = Resource {
        content: Some("changed content".to_string()),
        ..r1.clone()
    };
    let h1 = hash_desired_state(&r1);
    let h2 = hash_desired_state(&r2);
    assert_ne!(h1, h2, "hash must differ when resource content changes");
}

#[test]
fn test_fj036_hash_directory_empty() {
    let dir = tempfile::tempdir().unwrap();
    let h = hash_directory(dir.path()).unwrap();
    assert!(!h.is_empty(), "hash of empty directory must be non-empty");
    assert!(h.starts_with("blake3:"));
    assert_eq!(h.len(), 71);
}

#[test]
fn test_fj036_hash_string_deterministic() {
    let input = "forjar determinism check";
    let h1 = hash_string(input);
    let h2 = hash_string(input);
    assert_eq!(
        h1, h2,
        "hash_string must produce identical output for same input"
    );
    assert!(h1.starts_with("blake3:"));
}
