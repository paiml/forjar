use super::file::*;
use base64::Engine;
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
        driver_version: None,
        cuda_version: None,
        devices: vec![],
        persistence_mode: None,
        compute_mode: None,
        gpu_memory_limit_mb: None,
        pre_apply: None,
        post_apply: None,
    }
}

#[test]
fn test_fj007_check_file() {
    let r = make_file_resource("/etc/test.conf", None);
    let script = check_script(&r);
    assert!(script.contains("test -f '/etc/test.conf'"));
}

#[test]
fn test_fj007_apply_file_with_content() {
    let r = make_file_resource("/etc/exports", Some("/data 192.168.1.0/24(ro)"));
    let script = apply_script(&r);
    assert!(script.contains("cat > '/etc/exports'"));
    assert!(script.contains("FORJAR_EOF"));
    assert!(script.contains("/data 192.168.1.0/24(ro)"));
    assert!(script.contains("chown 'root:root'"));
    assert!(script.contains("chmod '0644'"));
}

#[test]
fn test_fj007_apply_directory() {
    let mut r = make_file_resource("/data/transcripts", None);
    r.state = Some("directory".to_string());
    r.owner = Some("noah".to_string());
    r.group = None;
    r.mode = Some("0755".to_string());
    let script = apply_script(&r);
    assert!(script.contains("mkdir -p '/data/transcripts'"));
    assert!(script.contains("chown 'noah'"));
    assert!(script.contains("chmod '0755'"));
}

#[test]
fn test_fj007_apply_absent() {
    let mut r = make_file_resource("/tmp/garbage", None);
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(script.contains("rm -rf '/tmp/garbage'"));
}

#[test]
fn test_fj007_apply_symlink() {
    let mut r = make_file_resource("/usr/local/bin/tool", None);
    r.state = Some("symlink".to_string());
    r.target = Some("/opt/tool/bin/tool".to_string());
    let script = apply_script(&r);
    assert!(script.contains("ln -sfn '/opt/tool/bin/tool' '/usr/local/bin/tool'"));
}

#[test]
fn test_fj007_heredoc_safe() {
    // Content with quotes and special chars should be safe inside heredoc
    let r = make_file_resource("/etc/test", Some("key=\"value\"\n$HOME/path"));
    let script = apply_script(&r);
    assert!(script.contains("FORJAR_EOF"));
    // Single-quoted heredoc delimiter prevents variable expansion
    assert!(script.contains("<<'FORJAR_EOF'"));
}

#[test]
fn test_fj007_check_script_directory() {
    let mut r = make_file_resource("/data/dir", None);
    r.state = Some("directory".to_string());
    let script = check_script(&r);
    assert!(script.contains("test -d '/data/dir'"));
    assert!(script.contains("exists:directory"));
}

#[test]
fn test_fj007_check_script_absent() {
    let mut r = make_file_resource("/tmp/gone", None);
    r.state = Some("absent".to_string());
    let script = check_script(&r);
    assert!(script.contains("test -e '/tmp/gone'"));
    assert!(script.contains("exists:present"));
}

#[test]
fn test_fj007_check_script_symlink() {
    let mut r = make_file_resource("/usr/bin/tool", None);
    r.state = Some("symlink".to_string());
    let script = check_script(&r);
    assert!(script.contains("test -L '/usr/bin/tool'"));
    assert!(script.contains("exists:symlink"));
}

#[test]
fn test_fj007_state_query_script() {
    let r = make_file_resource("/etc/config", None);
    let script = state_query_script(&r);
    assert!(script.contains("/etc/config"));
    assert!(script.contains("stat"));
    assert!(script.contains("MISSING"));
}

#[test]
fn test_fj007_apply_file_owner_no_group() {
    let mut r = make_file_resource("/etc/test.conf", Some("data"));
    r.group = None;
    let script = apply_script(&r);
    assert!(script.contains("chown 'root' '/etc/test.conf'"));
    assert!(!script.contains("chown 'root:"));
}

