//! forjar#893: the shipped binary must carry age encryption, `secrets encrypt`
//! must accept plaintext on stdin (never argv), and `secrets keygen -o` must
//! create the identity file 0600 from the start.
//!
//! Every test spawns `CARGO_BIN_EXE_forjar`, the artifact a stranger gets,
//! not an in-process function: the defect was a binary built without the
//! feature, which no in-process unit test under `--features encryption` sees.

use std::io::{Read, Write};
use std::process::{Command, Stdio};

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// The gate: the built binary carries encryption. Fails with
/// "compiled without encryption support" when the feature is off.
#[test]
fn built_binary_keygen_succeeds() {
    let out = forjar().args(["secrets", "keygen"]).output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "keygen failed (binary lacks encryption?): {stderr}"
    );
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("AGE-SECRET-KEY-"));
    assert!(!stderr.contains("compiled without encryption"));
}

#[cfg(unix)]
#[test]
fn keygen_output_file_is_created_0600() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("id.txt");
    let out = forjar()
        .args(["secrets", "keygen", "-o"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "identity file mode was {mode:o}");
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.contains("AGE-SECRET-KEY-"));
    // the secret key must not also go to stdout when -o is given
    assert!(!String::from_utf8_lossy(&out.stdout).contains("AGE-SECRET-KEY-"));
}

#[cfg(unix)]
#[test]
fn keygen_output_refuses_to_overwrite_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("id.txt");
    std::fs::write(&path, "keep me").unwrap();
    let out = forjar()
        .args(["secrets", "keygen", "-o"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep me");
}

fn recipient_of(identity_file: &std::path::Path) -> String {
    let body = std::fs::read_to_string(identity_file).unwrap();
    body.lines()
        .find_map(|l| l.strip_prefix("# public key: "))
        .expect("public key comment")
        .trim()
        .to_string()
}

/// Plaintext arrives on stdin AFTER the process is observed: its
/// /proc/<pid>/cmdline cannot contain a value it has not been given yet, and
/// the argv carries no value at all. Round-trips through decrypt.
#[cfg(target_os = "linux")]
#[test]
fn encrypt_reads_plaintext_from_stdin_never_argv() {
    let dir = tempfile::tempdir().unwrap();
    let id = dir.path().join("id.txt");
    assert!(forjar()
        .args(["secrets", "keygen", "-o"])
        .arg(&id)
        .status()
        .unwrap()
        .success());
    let recipient = recipient_of(&id);
    let plaintext = "hunter2-plaintext-sentinel";

    let mut child = forjar()
        .args(["secrets", "encrypt", "--stdin", "-r", &recipient])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let cmdline = std::fs::read(format!("/proc/{}/cmdline", child.id())).unwrap();
    assert!(
        !String::from_utf8_lossy(&cmdline).contains(plaintext),
        "plaintext in argv"
    );
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("{plaintext}\n").as_bytes())
        .unwrap();
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    assert!(child.wait().unwrap().success());
    let marker = stdout.trim();
    assert!(marker.starts_with("ENC[age,"), "{marker}");
    assert!(!marker.contains(plaintext));

    let dec = forjar()
        .args(["secrets", "decrypt", marker, "-i"])
        .arg(&id)
        .output()
        .unwrap();
    assert!(
        dec.status.success(),
        "{}",
        String::from_utf8_lossy(&dec.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&dec.stdout).trim(), plaintext);
}

/// `-` as the value means stdin too.
#[test]
fn encrypt_dash_value_means_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let id = dir.path().join("id.txt");
    assert!(forjar()
        .args(["secrets", "keygen", "-o"])
        .arg(&id)
        .status()
        .unwrap()
        .success());
    let recipient = recipient_of(&id);
    let mut child = forjar()
        .args(["secrets", "encrypt", "-", "-r", &recipient])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"abc").unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("ENC[age,"));
}

/// No value and no --stdin is an error, not a silent empty secret.
#[test]
fn encrypt_without_value_or_stdin_is_refused() {
    let out = forjar()
        .args(["secrets", "encrypt", "-r", "age1xxxx"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!out.status.success());
}
