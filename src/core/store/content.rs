//! GH-236: the digest of the bytes a store entry HOLDS.
//!
//! The store has always been able to answer "has this recipe with these inputs
//! already been built?" — that is what `path::store_path` computes, and it is
//! what makes staleness detection and cache lookup work. It could not answer
//! "are these bytes the bytes we produced?", because nothing ever recorded a
//! digest of a produced byte. Rewriting `<entry>/content/out.mp4` in place
//! changed nothing any API could observe.
//!
//! This is the second address, computed alongside the first, never instead of
//! it. `store_path` is untouched: changing it would re-address every entry
//! already on disk.

use std::path::Path;

/// BLAKE3 over an entry's `content/` tree.
///
/// Delegates to [`crate::tripwire::hasher::hash_directory`] deliberately, and
/// NOT to `provider_exec::hash_staging_dir`, even though the latter also hashes
/// a tree and was the issue's suggestion. Two reasons:
///
/// 1. `hash_staging_dir`'s walker does `std::fs::read(&path)` — it slurps each
///    whole file into RAM. The workload this issue exists for is a 149.9 GiB
///    store of rendered mp4; verifying it that way would OOM. `hash_directory`
///    streams at `STREAM_BUF_SIZE`.
/// 2. `hash_staging_dir` is the import path's live ADDRESS function. Changing
///    what it returns re-addresses every entry `forjar store-import` ever wrote.
///
/// An absent `content/` is an error, not an empty digest: a malformed entry
/// must not be sealable.
pub fn content_hash(content_dir: &Path) -> Result<String, String> {
    if !content_dir.is_dir() {
        return Err(format!(
            "no content directory at {} — an entry with no content cannot be sealed",
            content_dir.display()
        ));
    }
    crate::tripwire::hasher::hash_directory(content_dir)
}
