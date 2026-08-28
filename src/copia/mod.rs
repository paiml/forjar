//! FJ-242 (rolling): copia rolling delta sync for large-file provisioning.
//!
//! **The real fix** for insertion/deletion inefficiency: forjar now computes a
//! true rolling delta via the copia crate (`copia::CopiaSync::delta` finds block
//! matches at ANY byte offset, so a 1-byte insertion no longer re-transfers the
//! whole file — the old fixed-block engine did). No copia binary is staged on the
//! receiver: the receiver reports a `copia::Signature` computed with `od`+`awk`
//! (the Adler weak checksum, `(b<<16)|a`) and `b3sum` (the blake3 strong hash),
//! all universal tools. forjar does the rolling match locally with the library.
//!
//! Safety: if the receiver's checksums ever disagreed with copia (busybox, tool
//! differences) the delta degrades to all-literal — correct output, no benefit —
//! it can never corrupt (the strong hash is re-checked, and the whole file's
//! blake3 is verified before the atomic rename).

use base64::Engine;
use copia::{BlockSignature, DeltaOp, StrongHash, Sync as _};
// Re-exported so forjar's benches/tests can name them as forjar::copia::*.
pub use copia::{Delta, Signature};

/// Block size for delta sync. Smaller = finer matching + bigger signature.
pub const BLOCK_SIZE: usize = 2048;

/// Files larger than this threshold use delta sync (1 MiB).
pub const SIZE_THRESHOLD: u64 = 1_048_576;

/// POSIX-safe single-quoting for interpolating an untrusted string into a shell
/// script (closes the injection hole from bare `'{path}'` interpolation).
///
/// Delegates to the canonical escaper. A second implementation is a second
/// place for the escape idiom to be wrong: this one still emitted `'\''`,
/// which forjar's own I8 gate rejects (#350).
#[must_use]
pub fn shell_quote(s: &str) -> String {
    crate::core::shell_escape::sh_squote(s)
}

/// copia's weak (rolling Adler) checksum of a block, `(b<<16)|a` with
/// `a = Σ byte`, `b = Σ (len-i)·byte_i`, mod 65521. Kept in Rust as the reference
/// the receiver `awk` must match (verified in tests against `copia::RollingChecksum`).
#[must_use]
pub fn weak_checksum(block: &[u8]) -> u32 {
    const M: u64 = 65521;
    let len = block.len() as u64;
    let mut a: u64 = 0;
    let mut c: u64 = 0;
    for (i, &byte) in block.iter().enumerate() {
        a = (a + u64::from(byte)) % M;
        c = (c + (i as u64) * u64::from(byte)) % M;
    }
    // b = (len*a - c) mod M, with a,c already reduced; +M guards the subtraction.
    let b = (((len * a) % M) + M - c) % M;
    ((b << 16) | a) as u32
}

/// Receiver script: prints `NEW_FILE` if the file is absent, else `SIZE:<n>`
/// followed by one `<index> <weak> <blake3hex>` line per block. Pure `od`/`awk`/
/// `b3sum` — no staged binary. Falls back (empty output) if `b3sum` is missing so
/// the caller can full-transfer.
#[must_use]
pub fn signature_script(path: &str) -> String {
    let q = shell_quote(path);
    format!(
        "set -euo pipefail\n\
         FILE={q}\n\
         if [ ! -f \"$FILE\" ]; then echo NEW_FILE; exit 0; fi\n\
         command -v b3sum >/dev/null 2>&1 || {{ echo NO_B3SUM; exit 0; }}\n\
         SIZE=$(wc -c < \"$FILE\" | tr -d ' ')\n\
         echo \"SIZE:$SIZE\"\n\
         BS={BLOCK_SIZE}\n\
         WEAK=$(mktemp)\n\
         trap 'rm -f \"$WEAK\"' EXIT\n\
         od -An -v -tu1 \"$FILE\" | awk -v bs=$BS -v size=$SIZE '\n\
           {{ for(i=1;i<=NF;i++){{ byte=$i; blk=int(n/bs); pos=n%bs;\n\
                a[blk]=(a[blk]+byte)%65521; c[blk]=(c[blk]+pos*byte)%65521; n++ }} }}\n\
           END{{ nb=int((size+bs-1)/bs);\n\
             for(k=0;k<nb;k++){{ len=(k<nb-1)?bs:(size-(nb-1)*bs);\n\
               b=((len*a[k]-c[k])%65521+65521)%65521;\n\
               printf \"%d %d\\n\", k, b*65536+a[k] }} }}' > \"$WEAK\"\n\
         BLOCKS=$(( (SIZE + BS - 1) / BS ))\n\
         i=0\n\
         while [ $i -lt $BLOCKS ]; do\n\
           STRONG=$(dd if=\"$FILE\" bs=$BS skip=$i count=1 2>/dev/null | b3sum --no-names)\n\
           WK=$(sed -n \"$((i+1))p\" \"$WEAK\" | cut -d' ' -f2)\n\
           echo \"$i $WK $STRONG\"\n\
           i=$((i+1))\n\
         done"
    )
}

/// Parse a signature-script's output into a `copia::Signature`. Returns
/// `Ok(None)` for a new file / no-b3sum receiver (caller full-transfers).
///
/// # Errors
/// Malformed lines (bad index, weak, or non-64-hex strong hash).
pub fn parse_signature(output: &str) -> Result<Option<Signature>, String> {
    let mut file_size: u64 = 0;
    let mut blocks = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "NEW_FILE" || line == "NO_B3SUM" {
            return Ok(None);
        }
        if let Some(sz) = line.strip_prefix("SIZE:") {
            file_size = sz.parse().map_err(|_| format!("bad SIZE: {sz}"))?;
            continue;
        }
        let mut it = line.split_whitespace();
        let index: u32 = it
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("bad index: {line}"))?;
        let weak: u32 = it
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| format!("bad weak: {line}"))?;
        let strong_hex = it.next().ok_or_else(|| format!("missing strong: {line}"))?;
        let bytes =
            decode_hex32(strong_hex).ok_or_else(|| format!("bad strong hash: {strong_hex}"))?;
        blocks.push(BlockSignature::new(
            index,
            weak,
            StrongHash::from_bytes(bytes),
        ));
    }
    Ok(Some(Signature {
        block_size: BLOCK_SIZE,
        file_size,
        blocks,
    }))
}

