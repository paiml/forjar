use super::file::*;
use crate::core::types::{MachineTarget, Resource, ResourceType};

fn make_file_resource(path: &str, content: Option<&str>) -> Resource {
    Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("m1".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: Some(path.to_string()),
        content: content.map(|s| s.to_string()),
        source: None,
        target: None,
        owner: Some("root".to_string()),
        group: Some("root".to_string()),
        mode: Some("0644".to_string()),
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
        inputs: std::collections::HashMap::new(),
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
    }
}

#[test]
fn test_fj007_apply_pipefail() {
    let r = make_file_resource("/etc/test", Some("data"));
    let script = apply_script(&r);
    assert!(
        script.starts_with("set -euo pipefail"),
        "apply scripts must start with pipefail"
    );
}

#[test]
fn test_fj007_apply_file_no_content_no_source() {
    let mut r = make_file_resource("/etc/test", None);
    r.source = None;
    let script = apply_script(&r);
    assert!(!script.contains("cat >"));
    assert!(!script.contains("base64"));
    assert!(script.contains("chown 'root:root'"));
    assert!(script.contains("chmod '0644'"));
}

#[test]
fn test_fj007_apply_file_no_mode() {
    let mut r = make_file_resource("/etc/test", Some("data"));
    r.mode = None;
    let script = apply_script(&r);
    assert!(!script.contains("chmod"), "no chmod when mode is None");
}

#[test]
fn test_fj007_apply_file_no_owner() {
    let mut r = make_file_resource("/etc/test", Some("data"));
    r.owner = None;
    r.group = None;
    let script = apply_script(&r);
    assert!(!script.contains("chown"), "no chown when owner is None");
}

#[test]
fn test_fj007_symlink_default_target() {
    let mut r = make_file_resource("/usr/bin/link", None);
    r.state = Some("symlink".to_string());
    r.target = None;
    let script = apply_script(&r);
    assert!(script.contains("ln -sfn '/dev/null' '/usr/bin/link'"));
}

#[test]
fn test_fj007_state_query_has_fallback() {
    let r = make_file_resource("/etc/test", None);
    let script = state_query_script(&r);
    assert!(script.contains("stat -c"), "Linux stat format");
    assert!(script.contains("stat -f"), "macOS stat fallback");
    assert!(script.contains("blake3sum"), "BLAKE3 hash check");
    assert!(
        script.contains("sha256sum"),
        "SHA256 fallback when blake3sum unavailable"
    );
}

// --- FJ-132: File resource edge case tests ---

#[test]
fn test_fj132_apply_heredoc_hard_quoted() {
    let r = make_file_resource("/etc/test", Some("$HOME ${PATH} $(whoami)"));
    let script = apply_script(&r);
    assert!(
        script.contains("FORJAR_EOF"),
        "should use FORJAR_EOF heredoc marker"
    );
    assert!(script.contains("$HOME"));
    assert!(script.contains("${PATH}"));
}

#[test]
fn test_fj132_apply_directory_uses_mkdir() {
    let mut r = make_file_resource("/opt/app/data", None);
    r.state = Some("directory".to_string());
    let script = apply_script(&r);
    assert!(script.contains("mkdir -p '/opt/app/data'"));
    assert!(
        !script.contains("cat >"),
        "directory should not write file content"
    );
}

#[test]
fn test_fj132_apply_absent_uses_rm() {
    let mut r = make_file_resource("/etc/old.conf", None);
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(script.contains("rm -rf '/etc/old.conf'"));
}

#[test]
fn test_fj132_check_directory_state() {
    let mut r = make_file_resource("/opt/app", None);
    r.state = Some("directory".to_string());
    let script = check_script(&r);
    assert!(
        script.contains("test -d '/opt/app'"),
        "directory check should use test -d"
    );
}

#[test]
fn test_fj132_check_symlink_state() {
    let mut r = make_file_resource("/usr/bin/link", None);
    r.state = Some("symlink".to_string());
    r.target = Some("/opt/bin/tool".to_string());
    let script = check_script(&r);
    assert!(
        script.contains("test -L '/usr/bin/link'"),
        "symlink check should use test -L"
    );
}

#[test]
fn test_fj132_source_missing_file_generates_error() {
    let mut r = make_file_resource("/opt/app/binary", None);
    r.source = Some("nonexistent-builds/app".to_string());
    r.content = None;
    let script = apply_script(&r);
    assert!(
        script.contains("ERROR: cannot read source file"),
        "missing source file should generate error in script"
    );
}

// -- FJ-036: Additional file resource tests --

#[test]
fn test_fj036_file_apply_directory() {
    let mut r = make_file_resource("/var/lib/myapp/data", None);
    r.state = Some("directory".to_string());
    r.owner = Some("app".to_string());
    r.group = Some("app".to_string());
    r.mode = Some("0750".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("mkdir -p '/var/lib/myapp/data'"),
        "directory state must emit mkdir -p"
    );
    assert!(
        script.contains("chown 'app:app' '/var/lib/myapp/data'"),
        "directory with owner:group must emit chown"
    );
    assert!(
        script.contains("chmod '0750' '/var/lib/myapp/data'"),
        "directory with mode must emit chmod"
    );
}

