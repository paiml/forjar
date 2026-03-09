//! FJ-1327/FJ-1345: `forjar store` — store operations CLI (gc, list, diff, sync).

use crate::core::store::gc::{collect_roots, mark_and_sweep, GcConfig};
use crate::core::store::gc_exec;
use crate::core::store::meta::read_meta;
use crate::core::store::store_diff::{
    build_sync_plan, compute_diff, has_diffable_provenance, upstream_check_command,
};
use crate::core::store::sync_exec;
use crate::core::types::Machine;
use std::path::Path;

/// GC: delete unreachable store entries.
pub(crate) fn cmd_store_gc(
    store_dir: &Path,
    state_dir: &Path,
    dry_run: bool,
    older_than: Option<u64>,
    keep_generations: usize,
    json: bool,
) -> Result<(), String> {
    let _config = GcConfig {
        keep_generations,
        older_than_days: older_than,
    };

    // Collect roots from lock file and profile symlinks
    let lock_hashes = collect_lock_hashes(state_dir);
    let profile_hashes = collect_profile_hashes(store_dir);
    let gc_roots_dir = store_dir.join(".gc-roots");
    let gc_roots_path = if gc_roots_dir.is_dir() {
        Some(gc_roots_dir.as_path())
    } else {
        None
    };

    let roots = collect_roots(&profile_hashes, &lock_hashes, gc_roots_path);
    let report = mark_and_sweep(&roots, store_dir)?;

    if dry_run {
        let dry = gc_exec::sweep_dry_run(&report, store_dir);
        if json {
            let j = serde_json::json!({
                "live": report.live.len(),
                "dead": report.dead.len(),
                "total": report.total,
                "dry_run": true,
                "entries": dry.iter().map(|e| serde_json::json!({
                    "hash": e.hash, "size_bytes": e.size_bytes,
                })).collect::<Vec<_>>(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&j).unwrap_or_else(|_| "{}".to_string())
            );
        } else {
            println!("Store GC report (dry-run):");
            println!(
                "  Total: {} | Live: {} | Dead: {}",
                report.total,
                report.live.len(),
                report.dead.len()
            );
            for e in &dry {
                println!(
                    "  would delete: {} ({})",
                    &e.hash[..20.min(e.hash.len())],
                    human_bytes(e.size_bytes)
                );
            }
        }
    } else {
        let result = gc_exec::sweep(&report, store_dir)?;
        if json {
            let j = serde_json::json!({
                "removed": result.removed.len(),
                "bytes_freed": result.bytes_freed,
                "errors": result.errors.len(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&j).unwrap_or_else(|_| "{}".to_string())
            );
        } else {
            println!("Store GC complete:");
            println!(
                "  Removed: {} | Freed: {}",
                result.removed.len(),
                human_bytes(result.bytes_freed)
            );
            for hash in &result.removed {
                println!("  deleted: {}", &hash[..20.min(hash.len())]);
            }
            for (hash, err) in &result.errors {
                println!("  error: {} — {err}", &hash[..20.min(hash.len())]);
            }
        }
    }
    Ok(())
}

/// List store entries with optional provider info.
pub(crate) fn cmd_store_list(
    store_dir: &Path,
    show_provider: bool,
    json: bool,
) -> Result<(), String> {
    let entries = list_store_entries(store_dir)?;

    if json {
        let j = serde_json::json!({
            "count": entries.len(),
            "entries": entries.iter().map(|(hash, prov, arch)| {
                serde_json::json!({
                    "hash": hash,
                    "provider": prov,
                    "arch": arch,
                })
            }).collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&j).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("Store entries ({}):", entries.len());
        for (hash, prov, arch) in &entries {
            if show_provider {
                println!("  {} | {} | {}", &hash[..20.min(hash.len())], prov, arch);
            } else {
                println!("  {}", &hash[..20.min(hash.len())]);
            }
        }
    }
    Ok(())
}

/// Diff a store entry against upstream.
pub(crate) fn cmd_store_diff(hash: &str, store_dir: &Path, json: bool) -> Result<(), String> {
    let entry_dir = store_dir.join(hash.strip_prefix("blake3:").unwrap_or(hash));
    let meta = read_meta(&entry_dir).map_err(|e| format!("read meta for {hash}: {e}"))?;

    if !has_diffable_provenance(&meta) {
        return Err(format!("{hash}: no provenance metadata for diffing"));
    }

    let diff = compute_diff(&meta, None);
    let check_cmd = upstream_check_command(&meta);

    if json {
        let j = serde_json::to_string_pretty(&diff).unwrap_or_else(|_| "{}".to_string());
        println!("{j}");
    } else {
        println!("Store diff: {}", &hash[..20.min(hash.len())]);
        println!("  Provider: {}", diff.provider);
        if let Some(ref r) = diff.origin_ref {
            println!("  Origin: {r}");
        }
        println!("  Changed: {}", diff.upstream_changed);
        println!("  Derivation depth: {}", diff.derivation_chain_depth);
        if let Some(cmd) = check_cmd {
            println!("  Check upstream: {cmd}");
        }
    }
    Ok(())
}

/// Sync: re-import upstream and replay derivation chain.
pub(crate) fn cmd_store_sync(
    hash: &str,
    store_dir: &Path,
    apply: bool,
    json: bool,
) -> Result<(), String> {
    let entry_dir = store_dir.join(hash.strip_prefix("blake3:").unwrap_or(hash));
    let meta = read_meta(&entry_dir).map_err(|e| format!("read meta for {hash}: {e}"))?;

    let diff = compute_diff(&meta, None);
    let check_cmd = upstream_check_command(&meta);

    if apply {
        let machine = local_machine();
        let plan = build_sync_plan(&[(meta, None)]);
        let result = sync_exec::execute_sync(&plan, &machine, store_dir, Some(300))?;
        if json {
            let j = serde_json::json!({
                "hash": hash,
                "re_imported": result.re_imported.len(),
                "derivations_replayed": result.derivations_replayed,
                "new_profile_hash": result.new_profile_hash,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&j).unwrap_or_else(|_| "{}".to_string())
            );
        } else {
            println!("Store sync complete: {}", &hash[..20.min(hash.len())]);
            println!("  Re-imported: {}", result.re_imported.len());
            println!("  Derivations replayed: {}", result.derivations_replayed);
        }
    } else if json {
        let j = serde_json::json!({
            "hash": hash,
            "upstream_changed": diff.upstream_changed,
            "provider": diff.provider,
            "apply": false,
            "check_command": check_cmd,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&j).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("Store sync: {}", &hash[..20.min(hash.len())]);
        println!("  (dry-run — use --apply to execute)");
        if let Some(cmd) = check_cmd {
            println!("  Step 1: Check upstream via: {cmd}");
        }
        println!("  Step 2: Re-import from {} provider", diff.provider);
        if diff.derivation_chain_depth > 0 {
            println!(
                "  Step 3: Replay {} derivation steps",
                diff.derivation_chain_depth
            );
        }
    }
    Ok(())
}

/// Collect lock file hashes from state directory.
pub(super) fn collect_lock_hashes(state_dir: &Path) -> Vec<String> {
    let lock_path = state_dir.join("forjar.inputs.lock.yaml");
    crate::core::store::lockfile::read_lockfile(&lock_path)
        .map(|lf| lf.pins.values().map(|p| p.hash.clone()).collect())
        .unwrap_or_default()
}

/// Collect profile generation hashes from store.
pub(super) fn collect_profile_hashes(store_dir: &Path) -> Vec<String> {
    let profiles_dir = store_dir
        .parent()
        .map(|p| p.join("profiles"))
        .unwrap_or_default();
    if !profiles_dir.is_dir() {
        return Vec::new();
    }
    std::fs::read_dir(profiles_dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| {
                    std::fs::read_link(e.path())
                        .ok()
                        .and_then(|t| t.to_str().map(|s| s.to_string()))
                        .and_then(|s| {
                            s.split('/')
                                .find(|c| c.len() == 64)
                                .map(|c| format!("blake3:{c}"))
                        })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// List entries as (hash, provider, arch).
pub(super) fn list_store_entries(
    store_dir: &Path,
) -> Result<Vec<(String, String, String)>, String> {
    let rd =
        std::fs::read_dir(store_dir).map_err(|e| format!("read {}: {e}", store_dir.display()))?;
    let mut entries: Vec<(String, String, String)> = rd
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter(|e| e.file_name().to_string_lossy() != ".gc-roots")
        .map(|e| {
            let hash = format!("blake3:{}", e.file_name().to_string_lossy());
            let meta = read_meta(&e.path());
            let (prov, arch) = match meta {
                Ok(m) => (m.provider, m.arch),
                Err(_) => ("unknown".to_string(), "unknown".to_string()),
            };
            (hash, prov, arch)
        })
        .collect();
    entries.sort();
    Ok(entries)
}

/// Create a localhost Machine for local transport execution.
pub(super) fn local_machine() -> Machine {
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

pub(super) fn human_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    if bytes < 1_048_576 {
        return format!("{:.1} KB", bytes as f64 / 1024.0);
    }
    format!("{:.1} MB", bytes as f64 / 1_048_576.0)
}
