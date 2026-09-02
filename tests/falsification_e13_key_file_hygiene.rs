//! forjar#408 (CRUX audit E13), second file: key-file hygiene and the help
//! surface. Split from `falsification_e13_signing_key_argv.rs` when it crossed
//! the 500-line budget; same shim, same helpers, same binary.
//!
//! WHAT WAS OBSERVABLY WRONG. A 0644 key file was read without a word — ssh
//! refuses such a key outright — and `lock-verify-chain --help` was the one
//! key-taking command whose help nobody had asserted documents `file:`/`env:`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

/// A state dir holding one converged lock for machine `lambda`.
fn state_with_lock(dir: &Path) -> PathBuf {
    let state = dir.join("state");
    let md = state.join("lambda");
    std::fs::create_dir_all(&md).expect("state dir");
    std::fs::write(
        md.join("state.lock.yaml"),
        "schema: \"1.0\"\nmachine: lambda\nhostname: lambda\ngenerated_at: now\n\
         generator: forjar-test\nblake3_version: \"1\"\nresources: {}\n",
    )
    .expect("write lock");
    state
}

fn key_file(dir: &Path, name: &str, secret: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, secret).expect("write key file");
    p
}

fn run(args: &[&str]) -> Output {
    Command::new(forjar())
        .args(args)
        .output()
        .expect("spawn forjar")
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
    format!(
        "exit={}\nstdout:\n{}\nstderr:\n{}",
        code(out),
        stdout(out),
        stderr(out)
    )
}

/// A key file readable by other users is the argv leak one directory over:
/// ssh refuses it, forjar warns and names the mode. RED with `warn_if_shared`
/// removed.
#[test]
fn a_world_readable_key_file_is_warned_about() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let state = state_with_lock(dir.path());
    let key = key_file(dir.path(), "shared.key", "k9");
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o644)).expect("chmod 644");
    let spec = format!("file:{}", key.display());
    let out = run(&[
        "lock-sign",
        "--state-dir",
        &state.to_string_lossy(),
        "--key",
        &spec,
    ]);
    assert_eq!(
        code(&out),
        0,
        "signing with a shared key file still signs: {}",
        ctx(&out)
    );
    assert!(
        stderr(&out).contains("readable by other users") && stderr(&out).contains("0644"),
        "the mode must be named in a warning: {}",
        ctx(&out)
    );
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).expect("chmod 600");
    let out = run(&[
        "lock-sign",
        "--state-dir",
        &state.to_string_lossy(),
        "--key",
        &spec,
    ]);
    assert!(
        !stderr(&out).contains("readable by other users"),
        "a 0600 key file must not warn: {}",
        ctx(&out)
    );
}

/// The old help read "path to key file or inline" while the code hashed the
/// string verbatim — the file half did not exist. Help that documents a
/// capability the binary does not have is how a secret ends up in `ps`.
#[test]
fn help_must_document_the_indirect_forms() {
    for cmd in [
        "lock-sign",
        "lock-verify-sig",
        "lock-rotate-keys",
        "lock-verify-chain",
    ] {
        let out = run(&[cmd, "--help"]);
        let text = format!("{}{}", stdout(&out), stderr(&out));
        assert!(
            text.contains("file:") && text.contains("env:"),
            "{cmd} --help must document the file:/env: forms; got: {text}"
        );
        assert!(
            !text.contains("path to key file or inline"),
            "{cmd} --help still claims a bare path is read as a file: {text}"
        );
    }
}
