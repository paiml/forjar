//! E03 (paiml/forjar#405) — a "verify" that never reads the signature.
//!
//! Three surfaces claimed to verify a signature and then compared only a
//! BLAKE3 content hash, or nothing at all:
//!
//! `sign --verify` (`src/cli/recipe_signing.rs:75`) deserialised `signature`
//! and `signer` and never looked at either. `sign --pq --verify`
//! (`src/cli/pq_signing.rs:83`) did the same to `classical_sig` and `pq_sig`,
//! then printed "both signatures valid". `lock-verify-hmac`
//! (`src/cli/lock_audit.rs:174`) re-hashed the lock into `let _hash` and
//! incremented `verified`.
//!
//! The property these tests assert is NOT "the fields are compared". It is the
//! weaker, honest one that either fix satisfies:
//!
//!     No forjar verb may accept a forged signature and exit 0.
//!
//! A verb that reads the signature satisfies it. So does a verb that no longer
//! exists. Forjar took the second route (issue #405 option (b)): `sign --pq`
//! and `lock-verify-hmac` are gone, `sign` is now `digest`, and the surviving
//! signature surface is `lock-sign` / `lock-verify-sig`, which has always
//! recomputed a keyed hash and compared it.
//!
//! Each test names the mutation that turns it RED. Measured against the
//! unfixed tree before the fix landed:
//!
//!   forged_recipe_signature_is_never_accepted     — `{"valid": true}`, exit 0
//!   forged_dual_signature_is_never_accepted       — "both signatures valid", exit 0
//!   garbage_lock_hmac_is_never_reported_verified  — `{"verified":1,...}`, exit 0
//!   digest_verify_fails_on_a_one_byte_mutation    — no `digest` subcommand
//!   the_digest_sidecar_claims_no_signature        — no `digest` subcommand

use std::path::Path;
use std::process::{Command, Output};

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

fn run(args: &[&str]) -> Output {
    forjar().args(args).output().expect("spawn forjar")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// True when clap rejected the invocation because the verb/flag is not there.
///
/// Strict on purpose: "the binary exited non-zero" is not evidence a verb was
/// withdrawn — a panic, a missing file, or an unrelated error would all pass a
/// loose check and let a still-broken verb hide behind them.
fn withdrawn(out: &Output) -> bool {
    let e = stderr(out);
    !out.status.success()
        && (e.contains("unrecognized subcommand")
            || e.contains("unexpected argument")
            || e.contains("invalid subcommand"))
}

/// Overwrite every hex value of `field` in a JSON sidecar with `forged`.
fn forge_json_field(path: &Path, field: &str, forged: &str) {
    let text = std::fs::read_to_string(path).expect("read sidecar");
    let needle = format!("\"{field}\": \"");
    let start = text
        .find(&needle)
        .unwrap_or_else(|| panic!("sidecar has no {field} field:\n{text}"))
        + needle.len();
    let end = start + text[start..].find('"').expect("closing quote");
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..start]);
    out.push_str(forged);
    out.push_str(&text[end..]);
    std::fs::write(path, out).expect("write sidecar");
}

fn read_json(path: &Path) -> serde_json::Value {
    let text = std::fs::read_to_string(path).expect("read sidecar");
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("sidecar is not JSON ({e}):\n{text}"))
}

/// A state dir with one machine whose lock exists but is not signed by us.
fn state_with_one_lock(dir: &Path) -> std::path::PathBuf {
    let state = dir.join("state");
    let machine = state.join("lambda");
    std::fs::create_dir_all(&machine).expect("mkdir machine");
    std::fs::write(
        machine.join("state.lock.yaml"),
        "schema: \"1.0\"\nmachine: lambda\nhostname: lambda.local\n\
         generated_at: \"2026-01-01T00:00:00Z\"\ngenerator: forjar 1.23.1\n\
         blake3_version: \"1.5\"\nresources: {}\n",
    )
    .expect("write lock");
    state
}

// ── E03-a: the recipe "signature" ───────────────────────────────────────────

