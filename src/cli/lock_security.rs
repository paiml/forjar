//! Lock security.

use super::helpers::*;
use super::helpers_time::*;
use super::lock_ops::*;
use std::path::Path;

// ── FJ-475: lock verify-sig ──

/// Verify a single machine's signature, returning (valid, json_entry).
fn verify_machine_sig(
    state_dir: &Path,
    m: &str,
    key: &str,
) -> Result<Option<(bool, serde_json::Value)>, String> {
    use crate::tripwire::hasher;
    let lock_path = state_dir.join(m).join("state.lock.yaml");
    if !lock_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&lock_path).map_err(|e| format!("read lock: {e}"))?;
    let expected_sig = hasher::hash_string(&format!("{content}{key}"));
    let sig_path = state_dir.join(m).join("lock.sig");
    let actual_sig = std::fs::read_to_string(&sig_path).unwrap_or_default();
    let valid = actual_sig.trim() == expected_sig;
    let entry = serde_json::json!({
        "machine": m,
        "valid": valid,
        "expected": expected_sig.get(..16).unwrap_or(&expected_sig),
    });
    Ok(Some((valid, entry)))
}

pub(crate) fn cmd_lock_verify_sig(state_dir: &Path, key: &str, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut results: Vec<serde_json::Value> = Vec::new();
    let mut all_valid = true;
    for m in &machines {
        if let Some((valid, entry)) = verify_machine_sig(state_dir, m, key)? {
            if !valid {
                all_valid = false;
            }
            if json {
                results.push(entry);
            } else if valid {
                println!("{} {} — signature valid", green("✓"), m);
            } else {
                println!("{} {} — signature INVALID or missing", red("✗"), m);
            }
        }
    }
    if json {
        let out = serde_json::json!({ "signatures": results, "all_valid": all_valid });
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    }
    if all_valid {
        Ok(())
    } else {
        Err("One or more signatures invalid".to_string())
    }
}

// ── FJ-485: lock compact-all ──

pub(crate) fn cmd_lock_compact_all(state_dir: &Path, yes: bool, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    if machines.is_empty() {
        if json {
            println!("{}", serde_json::json!({"compacted": 0, "machines": []}));
        } else {
            println!("No machine locks found in {}", state_dir.display());
        }
        return Ok(());
    }
    if !yes {
        println!(
            "Will compact {} machine lock file(s). Use --yes to confirm.",
            machines.len()
        );
        return Ok(());
    }
    let mut compacted = 0;
    for _m in &machines {
        // Suppress inner output — we emit our own summary below
        let result = cmd_lock_compact(state_dir, true, false);
        if result.is_ok() {
            compacted += 1;
        }
    }
    if json {
        let result = serde_json::json!({"compacted": compacted, "total": machines.len()});
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else {
        println!(
            "{} Compacted {}/{} machine lock file(s)",
            green("✓"),
            compacted,
            machines.len()
        );
    }
    Ok(())
}

/// Print a single audit trail event line in text format.
fn print_audit_event_text(m: &str, val: &serde_json::Value) {
    let ts = val.get("timestamp").and_then(|v| v.as_str()).unwrap_or("?");
    let resource = val.get("resource").and_then(|v| v.as_str()).unwrap_or("?");
    let action = val.get("action").and_then(|v| v.as_str()).unwrap_or("?");
    println!("  [{ts}] {m} — {action} on {resource}");
}

// ── FJ-495: lock audit-trail ──

/// Parse event lines from a machine's event log, collecting JSON entries and printing text.
fn collect_audit_events(
    state_dir: &Path,
    m: &str,
    json: bool,
    entries: &mut Vec<serde_json::Value>,
) {
    let log_path = state_dir.join(format!("{m}.events.jsonl"));
    if !log_path.exists() {
        return;
    }
    let content = std::fs::read_to_string(&log_path).unwrap_or_default();
    for line in content.lines() {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            if json {
                entries.push(val);
            } else {
                print_audit_event_text(m, &val);
            }
        }
    }
}

pub(crate) fn cmd_lock_audit_trail(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let all_machines = discover_machines(state_dir);
    let machines: Vec<String> = if let Some(m) = machine {
        all_machines.into_iter().filter(|n| n == m).collect()
    } else {
        all_machines
    };
    let mut entries: Vec<serde_json::Value> = Vec::new();
    for m in &machines {
        collect_audit_events(state_dir, m, json, &mut entries);
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"audit_trail": entries}))
                .unwrap_or_default()
        );
    } else if entries.is_empty() {
        let has_any = machines
            .iter()
            .any(|m| state_dir.join(format!("{m}.events.jsonl")).exists());
        if !has_any {
            println!("No event logs found in {}", state_dir.display());
        }
    }
    Ok(())
}

// ── FJ-505: lock rotate-keys ──

pub(crate) fn cmd_lock_rotate_keys(
    state_dir: &Path,
    old_key: &str,
    new_key: &str,
    json: bool,
) -> Result<(), String> {
    use crate::tripwire::hasher;
    let machines = discover_machines(state_dir);
    let mut rotated = 0;
    for m in &machines {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).map_err(|e| format!("read lock: {e}"))?;
        let sig_path = state_dir.join(m).join("lock.sig");
        let old_sig = std::fs::read_to_string(&sig_path).unwrap_or_default();
        let expected_old = hasher::hash_string(&format!("{content}{old_key}"));
        if !old_sig.is_empty() && old_sig.trim() != expected_old {
            return Err(format!(
                "{m}: old key does not match existing signature — rotation aborted"
            ));
        }
        let new_sig = hasher::hash_string(&format!("{content}{new_key}"));
        std::fs::write(&sig_path, &new_sig).map_err(|e| format!("Failed to write sig: {e}"))?;
        rotated += 1;
    }
    if json {
        let result = serde_json::json!({"rotated": rotated, "total": machines.len()});
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else {
        println!(
            "{} Rotated signing keys for {} machine(s)",
            green("✓"),
            rotated
        );
    }
    Ok(())
}

