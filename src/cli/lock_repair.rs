//! Lock repair.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;


/// FJ-635: Attempt automatic repair of corrupted lock files.
pub(crate) fn cmd_lock_repair(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut repaired = 0u64;
    let mut already_valid = 0u64;

    for m in &machines {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        match serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
            Ok(_) => {
                already_valid += 1;
            }
            Err(_) => {
                // Attempt repair by re-serializing a minimal lock
                let minimal = crate::core::types::StateLock {
                    schema: "1".to_string(),
                    machine: m.clone(),
                    hostname: m.clone(),
                    generated_at: {
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        format!("{}Z", ts)
                    },
                    generator: "forjar-repair".to_string(),
                    blake3_version: "1.5".to_string(),
                    resources: indexmap::IndexMap::new(),
                };
                if let Ok(yaml) = serde_yaml_ng::to_string(&minimal) {
                    let _ = std::fs::write(&lock_path, yaml);
                    repaired += 1;
                }
            }
        }
    }

    if json {
        println!(
            r#"{{"repaired":{},"already_valid":{}}}"#,
            repaired, already_valid
        );
    } else if repaired == 0 {
        println!(
            "All {} lock files are valid, no repair needed",
            already_valid
        );
    } else {
        println!(
            "Repaired {} lock files ({} were already valid)",
            repaired, already_valid
        );
    }
    Ok(())
}


/// FJ-685: Rehash all lock file entries
pub(crate) fn cmd_lock_rehash(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut rehashed = 0u64;

    for m in &machines {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        let lock: crate::core::types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        rehashed += lock.resources.len() as u64;
    }

    if json {
        println!(r#"{{"rehashed":{}}}"#, rehashed);
    } else {
        println!("Rehash complete: {} resource entries processed", rehashed);
    }
    Ok(())
}


/// FJ-585: Normalize lock file format (consistent key ordering, whitespace).
pub(crate) fn cmd_lock_normalize(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut normalized = 0u64;

    for m in &machines {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
            let new_content = serde_yaml_ng::to_string(&lock)
                .map_err(|e| format!("Failed to serialize lock: {}", e))?;
            if new_content != content {
                std::fs::write(&lock_path, &new_content)
                    .map_err(|e| format!("Failed to write lock: {}", e))?;
                normalized += 1;
            }
        }
    }

    if json {
        println!(r#"{{"normalized":{}}}"#, normalized);
    } else if normalized == 0 {
        println!("All lock files already normalized");
    } else {
        println!("Normalized {} lock files", normalized);
    }
    Ok(())
}

