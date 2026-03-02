//! FJ-1323–FJ-1324: `forjar cache` — binary cache CLI commands.
//!
//! - `forjar cache list` — list local store entries
//! - `forjar cache push <remote>` — push to SSH remote
//! - `forjar cache pull <hash>` — pull from remote cache
//! - `forjar cache verify` — verify all store entries

use crate::core::store::cache::{ssh_command, CacheEntry, CacheSource};
use crate::core::store::cache_exec;
use crate::core::store::meta::read_meta;
use crate::core::types::Machine;
use std::path::Path;

/// List all local store entries.
pub(crate) fn cmd_cache_list(store_dir: &Path, json: bool) -> Result<(), String> {
    let entries = list_entries(store_dir)?;

    if json {
        let json_val = serde_json::json!({
            "store_dir": store_dir.display().to_string(),
            "count": entries.len(),
            "entries": entries,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&json_val).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("Store entries ({}):", entries.len());
        for entry in &entries {
            println!(
                "  {} | {} | {} | {}",
                &entry.store_hash[..20],
                entry.provider,
                entry.arch,
                human_size(entry.size_bytes),
            );
        }
    }
    Ok(())
}

/// Push store entries to an SSH remote.
pub(crate) fn cmd_cache_push(
    remote: &str,
    store_dir: &Path,
    hash: Option<&str>,
) -> Result<(), String> {
    let source = parse_remote(remote)?;
    let _ssh_cmd =
        ssh_command(&source).ok_or_else(|| "remote must be an SSH target".to_string())?;
    let entries = list_entries(store_dir)?;
    let machine = local_machine();

    let to_push: Vec<&CacheEntry> = if let Some(h) = hash {
        entries
            .iter()
            .filter(|e| e.store_hash.contains(h))
            .collect()
    } else {
        entries.iter().collect()
    };

    if to_push.is_empty() {
        println!("No entries to push");
        return Ok(());
    }

    let mut pushed = 0u64;
    for entry in &to_push {
        match cache_exec::push_to_cache(&source, &entry.store_hash, store_dir, &machine, Some(300))
        {
            Ok(()) => {
                println!(
                    "  pushed: {} ({})",
                    &entry.store_hash[..20],
                    human_size(entry.size_bytes)
                );
                pushed += 1;
            }
            Err(e) => println!("  error: {} — {e}", &entry.store_hash[..20]),
        }
    }
    println!("Pushed {pushed}/{} entries", to_push.len());
    Ok(())
}

/// Pull a store entry from remote cache.
pub(crate) fn cmd_cache_pull(
    hash: &str,
    source: Option<&str>,
    store_dir: &Path,
) -> Result<(), String> {
    let target_dir = store_dir.join(hash.strip_prefix("blake3:").unwrap_or(hash));
    if target_dir.exists() {
        println!("Entry already in local store: {hash}");
        return Ok(());
    }

    let cache_source = match source {
        Some(remote) => parse_remote(remote)?,
        None => {
            println!("Pull requires --source <user@host:path>");
            println!("  Target: {}", target_dir.display());
            return Ok(());
        }
    };

    let machine = local_machine();
    let result =
        cache_exec::pull_from_cache(&cache_source, hash, store_dir, &machine, Some(300))?;
    println!("Pulled {} to {}", result.store_hash, result.store_path);
    println!("  Bytes: {} | Verified: {}", result.bytes_transferred, result.verified);
    Ok(())
}

/// Verify all store entries by re-hashing.
pub(crate) fn cmd_cache_verify(store_dir: &Path, json: bool) -> Result<(), String> {
    let read_dir =
        std::fs::read_dir(store_dir).map_err(|e| format!("read {}: {e}", store_dir.display()))?;

    let mut verified = 0u64;
    let mut failed = 0u64;
    let mut results = Vec::new();

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".gc-roots" {
            continue;
        }

        let content_dir = path.join("content");
        if content_dir.is_dir() {
            let actual = crate::tripwire::hasher::hash_directory(&content_dir).unwrap_or_default();
            let expected = format!("blake3:{name}");
            let ok = actual == expected;
            if ok {
                verified += 1;
            } else {
                failed += 1;
            }
            results.push(serde_json::json!({
                "hash": name, "valid": ok,
                "expected": expected, "actual": actual,
            }));
        }
    }

    if json {
        let report = serde_json::json!({
            "verified": verified, "failed": failed,
            "results": results,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("Verified: {verified} | Failed: {failed}");
    }

    if failed > 0 {
        Err(format!("{failed} store entries failed verification"))
    } else {
        Ok(())
    }
}

/// List store entries as CacheEntry structs.
fn list_entries(store_dir: &Path) -> Result<Vec<CacheEntry>, String> {
    let read_dir =
        std::fs::read_dir(store_dir).map_err(|e| format!("read {}: {e}", store_dir.display()))?;

    let mut entries = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".gc-roots" {
            continue;
        }

        let meta = read_meta(&path);
        let (provider, arch, created) = match meta {
            Ok(m) => (m.provider.clone(), m.arch.clone(), m.created_at.clone()),
            Err(_) => ("unknown".to_string(), "unknown".to_string(), String::new()),
        };

        let size = dir_size(&path);
        entries.push(CacheEntry {
            store_hash: format!("blake3:{name}"),
            size_bytes: size,
            created_at: created,
            provider,
            arch,
        });
    }
    entries.sort_by(|a, b| a.store_hash.cmp(&b.store_hash));
    Ok(entries)
}

/// Parse "user@host:path" into a CacheSource::Ssh.
fn parse_remote(remote: &str) -> Result<CacheSource, String> {
    let (user_host, path) = remote
        .split_once(':')
        .ok_or_else(|| "expected format: user@host:/path".to_string())?;
    let (user, host) = user_host
        .split_once('@')
        .ok_or_else(|| "expected format: user@host:/path".to_string())?;
    Ok(CacheSource::Ssh {
        host: host.to_string(),
        user: user.to_string(),
        path: path.to_string(),
        port: None,
    })
}

fn dir_size(path: &Path) -> u64 {
    std::fs::read_dir(path)
        .map(|rd| {
            rd.flatten()
                .map(|e| {
                    let m = e
                        .metadata()
                        .unwrap_or_else(|_| std::fs::metadata(e.path()).unwrap());
                    if m.is_file() {
                        m.len()
                    } else {
                        0
                    }
                })
                .sum()
        })
        .unwrap_or(0)
}

fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    if bytes < 1_048_576 {
        return format!("{:.1} KB", bytes as f64 / 1024.0);
    }
    format!("{:.1} MB", bytes as f64 / 1_048_576.0)
}

fn local_machine() -> Machine {
    Machine {
        hostname: "localhost".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: std::env::consts::ARCH.to_string(),
        ssh_key: None,
        roles: Vec::new(),
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    }
}
