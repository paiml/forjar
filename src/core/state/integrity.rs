//! FJ-1270: State integrity verification via BLAKE3 sidecar hashes.
//!
//! After saving lock files, a `.b3` sidecar is written with the BLAKE3 hash.
//! Before apply, `verify_state_integrity()` checks that lock files match their sidecars.

use std::path::{Path, PathBuf};

/// Compute BLAKE3 hash of file contents and write to `.b3` sidecar.
pub fn write_b3_sidecar(lock_path: &Path) -> Result<(), String> {
    let content = std::fs::read(lock_path)
        .map_err(|e| format!("cannot read {}: {}", lock_path.display(), e))?;
    let hash = blake3::hash(&content);
    let sidecar = sidecar_path(lock_path);
    std::fs::write(&sidecar, hash.to_hex().as_str())
        .map_err(|e| format!("cannot write {}: {}", sidecar.display(), e))?;
    Ok(())
}

/// Derive the `.b3` sidecar path from a lock file path.
fn sidecar_path(lock_path: &Path) -> PathBuf {
    let mut p = lock_path.as_os_str().to_owned();
    p.push(".b3");
    PathBuf::from(p)
}

/// Result of a single file integrity check.
#[derive(Debug)]
pub enum IntegrityResult {
    /// File and sidecar match.
    Ok,
    /// Sidecar missing — not an error, just a warning.
    MissingSidecar(PathBuf),
    /// Hash mismatch — file was tampered or corrupted.
    HashMismatch {
        file: PathBuf,
        expected: String,
        actual: String,
    },
    /// Lock file is invalid YAML — likely corrupted.
    InvalidYaml(PathBuf, String),
}

/// Verify integrity of all state lock files in the state directory.
/// Returns a list of issues found. Empty list means all checks pass.
pub fn verify_state_integrity(state_dir: &Path) -> Vec<IntegrityResult> {
    let mut results = Vec::new();

    // Check global lock
    let global_lock = state_dir.join("forjar.lock.yaml");
    if global_lock.exists() {
        results.extend(check_file(&global_lock));
    }

    // Check per-machine lock files
    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let lock = path.join("state.lock.yaml");
                if lock.exists() {
                    results.extend(check_file(&lock));
                }
            }
        }
    }

    results
}

/// Check a single lock file for integrity.
fn check_file(lock_path: &Path) -> Vec<IntegrityResult> {
    let mut results = Vec::new();

    // Verify YAML is valid
    let content = match std::fs::read_to_string(lock_path) {
        Ok(c) => c,
        Err(e) => {
            results.push(IntegrityResult::InvalidYaml(
                lock_path.to_path_buf(),
                e.to_string(),
            ));
            return results;
        }
    };

    if let Err(e) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&content) {
        results.push(IntegrityResult::InvalidYaml(
            lock_path.to_path_buf(),
            e.to_string(),
        ));
        return results;
    }

    // Check BLAKE3 sidecar
    let sidecar = sidecar_path(lock_path);
    if !sidecar.exists() {
        results.push(IntegrityResult::MissingSidecar(lock_path.to_path_buf()));
        return results;
    }

    let expected_hash = match std::fs::read_to_string(&sidecar) {
        Ok(h) => h.trim().to_string(),
        Err(_) => {
            results.push(IntegrityResult::MissingSidecar(lock_path.to_path_buf()));
            return results;
        }
    };

    let content_bytes = content.into_bytes();
    let actual_hash = blake3::hash(&content_bytes).to_hex().to_string();

    if expected_hash != actual_hash {
        results.push(IntegrityResult::HashMismatch {
            file: lock_path.to_path_buf(),
            expected: expected_hash,
            actual: actual_hash,
        });
    } else {
        results.push(IntegrityResult::Ok);
    }

    results
}

/// Print integrity issues to stderr.
pub fn print_issues(results: &[IntegrityResult], verbose: bool) {
    for issue in results {
        match issue {
            IntegrityResult::MissingSidecar(p) if verbose => {
                eprintln!("warning: no integrity sidecar for {}", p.display());
            }
            IntegrityResult::HashMismatch {
                file,
                expected,
                actual,
            } => {
                eprintln!(
                    "ERROR: integrity check failed for {}: expected {}, got {}",
                    file.display(),
                    expected,
                    actual
                );
            }
            IntegrityResult::InvalidYaml(p, e) => {
                eprintln!("ERROR: corrupt state file {}: {}", p.display(), e);
            }
            _ => {}
        }
    }
}

/// Returns true if any result is a hard error (hash mismatch or invalid YAML).
pub fn has_errors(results: &[IntegrityResult]) -> bool {
    results.iter().any(|r| {
        matches!(
            r,
            IntegrityResult::HashMismatch { .. } | IntegrityResult::InvalidYaml(..)
        )
    })
}
