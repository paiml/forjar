//! Tests for the copia rolling delta engine (FJ-242 rolling).
#![allow(clippy::unwrap_used)]
use super::*;

fn blob(seed: u8, n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| seed.wrapping_add((i as u8).wrapping_mul(31)))
        .collect()
}

// ── the weak checksum must match copia bit-for-bit (else rolling degrades) ──
#[test]
fn weak_checksum_matches_copia() {
    for block in [
        b"".as_slice(),
        b"a",
        b"hello world",
        &blob(7, BLOCK_SIZE),
        &blob(200, BLOCK_SIZE - 3),
        &vec![0u8; 500],
        &vec![255u8; BLOCK_SIZE],
    ] {
        assert_eq!(
            weak_checksum(block),
            copia::RollingChecksum::new(block).digest(),
            "weak checksum diverges from copia for a {}-byte block",
            block.len()
        );
    }
}

// ── the receiver's (simulated) signature output parses to EXACTLY copia's own ──
fn sim_signature_output(data: &[u8]) -> String {
    let mut s = format!("SIZE:{}\n", data.len());
    for (i, chunk) in data.chunks(BLOCK_SIZE).enumerate() {
        let weak = weak_checksum(chunk);
        let strong = blake3::hash(chunk).to_hex();
        s.push_str(&format!("{i} {weak} {strong}\n"));
    }
    s
}

#[test]
fn parsed_signature_equals_copia_generate() {
    let data = blob(3, BLOCK_SIZE * 4 + 511);
    let parsed = parse_signature(&sim_signature_output(&data))
        .unwrap()
        .unwrap();
    let mut rdr = data.as_slice();
    let reference = copia::Signature::generate(&mut rdr, BLOCK_SIZE).unwrap();
    assert_eq!(parsed.block_size, reference.block_size);
    assert_eq!(parsed.blocks.len(), reference.blocks.len());
    for (p, r) in parsed.blocks.iter().zip(reference.blocks.iter()) {
        assert_eq!(
            p.weak_hash, r.weak_hash,
            "weak mismatch at block {}",
            p.index
        );
        assert_eq!(
            p.strong_hash.as_bytes(),
            r.strong_hash.as_bytes(),
            "strong mismatch"
        );
    }
}

fn reconstruct(basis: &[u8], delta: &copia::Delta) -> Vec<u8> {
    let mut out = Vec::new();
    for op in &delta.ops {
        match op {
            copia::DeltaOp::Copy { offset, len } => {
                let o = *offset as usize;
                out.extend_from_slice(&basis[o..o + *len as usize]);
            }
            copia::DeltaOp::Literal(d) => out.extend_from_slice(d),
        }
    }
    out
}

#[test]
fn rolling_delta_roundtrip_reconstructs_source() {
    let old = blob(1, BLOCK_SIZE * 6);
    let mut new = old.clone();
    new[BLOCK_SIZE * 2..BLOCK_SIZE * 2 + 10].fill(0xAB); // in-place edit
    let sig = parse_signature(&sim_signature_output(&old))
        .unwrap()
        .unwrap();
    let delta = rolling_delta(&new, &sig);
    assert_eq!(
        reconstruct(&old, &delta),
        new,
        "reconstruction must equal the source"
    );
}

#[test]
fn rolling_handles_insertion_without_full_retransfer() {
    // Insert bytes near the FRONT — fixed-block would re-transfer everything; rolling
    // must still find the shifted blocks (Copy ops), proving the real fix works.
    let old = blob(9, BLOCK_SIZE * 8);
    let mut new = Vec::new();
    new.extend_from_slice(&old[..100]);
    new.extend_from_slice(b"<<<INSERTED BYTES THAT SHIFT EVERYTHING AFTER>>>");
    new.extend_from_slice(&old[100..]);
    let sig = parse_signature(&sim_signature_output(&old))
        .unwrap()
        .unwrap();
    let delta = rolling_delta(&new, &sig);
    assert_eq!(
        reconstruct(&old, &delta),
        new,
        "reconstruction must be correct"
    );
    let copied: u64 = delta
        .ops
        .iter()
        .map(|op| match op {
            copia::DeltaOp::Copy { len, .. } => u64::from(*len),
            copia::DeltaOp::Literal(_) => 0,
        })
        .sum();
    // Most of the file is unchanged (just shifted) — rolling must Copy the bulk.
    assert!(
        copied > (old.len() as u64) / 2,
        "rolling should reuse >50% of the basis after an insertion, copied only {copied}"
    );
}

