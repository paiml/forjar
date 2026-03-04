//! FJ-242: Copia delta sync — rsync-style block-level file transfer.
//!
//! For source files > 1MB, transfers only changed blocks instead of the full
//! file. Uses BLAKE3 per-block hashing for change detection.
//! Falls back to base64 for new files (no remote state to diff against).

use base64::Engine;

/// Block size for delta sync (4KB).
pub const BLOCK_SIZE: usize = 4096;

/// Files larger than this threshold use delta sync (1MB).
pub const SIZE_THRESHOLD: u64 = 1_048_576;

/// A remote block signature (index + BLAKE3 hash).
#[derive(Debug, Clone, PartialEq)]
pub struct BlockSignature {
    pub index: usize,
    pub hash: String,
}

/// A delta operation: either copy an existing block or supply new data.
#[derive(Debug, Clone)]
pub enum DeltaOp {
    /// Copy block at `index` from the existing remote file.
    Copy { index: usize },
    /// Replace block with new literal data.
    Literal { data: Vec<u8> },
}

/// Compute per-block BLAKE3 signatures for local data.
pub fn compute_signatures(data: &[u8]) -> Vec<BlockSignature> {
    data.chunks(BLOCK_SIZE)
        .enumerate()
        .map(|(i, chunk)| {
            let hash = blake3::hash(chunk);
            BlockSignature {
                index: i,
                hash: hash.to_hex().to_string(),
            }
        })
        .collect()
}

/// Compute delta operations by comparing new data against remote signatures.
/// Matching blocks produce Copy ops; differing blocks produce Literal ops.
pub fn compute_delta(new_data: &[u8], remote_sigs: &[BlockSignature]) -> Vec<DeltaOp> {
    let mut ops = Vec::new();

    for (i, chunk) in new_data.chunks(BLOCK_SIZE).enumerate() {
        let local_hash = blake3::hash(chunk).to_hex().to_string();
        let matches_remote = remote_sigs
            .get(i)
            .map(|sig| sig.hash == local_hash)
            .unwrap_or(false);

        if matches_remote {
            ops.push(DeltaOp::Copy { index: i });
        } else {
            ops.push(DeltaOp::Literal {
                data: chunk.to_vec(),
            });
        }
    }

    ops
}

/// Count literal (changed) blocks in a delta.
pub fn literal_count(ops: &[DeltaOp]) -> usize {
    ops.iter()
        .filter(|op| matches!(op, DeltaOp::Literal { .. }))
        .count()
}

/// Total bytes in literal blocks.
pub fn literal_bytes(ops: &[DeltaOp]) -> usize {
    ops.iter()
        .map(|op| match op {
            DeltaOp::Literal { data } => data.len(),
            DeltaOp::Copy { .. } => 0,
        })
        .sum()
}

/// Generate a shell script to compute per-block BLAKE3 signatures on the remote.
/// Output format: "NEW_FILE" if file missing, else "SIZE:<n>" header + "INDEX HASH" per block.
pub fn signature_script(path: &str) -> String {
    format!(
        "set -euo pipefail\n\
         FILE='{path}'\n\
         if [ ! -f \"$FILE\" ]; then\n\
           echo 'NEW_FILE'\n\
           exit 0\n\
         fi\n\
         SIZE=$(stat -c %s \"$FILE\" 2>/dev/null || stat -f %z \"$FILE\")\n\
         echo \"SIZE:$SIZE\"\n\
         BLOCKS=$(( (SIZE + {BLOCK_SIZE} - 1) / {BLOCK_SIZE} ))\n\
         for i in $(seq 0 $((BLOCKS - 1))); do\n\
           HASH=$(dd if=\"$FILE\" bs={BLOCK_SIZE} skip=$i count=1 2>/dev/null | \
                  b3sum --no-names 2>/dev/null || \
                  dd if=\"$FILE\" bs={BLOCK_SIZE} skip=$i count=1 2>/dev/null | sha256sum | cut -d' ' -f1)\n\
           echo \"$i $HASH\"\n\
         done",
    )
}

/// Parse the output of a remote signature script.
/// Returns None if file is new (no remote signatures to diff against).
pub fn parse_signatures(output: &str) -> Result<Option<Vec<BlockSignature>>, String> {
    let mut sigs = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "NEW_FILE" {
            return Ok(None);
        }
        if line.starts_with("SIZE:") {
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return Err(format!("invalid signature line: {line}"));
        }
        let index: usize = parts[0]
            .parse()
            .map_err(|_| format!("invalid block index: {}", parts[0]))?;
        sigs.push(BlockSignature {
            index,
            hash: parts[1].to_string(),
        });
    }

    Ok(Some(sigs))
}

/// Generate a shell script to apply a delta patch on the remote.
/// Reconstructs the file from Copy (existing blocks) and Literal (new data) operations,
/// then sets ownership and mode.
pub fn patch_script(
    path: &str,
    ops: &[DeltaOp],
    owner: Option<&str>,
    group: Option<&str>,
    mode: Option<&str>,
) -> String {
    let mut lines = vec![
        "set -euo pipefail".to_string(),
        format!("TMPFILE='{}.forjar-delta.$$'", path),
        "rm -f \"$TMPFILE\"".to_string(),
    ];

    for op in ops {
        match op {
            DeltaOp::Copy { index } => {
                lines.push(format!(
                    "dd if='{path}' bs={BLOCK_SIZE} skip={index} count=1 >> \"$TMPFILE\" 2>/dev/null",
                ));
            }
            DeltaOp::Literal { data } => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(data);
                lines.push(format!("echo '{b64}' | base64 -d >> \"$TMPFILE\""));
            }
        }
    }

    // Atomic replace
    lines.push(format!("mv \"$TMPFILE\" '{path}'"));

    // Ownership
    if let Some(owner) = owner {
        if let Some(group) = group {
            lines.push(format!("chown '{owner}:{group}' '{path}'"));
        } else {
            lines.push(format!("chown '{owner}' '{path}'"));
        }
    }

    // Mode
    if let Some(mode) = mode {
        lines.push(format!("chmod '{mode}' '{path}'"));
    }

    lines.join("\n")
}

/// Check if a source file is eligible for copia delta sync (exists and > 1MB).
pub fn is_eligible(source_path: &str) -> bool {
    match std::fs::metadata(source_path) {
        Ok(meta) => meta.len() > SIZE_THRESHOLD,
        Err(_) => false,
    }
}

/// Full base64 apply script for new files (no remote state to diff against).
/// Used as fallback when signature_script returns NEW_FILE.
pub fn full_transfer_script(
    path: &str,
    source_path: &str,
    owner: Option<&str>,
    group: Option<&str>,
    mode: Option<&str>,
) -> Result<String, String> {
    let data = std::fs::read(source_path).map_err(|e| format!("{source_path}: {e}"))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);

    let mut lines = vec!["set -euo pipefail".to_string()];

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(path).parent() {
        if parent != std::path::Path::new("/") {
            lines.push(format!("mkdir -p '{}'", parent.display()));
        }
    }

    lines.push(format!("echo '{b64}' | base64 -d > '{path}'"));

    if let Some(owner) = owner {
        if let Some(group) = group {
            lines.push(format!("chown '{owner}:{group}' '{path}'"));
        } else {
            lines.push(format!("chown '{owner}' '{path}'"));
        }
    }

    if let Some(mode) = mode {
        lines.push(format!("chmod '{mode}' '{path}'"));
    }

    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests;
