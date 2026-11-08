//! forjar#893 round 5: clap's `help` subcommand, glued global flags, unknown
//! leading flags, the 16 MiB cap on --from-file, and --stdin vs --from-file.

use std::process::{Command, Stdio};

const SENTINEL: &str = "SEKRIT42";

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

fn all_output(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Every shape that made clap quote the sentinel stays redacted.
#[test]
fn help_and_glued_flag_shapes_do_not_leak() {
    let shapes: Vec<Vec<String>> = vec![
        vec!["help", "secrets", "encrypt", SENTINEL],
        vec!["help", "secrets", SENTINEL],
        vec!["help", SENTINEL],
        vec!["--verbose=SEKRIT42", "secrets", "encrypt"],
        vec!["--no-color=SEKRIT42", "secrets", "encrypt"],
        vec!["--SEKRIT42", "secrets", "encrypt"],
        vec!["-v", "--SEKRIT42", "secrets", "encrypt"],
    ]
    .into_iter()
    .map(|v| v.into_iter().map(String::from).collect())
    .collect();
    for args in shapes {
        let out = forjar().args(&args).stdin(Stdio::null()).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", all_output(&out));
        assert!(!all_output(&out).contains(SENTINEL), "{args:?}");
    }
}

/// The real clap help for a secrets command still displays (exit 0).
#[test]
fn plain_help_still_works() {
    let out = forjar().args(["help", "secrets"]).output().unwrap();
    assert!(out.status.success(), "{}", all_output(&out));
    let out = forjar().args(["secrets", "--help"]).output().unwrap();
    assert!(out.status.success());
}

/// A non-secrets command keeps clap's real error.
#[test]
fn non_secrets_still_shows_clap_error() {
    let out = forjar()
        .args(["apply", "-f", "secrets.yaml", "--bogus"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(all_output(&out).contains("--bogus"));
}

/// A file one byte over 16 MiB is refused: exit 1, the cap named, no marker.
#[test]
fn from_file_over_cap_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("big");
    let f = std::fs::File::create(&p).unwrap();
    f.set_len(16 * 1024 * 1024 + 1).unwrap();
    drop(f);
    let out = forjar()
        .args(["secrets", "encrypt", "-r"])
        .arg("age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq")
        .arg("--from-file")
        .arg(&p)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", all_output(&out));
    assert!(String::from_utf8_lossy(&out.stderr).contains("16 MiB"));
    assert!(!String::from_utf8_lossy(&out.stdout).contains("ENC[age,"));
}

/// --stdin together with --from-file is a usage error, not silently accepted.
#[test]
fn stdin_conflicts_with_from_file() {
    let out = forjar()
        .args([
            "secrets",
            "encrypt",
            "-r",
            "age1x",
            "--stdin",
            "--from-file",
            "x",
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{}", all_output(&out));
}
