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
