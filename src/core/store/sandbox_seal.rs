//! Refs #410: seal a sandbox's `$out` into the content-addressed store.
//!
//! Steps 8 and 9 of the sandbox plan used to be shell text: step 8 called
//! `forjar-hash-dir`, a binary forjar never shipped, and step 9 moved `$out`
//! to the literal path `<store>/HASH/content`. Neither needs a shell. This
//! does both in-process: it hashes `$out` with the store's own content hash,
//! renames it to `<store>/<blake3>/content`, and reads the entry back.
//!
//! Three refusals keep the returned hash honest:
//!
//! 1. A symlink or special file anywhere under `$out` is refused.
//!    `hash_directory` skips symlinks, so sealing one would return a hash that
//!    does not cover everything placed in the store.
//! 2. An entry that already exists under the hash must re-hash to it. Equal
//!    means the build is already sealed and `$out` is left alone; unequal
//!    means the store entry was altered, and nothing is overwritten.
//! 3. The moved entry is re-hashed after the rename and must match.
//!
//! The rename does not cross filesystems: `$out` and the store must share
//! one, or the move fails by name rather than falling back to a copy.

use super::content::content_hash;
use super::provider_exec::atomic_move_to_store;
use std::path::{Path, PathBuf};

/// A sealed sandbox output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedOutput {
    /// `blake3:<hex>` of the output tree
    pub output_hash: String,
    /// `<store>/<hex>/content`
    pub store_path: PathBuf,
    /// The entry was already in the store with this hash; `$out` was not moved
    pub already_present: bool,
}

/// Hash `out_dir` and move it atomically to `<store_dir>/<hex>/content`.
pub fn seal_output(out_dir: &Path, store_dir: &Path) -> Result<SealedOutput, String> {
    if !out_dir.is_dir() {
        return Err(format!(
            "no output directory at {} — a build that produced no $out cannot be sealed",
            out_dir.display()
        ));
    }
    refuse_unhashable(out_dir)?;

    let output_hash = content_hash(out_dir)?;
    let hex = output_hash.strip_prefix("blake3:").unwrap_or(&output_hash);
    let store_path = store_dir.join(hex).join("content");

    if store_path.exists() {
        let present = content_hash(&store_path)?;
        if present != output_hash {
            return Err(format!(
                "store entry {} hashes to {present}, not its address {output_hash}; refusing to overwrite it",
                store_path.display()
            ));
        }
        return Ok(SealedOutput {
            output_hash,
            store_path,
            already_present: true,
        });
    }

    atomic_move_to_store(out_dir, &store_path)?;

    let sealed = content_hash(&store_path)?;
    if sealed != output_hash {
        return Err(format!(
            "store entry {} hashes to {sealed} after the move, expected {output_hash}",
            store_path.display()
        ));
    }
    Ok(SealedOutput {
        output_hash,
        store_path,
        already_present: false,
    })
}

/// Refuse any entry under `dir` that is neither a regular file nor a directory.
fn refuse_unhashable(dir: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read dir {}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("read dir {}: {e}", dir.display()))?
            .path();
        let ft = std::fs::symlink_metadata(&path)
            .map_err(|e| format!("stat {}: {e}", path.display()))?
            .file_type();
        if ft.is_dir() {
            refuse_unhashable(&path)?;
        } else if !ft.is_file() {
            return Err(format!(
                "{} is not a regular file or directory; the store hash would not cover it, so $out is not sealed",
                path.display()
            ));
        }
    }
    Ok(())
}