/// FJ-515: Lock backup — create timestamped backup of all lock files.
pub(crate) fn cmd_lock_backup(state_dir: &Path, json: bool) -> Result<(), String> {
    if !state_dir.exists() {
        return Err(format!(
            "State directory not found: {}",
            state_dir.display()
        ));
    }

    let timestamp = chrono_now_compact();
    let backup_dir = state_dir.join(format!("backup-{timestamp}"));
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup dir: {e}"))?;

    let mut backed_up: Vec<String> = Vec::new();
    let entries =
        std::fs::read_dir(state_dir).map_err(|e| format!("Failed to read state dir: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".lock.yaml") || name.ends_with(".events.jsonl") {
                let dest = backup_dir.join(name);
                std::fs::copy(&path, &dest).map_err(|e| format!("Failed to copy {name}: {e}"))?;
                backed_up.push(name.to_string());
            }
        }
    }

    if json {
        let files: Vec<String> = backed_up.iter().map(|f| format!(r#""{f}""#)).collect();
        println!(
            r#"{{"backup_dir":"{}","files":[{}],"count":{}}}"#,
            backup_dir.display(),
            files.join(","),
            backed_up.len()
        );
    } else {
        println!(
            "{} Backed up {} files to {}",
            green("✓"),
            backed_up.len(),
            backup_dir.display()
        );
        for f in &backed_up {
            println!("  {f}");
        }
    }
    Ok(())
}

/// FJ-535: Lock verify chain — verify full chain of custody from signatures.
pub(crate) fn cmd_lock_verify_chain(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut chain_results: Vec<(String, bool, String)> = Vec::new(); // (machine, valid, detail)

    for m in &machines {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        let sig_path = state_dir.join(m).join("lock.sig");

        if !lock_path.exists() {
            chain_results.push((m.clone(), false, "lock file missing".to_string()));
            continue;
        }

        if !sig_path.exists() {
            chain_results.push((m.clone(), false, "signature file missing".to_string()));
            continue;
        }

        let sig_content = std::fs::read_to_string(&sig_path)
            .unwrap_or_default()
            .trim()
            .to_string();
        // Chain verification: confirm sig file is a valid BLAKE3 hash (keyed verification
        // requires the signing key — use lock-verify-sig for key-based checks)
        let sig_hash = sig_content.strip_prefix("blake3:").unwrap_or(&sig_content);
        if sig_hash.len() == 64 && sig_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            chain_results.push((
                m.clone(),
                true,
                "signature present and well-formed".to_string(),
            ));
        } else {
            chain_results.push((
                m.clone(),
                false,
                format!(
                    "malformed signature: {}",
                    &sig_content[..sig_content.len().min(20)]
                ),
            ));
        }
    }

    if json {
        let entries: Vec<String> = chain_results
            .iter()
            .map(|(m, valid, detail)| {
                format!(r#"{{"machine":"{m}","valid":{valid},"detail":"{detail}"}}"#)
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else {
        println!("Lock chain verification:\n");
        for (m, valid, detail) in &chain_results {
            let icon = if *valid { green("✓") } else { red("✗") };
            println!("  {icon} {m} — {detail}");
        }
    }
    Ok(())
}

/// FJ-545: Lock stats — show lock file statistics.
pub(crate) fn cmd_lock_stats(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut stats: Vec<(String, u64, usize, String)> = Vec::new(); // (machine, size_bytes, resource_count, age)

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    for m in &machines {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        if !lock_path.exists() {
            continue;
        }

        let meta = std::fs::metadata(&lock_path).ok();
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let age_secs = meta
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| now.saturating_sub(d.as_secs()))
            .unwrap_or(0);

        let age_str = if age_secs > 86400 {
            format!("{}d", age_secs / 86400)
        } else if age_secs > 3600 {
            format!("{}h", age_secs / 3600)
        } else {
            format!("{}m", age_secs / 60)
        };

        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        let resource_count = content.matches("  type:").count();

        stats.push((m.clone(), size, resource_count, age_str));
    }

    if json {
        let entries: Vec<String> = stats
            .iter()
            .map(|(m, s, c, a)| {
                format!(r#"{{"machine":"{m}","size_bytes":{s},"resources":{c},"age":"{a}"}}"#)
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else if stats.is_empty() {
        println!("No lock files found in {}", state_dir.display());
    } else {
        println!("Lock file statistics:\n");
        for (m, s, c, a) in &stats {
            println!("  {m} — {s} bytes, {c} resources, {a} old");
        }
        let total_size: u64 = stats.iter().map(|(_, s, _, _)| s).sum();
        let total_resources: usize = stats.iter().map(|(_, _, c, _)| c).sum();
        println!(
            "\n  Total: {} machines, {} bytes, {} resources",
            stats.len(),
            total_size,
            total_resources
        );
    }
    Ok(())
}