/// FALSIFY-E03-001 — a forged `signature`/`signer` must never verify.
///
/// RED under: restoring `cli::recipe_signing` and the `sign` verb, whose
/// `verify_recipe` compared `current_hash == sig.blake3_hash` and copied
/// `sig.signer` into the result untouched.
#[test]
fn forged_recipe_signature_is_never_accepted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let recipe = dir.path().join("f.txt");
    std::fs::write(&recipe, "version: \"1.0\"\n").expect("write recipe");
    let r = recipe.to_string_lossy().to_string();

    let signed = run(&["sign", &r, "--signer", "ci"]);
    if !signed.status.success() {
        assert!(
            withdrawn(&signed),
            "`forjar sign` failed for a reason other than being withdrawn.\n\
             stderr: {}",
            stderr(&signed)
        );
        return;
    }

    // The verb exists, so it must actually read what it signed.
    let sidecar = dir.path().join("f.sig.json");
    forge_json_field(&sidecar, "signature", "deadbeef");
    forge_json_field(&sidecar, "signer", "root@prod");

    let verified = run(&["sign", &r, "--verify", "--json"]);
    assert!(
        !verified.status.success(),
        "`sign --verify` accepted a signature forged to \"deadbeef\" by \
         \"root@prod\" and exited 0.\nstdout: {}",
        stdout(&verified)
    );
}

/// FALSIFY-E03-002 — a forged dual (classical + PQ) signature must never verify.
///
/// RED under: restoring `cli::pq_signing` and the `--pq` flag, whose
/// `dual_verify` set `classical_valid`, `pq_valid` and `both_valid` all three
/// from one content-hash comparison and printed "both signatures valid".
#[test]
fn forged_dual_signature_is_never_accepted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("f.txt");
    std::fs::write(&file, "payload\n").expect("write file");
    let f = file.to_string_lossy().to_string();

    let signed = run(&["sign", &f, "--pq", "--signer", "ci"]);
    if !signed.status.success() {
        assert!(
            withdrawn(&signed),
            "`forjar sign --pq` failed for a reason other than being \
             withdrawn.\nstderr: {}",
            stderr(&signed)
        );
        assert!(
            !dir.path().join("f.dual-sig.json").exists(),
            "a withdrawn `--pq` still wrote a dual-signature sidecar"
        );
        return;
    }

    let sidecar = dir.path().join("f.dual-sig.json");
    forge_json_field(&sidecar, "classical_sig", "deadbeef");
    forge_json_field(&sidecar, "pq_sig", "deadbeef");

    let verified = run(&["sign", &f, "--pq", "--verify", "--json"]);
    assert!(
        !verified.status.success(),
        "`sign --pq --verify` accepted two signatures forged to \"deadbeef\" \
         and exited 0.\nstdout: {}",
        stdout(&verified)
    );
}

// ── E03-b: the lock HMAC ────────────────────────────────────────────────────

/// FALSIFY-E03-003 — a garbage lock signature must never count as "verified".
///
/// The fixture reproduces the issue exactly: a sig file at the path
/// `lock-verify-hmac` looked for (`<state>/<machine>.lock.yaml.sig`, which is
/// not even where `lock-sign` writes), holding bytes that are not a signature
/// of anything.
///
/// RED under: restoring `cmd_lock_verify_hmac`, which reached
/// `let _hash = hasher::hash_string(&content); verified += 1;` for any lock
/// with a sig file beside it, and returned `Ok(())` unconditionally.
#[test]
fn garbage_lock_hmac_is_never_reported_verified() {
    let dir = tempfile::tempdir().expect("tempdir");
    let state = state_with_one_lock(dir.path());
    std::fs::write(state.join("lambda.lock.yaml.sig"), "not-a-signature\n").expect("write sig");
    let s = state.to_string_lossy().to_string();

    let out = run(&["lock-verify-hmac", "--state-dir", &s, "--json"]);
    if !out.status.success() {
        assert!(
            withdrawn(&out),
            "`forjar lock-verify-hmac` failed for a reason other than being \
             withdrawn.\nstderr: {}",
            stderr(&out)
        );
        return;
    }
    panic!(
        "`lock-verify-hmac` exited 0 on a lock whose only signature is the \
         literal bytes \"not-a-signature\".\nstdout: {}",
        stdout(&out)
    );
}