#[test]
fn test_fj007_apply_directory_owner_and_group() {
    let mut r = make_file_resource("/data/exports", None);
    r.state = Some("directory".to_string());
    r.owner = Some("nfs".to_string());
    r.group = Some("nfs".to_string());
    r.mode = None;
    let script = apply_script(&r);
    assert!(script.contains("mkdir -p '/data/exports'"));
    assert!(script.contains("chown 'nfs:nfs' '/data/exports'"));
}

#[test]
fn test_fj007_check_script_unknown_state() {
    let mut r = make_file_resource("/tmp/test", None);
    r.state = Some("custom-state".to_string());
    let script = check_script(&r);
    assert!(script.contains("unsupported file state: custom-state"));
}

#[test]
fn test_fj007_apply_script_unknown_state() {
    let mut r = make_file_resource("/tmp/test", None);
    r.state = Some("custom-state".to_string());
    let script = apply_script(&r);
    assert!(script.contains("unsupported file state: custom-state"));
}

#[test]
fn test_fj007_check_script_explicit_file_state() {
    // Verify explicit "file" state works the same as default
    let mut r = make_file_resource("/etc/conf", None);
    r.state = Some("file".to_string());
    let script = check_script(&r);
    assert!(script.contains("test -f '/etc/conf'"));
    assert!(script.contains("exists:file"));
}

#[test]
fn test_fj007_apply_file_at_root_no_mkdir() {
    // File at root path (/) should NOT have `mkdir -p '/'`
    let mut r = make_file_resource("/init", Some("boot script"));
    r.owner = None;
    let script = apply_script(&r);
    assert!(script.contains("cat > '/init'"));
    assert!(!script.contains("mkdir -p '/'"));
}

#[test]
fn test_fj035_source_file_base64() {
    // Create a temp file to use as source
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("config.txt");
    std::fs::write(&source_path, "hello world\n").unwrap();

    let mut r = make_file_resource("/etc/app/config.txt", None);
    r.source = Some(source_path.to_str().unwrap().to_string());
    let script = apply_script(&r);

    // Should use base64 decode pipeline
    assert!(script.contains("base64 -d"));
    assert!(script.contains("/etc/app/config.txt"));
    // Should contain the base64 encoding of "hello world\n"
    let expected_b64 = base64::engine::general_purpose::STANDARD.encode(b"hello world\n");
    assert!(script.contains(&expected_b64));
}

#[test]
fn test_fj035_source_file_missing() {
    let mut r = make_file_resource("/etc/app/config.txt", None);
    r.source = Some("/nonexistent/path/file.txt".to_string());
    let script = apply_script(&r);
    assert!(script.contains("ERROR: cannot read source file"));
}

#[test]
fn test_fj035_source_takes_precedence_over_content() {
    // When both source and content are set, source wins (though validator rejects this)
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("from-source.txt");
    std::fs::write(&source_path, "from source").unwrap();

    let mut r = make_file_resource("/etc/test", Some("from content"));
    r.source = Some(source_path.to_str().unwrap().to_string());
    let script = apply_script(&r);

    // Source path is checked first, so base64 should be used
    assert!(script.contains("base64 -d"));
    assert!(!script.contains("FORJAR_EOF"));
}

#[test]
fn test_fj035_source_binary_file() {
    // Verify binary content is safely transferred via base64
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("binary.bin");
    let binary_data: Vec<u8> = (0..=255).collect();
    std::fs::write(&source_path, &binary_data).unwrap();

    let mut r = make_file_resource("/opt/bin/data.bin", None);
    r.source = Some(source_path.to_str().unwrap().to_string());
    let script = apply_script(&r);

    assert!(script.contains("base64 -d"));
    assert!(script.contains("/opt/bin/data.bin"));
}

#[test]
fn test_fj007_apply_file_creates_parent_dir() {
    let r = make_file_resource("/etc/app/nested/config.yaml", Some("key: val"));
    let script = apply_script(&r);
    assert!(script.contains("mkdir -p '/etc/app/nested'"));
    let mkdir_idx = script.find("mkdir -p").unwrap();
    let cat_idx = script.find("cat >").unwrap();
    assert!(
        mkdir_idx < cat_idx,
        "mkdir must precede cat in apply script"
    );
}
