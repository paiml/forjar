//! forjar#893 round 4: redaction only for the secrets family, no path echoes,
//! a 16 MiB plaintext cap. Every test spawns the built binary.

use std::io::Write;
use std::process::{Command, Stdio};

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

/// A config file named `secrets*.yaml` must not swallow clap's real error.
#[test]
fn non_secrets_commands_keep_clap_errors() {
    for (args, needle) in [
        (vec!["apply", "-f", "secrets.yaml", "--bogus"], "--bogus"),
        (
            vec!["validate", "-f", "secrets-prod.yaml", "--nope"],
            "--nope",
        ),
    ] {
        let out = forjar().args(&args).stdin(Stdio::null()).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let text = all_output(&out);
        assert!(text.contains(needle), "{args:?}: {text}");
        assert!(!text.contains("not shown"), "{args:?}: {text}");
    }
}

/// A secret glued onto the subcommand word stays redacted.
#[test]
fn glued_subcommand_position_stays_redacted() {
    for args in [
        vec!["secrets=hunter2", "encrypt"],
        vec!["secretshunter2", "encrypt"],
        vec!["secret=hunter2"],
        vec!["-v", "secret=hunter2"],
    ] {
        let out = forjar().args(&args).stdin(Stdio::null()).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(!all_output(&out).contains("hunter2"), "{args:?}");
    }
}

/// keygen -o / decrypt -i / view / rekey / rotate echo no path, only the kind.
#[test]
fn secrets_family_errors_echo_no_path() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("hunter2SECRET-missing");
    let m = missing.to_str().unwrap();
    let existing = dir.path().join("hunter2SECRET-existing");
    std::fs::write(&existing, "x").unwrap();
    let e = existing.to_str().unwrap();
    for args in [
        vec!["secrets", "keygen", "-o", e],
        vec!["secrets", "decrypt", "ENC[age,x]", "-i", m],
        vec!["secrets", "view", "-f", m],
        vec!["secrets", "rekey", "-f", m, "-r", "age1x"],
        vec!["secrets", "rotate", "-f", m, "-r", "age1x", "--re-encrypt"],
    ] {
        let out = forjar().args(&args).stdin(Stdio::null()).output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{args:?}: {}", all_output(&out));
        let text = all_output(&out);
        assert!(!text.contains("hunter2SECRET"), "{args:?}: {text}");
    }
    // The success path of keygen -o also names no path.
    let ok = dir.path().join("hunter2SECRET-new");
    let out = forjar()
        .args(["secrets", "keygen", "-o"])
        .arg(&ok)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(!all_output(&out).contains("hunter2SECRET"));
}

/// More than 16 MiB on stdin is refused with a fixed message, exit 1.
#[test]
fn oversized_stdin_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let id = dir.path().join("id.txt");
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
    let mut child = forjar()
        .args(["secrets", "encrypt", "-r", &r])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let chunk = vec![b'a'; 1 << 20];
    let writer = std::thread::spawn(move || {
        // 17 MiB; the child may stop reading early, so ignore EPIPE.
        for _ in 0..17 {
            if stdin.write_all(&chunk).is_err() {
                break;
            }
        }
    });
    let out = child.wait_with_output().unwrap();
    writer.join().unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", all_output(&out));
    assert!(all_output(&out).contains("16 MiB"), "{}", all_output(&out));
}
