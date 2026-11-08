//! forjar#893 round 3: the secret must not reach any output stream via a
//! mis-scripted invocation (empty shell variable, glued words) nor a panic dump.
//! Every test spawns the built binary under umask 022.

use std::io::Write;
use std::process::{Command, Stdio};

fn forjar() -> Command {
    let mut c = Command::new("sh");
    c.args(["-c", "umask 022; exec \"$0\" \"$@\""])
        .arg(env!("CARGO_BIN_EXE_forjar"));
    c
}

fn new_identity(dir: &std::path::Path) -> (std::path::PathBuf, String) {
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

fn assert_no_echo(out: &std::process::Output, needles: &[&str]) {
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    for n in needles {
        assert!(!all.contains(n), "value echoed ({n}) in: {all}");
    }
}

fn run_with_stdin(args: &[&str], input: &str) -> std::process::Output {
    let mut child = forjar()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _ = child.stdin.take().unwrap().write_all(input.as_bytes());
    child.wait_with_output().unwrap()
}

/// Round 3 A: the shell drops an empty `$KEY`, so the secret lands in the
/// recipient slot (or glued onto a real recipient). Never echo the recipient.
#[test]
fn bad_recipient_is_never_echoed() {
    let dir = tempfile::tempdir().unwrap();
    let (_id, r) = new_identity(dir.path());
    let glued = format!("{r}hunter2SECRET");
    for args in [
        vec!["secrets", "encrypt", "-r", "hunter2SECRET"],
        vec!["secrets", "encrypt", "-r", glued.as_str()],
        vec![
            "secrets",
            "encrypt",
            "-r",
            r.as_str(),
            "-r",
            "hunter2SECRET",
        ],
    ] {
        let out = run_with_stdin(&args, "x\n");
        assert!(!out.status.success(), "{args:?}");
        assert_no_echo(&out, &["hunter2SECRET", "hunter2"]);
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("invalid recipient #"), "stderr: {err}");
    }
    let out = run_with_stdin(
        &[
            "secrets",
            "encrypt",
            "-r",
            r.as_str(),
            "-r",
            "hunter2SECRET",
        ],
        "x\n",
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("invalid recipient #2"));
}

/// Round 3 B: a secret given where `--from-file` wants a path is not echoed.
#[test]
fn from_file_path_is_never_echoed() {
    let dir = tempfile::tempdir().unwrap();
    let (_id, r) = new_identity(dir.path());
    let out = forjar()
        .args([
            "secrets",
            "encrypt",
            "-r",
            &r,
            "--from-file",
            "hunter2SECRET",
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_no_echo(&out, &["hunter2SECRET", "hunter2"]);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("cannot read the --from-file path"),
        "stderr: {err}"
    );
}

/// Round 3 C: a secret glued onto a subcommand word makes clap quote it.
#[test]
fn glued_subcommand_word_is_never_echoed() {
    for args in [
        vec!["secrets", "encXhunter2SECRET"],
        vec!["secretshunter2SECRET", "encrypt"],
        vec!["secrets=hunter2SECRET", "encrypt"],
        vec!["secrets", "encrypt", "--bogushunter2SECRET"],
    ] {
        let out = forjar().args(&args).stdin(Stdio::null()).output().unwrap();
        assert!(!out.status.success(), "{args:?}");
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert_no_echo(&out, &["hunter2SECRET", "hunter2"]);
    }
}

/// Round 3 C: redaction must not eat --help / --version.
#[test]
fn help_and_version_survive_redaction() {
    let v = forjar().arg("--version").output().unwrap();
    assert!(v.status.success());
    assert!(String::from_utf8_lossy(&v.stdout).contains("forjar"));
    let h = forjar().args(["secrets", "--help"]).output().unwrap();
    assert!(h.status.success());
    assert!(String::from_utf8_lossy(&h.stdout).contains("Usage"));
}

/// Round 3 D: a failing stdout (ENOSPC) must be a plain exit 1 with a fixed
/// message, not a panic/abort (exit 134 and a core dump holding the plaintext).
#[cfg(target_os = "linux")]
#[test]
fn full_stdout_is_a_clean_exit_not_a_panic() {
    let dir = tempfile::tempdir().unwrap();
    let (id, r) = new_identity(dir.path());
    let full = || {
        std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .unwrap()
    };
    let mut child = forjar()
        .args(["secrets", "encrypt", "-r", &r])
        .stdin(Stdio::piped())
        .stdout(Stdio::from(full()))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"S\n").unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(1), "{:?}", out.status);
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot write to stdout"));

    let out = forjar()
        .args(["secrets", "keygen"])
        .stdout(Stdio::from(full()))
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{:?}", out.status);
    let marker = {
        let o = run_with_stdin(&["secrets", "encrypt", "-r", &r], "S\n");
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    };
    let out = forjar()
        .args(["secrets", "decrypt", &marker, "-i"])
        .arg(&id)
        .stdout(Stdio::from(full()))
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{:?}", out.status);
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot write to stdout"));
}