#[test]
fn test_fj036_file_apply_absent() {
    let mut r = make_file_resource("/etc/legacy/old.conf", None);
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("rm -rf '/etc/legacy/old.conf'"),
        "absent state must emit rm -rf with the path"
    );
    assert!(!script.contains("chown"), "absent should not emit chown");
    assert!(!script.contains("chmod"), "absent should not emit chmod");
}

#[test]
fn test_fj036_file_apply_symlink() {
    let mut r = make_file_resource("/etc/nginx/sites-enabled/mysite", None);
    r.state = Some("symlink".to_string());
    r.target = Some("/etc/nginx/sites-available/mysite".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains(
            "ln -sfn '/etc/nginx/sites-available/mysite' '/etc/nginx/sites-enabled/mysite'"
        ),
        "symlink state must emit ln -sfn with target then path"
    );
}

#[test]
fn test_fj036_file_check_directory() {
    let mut r = make_file_resource("/var/data", None);
    r.state = Some("directory".to_string());
    let script = check_script(&r);
    assert!(
        script.contains("test -d '/var/data'"),
        "directory check must use test -d"
    );
    assert!(
        script.contains("exists:directory"),
        "directory check must report exists:directory on success"
    );
    assert!(
        script.contains("missing:directory"),
        "directory check must report missing:directory on failure"
    );
}

#[test]
fn test_fj153_file_owner_no_group() {
    let mut r = make_file_resource("/etc/conf", Some("data"));
    r.owner = Some("deploy".to_string());
    r.group = None;
    let script = apply_script(&r);
    assert!(script.contains("chown 'deploy' '/etc/conf'"));
    assert!(!script.contains("chown 'deploy:"));
}

#[test]
fn test_fj153_file_dir_owner_no_group() {
    let mut r = make_file_resource("/var/data", None);
    r.state = Some("directory".to_string());
    r.owner = Some("app".to_string());
    r.group = None;
    let script = apply_script(&r);
    assert!(script.contains("chown 'app' '/var/data'"));
    assert!(!script.contains("chown 'app:"));
}

#[test]
fn test_fj153_file_no_owner_no_mode() {
    let mut r = make_file_resource("/tmp/test", Some("hello"));
    r.owner = None;
    r.group = None;
    r.mode = None;
    let script = apply_script(&r);
    assert!(!script.contains("chown"));
    assert!(!script.contains("chmod"));
    assert!(script.contains("hello"));
}

#[test]
fn test_fj153_file_check_symlink() {
    let mut r = make_file_resource("/etc/link", None);
    r.state = Some("symlink".to_string());
    let script = check_script(&r);
    assert!(script.contains("test -L '/etc/link'"));
    assert!(script.contains("exists:symlink"));
    assert!(script.contains("missing:symlink"));
}

#[test]
fn test_fj153_file_check_absent() {
    let mut r = make_file_resource("/tmp/old", None);
    r.state = Some("absent".to_string());
    let script = check_script(&r);
    assert!(script.contains("test -e '/tmp/old'"));
    assert!(script.contains("exists:present"));
    assert!(script.contains("missing:absent"));
}

#[test]
fn test_fj153_file_check_unknown_state() {
    let mut r = make_file_resource("/tmp/x", None);
    r.state = Some("custom".to_string());
    let script = check_script(&r);
    assert!(script.contains("unsupported file state: custom"));
}

#[test]
fn test_fj153_file_apply_unknown_state() {
    let mut r = make_file_resource("/tmp/x", None);
    r.state = Some("custom".to_string());
    let script = apply_script(&r);
    assert!(script.contains("unsupported file state: custom"));
}

#[test]
fn test_fj153_file_symlink_default_target() {
    let mut r = make_file_resource("/etc/link", None);
    r.state = Some("symlink".to_string());
    r.target = None;
    let script = apply_script(&r);
    assert!(script.contains("ln -sfn '/dev/null' '/etc/link'"));
}

#[test]
fn test_fj153_file_root_path_no_mkdir() {
    let r = make_file_resource("/test.conf", Some("data"));
    let script = apply_script(&r);
    assert!(
        !script.contains("mkdir -p '/'"),
        "should not mkdir -p for root path"
    );
}

#[test]
fn test_fj153_file_state_query_default_path() {
    let mut r = make_file_resource("/x", None);
    r.path = None;
    let script = state_query_script(&r);
    assert!(script.contains("/dev/null"));
}

#[test]
fn test_fj036_file_apply_chown_group() {
    let mut r = make_file_resource("/etc/app/config.yaml", Some("port: 8080"));
    r.owner = Some("deploy".to_string());
    r.group = Some("www-data".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("chown 'deploy:www-data' '/etc/app/config.yaml'"),
        "chown must include owner:group format when both are provided"
    );
}
