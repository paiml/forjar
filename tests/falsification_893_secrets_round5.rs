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

// ---- round 6 ----

/// Global flag with a SPACE-separated value: the secret takes the subcommand
/// slot and clap says `unrecognized subcommand 'S'`. Must stay redacted.
#[test]
fn flag_then_secret_then_secrets_does_not_leak() {
    let shapes: Vec<Vec<&str>> = vec![
        vec!["--verbose", SENTINEL, "secrets", "encrypt"],
        vec!["-v", SENTINEL, "secrets", "encrypt"],
        vec!["-vv", SENTINEL, "secrets", "encrypt", "-r", "x"],
        vec!["--no-color", SENTINEL, "secrets", "encrypt"],
        vec!["--no-color", SENTINEL, "Secrets", "encrypt"],
        vec!["-v", SENTINEL, "secretsX"],
    ];
    for args in shapes {
        let out = forjar().args(&args).stdin(Stdio::null()).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", all_output(&out));
        assert!(!all_output(&out).contains(SENTINEL), "{args:?}");
    }
}

/// The InvalidSubcommand rule must not over-redact other commands.
#[test]
fn invalid_subcommand_without_secrets_word_keeps_clap_error() {
    for args in [
        vec!["apply", "-f", "secrets.yaml", "--bogus"],
        vec!["apply", "--bogus"],
        vec!["-v", "bogusword", "apply"],
    ] {
        let out = forjar().args(&args).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(all_output(&out).contains("bogus"), "{args:?}");
    }
}

/// `forjar help bogus` gets its own fixed message, not the `secrets` one.
#[test]
fn help_bogus_has_its_own_message() {
    let out = forjar().args(["help", SENTINEL]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let s = all_output(&out);
    assert!(s.contains("invalid arguments for `forjar help`"), "{s}");
    assert!(!s.contains("forjar secrets"), "{s}");
    assert!(!s.contains(SENTINEL), "{s}");
}

#[cfg(target_os = "linux")]
mod full_stdout {
    use super::*;

    fn full() -> std::fs::File {
        std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .unwrap()
    }

    fn run_full(args: &[&std::ffi::OsStr]) -> std::process::Output {
        forjar()
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::from(full()))
            .stderr(Stdio::piped())
            .output()
            .unwrap()
    }

    fn assert_clean_exit(out: &std::process::Output, what: &str) {
        assert_eq!(out.status.code(), Some(1), "{what}: {:?}", out.status);
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("cannot write to stdout"), "{what}: {err}");
        assert!(!err.contains("panicked"), "{what}: {err}");
    }

    fn setup(dir: &std::path::Path) -> (std::path::PathBuf, String) {
        let id = dir.join("id.txt");
        assert!(forjar()
            .args(["secrets", "keygen", "-o"])
            .arg(&id)
            .status()
            .unwrap()
            .success());
        let body = std::fs::read_to_string(&id).unwrap();
        let r = body
            .lines()
            .find_map(|l| l.strip_prefix("# public key: "))
            .unwrap()
            .trim()
            .to_string();
        (id, r)
    }

    fn marker_file(dir: &std::path::Path, r: &str) -> std::path::PathBuf {
        use std::io::Write;
        let mut c = forjar()
            .args(["secrets", "encrypt", "-r", r])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        c.stdin.take().unwrap().write_all(b"S\n").unwrap();
        let m = String::from_utf8(c.wait_with_output().unwrap().stdout).unwrap();
        let p = dir.join("s.yaml");
        std::fs::write(&p, format!("k: {}\n", m.trim())).unwrap();
        p
    }

    fn os(s: &str) -> &std::ffi::OsStr {
        std::ffi::OsStr::new(s)
    }

    #[test]
    fn view_rekey_rotate_full_stdout_exit_one() {
        let dir = tempfile::tempdir().unwrap();
        let (id, r) = setup(dir.path());
        let f = marker_file(dir.path(), &r);
        let plain = dir.path().join("plain.yaml");
        std::fs::write(&plain, "a: b\n").unwrap();
        let state = dir.path().join("state");

        let out = run_full(&[
            os("secrets"),
            os("view"),
            os("-f"),
            f.as_os_str(),
            os("-i"),
            id.as_os_str(),
        ]);
        assert_clean_exit(&out, "view marker");
        let out = run_full(&[os("secrets"), os("view"), os("-f"), plain.as_os_str()]);
        assert_clean_exit(&out, "view no-marker");

        let out = run_full(&[
            os("secrets"),
            os("rekey"),
            os("-f"),
            f.as_os_str(),
            os("-i"),
            id.as_os_str(),
            os("-r"),
            os(&r),
        ]);
        assert_clean_exit(&out, "rekey");
        let out = run_full(&[
            os("secrets"),
            os("rekey"),
            os("-f"),
            plain.as_os_str(),
            os("-r"),
            os(&r),
        ]);
        assert_clean_exit(&out, "rekey no-marker");

        let out = run_full(&[
            os("secrets"),
            os("rotate"),
            os("-f"),
            f.as_os_str(),
            os("-i"),
            id.as_os_str(),
            os("-r"),
            os(&r),
            os("--re-encrypt"),
            os("--state-dir"),
            state.as_os_str(),
        ]);
        assert_clean_exit(&out, "rotate");
        let out = run_full(&[
            os("secrets"),
            os("rotate"),
            os("-f"),
            plain.as_os_str(),
            os("-r"),
            os(&r),
            os("--re-encrypt"),
            os("--state-dir"),
            state.as_os_str(),
        ]);
        assert_clean_exit(&out, "rotate no-marker");
    }
}
