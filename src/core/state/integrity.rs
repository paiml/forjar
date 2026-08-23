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
        /// Path to the lock file that failed verification.
        file: PathBuf,
        /// Expected BLAKE3 hash from sidecar.
        expected: String,
        /// Actual BLAKE3 hash computed from file contents.
        actual: String,
    },
    /// Lock file is invalid YAML — likely corrupted.
    InvalidYaml(PathBuf, String),
    /// Sidecar survives but the lock file it seals is gone.
    ///
    /// `save_lock` writes the lock and its `.b3` together, so a lone sidecar is
    /// positive evidence that a lock existed and was removed. Walking only the
    /// locks that still exist makes that deletion invisible — the scan finds
    /// nothing to check and reports success.
    MissingLock(PathBuf),
}

/// Verify integrity of all state lock files in the state directory.
/// Returns a list of issues found. Empty list means all checks pass.
pub fn verify_state_integrity(state_dir: &Path) -> Vec<IntegrityResult> {
    let mut results = Vec::new();

    // Check global lock
    results.extend(check_lock_slot(&state_dir.join("forjar.lock.yaml")));

    // Check per-machine lock files
    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(check_lock_slot(&path.join("state.lock.yaml")));
            }
        }
    }

    results
}

/// Check one lock *slot* — the lock file and the sidecar that seals it.
///
/// An absent lock is only interesting when its sidecar survives: that pairing is
/// proof the lock was deleted rather than never written.
fn check_lock_slot(lock_path: &Path) -> Vec<IntegrityResult> {
    if lock_path.exists() {
        check_lock_file(lock_path)
    } else if sidecar_path(lock_path).exists() {
        vec![IntegrityResult::MissingLock(lock_path.to_path_buf())]
    } else {
        Vec::new()
    }
}

/// Check a single lock file for integrity: valid YAML, sidecar present, BLAKE3 match.
fn check_lock_file(lock_path: &Path) -> Vec<IntegrityResult> {
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
            IntegrityResult::MissingLock(p) => {
                eprintln!(
                    "ERROR: lock file {} is missing but its BLAKE3 sidecar survives — \
                     the lock was deleted",
                    p.display()
                );
            }
            _ => {}
        }
    }
}

/// Returns true if any result is a hard error (hash mismatch, invalid YAML,
/// or a lock deleted out from under a surviving sidecar).
pub fn has_errors(results: &[IntegrityResult]) -> bool {
    results.iter().any(|r| {
        matches!(
            r,
            IntegrityResult::HashMismatch { .. }
                | IntegrityResult::InvalidYaml(..)
                | IntegrityResult::MissingLock(..)
        )
    })
}

/// One-line reason a result is a verification failure; `None` for [`IntegrityResult::Ok`].
///
/// `has_errors` deliberately tolerates a missing sidecar so `apply` still runs on a
/// state directory written before sidecars existed. A command whose *entire purpose*
/// is integrity has no such excuse: with no sidecar it has no instrument, and an
/// absent verifier is a NO-GO rather than a pass. Those commands use
/// [`failure_reason`] / [`failure_reasons`], which count every non-`Ok` result —
/// including `MissingSidecar`.
pub fn failure_reason(result: &IntegrityResult) -> Option<String> {
    match result {
        IntegrityResult::Ok => None,
        IntegrityResult::MissingSidecar(p) => Some(format!(
            "no BLAKE3 sidecar for {} — integrity cannot be verified",
            p.display()
        )),
        IntegrityResult::MissingLock(p) => Some(format!(
            "lock file {} is missing but its BLAKE3 sidecar survives — the lock was deleted",
            p.display()
        )),
        IntegrityResult::HashMismatch {
            file,
            expected,
            actual,
        } => Some(format!(
            "BLAKE3 mismatch for {} — sidecar says {}, file hashes to {}",
            file.display(),
            expected,
            actual
        )),
        IntegrityResult::InvalidYaml(p, e) => {
            Some(format!("corrupt state file {}: {}", p.display(), e))
        }
    }
}

/// The lock file a result concerns; `None` for [`IntegrityResult::Ok`].
pub fn result_path(result: &IntegrityResult) -> Option<&Path> {
    match result {
        IntegrityResult::Ok => None,
        IntegrityResult::MissingSidecar(p)
        | IntegrityResult::MissingLock(p)
        | IntegrityResult::InvalidYaml(p, _) => Some(p),
        IntegrityResult::HashMismatch { file, .. } => Some(file),
    }
}

/// Every failure in `results`, as one-line reasons. Empty when everything verified.
///
/// This is the strict predicate the integrity commands use: non-empty means at least
/// one lock could not be verified, for ANY reason including a missing sidecar.
pub fn failure_reasons(results: &[IntegrityResult]) -> Vec<String> {
    results.iter().filter_map(failure_reason).collect()
}
