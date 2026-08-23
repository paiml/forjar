//! `forjar lock-verify-chain` must be able to FAIL.
//!
//! The command builds a list of per-machine results with `valid=false` for
//! "signature file missing" / "malformed signature", prints them in red — and
//! then returns `Ok(())` unconditionally. Every failure mode exits 0, including
//! a state directory that does not exist. Any CI or release gate that invokes
//! it is inert: it passes precisely when the evidence it was meant to check is
//! absent or broken.
//!
//! Driven through the binary rather than the function, deliberately: the claim
//! under test is an EXIT CODE, and an exit code is a property of the process.
//!
//! forjar's exit codes: 0 success, 1 general error (`ErrorClass::Other`).
//! clap's own usage errors exit 2 — so every failure assertion here pins
//! `code == 1`, which a missing flag or a bad invocation cannot satisfy.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

const LOCK_YAML: &str =
    "schema: \"1\"\nmachine: local\nhostname: local\ngenerator: forjar\nresources: {}\n";

/// A state dir holding one machine (`local`) with a lock file and no signature.
fn state_with_lock(dir: &Path) -> PathBuf {
    let state = dir.join("state");
    std::fs::create_dir_all(state.join("local")).unwrap();
    std::fs::write(state.join("local/state.lock.yaml"), LOCK_YAML).unwrap();
    state
}

fn write_sig(state: &Path, sig: &str) {
    std::fs::write(state.join("local/lock.sig"), sig).unwrap();
}

fn run(args: &[&str]) -> Output {
    forjar().args(args).output().expect("spawn forjar")
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

/// A lock with no signature at all has no chain of custody to verify.
#[test]
fn missing_signature_must_not_exit_zero() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    let out = run(&[
        "lock-verify-chain",
        "--state-dir",
        state.to_str().unwrap(),
        "--presence-only",
    ]);
    assert_eq!(
        code(&out),
        1,
        "a missing signature must fail the gate\nstdout: {}\nstderr: {}",
        stdout(&out),
        stderr(&out)
    );
}

/// A signature that is not a hash at all is not a chain.
#[test]
fn malformed_signature_must_not_exit_zero() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    write_sig(&state, "not-a-hash");
    let out = run(&[
        "lock-verify-chain",
        "--state-dir",
        state.to_str().unwrap(),
        "--presence-only",
    ]);
    assert_eq!(
        code(&out),
        1,
        "a malformed signature must fail the gate\nstdout: {}\nstderr: {}",
        stdout(&out),
        stderr(&out)
    );
}

/// Defect (a): 64 zeros is well-formed hex and completely unrelated to the
/// lock. Well-formedness is not custody.
#[test]
fn signature_unrelated_to_the_lock_must_not_exit_zero() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    write_sig(&state, &"0".repeat(64));
    let out = run(&[
        "lock-verify-chain",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        "k",
    ]);
    assert_eq!(
        code(&out),
        1,
        "a signature not derived from the lock must fail the gate\nstdout: {}\nstderr: {}",
        stdout(&out),
        stderr(&out)
    );
}

/// The property the command claims to have: sign, then modify the lock. The
/// signature is still a well-formed 64-char hex hash — of the OLD content.
#[test]
fn a_lock_edited_after_signing_must_not_exit_zero() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    let signed = run(&[
        "lock-sign",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        "k",
    ]);
    assert_eq!(code(&signed), 0, "lock-sign: {}", stderr(&signed));

    std::fs::write(
        state.join("local/state.lock.yaml"),
        format!("{LOCK_YAML}# tampered\n"),
    )
    .unwrap();

    let out = run(&[
        "lock-verify-chain",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        "k",
    ]);
    assert_eq!(
        code(&out),
        1,
        "a lock edited after signing must fail the gate\nstdout: {}\nstderr: {}",
        stdout(&out),
        stderr(&out)
    );
}

/// Positive control. A gate that can never pass is as useless as one that can
/// never fail — a correctly signed lock must still verify.
#[test]
fn a_correctly_signed_lock_verifies() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    let signed = run(&[
        "lock-sign",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        "k",
    ]);
    assert_eq!(code(&signed), 0, "lock-sign: {}", stderr(&signed));

    let out = run(&[
        "lock-verify-chain",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        "k",
    ]);
    assert_eq!(
        code(&out),
        0,
        "a correctly signed lock must pass\nstdout: {}\nstderr: {}",
        stdout(&out),
        stderr(&out)
    );
}

