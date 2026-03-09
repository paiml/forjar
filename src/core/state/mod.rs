//! FJ-013: Lock file management — load, save (atomic), path derivation.

pub mod integrity;
pub mod reconstruct;

use super::types::{ApplyResult, GlobalLock, MachineSummary, StateLock};
use provable_contracts_macros::contract;
use std::path::{Path, PathBuf};

/// Derive the lock file path for a machine within the state directory.
pub fn lock_file_path(state_dir: &Path, machine: &str) -> PathBuf {
    state_dir.join(machine).join("state.lock.yaml")
}

/// Load a lock file for a machine. Returns None if the file doesn't exist.
pub fn load_lock(state_dir: &Path, machine: &str) -> Result<Option<StateLock>, String> {
    let path = lock_file_path(state_dir, machine);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    let lock: StateLock = serde_yaml_ng::from_str(&content)
        .map_err(|e| format!("invalid lock file {}: {}", path.display(), e))?;
    Ok(Some(lock))
}

/// Save a lock file atomically (write to temp, then rename).
#[contract("execution-safety-v1", equation = "atomic_write")]
pub fn save_lock(state_dir: &Path, lock: &StateLock) -> Result<(), String> {
    let path = lock_file_path(state_dir, &lock.machine);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create dir {}: {}", parent.display(), e))?;
    }

    let yaml = serde_yaml_ng::to_string(lock).map_err(|e| format!("serialize error: {e}"))?;

    // Write to temp file, then rename for crash-safe persistence
    let tmp_path = path.with_extension("lock.yaml.tmp");
    std::fs::write(&tmp_path, &yaml)
        .map_err(|e| format!("cannot write {}: {}", tmp_path.display(), e))?;
    std::fs::rename(&tmp_path, &path).map_err(|e| {
        format!(
            "cannot rename {} → {}: {}",
            tmp_path.display(),
            path.display(),
            e
        )
    })?;

    // FJ-1270: Write BLAKE3 integrity sidecar
    let _ = integrity::write_b3_sidecar(&path);

    // FJ-2200: Atomicity postcondition — file exists and temp is gone
    debug_assert!(path.exists(), "save_lock: file does not exist after write");
    debug_assert!(
        !tmp_path.exists(),
        "save_lock: temp file still exists after rename"
    );

    Ok(())
}

/// Path to the global lock file.
pub fn global_lock_path(state_dir: &Path) -> PathBuf {
    state_dir.join("forjar.lock.yaml")
}