fn decode_hex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

/// Compute the ROLLING delta of `source` against the remote `signature`, locally,
/// via the copia crate. This is where a byte-shift is handled without re-transfer.
#[must_use]
pub fn rolling_delta(source: &[u8], signature: &Signature) -> Delta {
    copia::CopiaSync::with_block_size(signature.block_size)
        .delta(source, signature)
        .unwrap_or_else(|_| {
            // Never fail provisioning on a delta error — full literal is always correct.
            let mut d = Delta::new(
                signature.block_size as u32,
                source.len() as u64,
                signature.file_size,
            );
            d.push_literal(source);
            d
        })
}

/// Generate the hardened patch script from a copia rolling `Delta`. Reconstructs
/// into a temp file (byte-range copies via `tail|head`, literal inserts via a
/// base64 heredoc), VERIFIES the whole-file blake3, sets perms, then atomically
/// renames — the receiver side of the delta.
#[must_use]
pub fn patch_script(
    path: &str,
    delta: &Delta,
    expected_blake3: &str,
    owner: Option<&str>,
    group: Option<&str>,
    mode: Option<&str>,
) -> String {
    let dest = shell_quote(path);
    let mut lines = vec![
        "set -euo pipefail".to_string(),
        format!("DEST={dest}"),
        "TMPFILE=\"$DEST.forjar-delta.$$\"".to_string(),
        "trap 'rm -f \"$TMPFILE\"' EXIT".to_string(),
        "rm -f \"$TMPFILE\"".to_string(),
    ];
    for op in &delta.ops {
        match op {
            DeltaOp::Copy { offset, len } => {
                // POSIX byte-range copy from the basis (1-based for `tail -c +N`).
                lines.push(format!(
                    "tail -c +{} \"$DEST\" | head -c {len} >> \"$TMPFILE\"",
                    offset + 1
                ));
            }
            DeltaOp::Literal(data) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(data);
                lines.push(format!(
                    "base64 -d >> \"$TMPFILE\" <<'FORJAR_B64'\n{b64}\nFORJAR_B64"
                ));
            }
        }
    }
    lines.push(format!("EXPECT={}", shell_quote(expected_blake3)));
    lines.push(
        "if command -v b3sum >/dev/null 2>&1; then \
         GOT=$(b3sum --no-names \"$TMPFILE\"); \
         if [ \"$GOT\" != \"$EXPECT\" ]; then echo 'copia: integrity mismatch, aborting' >&2; exit 1; fi; \
         fi"
            .to_string(),
    );
    if let Some(owner) = owner {
        let spec = match group {
            Some(g) => format!("{owner}:{g}"),
            None => owner.to_string(),
        };
        lines.push(format!("chown {} \"$TMPFILE\"", shell_quote(&spec)));
    }
    if let Some(mode) = mode {
        lines.push(format!("chmod {} \"$TMPFILE\"", shell_quote(mode)));
    }
    lines.push("mv \"$TMPFILE\" \"$DEST\"".to_string());
    lines.push("trap - EXIT".to_string());
    lines.join("\n")
}

/// Full base64 apply script for a new file (no remote basis). Hardened: heredoc
/// (no ARG_MAX), temp-file staging, perms before the atomic rename, cleanup trap.
///
/// # Errors
/// Reading the local source file.
pub fn full_transfer_script(
    path: &str,
    source_path: &str,
    owner: Option<&str>,
    group: Option<&str>,
    mode: Option<&str>,
) -> Result<String, String> {
    let data = std::fs::read(source_path).map_err(|e| format!("{source_path}: {e}"))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
    let dest = shell_quote(path);
    let mut lines = vec![
        "set -euo pipefail".to_string(),
        format!("DEST={dest}"),
        "TMPFILE=\"$DEST.forjar-new.$$\"".to_string(),
        "trap 'rm -f \"$TMPFILE\"' EXIT".to_string(),
    ];
    if let Some(parent) = std::path::Path::new(path).parent() {
        if parent != std::path::Path::new("/") {
            lines.push(format!(
                "mkdir -p {}",
                shell_quote(&parent.display().to_string())
            ));
        }
    }
    lines.push(format!(
        "base64 -d > \"$TMPFILE\" <<'FORJAR_B64'\n{b64}\nFORJAR_B64"
    ));
    if let Some(owner) = owner {
        let spec = match group {
            Some(g) => format!("{owner}:{g}"),
            None => owner.to_string(),
        };
        lines.push(format!("chown {} \"$TMPFILE\"", shell_quote(&spec)));
    }
    if let Some(mode) = mode {
        lines.push(format!("chmod {} \"$TMPFILE\"", shell_quote(mode)));
    }
    lines.push("mv \"$TMPFILE\" \"$DEST\"".to_string());
    lines.push("trap - EXIT".to_string());
    Ok(lines.join("\n"))
}

/// Whether a source file is eligible for delta sync (exists and > 1 MiB).
#[must_use]
pub fn is_eligible(source_path: &str) -> bool {
    std::fs::metadata(source_path).is_ok_and(|m| m.len() > SIZE_THRESHOLD)
}

#[cfg(test)]
mod tests;