/// The wrong key must not verify — otherwise the key is decoration.
#[test]
fn the_wrong_key_must_not_exit_zero() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    let signed = run(&[
        "lock-sign",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        "right",
    ]);
    assert_eq!(code(&signed), 0, "lock-sign: {}", stderr(&signed));

    let out = run(&[
        "lock-verify-chain",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        "wrong",
    ]);
    assert_eq!(
        code(&out),
        1,
        "the wrong key must fail the gate\nstdout: {}\nstderr: {}",
        stdout(&out),
        stderr(&out)
    );
}

/// Absent evidence is not a verified chain.
#[test]
fn a_nonexistent_state_dir_must_not_exit_zero() {
    let d = tempfile::tempdir().unwrap();
    let missing = d.path().join("no-such-state");
    let out = run(&[
        "lock-verify-chain",
        "--state-dir",
        missing.to_str().unwrap(),
        "--presence-only",
    ]);
    assert_eq!(
        code(&out),
        1,
        "a missing state dir must fail the gate\nstdout: {}\nstderr: {}",
        stdout(&out),
        stderr(&out)
    );
}

/// Neither is an empty one: a chain over zero locks proves nothing.
#[test]
fn an_empty_state_dir_must_not_exit_zero() {
    let d = tempfile::tempdir().unwrap();
    let state = d.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let out = run(&[
        "lock-verify-chain",
        "--state-dir",
        state.to_str().unwrap(),
        "--presence-only",
    ]);
    assert_eq!(
        code(&out),
        1,
        "an empty state dir must fail the gate\nstdout: {}\nstderr: {}",
        stdout(&out),
        stderr(&out)
    );
}

/// Defect (b): the `detail` field splices raw bytes of the signature file into
/// hand-rolled JSON. A signature containing `"` produced output that jq refuses.
#[test]
fn json_survives_a_quote_in_the_signature_file() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    write_sig(&state, "bad\"quote\\backslash\n");
    let out = run(&[
        "lock-verify-chain",
        "--state-dir",
        state.to_str().unwrap(),
        "--presence-only",
        "--json",
    ]);
    let text = stdout(&out);
    serde_json::from_str::<serde_json::Value>(&text)
        .unwrap_or_else(|e| panic!("--json emitted unparseable JSON ({e}): {text}"));
    assert_eq!(
        code(&out),
        1,
        "a malformed signature must still fail the gate in --json mode\nstdout: {text}"
    );
}

/// `--json` must stay machine-readable on the failure paths too, or a gate
/// that pipes it into jq breaks exactly when it matters.
#[test]
fn json_stays_parseable_when_the_state_dir_is_missing() {
    let d = tempfile::tempdir().unwrap();
    let missing = d.path().join("no-such-state");
    let out = run(&[
        "lock-verify-chain",
        "--state-dir",
        missing.to_str().unwrap(),
        "--presence-only",
        "--json",
    ]);
    let text = stdout(&out);
    serde_json::from_str::<serde_json::Value>(&text)
        .unwrap_or_else(|e| panic!("--json emitted unparseable JSON ({e}): {text}"));
    assert_eq!(code(&out), 1, "stdout: {text}");
}

/// The malformed-signature detail truncates the file's bytes with a byte
/// slice. A multibyte character straddling byte 20 panics the process.
#[test]
fn a_multibyte_signature_must_not_panic() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    write_sig(&state, &"☃".repeat(10)); // 3 bytes each: byte 20 is mid-char
    let out = run(&[
        "lock-verify-chain",
        "--state-dir",
        state.to_str().unwrap(),
        "--presence-only",
    ]);
    let err = stderr(&out);
    assert!(
        !err.contains("panicked"),
        "verify-chain panicked on a multibyte signature: {err}"
    );
    assert_eq!(code(&out), 1, "stdout: {}\nstderr: {err}", stdout(&out));
}

/// Without a key the command cannot verify custody at all. It must say so
/// rather than exit 0 — an absent verifier is a NO-GO, never a pass.
#[test]
fn without_a_key_the_command_refuses_to_claim_verification() {
    let d = tempfile::tempdir().unwrap();
    let state = state_with_lock(d.path());
    let signed = run(&[
        "lock-sign",
        "--state-dir",
        state.to_str().unwrap(),
        "--key",
        "k",
    ]);
    assert_eq!(code(&signed), 0, "lock-sign: {}", stderr(&signed));

    let out = run(&["lock-verify-chain", "--state-dir", state.to_str().unwrap()]);
    assert_eq!(
        code(&out),
        1,
        "a bare invocation must not report a verified chain\nstdout: {}\nstderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("--key") || stdout(&out).contains("--key"),
        "the refusal must name the flag that fixes it\nstdout: {}\nstderr: {}",
        stdout(&out),
        stderr(&out)
    );
}
