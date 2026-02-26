//! FJ-007: File/directory resource handler.

use crate::core::types::Resource;
use base64::Engine;

/// Read a local file and return its base64-encoded content.
fn source_file_base64(path: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {}", path, e))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

/// Generate shell to check file state.
pub fn check_script(resource: &Resource) -> String {
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let state = resource.state.as_deref().unwrap_or("file");

    match state {
        "directory" => format!(
            "test -d '{}' && echo 'exists:directory' || echo 'missing:directory'",
            path
        ),
        "absent" => format!(
            "test -e '{}' && echo 'exists:present' || echo 'missing:absent'",
            path
        ),
        "symlink" => format!(
            "test -L '{}' && echo 'exists:symlink' || echo 'missing:symlink'",
            path
        ),
        "file" => format!(
            "test -f '{}' && echo 'exists:file' || echo 'missing:file'",
            path
        ),
        other => format!("echo 'unsupported file state: {}'", other),
    }
}

/// Generate shell to converge file to desired state.
pub fn apply_script(resource: &Resource) -> String {
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let state = resource.state.as_deref().unwrap_or("file");

    let mut lines = vec!["set -euo pipefail".to_string()];

    match state {
        "directory" => {
            lines.push(format!("mkdir -p '{}'", path));
            if let Some(ref owner) = resource.owner {
                if let Some(ref group) = resource.group {
                    lines.push(format!("chown '{}:{}' '{}'", owner, group, path));
                } else {
                    lines.push(format!("chown '{}' '{}'", owner, path));
                }
            }
            if let Some(ref mode) = resource.mode {
                lines.push(format!("chmod '{}' '{}'", mode, path));
            }
        }
        "absent" => {
            lines.push(format!("rm -rf '{}'", path));
        }
        "symlink" => {
            let target = resource.target.as_deref().unwrap_or("/dev/null");
            lines.push(format!("ln -sfn '{}' '{}'", target, path));
        }
        "file" => {
            // Regular file
            if let Some(parent) = std::path::Path::new(path).parent() {
                if parent != std::path::Path::new("/") {
                    lines.push(format!("mkdir -p '{}'", parent.display()));
                }
            }
            if let Some(ref source) = resource.source {
                // Read local file and base64-encode for safe transport
                match source_file_base64(source) {
                    Ok(b64) => {
                        lines.push(format!("echo '{}' | base64 -d > '{}'", b64, path));
                    }
                    Err(e) => {
                        lines.push(format!(
                            "echo 'ERROR: cannot read source file: {}'; exit 1",
                            e
                        ));
                    }
                }
            } else if let Some(ref content) = resource.content {
                // Write content via heredoc (safe, no injection)
                lines.push(format!(
                    "cat > '{}' <<'FORJAR_EOF'\n{}\nFORJAR_EOF",
                    path, content
                ));
            }
            if let Some(ref owner) = resource.owner {
                if let Some(ref group) = resource.group {
                    lines.push(format!("chown '{}:{}' '{}'", owner, group, path));
                } else {
                    lines.push(format!("chown '{}' '{}'", owner, path));
                }
            }
            if let Some(ref mode) = resource.mode {
                lines.push(format!("chmod '{}' '{}'", mode, path));
            }
        }
        other => {
            lines.push(format!("echo 'unsupported file state: {}'", other));
        }
    }

    lines.join("\n")
}

/// Generate shell to query file state (for hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    format!(
        "if [ -e '{}' ]; then\n\
           stat -c 'owner=%U group=%G mode=%a size=%s' '{}' 2>/dev/null || \
           stat -f 'owner=%Su group=%Sg mode=%Lp size=%z' '{}' 2>/dev/null\n\
           if [ -f '{}' ]; then\n\
             cat '{}' | blake3sum 2>/dev/null || sha256sum '{}' | cut -d' ' -f1\n\
           fi\n\
         else\n\
           echo 'MISSING'\n\
         fi",
        path, path, path, path, path, path
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

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
        // File with neither content nor source — just ownership/mode
        let mut r = make_file_resource("/etc/test", None);
        r.source = None;
        let script = apply_script(&r);
        assert!(!script.contains("cat >"));
        assert!(!script.contains("base64"));
        // But chown/chmod still happen
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
        // Symlink without explicit target uses /dev/null
        let mut r = make_file_resource("/usr/bin/link", None);
        r.state = Some("symlink".to_string());
        r.target = None;
        let script = apply_script(&r);
        assert!(script.contains("ln -sfn '/dev/null' '/usr/bin/link'"));
    }

    #[test]
    fn test_fj007_state_query_has_fallback() {
        // state_query_script should have both Linux stat and macOS stat fallback
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
        // Content should use hard-quoted heredoc to prevent variable expansion
        let r = make_file_resource("/etc/test", Some("$HOME ${PATH} $(whoami)"));
        let script = apply_script(&r);
        assert!(
            script.contains("FORJAR_EOF"),
            "should use FORJAR_EOF heredoc marker"
        );
        // The content itself should appear literally, not expanded
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
        // Source pointing to nonexistent file should generate error script
        let mut r = make_file_resource("/opt/app/binary", None);
        r.source = Some("nonexistent-builds/app".to_string());
        r.content = None;
        let script = apply_script(&r);
        assert!(
            script.contains("ERROR: cannot read source file"),
            "missing source file should generate error in script"
        );
    }

    // ── FJ-036: Additional file resource tests ───────────────────────

    #[test]
    fn test_fj036_file_apply_directory() {
        // state="directory" must generate mkdir -p
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
        // state="absent" must generate rm -rf
        let mut r = make_file_resource("/etc/legacy/old.conf", None);
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        assert!(
            script.contains("rm -rf '/etc/legacy/old.conf'"),
            "absent state must emit rm -rf with the path"
        );
        // absent should not emit chown or chmod
        assert!(!script.contains("chown"), "absent should not emit chown");
        assert!(!script.contains("chmod"), "absent should not emit chmod");
    }

    #[test]
    fn test_fj036_file_apply_symlink() {
        // state="symlink" must generate ln -sfn with target
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
        // check for directory must use test -d
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
        // chown must include owner:group when both are set
        let mut r = make_file_resource("/etc/app/config.yaml", Some("port: 8080"));
        r.owner = Some("deploy".to_string());
        r.group = Some("www-data".to_string());
        let script = apply_script(&r);
        assert!(
            script.contains("chown 'deploy:www-data' '/etc/app/config.yaml'"),
            "chown must include owner:group format when both are provided"
        );
    }
}
