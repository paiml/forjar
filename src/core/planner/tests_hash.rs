use super::*;
use super::tests_helpers::make_base_resource;
use crate::core::types::MachineTarget;
use std::collections::HashMap;

#[test]
fn test_fj004_hash_deterministic() {
let r = Resource {
    resource_type: ResourceType::Package,
    machine: MachineTarget::Single("m1".to_string()),
    state: None,
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
    pre_apply: None,
    post_apply: None,
        lifecycle: None,
};
let h1 = hash_desired_state(&r);
let h2 = hash_desired_state(&r);
assert_eq!(h1, h2);
assert!(h1.starts_with("blake3:"));
}

#[test]
fn test_fj004_hash_includes_all_fields() {
let r1 = Resource {
    resource_type: ResourceType::Mount,
    machine: MachineTarget::Single("m1".to_string()),
    state: Some("mounted".to_string()),
    depends_on: vec![],
    provider: None,
    packages: vec![],
    version: None,
    path: Some("/mnt/data".to_string()),
    content: None,
    source: Some("192.168.1.1:/data".to_string()),
    target: Some("/mnt/target".to_string()),
    owner: Some("root".to_string()),
    group: Some("root".to_string()),
    mode: Some("0755".to_string()),
    name: Some("data-mount".to_string()),
    enabled: None,
    restart_on: vec![],
    triggers: vec![],
    fs_type: Some("nfs".to_string()),
    options: Some("ro,hard".to_string()),
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
    pre_apply: None,
    post_apply: None,
        lifecycle: None,
};
// Changing any field should change the hash
let mut r2 = r1.clone();
r2.fs_type = Some("ext4".to_string());
assert_ne!(hash_desired_state(&r1), hash_desired_state(&r2));

let mut r3 = r1.clone();
r3.options = Some("rw".to_string());
assert_ne!(hash_desired_state(&r1), hash_desired_state(&r3));

let mut r4 = r1.clone();
r4.name = Some("other-mount".to_string());
assert_ne!(hash_desired_state(&r1), hash_desired_state(&r4));
}

#[test]
fn test_fj004_hash_content_change_changes_hash() {
let r1 = Resource {
    resource_type: ResourceType::File,
    machine: MachineTarget::Single("m1".to_string()),
    state: None,
    depends_on: vec![],
    provider: None,
    packages: vec![],
    version: None,
    path: Some("/etc/test".to_string()),
    content: Some("version=1".to_string()),
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
    pre_apply: None,
    post_apply: None,
        lifecycle: None,
};
let mut r2 = r1.clone();
r2.content = Some("version=2".to_string());
assert_ne!(
    hash_desired_state(&r1),
    hash_desired_state(&r2),
    "content change must change hash"
);
}

#[test]
fn test_hash_sensitive_to_image() {
let mut r = make_base_resource(ResourceType::Docker);
r.image = Some("myapp:v1".to_string());
let h1 = hash_desired_state(&r);
r.image = Some("myapp:v2".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "image change must change hash");
}

#[test]
fn test_hash_sensitive_to_schedule() {
let mut r = make_base_resource(ResourceType::Cron);
r.schedule = Some("0 * * * *".to_string());
let h1 = hash_desired_state(&r);
r.schedule = Some("*/5 * * * *".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "schedule change must change hash");
}

#[test]
fn test_hash_sensitive_to_port() {
let mut r = make_base_resource(ResourceType::Network);
r.port = Some("80".to_string());
let h1 = hash_desired_state(&r);
r.port = Some("443".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "port change must change hash");
}

#[test]
fn test_hash_sensitive_to_restart_policy() {
let mut r = make_base_resource(ResourceType::Docker);
r.restart = Some("always".to_string());
let h1 = hash_desired_state(&r);
r.restart = Some("unless-stopped".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "restart policy change must change hash");
}

#[test]
fn test_hash_sensitive_to_enabled() {
let mut r = make_base_resource(ResourceType::Service);
r.enabled = Some(true);
let h1 = hash_desired_state(&r);
r.enabled = Some(false);
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "enabled change must change hash");
}

#[test]
fn test_hash_sensitive_to_ports_list() {
let mut r = make_base_resource(ResourceType::Docker);
r.ports = vec!["8080:80".to_string()];
let h1 = hash_desired_state(&r);
r.ports = vec!["8080:80".to_string(), "443:443".to_string()];
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "ports list change must change hash");
}

#[test]
fn test_hash_sensitive_to_environment() {
let mut r = make_base_resource(ResourceType::Docker);
r.environment = vec!["KEY=val1".to_string()];
let h1 = hash_desired_state(&r);
r.environment = vec!["KEY=val2".to_string()];
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "environment change must change hash");
}

