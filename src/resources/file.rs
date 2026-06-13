//! FJ-007: File/directory resource handler.

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;
use base64::Engine;

/// Read a local file and return its base64-encoded content.
fn source_file_base64(path: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

/// Generate shell to check file state.
pub fn check_script(resource: &Resource) -> String {
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let state = resource.state.as_deref().unwrap_or("file");
    let p = sh_squote(path);

    match state {
        "directory" => {
            format!("test -d {p} && echo 'exists:directory' || echo 'missing:directory'")
        }
        "absent" => format!("test -e {p} && echo 'exists:present' || echo 'missing:absent'"),
        "symlink" => format!("test -L {p} && echo 'exists:symlink' || echo 'missing:symlink'"),
        "file" => format!("test -f {p} && echo 'exists:file' || echo 'missing:file'"),
        // `other` is the config-derived state string; escape the label.
        other => format!(
            "echo {}",
            sh_squote(&format!("unsupported file state: {other}"))
        ),
    }
}

/// Append chown/chmod lines for the given resource ownership and mode.
fn push_ownership_lines(lines: &mut Vec<String>, path: &str, resource: &Resource) {
    let p = sh_squote(path);
    if let Some(ref owner) = resource.owner {
        if let Some(ref group) = resource.group {
            lines.push(format!(
                "chown {} {}",
                sh_squote(&format!("{owner}:{group}")),
                p
            ));
        } else {
            lines.push(format!("chown {} {}", sh_squote(owner), p));
        }
    }
    if let Some(ref mode) = resource.mode {
        lines.push(format!("chmod {} {}", sh_squote(mode), p));
    }
}

/// Generate the file-content write commands (source or inline content).
fn push_file_content_lines(lines: &mut Vec<String>, path: &str, resource: &Resource) {
    let p = sh_squote(path);
    if let Some(ref source) = resource.source {
        match source_file_base64(source) {
            Ok(b64) => {
                // b64 is forjar-generated (alphabet is shell-safe) but quote it
                // through the helper for uniformity.
                lines.push(format!("echo {} | base64 -d > {}", sh_squote(&b64), p));
            }
            Err(e) => {
                // `e` embeds the config-derived source path; escape the whole
                // message so a path with a quote can't break out of echo.
                lines.push(format!(
                    "echo {}; exit 1",
                    sh_squote(&format!("ERROR: cannot read source file: {e}"))
                ));
            }
        }
    } else if let Some(ref content) = resource.content {
        // Heredoc body is literal (quoted FORJAR_EOF), so content is not shell.
        lines.push(format!("cat > {p} <<'FORJAR_EOF'\n{content}\nFORJAR_EOF"));
    }
}

/// Generate shell to converge file to desired state.
pub fn apply_script(resource: &Resource) -> String {
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let state = resource.state.as_deref().unwrap_or("file");
    let p = sh_squote(path);

    let mut lines = vec!["set -euo pipefail".to_string()];

    match state {
        "directory" => {
            lines.push(format!("mkdir -p {p}"));
            push_ownership_lines(&mut lines, path, resource);
        }
        "absent" => {
            lines.push(format!("rm -rf {p}"));
        }
        "symlink" => {
            let target = resource.target.as_deref().unwrap_or("/dev/null");
            lines.push(format!("ln -sfn {} {p}", sh_squote(target)));
        }
        "file" => {
            if let Some(parent) = std::path::Path::new(path).parent() {
                if parent != std::path::Path::new("/") {
                    lines.push(format!(
                        "mkdir -p {}",
                        sh_squote(&parent.display().to_string())
                    ));
                }
            }
            push_file_content_lines(&mut lines, path, resource);
            push_ownership_lines(&mut lines, path, resource);
        }
        other => {
            // `other` is the config-derived state string; escape the label.
            lines.push(format!(
                "echo {}",
                sh_squote(&format!("unsupported file state: {other}"))
            ));
        }
    }

    lines.join("\n")
}

/// Generate shell to query file state (for hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let p = sh_squote(path);
    format!(
        "if [ -e {p} ]; then\n\
           stat -c 'owner=%U group=%G mode=%a size=%s' {p} 2>/dev/null || \
           stat -f 'owner=%Su group=%Sg mode=%Lp size=%z' {p} 2>/dev/null\n\
           if [ -f {p} ]; then\n\
             cat {p} | blake3sum 2>/dev/null || sha256sum {p} | cut -d' ' -f1\n\
           fi\n\
         else\n\
           echo 'MISSING'\n\
         fi"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn file_resource(path: &str) -> Resource {
        Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("m1".to_string()),
            path: Some(path.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn fj154_file_path_with_quote_is_escaped() {
        // Injection payload in path must be neutralized, not break out.
        let mut r = file_resource("/etc/x';reboot;'");
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        // The raw `;reboot;` is never left as bare shell — quote was escaped.
        assert!(script.contains("'/etc/x'\\'';reboot;'\\'''"));
        assert!(!script.contains("rm -rf '/etc/x';reboot"));
    }

    #[test]
    fn fj154_owner_injection_neutralized() {
        // Defect #14 canonical example: owner `x';reboot;'`.
        let mut r = file_resource("/etc/foo");
        r.state = Some("directory".to_string());
        r.owner = Some("x';reboot;'".to_string());
        let script = apply_script(&r);
        assert!(script.contains("'x'\\'';reboot;'\\'''"));
        // No bare `chown 'x';reboot` breakout.
        assert!(!script.contains("chown 'x';reboot"));
    }

    #[test]
    fn fj154_owner_group_mode_quoted() {
        let mut r = file_resource("/etc/foo");
        r.state = Some("directory".to_string());
        r.owner = Some("noah".to_string());
        r.group = Some("staff".to_string());
        r.mode = Some("0644".to_string());
        let script = apply_script(&r);
        assert!(script.contains("chown 'noah:staff' '/etc/foo'"));
        assert!(script.contains("chmod '0644' '/etc/foo'"));
    }

    #[test]
    fn fj154_symlink_target_quoted() {
        let mut r = file_resource("/link");
        r.state = Some("symlink".to_string());
        r.target = Some("/real/target".to_string());
        let script = apply_script(&r);
        assert!(script.contains("ln -sfn '/real/target' '/link'"));
    }

    #[test]
    fn fj154_inline_content_path_quoted() {
        let mut r = file_resource("/etc/conf");
        r.state = Some("file".to_string());
        r.content = Some("hello".to_string());
        let script = apply_script(&r);
        assert!(script.contains("cat > '/etc/conf' <<'FORJAR_EOF'"));
        assert!(script.contains("hello"));
    }

    #[test]
    fn fj154_check_and_query_paths_quoted() {
        let r = file_resource("/etc/foo");
        assert!(check_script(&r).contains("test -f '/etc/foo'"));
        assert!(state_query_script(&r).contains("[ -e '/etc/foo' ]"));
    }

    #[test]
    fn fj165_source_read_error_message_injection_neutralized() {
        // #165 (#161 sweep gap): when the source file can't be read, the error
        // message embeds the config-derived source path. A path with command
        // substitution must stay inside the single-quoted echo word.
        let mut r = file_resource("/etc/conf");
        r.state = Some("file".to_string());
        // Nonexistent path (read fails) carrying an injection payload.
        r.source = Some("/no/such$(touch /tmp/pwn)".to_string());
        let script = apply_script(&r);
        // The `$(` payload is inside a single-quoted echo word.
        assert!(script.contains("echo 'ERROR: cannot read source file: /no/such$(touch /tmp/pwn)"));
        assert!(script.contains("; exit 1"));
        // No bare command substitution outside quotes.
        assert!(!script.contains("echo ERROR"));
        assert!(!script.contains(": /no/such' $(touch"));
    }

    #[test]
    fn fj165_unsupported_state_label_injection_neutralized() {
        // #165 (#161 sweep gap): the `other` arm echoes the config-derived
        // state string raw — escape it in both check_script and apply_script.
        let mut r = file_resource("/etc/foo");
        r.state = Some("x$(touch /tmp/pwn)".to_string());
        let check = check_script(&r);
        let apply = apply_script(&r);
        assert!(check.contains("echo 'unsupported file state: x$(touch /tmp/pwn)'"));
        assert!(apply.contains("echo 'unsupported file state: x$(touch /tmp/pwn)'"));
        // No bare (unquoted) label, and no break-out of the single-quoted word.
        assert!(!check.contains("echo unsupported"));
        assert!(!check.contains("' $(touch"));
    }
}
