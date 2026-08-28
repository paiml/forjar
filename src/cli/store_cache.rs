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
    let result = cache_exec::pull_from_cache(&cache_source, hash, store_dir, &machine, Some(300))?;
    println!("Pulled {} to {}", result.store_hash, result.store_path);
    println!(
        "  Bytes: {} | Verified: {}",
        result.bytes_transferred, result.verified
    );
    Ok(())
}

/// Verify all store entries by re-hashing.
///
/// GH-236: this used to compare `hash_directory(<entry>/content)` against
/// `blake3:<directory name>` — against the entry's ADDRESS, with a hash
/// function that had not produced it. An entry written by
/// `forjar store-import` is addressed with `provider_exec::hash_staging_dir`,
/// a different preimage under a different domain tag, so this reported 100%
/// failure on any store built by an import while a conda entry (also
/// `hash_directory`) passed. It now compares against `meta.output_hash`, the
/// digest the entry itself recorded. The rendering lives beside
/// `forjar store verify` in `store_verify`, because the two now answer the
/// same question and differ only in output shape.
pub(crate) fn cmd_cache_verify(store_dir: &Path, json: bool) -> Result<(), String> {
    super::store_verify::cmd_cache_verify_report(store_dir, json)
}

/// List store entries as CacheEntry structs.
fn list_entries(store_dir: &Path) -> Result<Vec<CacheEntry>, String> {
    let read_dir =
        std::fs::read_dir(store_dir).map_err(|e| format!("read {}: {e}", store_dir.display()))?;

    let mut entries: Vec<CacheEntry> = read_dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| p.file_name().is_some_and(|n| n != ".gc-roots"))
        .map(|p| cache_entry(&p))
        .collect();
    entries.sort_by(|a, b| a.store_hash.cmp(&b.store_hash));
    Ok(entries)
}

/// Describe one store entry directory. An unreadable `meta.yaml` yields
/// "unknown" rather than dropping the entry: it is on disk either way.
fn cache_entry(path: &Path) -> CacheEntry {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let (provider, arch, created) = match read_meta(path) {
        Ok(m) => (m.provider, m.arch, m.created_at),
        Err(_) => ("unknown".to_string(), "unknown".to_string(), String::new()),
    };
    CacheEntry {
        store_hash: format!("blake3:{name}"),
        size_bytes: dir_size(path),
        created_at: created,
        provider,
        arch,
    }
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
                    let Ok(m) = e.metadata().or_else(|_| std::fs::metadata(e.path())) else {
                        return 0;
                    };
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
        allowed_operators: vec![],
    }
}