/// Load the global lock file. Returns None if it doesn't exist.
pub fn load_global_lock(state_dir: &Path) -> Result<Option<GlobalLock>, String> {
    let path = global_lock_path(state_dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    let lock: GlobalLock = serde_yaml_ng::from_str(&content)
        .map_err(|e| format!("invalid global lock {}: {}", path.display(), e))?;
    Ok(Some(lock))
}

/// Save the global lock file atomically.
pub fn save_global_lock(state_dir: &Path, lock: &GlobalLock) -> Result<(), String> {
    std::fs::create_dir_all(state_dir)
        .map_err(|e| format!("cannot create dir {}: {}", state_dir.display(), e))?;

    let path = global_lock_path(state_dir);
    let yaml = serde_yaml_ng::to_string(lock).map_err(|e| format!("serialize error: {e}"))?;

    let tmp_path = path.with_extension("lock.yaml.tmp");
    std::fs::write(&tmp_path, &yaml)
        .map_err(|e| format!("cannot write {}: {}", tmp_path.display(), e))?;
    std::fs::rename(&tmp_path, &path).map_err(|e| {
        format!(
            "cannot rename {} → {}: {}",
            tmp_path.display(),
            path.display(),
            e
        )
    })?;

    // FJ-1270: Write BLAKE3 integrity sidecar
    let _ = integrity::write_b3_sidecar(&path);

    Ok(())
}

/// Create a new GlobalLock with machine summaries.
pub fn new_global_lock(name: &str) -> GlobalLock {
    use crate::tripwire::eventlog::now_iso8601;
    GlobalLock {
        schema: "1.0".to_string(),
        name: name.to_string(),
        last_apply: now_iso8601(),
        generator: format!("forjar {}", env!("CARGO_PKG_VERSION")),
        machines: indexmap::IndexMap::new(),
        outputs: indexmap::IndexMap::new(),
    }
}

/// Update global lock with results from an apply.
pub fn update_global_lock(
    state_dir: &Path,
    config_name: &str,
    machine_results: &[(String, usize, usize, usize)], // (name, total, converged, failed)
) -> Result<(), String> {
    use crate::tripwire::eventlog::now_iso8601;
    let mut lock = load_global_lock(state_dir)?.unwrap_or_else(|| new_global_lock(config_name));
    lock.name = config_name.to_string();
    lock.last_apply = now_iso8601();
    lock.generator = format!("forjar {}", env!("CARGO_PKG_VERSION"));

    for (name, total, converged, failed) in machine_results {
        lock.machines.insert(
            name.clone(),
            MachineSummary {
                resources: *total,
                converged: *converged,
                failed: *failed,
                last_apply: now_iso8601(),
            },
        );
    }

    save_global_lock(state_dir, &lock)
}

/// FJ-1260: Resolve all output values from a config into a flat map.
pub fn resolve_outputs(config: &super::types::ForjarConfig) -> indexmap::IndexMap<String, String> {
    let mut resolved = indexmap::IndexMap::new();
    for (k, output) in &config.outputs {
        let value = super::resolver::resolve_template_with_secrets(
            &output.value,
            &config.params,
            &config.machines,
            &config.secrets,
        )
        .unwrap_or_else(|_| output.value.clone());
        resolved.insert(k.clone(), value);
    }
    resolved
}

/// FJ-1260: Persist resolved outputs into the global lock file.
pub fn persist_outputs(
    state_dir: &Path,
    config_name: &str,
    outputs: &indexmap::IndexMap<String, String>,
) -> Result<(), String> {
    let mut lock = load_global_lock(state_dir)?.unwrap_or_else(|| new_global_lock(config_name));
    lock.outputs = outputs.clone();
    save_global_lock(state_dir, &lock)
}

/// Create a new empty StateLock for a machine.
pub fn new_lock(machine: &str, hostname: &str) -> StateLock {
    use crate::tripwire::eventlog::now_iso8601;
    StateLock {
        schema: "1.0".to_string(),
        machine: machine.to_string(),
        hostname: hostname.to_string(),
        generated_at: now_iso8601(),
        generator: format!("forjar {}", env!("CARGO_PKG_VERSION")),
        blake3_version: "1.8".to_string(),
        resources: indexmap::IndexMap::new(),
    }
}

/// FJ-262: Save per-machine apply report to `state/<machine>/last-apply.yaml`.
pub fn save_apply_report(state_dir: &Path, result: &ApplyResult) -> Result<(), String> {
    let dir = state_dir.join(&result.machine);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("cannot create dir {}: {}", dir.display(), e))?;
    let path = dir.join("last-apply.yaml");
    let yaml =
        serde_yaml_ng::to_string(result).map_err(|e| format!("serialize report error: {e}"))?;
    std::fs::write(&path, &yaml).map_err(|e| format!("cannot write {}: {}", path.display(), e))?;
    Ok(())
}

/// FJ-262: Load last apply report for a machine.
pub fn load_apply_report(state_dir: &Path, machine: &str) -> Result<Option<String>, String> {
    let path = state_dir.join(machine).join("last-apply.yaml");
    if !path.exists() {
        return Ok(None);
    }
    std::fs::read_to_string(&path)
        .map(Some)
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))
}

// ============================================================================
// FJ-266: State locking — prevent concurrent applies
// ============================================================================

/// Path to the process lock file.
pub(super) fn process_lock_path(state_dir: &Path) -> PathBuf {
    state_dir.join(".forjar.lock")
}

/// Acquire an exclusive process lock. Returns an error if another apply is running.
/// Stale locks (PID no longer running) are automatically removed.
pub fn acquire_process_lock(state_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(state_dir).map_err(|e| format!("cannot create state dir: {e}"))?;

    let lock_path = process_lock_path(state_dir);

    // Check for existing lock
    if lock_path.exists() {
        let content = std::fs::read_to_string(&lock_path)
            .map_err(|e| format!("cannot read lock file: {e}"))?;
        if let Some(pid) = parse_lock_pid(&content) {
            if is_pid_running(pid) {
                return Err(format!(
                    "state directory is locked by PID {} ({}). \
                     If this is stale, run: forjar apply --force-unlock",
                    pid,
                    lock_path.display()
                ));
            }
            // Stale lock — PID no longer running, remove it
        }
        let _ = std::fs::remove_file(&lock_path);
    }

    // Write our PID
    let pid = std::process::id();
    let content = format!(
        "pid: {}\nstarted_at: {}\n",
        pid,
        crate::tripwire::eventlog::now_iso8601()
    );
    std::fs::write(&lock_path, content).map_err(|e| format!("cannot write lock file: {e}"))?;
    Ok(())
}

