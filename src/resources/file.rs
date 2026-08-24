//! FJ-007: File/directory resource handler.

use crate::core::shell_escape::{sh_squote, sh_write_file};
use crate::core::types::Resource;
use crate::resources::verdict;

/// Read a local file, or describe why it could not be read.
fn read_source_file(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{path}: {e}"))
}

/// Generate shell to check file state.
pub fn check_script(resource: &Resource) -> String {
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let state = resource.state.as_deref().unwrap_or("file");
    let p = sh_squote(path);

    match state {
        "directory" => verdict::single(
            &format!("test -d {p}"),
            "exists:directory",
            "missing:directory",
        ),
        // INVERTED: `absent` converges when the path is GONE, so the passing
        // condition is the negation and `missing:` is the SUCCESS marker. This
        // is why the verdict cannot be derived from the marker text at the
        // codegen boundary — only the generator knows which way the resource
        // points.
        "absent" => verdict::single(
            &format!("! test -e {p}"),
            "missing:absent",
            "exists:present",
        ),
        "symlink" => verdict::single(&format!("test -L {p}"), "exists:symlink", "missing:symlink"),
        "file" => verdict::single(&format!("test -f {p}"), "exists:file", "missing:file"),
        // `other` is the config-derived state string; escape the label. An
        // unrecognised state is not a pass — forjar cannot show the resource
        // is converged, so it must say so.
        other => verdict::check_script_from(&[verdict::always_diverged(&format!(
            "unsupported file state: {other}"
        ))]),
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
    if let Some(ref source) = resource.source {
        match read_source_file(source) {
            Ok(bytes) => {
                lines.push(sh_write_file(path, &bytes));
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
        // C8 (GH #296): inline content is DATA and must never reach the target's shell
        // parser. It used to be interpolated into a `<<'FORJAR_EOF'` heredoc
        // under a comment claiming the body was literal; a body is literal only
        // until a line equals the delimiter, so content containing `FORJAR_EOF`
        // closed the heredoc and executed the remainder as shell. `sh_write_file`
        // has no delimiter to hit, and is byte-exact (a heredoc always appends a
        // trailing newline the declared content may not have).
        lines.push(sh_write_file(path, content.as_bytes()));
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
    // SIZE IS REPORTED FOR FILES ONLY — NEVER FOR DIRECTORIES.
    //
    // This digest becomes `details.live_hash`, which drift compares against a
    // fresh run of this same script. `stat`'s `size` for a DIRECTORY is the
    // space its entry table occupies, and that grows as files are added
    // inside: measured 4096 -> 12288 at 400 entries. Folding it in would make
    // every managed directory permanently "drifted" the moment anything wrote
    // into it — and once the apply gate consults drift, permanently
    // un-appliable. A directory's identity under forjar is
    // owner/group/mode/existence; how many files someone put inside it is not
    // drift. (forjar#305; guarded by
    // tests/falsification_apply_sees_the_target_file.rs::
    // a_managed_directory_does_not_drift_when_its_contents_change.)
    //
    // For a regular file the size is redundant with the content hash below,
    // but harmless and cheap, so it stays: it keeps the digest meaningful for
    // a file whose hashing tool is unavailable on the target.
    format!(
        "if [ -e {p} ]; then\n\
           if [ -d {p} ]; then\n\
             stat -c 'owner=%U group=%G mode=%a' {p} 2>/dev/null || \
             stat -f 'owner=%Su group=%Sg mode=%Lp' {p} 2>/dev/null\n\
           else\n\
             stat -c 'owner=%U group=%G mode=%a size=%s' {p} 2>/dev/null || \
             stat -f 'owner=%Su group=%Sg mode=%Lp size=%z' {p} 2>/dev/null\n\
             if [ -f {p} ]; then\n\
               cat {p} | blake3sum 2>/dev/null || sha256sum {p} | cut -d' ' -f1\n\
             fi\n\
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
        // The destination path is one quoted shell word...
        assert!(script.contains("| base64 -d > '/etc/conf'"));
        // ...and the declared content is what the script deploys there.
        // C8 (GH #296): assert on the decoded payload, not on the script text — the
        // old `script.contains("hello")` was equally true of a script whose
        // heredoc had already closed and thrown the rest away.
        assert_eq!(
            crate::core::shell_escape::decode_written_file(&script, "/etc/conf"),
            Some(b"hello".to_vec())
        );
    }

    #[test]
    fn cbc8_inline_content_cannot_close_a_heredoc() {
        // The blocker, at the codegen boundary: content carrying the old fixed
        // delimiter plus a command must appear NOWHERE as shell.
        let mut r = file_resource("/etc/conf");
        r.state = Some("file".to_string());
        let payload = "ok\nFORJAR_EOF\nreboot\n";
        r.content = Some(payload.to_string());
        let script = apply_script(&r);
        assert!(!script.contains("FORJAR_EOF"), "{script}");
        assert!(!script.contains("reboot"), "{script}");
        assert_eq!(
            crate::core::shell_escape::decode_written_file(&script, "/etc/conf"),
            Some(payload.as_bytes().to_vec())
        );
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
