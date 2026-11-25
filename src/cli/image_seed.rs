//! Autoinstall seed helpers: disk layout, SSH key reading, tilde expansion.

use crate::core::error::ForjarError;

/// Map the disk argument to an autoinstall storage layout YAML snippet.
pub(crate) fn storage_layout_yaml(disk: &str) -> Result<String, ForjarError> {
    match disk {
        "auto-lvm" => Ok("    layout:\n      name: lvm".to_string()),
        "auto-zfs" => Ok("    layout:\n      name: zfs".to_string()),
        _ if disk.starts_with("serial:") => {
            let serial = disk.strip_prefix("serial:").unwrap_or("");
            if serial.trim().is_empty() || serial.contains(['\n', '\r']) {
                return Err(ForjarError::from_untyped(
                    "invalid disk serial pattern: serial:<glob> requires a non-empty single-line glob",
                ));
            }
            let quoted = format!("'{}'", serial.replace('\'', "''"));
            Ok(format!("    layout:\n      name: lvm\n      match:\n        serial: {quoted}"))
        }
        _ if disk.starts_with("/dev/disk/by-") => Err(ForjarError::from_untyped(format!(
            "disk path '{disk}' under /dev/disk/by-* is not supported; use serial:<glob> or /dev/<kernel-name>"
        ))),
        _ => {
            if let Some(kname) = disk.strip_prefix("/dev/") {
                if !kname.is_empty() && !kname.contains('/') && !kname.chars().any(char::is_whitespace) {
                    return Ok(format!(
                        "    layout:\n      name: lvm\n      match:\n        path: {disk}"
                    ));
                }
            }
            Err(ForjarError::from_untyped(format!(
                "unknown disk layout: {disk} (use auto-lvm, auto-zfs, serial:<glob>, or /dev/<kernel-name>)"
            )))
        }
    }
}

/// Read SSH public key file (tries .pub extension).
/// Errors when no key path is configured, the file cannot be read, or no keys exist.
pub fn read_ssh_pub_key(ssh_key: Option<&str>) -> Result<Vec<String>, ForjarError> {
    let key_path = ssh_key
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .ok_or_else(|| {
            ForjarError::from_untyped(
                "machine has no ssh_key configured; an SSH public key is required for autoinstall",
            )
        })?;
    let pub_path = if key_path.ends_with(".pub") {
        key_path.to_string()
    } else {
        format!("{key_path}.pub")
    };
    let expanded = expand_tilde(&pub_path);
    let content = std::fs::read_to_string(&expanded)
        .map_err(|e| ForjarError::from_untyped(format!("read SSH public key '{expanded}': {e}")))?;
    let keys: Vec<String> = content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect();
    if keys.is_empty() {
        return Err(ForjarError::from_untyped(format!(
            "no SSH public keys found in '{expanded}'"
        )));
    }
    Ok(keys)
}

/// Expand ~ to $HOME in a path string.
pub fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    path.to_string()
}
