//! E13: the lock signing key must not have to travel on `argv`.
//!
//! `--key` was a bare `String` documented "path to key file or inline". The
//! "path to key file" half was never implemented — the string was hashed
//! verbatim — so the only way to sign anything was to put the secret itself
//! on the command line, where every local user reads it out of `ps`.
//!
//! The irony this pins down: forjar already feeds `script:` bodies to remote
//! shells over stdin *specifically* so they stay out of `ps`. The signing key,
//! which is actual key material, got no such care.
//!
//! Driven through the binary, not the functions: the claim is about what a
//! process can be asked to do without its secret appearing in its own argv,
//! and argv is a property of a process.
//!
//! forjar exit codes: 0 success, 1 general error. clap usage errors exit 2, so
//! every failure assertion pins `code == 1` — a typo'd flag cannot satisfy it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const LOCK_YAML: &str =
    "schema: \"1\"\nmachine: local\nhostname: local\ngenerator: forjar\nresources: {}\n";

/// The secret that must never need to appear on a command line.
const SECRET: &str = "s3cret-signing-key-do-not-put-me-in-ps";

fn state_with_lock(dir: &Path) -> PathBuf {
    let state = dir.join("state");
    std::fs::create_dir_all(state.join("local")).unwrap();
    std::fs::write(state.join("local/state.lock.yaml"), LOCK_YAML).unwrap();
    state
}

fn sig_path(state: &Path) -> PathBuf {
    state.join("local/lock.sig")
}

fn key_file(dir: &Path, name: &str, secret: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, format!("{secret}\n")).unwrap();
    p
}

fn run(args: &[&str]) -> Output {
    run_env(args, &[])
}

fn run_env(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_forjar"));
    cmd.args(args);
    // Never inherit a stray key ref from the developer's shell.
    cmd.env_remove("FORJAR_E13_KEY");
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().expect("spawn forjar")
}

fn code(out: &Output) -> i32 {
    out.status.code().unwrap_or(-1)
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn ctx(out: &Output) -> String {
    format!("\nstdout: {}\nstderr: {}", stdout(out), stderr(out))
}

// ── the key must be resolvable from a FILE, off argv ──────────────────

/// Sign naming the key by file, then verify with the key material itself.
///
/// The cross-check is the whole point: signing and verifying with the *same*
/// spec passes even when the spec is being hashed verbatim, so it proves
/// nothing. `file:<path>` must produce the signature the SECRET produces.
#[test]
fn key_file_ref_must_resolve_to_the_files_contents() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    let kf = key_file(d.path(), "signing.key", SECRET);

    let signed = run(&[
        "lock-sign",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        &format!("file:{}", kf.display()),
    ]);
    assert_eq!(
        code(&signed),
        0,
        "signing by key file must work{}",
        ctx(&signed)
    );

    let verified = run(&[
        "lock-verify-sig",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        SECRET,
    ]);
    assert_eq!(
        code(&verified),
        0,
        "a signature made with --key file:<path> must be the signature of the \
         file's CONTENTS, not of the literal string \"file:<path>\"{}",
        ctx(&verified)
    );

    // An indirect reference is the sanctioned form: it must not be nagged at.
    let se = stderr(&signed).to_lowercase();
    assert!(
        !se.contains("deprecat"),
        "file: refs are the supported form and must not warn: {}",
        stderr(&signed)
    );
}

/// A key file that is not there must fail, not sign with the literal spec.
///
/// This is the silent-corruption case: unfixed, `--key file:/nope` hashes the
/// string "file:/nope" and reports "Signed 1 lock file(s)" — a lock signed
/// with a key nobody holds, which every later verify will reject.
#[test]
fn missing_key_file_must_fail_without_writing_a_signature() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    let missing = d.path().join("nope.key");

    let out = run(&[
        "lock-sign",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        &format!("file:{}", missing.display()),
    ]);
    assert_eq!(
        code(&out),
        1,
        "an unreadable key file must be an error{}",
        ctx(&out)
    );
    assert!(
        !sig_path(&state).exists(),
        "no signature may be written when the key could not be read{}",
        ctx(&out)
    );
}

