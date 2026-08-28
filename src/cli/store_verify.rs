//! GH-236: `forjar store verify` — do the entries still hold the bytes they
//! recorded?
//!
//! Lives in its own file rather than in `store_ops.rs`, which is 361 of its
//! 500 permitted lines.

use crate::core::store::verify::{verify_store, EntryStatus, EntryVerdict};
use std::path::{Path, PathBuf};

/// Verify every entry in a store against its recorded output digest.
///
/// Exits non-zero on any mismatch or malformed entry so this can be a cron or
/// CI gate. `--repair` deletes mismatching entries so the next build or cache
/// pull re-creates them; it never touches an `Unsealed` entry, because an entry
/// written before schema 1.1 has no recorded digest to be wrong about and
/// deleting it would destroy data on no evidence at all.
pub(crate) fn cmd_store_verify(store_dir: &Path, repair: bool, json: bool) -> Result<(), String> {
    let verdicts = verify_store(store_dir)?;
    let repaired = if repair {
        repair_mismatches(store_dir, &verdicts)?
    } else {
        Vec::new()
    };

    if json {
        print_json(&verdicts, &repaired);
    } else {
        print_text(&verdicts, &repaired);
    }

    let remaining = verdicts
        .iter()
        .filter(|v| v.is_failure())
        .filter(|v| !repaired.contains(&v.hash))
        .count();
    if remaining > 0 {
        return Err(format!(
            "{remaining} store entries failed verification: the bytes on disk are not the \
             bytes the entry recorded, or the entry is too malformed to check"
        ));
    }
    Ok(())
}

/// Delete each mismatching entry. Returns the hashes actually removed.
fn repair_mismatches(store_dir: &Path, verdicts: &[EntryVerdict]) -> Result<Vec<String>, String> {
    let mut removed = Vec::new();
    for verdict in verdicts {
        if !matches!(verdict.status, EntryStatus::Mismatch { .. }) {
            continue;
        }
        let path = entry_path(store_dir, &verdict.hash);
        std::fs::remove_dir_all(&path)
            .map_err(|e| format!("repair: cannot remove {}: {e}", path.display()))?;
        removed.push(verdict.hash.clone());
    }
    Ok(removed)
}

fn entry_path(store_dir: &Path, hash: &str) -> PathBuf {
    store_dir.join(hash.strip_prefix("blake3:").unwrap_or(hash))
}

/// One human-readable word per verdict.
fn label(status: &EntryStatus) -> &'static str {
    match status {
        EntryStatus::Ok => "ok",
        EntryStatus::Mismatch { .. } => "MISMATCH",
        EntryStatus::Unsealed => "unsealed",
        EntryStatus::Malformed(_) => "MALFORMED",
    }
}

fn detail(status: &EntryStatus) -> String {
    match status {
        EntryStatus::Ok => String::new(),
        EntryStatus::Mismatch { expected, actual } => {
            format!("recorded {expected}, holds {actual}")
        }
        EntryStatus::Unsealed => {
            "written before schema 1.1; no output digest to compare against".to_string()
        }
        EntryStatus::Malformed(e) => e.clone(),
    }
}

fn print_text(verdicts: &[EntryVerdict], repaired: &[String]) {
    for v in verdicts {
        let detail = detail(&v.status);
        if detail.is_empty() {
            println!("{:<10} {}", label(&v.status), v.hash);
        } else {
            println!("{:<10} {} — {detail}", label(&v.status), v.hash);
        }
    }
    let ok = verdicts
        .iter()
        .filter(|v| v.status == EntryStatus::Ok)
        .count();
    let unsealed = verdicts
        .iter()
        .filter(|v| v.status == EntryStatus::Unsealed)
        .count();
    let failed = verdicts.iter().filter(|v| v.is_failure()).count();
    println!("Verified: {ok} | Unsealed: {unsealed} | Failed: {failed}");
    if !repaired.is_empty() {
        println!("Repaired (removed): {}", repaired.len());
    }
}

fn print_json(verdicts: &[EntryVerdict], repaired: &[String]) {
    let results: Vec<_> = verdicts
        .iter()
        .map(|v| {
            serde_json::json!({
                "hash": v.hash,
                "status": label(&v.status),
                "valid": v.status == EntryStatus::Ok,
                "detail": detail(&v.status),
            })
        })
        .collect();
    let report = serde_json::json!({
        "verified": verdicts.iter().filter(|v| v.status == EntryStatus::Ok).count(),
        "unsealed": verdicts.iter().filter(|v| v.status == EntryStatus::Unsealed).count(),
        "failed": verdicts.iter().filter(|v| v.is_failure()).count(),
        "repaired": repaired,
        "results": results,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );
}

/// `forjar cache verify`, rendered in the shape it has always had.
///
/// Lives here rather than in `store_cache.rs` because `cache verify` and
/// `store verify` now answer the same question from the same verdicts and
/// differ only in output shape. The JSON keys — `verified`, `failed`,
/// `results[].hash|valid|expected|actual` — are unchanged, but what is being
/// COMPARED changed: see the note on `store_cache::cmd_cache_verify`. An
/// unsealed (pre-schema-1.1) entry is neither verified nor failed.
pub(crate) fn cmd_cache_verify_report(store_dir: &Path, json: bool) -> Result<(), String> {
    let verdicts = verify_store(store_dir)?;
    let verified = verdicts
        .iter()
        .filter(|v| v.status == EntryStatus::Ok)
        .count();
    let failed = verdicts.iter().filter(|v| v.is_failure()).count();

    if json {
        let results: Vec<_> = verdicts.iter().map(cache_verdict_json).collect();
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

/// One entry's row in the shape `forjar cache verify --json` has always had.
fn cache_verdict_json(verdict: &EntryVerdict) -> serde_json::Value {
    let name = verdict
        .hash
        .strip_prefix("blake3:")
        .unwrap_or(&verdict.hash);
    let (expected, actual) = match &verdict.status {
        EntryStatus::Ok => (String::new(), String::new()),
        EntryStatus::Mismatch { expected, actual } => (expected.clone(), actual.clone()),
        EntryStatus::Unsealed => (String::new(), String::new()),
        EntryStatus::Malformed(e) => (String::new(), e.clone()),
    };
    serde_json::json!({
        "hash": name,
        "valid": verdict.status == EntryStatus::Ok,
        "expected": expected,
        "actual": actual,
    })
}