/// Release the process lock.
pub fn release_process_lock(state_dir: &Path) {
    let lock_path = process_lock_path(state_dir);
    let _ = std::fs::remove_file(&lock_path);
}

/// Force-remove the process lock (for --force-unlock).
pub fn force_unlock(state_dir: &Path) -> Result<(), String> {
    let lock_path = process_lock_path(state_dir);
    if !lock_path.exists() {
        return Ok(());
    }
    std::fs::remove_file(&lock_path).map_err(|e| format!("cannot remove lock file: {e}"))
}

/// Parse PID from lock file content.
pub(super) fn parse_lock_pid(content: &str) -> Option<u32> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("pid:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

/// Check if a PID is still running (Linux-specific: /proc/<pid> exists).
fn is_pid_running(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

// ============================================================================
// FJ-1240: State encryption with age
// ============================================================================

/// Encrypt all YAML state files in the state directory using `age`.
/// Requires `FORJAR_AGE_KEY` env var (public key for encryption).
pub fn encrypt_state_files(state_dir: &Path) -> Result<(), String> {
    let pubkey = std::env::var("FORJAR_AGE_KEY")
        .map_err(|_| "FORJAR_AGE_KEY env var required for --encrypt-state".to_string())?;

    for entry in walk_yaml_files(state_dir) {
        let encrypted_path = entry.with_extension("yaml.age");
        let status = std::process::Command::new("age")
            .args(["-r", &pubkey, "-o"])
            .arg(&encrypted_path)
            .arg(&entry)
            .status()
            .map_err(|e| format!("age encrypt failed for {}: {}", entry.display(), e))?;
        if !status.success() {
            return Err(format!("age encrypt failed for {}", entry.display()));
        }
        std::fs::remove_file(&entry)
            .map_err(|e| format!("remove plaintext {}: {}", entry.display(), e))?;
    }
    Ok(())
}

/// Decrypt all `.age` state files in the state directory using `age`.
/// Requires `FORJAR_AGE_IDENTITY` env var (private key file path).
pub fn decrypt_state_files(state_dir: &Path) -> Result<(), String> {
    let identity = std::env::var("FORJAR_AGE_IDENTITY")
        .map_err(|_| "FORJAR_AGE_IDENTITY env var required to decrypt state".to_string())?;

    for entry in walk_age_files(state_dir) {
        let yaml_path = PathBuf::from(entry.to_string_lossy().replace(".yaml.age", ".yaml"));
        let status = std::process::Command::new("age")
            .args(["-d", "-i", &identity, "-o"])
            .arg(&yaml_path)
            .arg(&entry)
            .status()
            .map_err(|e| format!("age decrypt failed for {}: {}", entry.display(), e))?;
        if !status.success() {
            return Err(format!("age decrypt failed for {}", entry.display()));
        }
        std::fs::remove_file(&entry)
            .map_err(|e| format!("remove encrypted {}: {}", entry.display(), e))?;
    }
    Ok(())
}

/// Walk state directory for YAML files (lock files and reports).
fn walk_yaml_files(state_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_yaml_files(&path));
            } else if path.extension().is_some_and(|e| e == "yaml") {
                files.push(path);
            }
        }
    }
    files
}

/// Walk state directory for .age encrypted files.
fn walk_age_files(state_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_age_files(&path));
            } else if path.to_string_lossy().ends_with(".yaml.age") {
                files.push(path);
            }
        }
    }
    files
}

#[cfg(test)]
mod tests_basic;
#[cfg(test)]
mod tests_edge;
#[cfg(test)]
mod tests_encrypt;
#[cfg(test)]
mod tests_global_lock;
#[cfg(test)]
mod tests_helpers;
#[cfg(test)]
mod tests_integrity;
#[cfg(test)]
mod tests_integrity_cov;
#[cfg(test)]
mod tests_outputs;
#[cfg(test)]
mod tests_process_lock;
#[cfg(test)]
mod tests_reconstruct;
#[cfg(test)]
mod tests_state_cov;