/// An empty key file is not a key.
#[test]
fn empty_key_file_must_fail_without_writing_a_signature() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    let kf = d.path().join("empty.key");
    std::fs::write(&kf, "\n").unwrap();

    let out = run(&[
        "lock-sign",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        &format!("file:{}", kf.display()),
    ]);
    assert_eq!(
        code(&out),
        1,
        "an empty key file must be an error{}",
        ctx(&out)
    );
    assert!(
        !sig_path(&state).exists(),
        "no signature may be written for an empty key{}",
        ctx(&out)
    );
    // Exit 1 alone is what ANY initialisation failure prints (E13 quorum, agy
    // lane); the refusal must name the key source's own reason.
    assert!(
        stderr(&out).contains("is empty"),
        "the failure must come from the key source, not from somewhere else: {}",
        ctx(&out)
    );
}

// ── the key must be resolvable from the ENVIRONMENT, off argv ─────────

#[test]
fn env_ref_must_resolve_to_the_variables_value() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());

    let signed = run_env(
        &[
            "lock-sign",
            "--state-dir",
            state.to_str().unwrap(),
            "--key",
            "env:FORJAR_E13_KEY",
        ],
        &[("FORJAR_E13_KEY", SECRET)],
    );
    assert_eq!(
        code(&signed),
        0,
        "signing by env ref must work{}",
        ctx(&signed)
    );

    let verified = run(&[
        "lock-verify-sig",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        SECRET,
    ]);
    assert_eq!(
        code(&verified),
        0,
        "a signature made with --key env:VAR must be the signature of the \
         VARIABLE'S VALUE, not of the literal string \"env:VAR\"{}",
        ctx(&verified)
    );
}

#[test]
fn unset_env_ref_must_fail_without_writing_a_signature() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());

    let out = run(&[
        "lock-sign",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        "env:FORJAR_E13_KEY",
    ]);
    assert_eq!(
        code(&out),
        1,
        "an unset env var must be an error{}",
        ctx(&out)
    );
    assert!(
        !sig_path(&state).exists(),
        "no signature may be written when the env var is unset{}",
        ctx(&out)
    );
    // Exit 1 alone is what ANY initialisation failure prints (E13 quorum, agy
    // lane); the refusal must name the key source's own reason.
    assert!(
        stderr(&out).contains("is not set"),
        "the failure must come from the key source, not from somewhere else: {}",
        ctx(&out)
    );
}

// ── the other two commands that take key material ─────────────────────

#[test]
fn rotate_keys_must_resolve_both_key_refs() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    let old = key_file(d.path(), "old.key", SECRET);
    let new = key_file(d.path(), "new.key", "the-next-signing-key");

    let signed = run(&[
        "lock-sign",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        &format!("file:{}", old.display()),
    ]);
    assert_eq!(code(&signed), 0, "sign{}", ctx(&signed));

    let rotated = run(&[
        "lock-rotate-keys",
        "--state-dir",
        state.to_str().unwrap(),
        "--old-key",
        &format!("file:{}", old.display()),
        "--new-key",
        &format!("file:{}", new.display()),
    ]);
    assert_eq!(code(&rotated), 0, "rotation by key file{}", ctx(&rotated));

    let verified = run(&[
        "lock-verify-sig",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        "the-next-signing-key",
    ]);
    assert_eq!(
        code(&verified),
        0,
        "--new-key file:<path> must rotate to the file's CONTENTS{}",
        ctx(&verified)
    );
}