/// FALSIFY-E03-004 — the honest twin rejects a one-byte mutation.
///
/// This is issue #405's success criterion applied to the surface that SURVIVED
/// the subtraction. Stated honestly: it was already green before the fix, so
/// it falsifies nothing about #405 — it is the regression guard that keeps the
/// one real signature check real now that the three fake ones are gone.
///
/// RED under: deleting the `actual_sig.trim() == expected_sig` comparison in
/// `cli::lock_security::verify_machine_sig`.
#[test]
fn lock_verify_sig_rejects_a_one_byte_mutation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let state = state_with_one_lock(dir.path());
    let s = state.to_string_lossy().to_string();

    assert!(
        run(&["lock-sign", "--state-dir", &s, "--key", "k9"])
            .status
            .success(),
        "lock-sign failed"
    );
    assert!(
        run(&["lock-verify-sig", "--state-dir", &s, "--key", "k9"])
            .status
            .success(),
        "a freshly signed lock did not verify"
    );

    let sig_path = state.join("lambda").join("lock.sig");
    let sig = std::fs::read_to_string(&sig_path).expect("read lock.sig");
    let mut bytes = sig.into_bytes();
    bytes[0] = if bytes[0] == b'a' { b'b' } else { b'a' };
    std::fs::write(&sig_path, bytes).expect("write mutated sig");

    let out = run(&["lock-verify-sig", "--state-dir", &s, "--key", "k9"]);
    assert!(
        !out.status.success(),
        "lock-verify-sig accepted a signature with one byte flipped.\n\
         stdout: {}",
        stdout(&out)
    );
}

// ── E03-c: what `sign` actually was ─────────────────────────────────────────

/// FALSIFY-E03-005 — `digest --verify` fails when the recorded hash is mutated.
///
/// RED under: not shipping `digest` at all (measured before the fix:
/// "unrecognized subcommand 'digest'"), or dropping the
/// `current_hash == recorded.blake3_hash` comparison.
#[test]
fn digest_verify_fails_on_a_one_byte_mutation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("f.txt");
    std::fs::write(&file, "version: \"1.0\"\n").expect("write file");
    let f = file.to_string_lossy().to_string();

    let out = run(&["digest", &f]);
    assert!(
        out.status.success(),
        "`forjar digest` failed.\nstderr: {}",
        stderr(&out)
    );

    let sidecar = dir.path().join("f.digest.json");
    assert!(
        run(&["digest", &f, "--verify", "--json"]).status.success(),
        "a fresh digest did not verify"
    );

    let recorded = read_json(&sidecar)["blake3_hash"]
        .as_str()
        .expect("blake3_hash is a string")
        .to_string();
    let mut chars: Vec<char> = recorded.chars().collect();
    chars[0] = if chars[0] == 'a' { 'b' } else { 'a' };
    forge_json_field(
        &sidecar,
        "blake3_hash",
        &chars.into_iter().collect::<String>(),
    );

    let verified = run(&["digest", &f, "--verify", "--json"]);
    assert!(
        !verified.status.success(),
        "`digest --verify` exited 0 with one byte of the recorded hash \
         flipped.\nstdout: {}",
        stdout(&verified)
    );
    assert!(
        stdout(&verified).contains("\"valid\": false"),
        "`digest --verify --json` did not report valid:false.\nstdout: {}",
        stdout(&verified)
    );
}

/// FALSIFY-E03-006 — the digest sidecar makes no signature claim.
///
/// The old sidecar carried `"signature"`, `"signer"` and
/// `"algorithm": "blake3-hmac"`. None of the three was ever verified, and the
/// last is not even true — a plain BLAKE3 of public inputs is not an HMAC. A
/// consumer reading those keys would be reading a lie, so they are gone.
///
/// RED under: reintroducing any of those keys into the sidecar.
#[test]
fn the_digest_sidecar_claims_no_signature() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("f.txt");
    std::fs::write(&file, "payload\n").expect("write file");
    let f = file.to_string_lossy().to_string();

    assert!(run(&["digest", &f]).status.success(), "digest failed");

    let sidecar = read_json(&dir.path().join("f.digest.json"));
    let obj = sidecar.as_object().expect("sidecar is an object");
    for forbidden in ["signature", "signer", "classical_sig", "pq_sig"] {
        assert!(
            !obj.contains_key(forbidden),
            "digest sidecar still carries a `{forbidden}` key nothing verifies:\n{sidecar:#}"
        );
    }
    let text = sidecar.to_string().to_lowercase();
    for lie in ["hmac", "slh-dsa", "signature"] {
        assert!(
            !text.contains(lie),
            "digest sidecar still advertises `{lie}`:\n{sidecar:#}"
        );
    }
}
