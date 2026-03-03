use super::tests_helpers::make_base_resource;
use super::*;

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