#[test]
fn verify_chain_must_resolve_a_key_ref() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    let kf = key_file(d.path(), "chain.key", SECRET);

    // Signed with the material itself; the chain check names it indirectly.
    let signed = run(&[
        "lock-sign",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        SECRET,
    ]);
    assert_eq!(code(&signed), 0, "sign{}", ctx(&signed));

    let out = run(&[
        "lock-verify-chain",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        &format!("file:{}", kf.display()),
    ]);
    assert_eq!(
        code(&out),
        0,
        "lock-verify-chain --key file:<path> must verify against the file's \
         CONTENTS{}",
        ctx(&out)
    );
}

/// `lock-verify-sig` must resolve its OWN key ref — the verifier is a second
/// call site, not a passenger on `lock-sign`.
///
/// Deliberately crossed the other way from
/// `key_file_ref_must_resolve_to_the_files_contents`: sign with the material,
/// verify by naming a file. Every other test in this file verifies with a
/// literal, so an unresolved verifier would sail through all of them while
/// calling a perfectly good signature invalid in the field.
#[test]
fn verify_sig_must_resolve_a_key_ref() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    let kf = key_file(d.path(), "verify.key", SECRET);

    let signed = run(&[
        "lock-sign",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        SECRET,
    ]);
    assert_eq!(code(&signed), 0, "sign{}", ctx(&signed));

    let out = run(&[
        "lock-verify-sig",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        &format!("file:{}", kf.display()),
    ]);
    assert_eq!(
        code(&out),
        0,
        "lock-verify-sig --key file:<path> must verify against the file's \
         CONTENTS, not the literal string \"file:<path>\"{}",
        ctx(&out)
    );

    // An unreadable key file must be reported AS an unreadable key file.
    // Exit 1 alone cannot tell the two apart: an unresolved verifier also
    // exits 1, but for the wrong reason — it says the signature is bad when
    // the truth is that the operator's key never got read. Pin the reason.
    let missing = run(&[
        "lock-verify-sig",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        &format!("file:{}", d.path().join("absent.key").display()),
    ]);
    assert_eq!(
        code(&missing),
        1,
        "an unreadable key file must fail{}",
        ctx(&missing)
    );
    assert!(
        stderr(&missing).contains("cannot read key file"),
        "an unreadable key must be named as such, not reported as an invalid \
         signature; stderr was: {}",
        stderr(&missing)
    );
}

// ── inline key material still works, but is loudly deprecated ─────────

/// Compatibility is kept — and paid for with a warning that names the risk,
/// the replacement, and the release the escape hatch disappears in.
#[test]
fn inline_key_must_warn_with_a_named_removal_version() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());

    let out = run(&[
        "lock-sign",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        SECRET,
    ]);
    assert_eq!(
        code(&out),
        0,
        "an inline key must keep working for now{}",
        ctx(&out)
    );

    let err = stderr(&out);
    let lower = err.to_lowercase();
    for needle in ["--key", "deprecat", "ps", "file:", "env:"] {
        assert!(
            lower.contains(needle),
            "the inline-key warning must mention {needle:?}; stderr was: {err}"
        );
    }
    assert!(
        err.contains("2.0.0"),
        "the warning must name the removal version; stderr was: {err}"
    );
    // The warning must not leak the very secret it is complaining about.
    assert!(
        !err.contains(SECRET),
        "the warning must not print the key material: {err}"
    );
}

#[test]
fn rotate_keys_must_warn_for_each_inline_key() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());

    let signed = run(&[
        "lock-sign",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        "old",
    ]);
    assert_eq!(code(&signed), 0, "sign{}", ctx(&signed));

    let out = run(&[
        "lock-rotate-keys",
        "--state-dir",
        state.to_str().unwrap(),
        "--old-key",
        "old",
        "--new-key",
        "new",
    ]);
    assert_eq!(
        code(&out),
        0,
        "inline rotation must still work{}",
        ctx(&out)
    );
    let err = stderr(&out);
    assert!(
        err.contains("--old-key") && err.contains("--new-key"),
        "both inline key flags must be named in the warning; stderr was: {err}"
    );
}

// ── the help string must stop lying ───────────────────────────────────
