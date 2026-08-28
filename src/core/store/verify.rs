//! GH-236: does a store entry still hold the bytes it recorded?
//!
//! Before this, nothing could answer that. Two verifiers shipped, and each
//! compared the re-hashed content against the entry's DIRECTORY NAME using a
//! hash function that had not produced it:
//!
//! * `cli::store_cache::cmd_cache_verify` hashed with `tripwire::hash_directory`;
//! * an entry written by `forjar store-import` was addressed with
//!   `provider_exec::hash_staging_dir`, a different preimage under a different
//!   domain tag.
//!
//! So `forjar cache verify` reported 100% failure on any store built by
//! `store-import`, while a conda entry (also `hash_directory`) passed. Three
//! addressing schemes coexisted with no field saying which one an entry
//! carried. The fix is not a fourth guess: it is to compare against
//! `meta.output_hash`, which the entry itself records.

use super::content::content_hash;
use super::meta::read_meta;
use std::path::{Path, PathBuf};

/// What re-hashing an entry's content proved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryStatus {
    /// The content hashes to exactly what `meta.output_hash` recorded.
    Ok,
    /// The bytes on disk are not the bytes this entry recorded. Bit rot, a
    /// partial write, an interrupted move, or an edit under `content/`.
    Mismatch {
        /// What `meta.output_hash` says the content should hash to.
        expected: String,
        /// What it hashes to now.
        actual: String,
    },
    /// A valid entry written before schema 1.1, carrying no output digest.
    /// Reported, never counted as a failure — there is nothing to compare
    /// against, and calling that corruption would be a lie.
    Unsealed,
    /// The entry is not readable as an entry at all: no `meta.yaml`, or no
    /// `content/`. Counted as a failure, because `write_meta` is part of every
    /// entry's creation, so its absence means the entry was never finished.
    Malformed(String),
}

/// One entry's verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryVerdict {
    /// The entry's store hash (`blake3:` + directory name).
    pub hash: String,
    /// What verification found.
    pub status: EntryStatus,
}

impl EntryVerdict {
    /// Is this verdict a failure for exit-code purposes?
    ///
    /// `Unsealed` deliberately is not: a pre-1.1 store must not turn a CI gate
    /// red for having been written by an older forjar.
    pub fn is_failure(&self) -> bool {
        matches!(
            self.status,
            EntryStatus::Mismatch { .. } | EntryStatus::Malformed(_)
        )
    }
}

/// Verify one store entry directory.
pub fn verify_entry(entry_dir: &Path) -> EntryVerdict {
    let hash = format!(
        "blake3:{}",
        entry_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    );
    EntryVerdict {
        hash,
        status: entry_status(entry_dir),
    }
}

/// The verdict itself, split out so `verify_entry` stays a two-liner and this
/// stays a flat four-arm decision.
fn entry_status(entry_dir: &Path) -> EntryStatus {
    let expected = match recorded_output_hash(entry_dir) {
        Ok(Some(expected)) => expected,
        Ok(None) => return EntryStatus::Unsealed,
        Err(e) => return EntryStatus::Malformed(e),
    };
    match content_hash(&entry_dir.join("content")) {
        Ok(actual) if actual == expected => EntryStatus::Ok,
        Ok(actual) => EntryStatus::Mismatch { expected, actual },
        Err(e) => EntryStatus::Malformed(e),
    }
}

/// `Ok(None)` means a readable schema-1.0 entry with nothing to compare to.
fn recorded_output_hash(entry_dir: &Path) -> Result<Option<String>, String> {
    read_meta(entry_dir).map(|meta| meta.output_hash)
}

/// Verify every entry in a store, sorted by hash.
///
/// An absent store directory is an EMPTY store, not an error — the GH-239
/// precedent from `cli::store_ops::list_store_entries`: the store is created by
/// the first import, not by asking about it.
pub fn verify_store(store_dir: &Path) -> Result<Vec<EntryVerdict>, String> {
    let read_dir = match std::fs::read_dir(store_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("read {}: {e}", store_dir.display())),
    };

    let mut entries: Vec<PathBuf> = read_dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| p.file_name().is_some_and(|n| n != ".gc-roots"))
        .collect();
    entries.sort();

    Ok(entries.iter().map(|p| verify_entry(p)).collect())
}
