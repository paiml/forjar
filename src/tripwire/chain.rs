//! FJ-1386: Tamper-evident transparency log — BLAKE3 chain hashing on events.
//!
//! Each event line's BLAKE3 hash incorporates the hash of the previous line,
//! creating a tamper-evident chain. Modifying or removing any event invalidates
//! all subsequent chain hashes.

use crate::tripwire::hasher;
use std::path::Path;

/// Result of verifying a chain hash on an events.jsonl file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainVerification {
    /// Total lines processed.
    pub total_lines: usize,
    /// Lines that passed chain verification.
    pub verified: usize,
    /// Lines where chain verification failed (line number, detail).
    pub failures: Vec<(usize, String)>,
    /// Final chain hash.
    pub chain_hash: String,
}

/// Compute a chain hash for a JSONL event log.
/// Returns the final hash of the chain: H(line_N || H(line_{N-1} || ...)).
pub fn compute_chain_hash(events_path: &Path) -> Result<String, String> {
    let content = std::fs::read_to_string(events_path).map_err(|e| format!("read events: {e}"))?;
    let mut chain_hash = String::from("genesis");
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let input = format!("{chain_hash}:{line}");
        chain_hash = hasher::hash_string(&input);
    }
    Ok(chain_hash)
}

/// Write a `.chain` sidecar file containing the chain hash for an events.jsonl.
pub fn write_chain_sidecar(events_path: &Path) -> Result<(), String> {
    let chain_hash = compute_chain_hash(events_path)?;
    let sidecar = chain_sidecar_path(events_path);
    std::fs::write(&sidecar, &chain_hash).map_err(|e| format!("write chain sidecar: {e}"))?;
    Ok(())
}

/// Verify a chain sidecar matches the computed chain hash.
pub fn verify_chain(events_path: &Path) -> Result<ChainVerification, String> {
    let content = std::fs::read_to_string(events_path).map_err(|e| format!("read events: {e}"))?;
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    let total_lines = lines.len();

    let mut chain_hash = String::from("genesis");
    for line in &lines {
        let input = format!("{chain_hash}:{line}");
        chain_hash = hasher::hash_string(&input);
    }

    let sidecar = chain_sidecar_path(events_path);
    let mut failures = Vec::new();

    if sidecar.exists() {
        let stored =
            std::fs::read_to_string(&sidecar).map_err(|e| format!("read chain sidecar: {e}"))?;
        let stored = stored.trim();
        if stored != chain_hash {
            failures.push((
                total_lines,
                format!("chain hash mismatch: stored={stored}, computed={chain_hash}"),
            ));
        }
    }

    Ok(ChainVerification {
        total_lines,
        verified: if failures.is_empty() { total_lines } else { 0 },
        failures,
        chain_hash,
    })
}

/// Path to the chain sidecar file for an events.jsonl.
fn chain_sidecar_path(events_path: &Path) -> std::path::PathBuf {
    events_path.with_extension("chain")
}

/// Verify all event logs in a state directory.
pub fn verify_all_chains(state_dir: &Path) -> Vec<(String, ChainVerification)> {
    let mut results = Vec::new();
    let Ok(entries) = std::fs::read_dir(state_dir) else {
        return results;
    };
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let machine = entry.file_name().to_string_lossy().to_string();
        let events_path = entry.path().join("events.jsonl");
        if events_path.exists() {
            match verify_chain(&events_path) {
                Ok(v) => results.push((machine, v)),
                Err(e) => results.push((
                    machine,
                    ChainVerification {
                        total_lines: 0,
                        verified: 0,
                        failures: vec![(0, e)],
                        chain_hash: String::new(),
                    },
                )),
            }
        }
    }
    results
}