#[test]
fn test_hash_sensitive_to_volumes() {
let mut r = make_base_resource(ResourceType::Docker);
r.volumes = vec!["/host:/container".to_string()];
let h1 = hash_desired_state(&r);
r.volumes = vec!["/other:/container".to_string()];
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "volumes change must change hash");
}

#[test]
fn test_hash_sensitive_to_from_addr() {
let mut r = make_base_resource(ResourceType::Network);
r.from_addr = Some("10.0.0.0/8".to_string());
let h1 = hash_desired_state(&r);
r.from_addr = Some("192.168.0.0/16".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "from_addr change must change hash");
}

#[test]
fn test_hash_sensitive_to_protocol() {
let mut r = make_base_resource(ResourceType::Network);
r.protocol = Some("tcp".to_string());
let h1 = hash_desired_state(&r);
r.protocol = Some("udp".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "protocol change must change hash");
}

#[test]
fn test_hash_sensitive_to_action() {
let mut r = make_base_resource(ResourceType::Network);
r.action = Some("allow".to_string());
let h1 = hash_desired_state(&r);
r.action = Some("deny".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "action change must change hash");
}

#[test]
fn test_hash_sensitive_to_shell() {
let mut r = make_base_resource(ResourceType::User);
r.shell = Some("/bin/bash".to_string());
let h1 = hash_desired_state(&r);
r.shell = Some("/bin/zsh".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "shell change must change hash");
}

#[test]
fn test_hash_sensitive_to_home() {
let mut r = make_base_resource(ResourceType::User);
r.home = Some("/home/deploy".to_string());
let h1 = hash_desired_state(&r);
r.home = Some("/opt/deploy".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "home change must change hash");
}

#[test]
fn test_hash_sensitive_to_target() {
let mut r = make_base_resource(ResourceType::File);
r.target = Some("/etc/nginx/nginx.conf".to_string());
let h1 = hash_desired_state(&r);
r.target = Some("/etc/nginx/sites.conf".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "target change must change hash");
}

#[test]
fn test_hash_sensitive_to_source() {
let mut r = make_base_resource(ResourceType::File);
r.source = Some("/tmp/src1".to_string());
let h1 = hash_desired_state(&r);
r.source = Some("/tmp/src2".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "source change must change hash");
}

#[test]
fn test_hash_sensitive_to_command() {
let mut r = make_base_resource(ResourceType::Cron);
r.command = Some("/usr/bin/backup.sh".to_string());
let h1 = hash_desired_state(&r);
r.command = Some("/usr/bin/cleanup.sh".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "command change must change hash");
}

#[test]
fn test_hash_sensitive_to_fs_type() {
let mut r = make_base_resource(ResourceType::Mount);
r.fs_type = Some("nfs".to_string());
let h1 = hash_desired_state(&r);
r.fs_type = Some("ext4".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "fs_type change must change hash");
}

#[test]
fn test_hash_sensitive_to_options() {
let mut r = make_base_resource(ResourceType::Mount);
r.options = Some("rw,noatime".to_string());
let h1 = hash_desired_state(&r);
r.options = Some("ro,noatime".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "options change must change hash");
}

#[test]
fn test_hash_sensitive_to_version() {
let mut r = make_base_resource(ResourceType::Package);
r.version = Some("1.0.0".to_string());
let h1 = hash_desired_state(&r);
r.version = Some("2.0.0".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "version change must change hash");
}

#[test]
fn test_hash_sensitive_to_restart() {
let mut r = make_base_resource(ResourceType::Docker);
r.restart = Some("always".to_string());
let h1 = hash_desired_state(&r);
r.restart = Some("unless-stopped".to_string());
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "restart policy change must change hash");
}

#[test]
fn test_hash_sensitive_to_restart_on() {
let mut r = make_base_resource(ResourceType::Service);
r.restart_on = vec!["config-file".to_string()];
let h1 = hash_desired_state(&r);
r.restart_on = vec!["other-file".to_string()];
let h2 = hash_desired_state(&r);
assert_ne!(h1, h2, "restart_on change must change hash");
}

#[test]
fn test_fj132_hash_deterministic() {
// Same resource hashed twice should produce identical hash
let r = make_base_resource(ResourceType::Package);
let h1 = hash_desired_state(&r);
let h2 = hash_desired_state(&r);
assert_eq!(h1, h2, "hashing must be deterministic");
}

#[test]
fn test_fj132_hash_format() {
let r = make_base_resource(ResourceType::File);
let h = hash_desired_state(&r);
assert!(h.starts_with("blake3:"), "hash should have blake3: prefix");
assert_eq!(h.len(), 71, "blake3 hash should be 71 chars");
}
