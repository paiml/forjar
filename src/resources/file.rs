//! FJ-007: File/directory resource handler.

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

    match state {
        "directory" => format!(
            "test -d '{path}' && echo 'exists:directory' || echo 'missing:directory'"
        ),
        "absent" => format!(
            "test -e '{path}' && echo 'exists:present' || echo 'missing:absent'"
        ),
        "symlink" => format!(
            "test -L '{path}' && echo 'exists:symlink' || echo 'missing:symlink'"
        ),
        "file" => format!(
            "test -f '{path}' && echo 'exists:file' || echo 'missing:file'"
        ),
        other => format!("echo 'unsupported file state: {other}'"),
    }
}

/// Append chown/chmod lines for the given resource ownership and mode.
fn push_ownership_lines(lines: &mut Vec<String>, path: &str, resource: &Resource) {
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

/// Generate the file-content write commands (source or inline content).
fn push_file_content_lines(lines: &mut Vec<String>, path: &str, resource: &Resource) {
    if let Some(ref source) = resource.source {
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
        lines.push(format!(
            "cat > '{}' <<'FORJAR_EOF'\n{}\nFORJAR_EOF",
            path, content
        ));
    }
}

/// Generate shell to converge file to desired state.
pub fn apply_script(resource: &Resource) -> String {
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let state = resource.state.as_deref().unwrap_or("file");

    let mut lines = vec!["set -euo pipefail".to_string()];

    match state {
        "directory" => {
            lines.push(format!("mkdir -p '{path}'"));
            push_ownership_lines(&mut lines, path, resource);
        }
        "absent" => {
            lines.push(format!("rm -rf '{path}'"));
        }
        "symlink" => {
            let target = resource.target.as_deref().unwrap_or("/dev/null");
            lines.push(format!("ln -sfn '{target}' '{path}'"));
        }
        "file" => {
            if let Some(parent) = std::path::Path::new(path).parent() {
                if parent != std::path::Path::new("/") {
                    lines.push(format!("mkdir -p '{}'", parent.display()));
                }
            }
            push_file_content_lines(&mut lines, path, resource);
            push_ownership_lines(&mut lines, path, resource);
        }
        other => {
            lines.push(format!("echo 'unsupported file state: {other}'"));
        }
    }

    lines.join("\n")
}

/// Generate shell to query file state (for hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    format!(
        "if [ -e '{path}' ]; then\n\
           stat -c 'owner=%U group=%G mode=%a size=%s' '{path}' 2>/dev/null || \
           stat -f 'owner=%Su group=%Sg mode=%Lp size=%z' '{path}' 2>/dev/null\n\
           if [ -f '{path}' ]; then\n\
             cat '{path}' | blake3sum 2>/dev/null || sha256sum '{path}' | cut -d' ' -f1\n\
           fi\n\
         else\n\
           echo 'MISSING'\n\
         fi"
    )
}
