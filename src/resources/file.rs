//! FJ-007: File/directory resource handler.

use crate::core::types::Resource;

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
        _ => format!(
            "test -f '{}' && echo 'exists:file' || echo 'missing:file'",
            path
        ),
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
        _ => {
            // Regular file
            if let Some(parent) = std::path::Path::new(path).parent() {
                if parent != std::path::Path::new("/") {
                    lines.push(format!("mkdir -p '{}'", parent.display()));
                }
            }
            if let Some(ref content) = resource.content {
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
            fs_type: None,
            options: None,
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
    fn test_fj007_apply_file_at_root_no_mkdir() {
        // File at root path (/) should NOT have `mkdir -p '/'`
        let mut r = make_file_resource("/init", Some("boot script"));
        r.owner = None;
        let script = apply_script(&r);
        assert!(script.contains("cat > '/init'"));
        assert!(!script.contains("mkdir -p '/'"));
    }
}
