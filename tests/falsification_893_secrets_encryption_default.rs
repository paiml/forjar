//! forjar#893: the shipped binary must carry age encryption, `secrets encrypt`
//! must accept plaintext on stdin (never argv), and `secrets keygen -o` must
//! create the identity file 0600 from the start.
//!
//! Every test spawns `CARGO_BIN_EXE_forjar`, the artifact a stranger gets,
//! not an in-process function: the defect was a binary built without the
//! feature, which no in-process unit test under `--features encryption` sees.

use std::io::Write;
use std::process::{Command, Stdio};

/// The binary runs under umask 022 whatever the harness umask is: under 077 a
/// dropped `mode(0o600)` would still yield 0600 and go undetected.
fn forjar() -> Command {
    let mut c = Command::new("sh");
    c.args(["-c", "umask 022; exec \"$0\" \"$@\""])
        .arg(env!("CARGO_BIN_EXE_forjar"));
    c
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
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("cannot create"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
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

/// Plaintext arrives on stdin only; the marker on stdout is ciphertext that
/// does not contain it, and it round-trips through decrypt.
#[cfg(target_os = "linux")]
#[test]
fn encrypt_stdin_round_trips_and_marker_hides_plaintext() {
    use std::io::Read;
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

/// No value and an empty stdin is an error, not a silent empty secret: the
/// refusal says so, and no marker is printed.
#[test]
fn encrypt_without_value_or_stdin_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (_id, r) = new_identity(dir.path());
    let out = forjar()
        .args(["secrets", "encrypt", "-r", &r])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("no plaintext given"), "stderr: {err}");
    assert!(!String::from_utf8_lossy(&out.stdout).contains("ENC[age,"));
}

const PEM: &str = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEAsentinelpemline1\nsentinelpemline2ABCDEF\n-----END RSA PRIVATE KEY-----";

fn new_identity(dir: &std::path::Path) -> (std::path::PathBuf, String) {
    let id = dir.join("id.txt");
    assert!(forjar()
        .args(["secrets", "keygen", "-o"])
        .arg(&id)
        .status()
        .unwrap()
        .success());
    let r = recipient_of(&id);
    (id, r)
}

fn decrypt(id: &std::path::Path, marker: &str) -> String {
    let dec = forjar()
        .args(["secrets", "decrypt", marker, "-i"])
        .arg(id)
        .output()
        .unwrap();
    assert!(
        dec.status.success(),
        "{}",
        String::from_utf8_lossy(&dec.stderr)
    );
    String::from_utf8_lossy(&dec.stdout).trim_end().to_string()
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

#[test]
fn pem_round_trips_via_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let (id, r) = new_identity(dir.path());
    let mut child = forjar()
        .args(["secrets", "encrypt", "-r", &r])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("{PEM}\n").as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let marker = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert_eq!(decrypt(&id, &marker), PEM);
}

#[test]
fn pem_round_trips_via_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let (id, r) = new_identity(dir.path());
    let f = dir.path().join("key.pem");
    std::fs::write(&f, format!("{PEM}\n")).unwrap();
    let out = forjar()
        .args(["secrets", "encrypt", "-r", &r, "--from-file"])
        .arg(&f)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let marker = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert_eq!(decrypt(&id, &marker), PEM);
}

/// A secret passed positionally (the old form) is refused, and no output
/// stream carries any part of it, however it begins.
#[test]
fn positional_value_is_refused_and_never_echoed() {
    let dir = tempfile::tempdir().unwrap();
    let (_id, r) = new_identity(dir.path());
    for v in [
        PEM,
        "-----BEGIN-sentinelx",
        "-sentinel-dash",
        "plain-sentinel-zz",
    ] {
        for pos in 0..2 {
            let mut args = vec!["secrets", "encrypt"];
            if pos == 0 {
                args.push(v);
            }
            args.extend(["-r", r.as_str()]);
            if pos == 1 {
                args.push(v);
            }
            let out = forjar().args(&args).stdin(Stdio::null()).output().unwrap();
            assert!(!out.status.success(), "accepted positional {v:?}");
            assert_no_echo(&out, &[v, "sentinel", "BEGIN"]);
            // Refused for the right reason (not a bad recipient), and nothing
            // was encrypted.
            let err = String::from_utf8_lossy(&out.stderr);
            assert!(
                err.contains("invalid arguments for `forjar secrets encrypt`")
                    || err.contains("must not be passed as an argument"),
                "unexpected refusal for {v:?}: {err}"
            );
            assert!(!String::from_utf8_lossy(&out.stdout).contains("ENC[age,"));
        }
    }
}

/// Stray flags and `--flag=value` forms are clap errors; its rendering must
/// not carry the value either.
#[test]
fn clap_errors_on_secrets_encrypt_never_echo_value() {
    for args in [
        vec![
            "secrets",
            "encrypt",
            "-r",
            "age1xxxx",
            "--bogus=-----BEGIN-sentinelq",
        ],
        vec![
            "secrets",
            "encrypt",
            "-r",
            "age1xxxx",
            "--",
            "-----BEGIN-sentinelq",
        ],
        vec![
            "secrets",
            "encrypt",
            "--stdin=-----BEGIN-sentinelq",
            "-r",
            "age1xxxx",
        ],
        vec![
            "secrets",
            "encrypt",
            "-----BEGIN-sentinelq",
            "extra-sentinelq",
        ],
        // A global flag between `secrets` and `encrypt` must not defeat the
        // redaction (the verb is not always the next argv word).
        vec![
            "secrets",
            "-v",
            "encrypt",
            "--stdin=SEKRIT3-sentinelq",
            "-r",
            "age1xxxx",
        ],
        vec![
            "secrets",
            "--no-color",
            "encrypt",
            "--stdin=SEKRIT5-sentinelq",
            "-r",
            "age1xxxx",
        ],
        vec![
            "--no-color",
            "secrets",
            "-v",
            "encrypt",
            "--bogus=SEKRIT6-sentinelq",
        ],
    ] {
        let out = forjar().args(&args).stdin(Stdio::null()).output().unwrap();
        assert!(!out.status.success(), "{args:?}");
        assert_no_echo(&out, &["sentinelq", "BEGIN", "SEKRIT"]);
    }
}

/// A non-UTF-8 argv value must never panic (std::env::args() panics and prints
/// the whole value, exit 134) and must never be echoed.
#[cfg(unix)]
#[test]
fn non_utf8_secret_argv_is_never_echoed_or_panics() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let kg = forjar().args(["secrets", "keygen"]).output().unwrap();
    // `keygen` prints the identity on stdout and the public key on stderr.
    let recipient = String::from_utf8_lossy(&kg.stderr)
        .split_whitespace()
        .find(|w| w.starts_with("age1"))
        .unwrap_or("age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq")
        .to_string();
    let bad = OsStr::from_bytes(b"SEKRIT\xff");
    let out = forjar()
        .args(["secrets", "encrypt", "-r", &recipient])
        .arg(bad)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let all = [out.stdout.clone(), out.stderr.clone()].concat();
    assert!(
        !String::from_utf8_lossy(&all).contains("SEKRIT"),
        "secret leaked: {}",
        String::from_utf8_lossy(&all)
    );
    let code = out.status.code();
    assert!(
        code != Some(134) && code != Some(101) && code.is_some(),
        "panicked/aborted: {code:?}"
    );
}