// ── receiver script uses only portable tools; no staged binary ──
#[test]
fn signature_script_is_portable_and_binary_free() {
    let s = signature_script("/opt/model.gguf");
    assert!(s.contains("od -An -v -tu1"));
    assert!(s.contains("b3sum --no-names"));
    assert!(
        s.contains("NO_B3SUM"),
        "must fall back cleanly if b3sum is absent"
    );
    assert!(
        !s.contains("copia "),
        "no staged copia binary is invoked on the receiver"
    );
    assert!(s.contains("FILE='/opt/model.gguf'"));
}

// ── patch script hardening (unchanged security properties) ──
#[test]
fn patch_script_hardening() {
    let mut d = copia::Delta::new(BLOCK_SIZE as u32, 10, 10);
    d.push_copy(0, 5);
    d.push_literal(b"hello");
    let script = patch_script("/etc/secret", &d, "b3hex", Some("root"), None, Some("0600"));
    // byte-range copy via tail|head, literal via heredoc (no echo argv)
    assert!(script.contains("tail -c +1 \"$DEST\" | head -c 5"));
    assert!(script.contains("base64 -d >> \"$TMPFILE\" <<'FORJAR_B64'"));
    // integrity verify + trap
    assert!(
        script.contains("integrity mismatch") && script.contains("trap 'rm -f \"$TMPFILE\"' EXIT")
    );
    // perms BEFORE the rename
    let chmod_at = script.find("chmod '0600' \"$TMPFILE\"").unwrap();
    let mv_at = script.find("mv \"$TMPFILE\" \"$DEST\"").unwrap();
    assert!(
        chmod_at < mv_at,
        "perms must be set before the atomic rename"
    );

    // ...and OWNERSHIP before it too. copia-provisioning-v1 FALSIFY-COPIA-001
    // claims "chmod/chown on $TMPFILE strictly before `mv`", and only the chmod
    // half was ever asserted. `chown` is emitted (copia/mod.rs:212,259) and its
    // ordering was untested, so the contract made a claim about half a property.
    // The window this guards is ownership as much as mode: a 0600 file owned by
    // the wrong user is still readable by the wrong user.
    let chown_at = script
        .find("chown 'root' \"$TMPFILE\"")
        .expect("owner was requested, so chown must be emitted");
    assert!(
        chown_at < mv_at,
        "ownership must be set before the atomic rename"
    );
}

#[test]
fn patch_script_shell_quotes_path() {
    let d = copia::Delta::new(BLOCK_SIZE as u32, 0, 0);
    let script = patch_script("/etc/a'b", &d, "h", None, None, None);
    assert!(script.contains(r#"DEST='/etc/a'\''b'"#));
}

#[test]
fn full_transfer_is_hardened() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("s");
    std::fs::write(&src, b"data").unwrap();
    let s = full_transfer_script(
        "/opt/t",
        src.to_str().unwrap(),
        Some("app"),
        None,
        Some("0644"),
    )
    .unwrap();
    assert!(s.contains("base64 -d > \"$TMPFILE\" <<'FORJAR_B64'"));
    let chmod_at = s.find("chmod '0644' \"$TMPFILE\"").unwrap();
    let mv_at = s.find("mv \"$TMPFILE\" \"$DEST\"").unwrap();
    assert!(chmod_at < mv_at);
}

#[test]
fn is_eligible_thresholds() {
    assert!(!is_eligible("/nonexistent/xyz"));
    let dir = tempfile::tempdir().unwrap();
    let small = dir.path().join("small");
    std::fs::write(&small, b"tiny").unwrap();
    assert!(!is_eligible(small.to_str().unwrap()));
}
