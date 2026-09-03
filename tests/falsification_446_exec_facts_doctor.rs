//! Refs #446: one-off remote exec, host facts, and a machine-level doctor.
//!
//! THE FLAW THIS CLOSES.
//!
//! Provisioning a machine, an operator hit `Permission denied` on a curl into a
//! destination directory. forjar could tell them the apply failed. It could not
//! tell them who owns the directory, what its mode is, which identity forjar
//! connects as, whether that identity has sudo, or whether the remote `PATH`
//! even contains `/usr/local/bin` — the recurring cause of "command not found"
//! over SSH. Every one of those questions had to be answered by hand, outside
//! forjar, with an ad-hoc ssh session, because forjar had no verb for "run this
//! one command over there" and no verb for "describe that host".
//!
//! WHAT THIS TEST MUST NOT BECOME. Asserting that a formatter renders a table
//! would prove nothing: the defect is the ABSENCE of the capability at the
//! process boundary. Every case here therefore spawns the REAL binary against a
//! REAL config and asserts on REAL bytes and REAL exit codes — never on a
//! summary line alone, which is exactly the shape that would stay green if the
//! verb became a no-op.

use std::path::Path;
use std::process::{Command, Output};

/// A config whose only machine is this host, reached without SSH.
fn config(dest: &Path) -> String {
    format!(
        r#"version: '1.0'
name: fj446
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  greeting:
    type: file
    machine: local
    path: {}
    content: hi
"#,
        dest.display()
    )
}

/// Write `forjar.yaml` into `dir` and return its path.
fn write_config(dir: &Path, dest: &Path) -> std::path::PathBuf {
    let path = dir.join("forjar.yaml");
    std::fs::write(&path, config(dest)).expect("write config");
    path
}

fn forjar(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("forjar runs")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn combined(out: &Output) -> String {
    format!("{}{}", stdout(out), stderr(out))
}

/// (a) The remote command's streams and its EXIT CODE reach the operator.
#[test]
fn fj446_exec_forwards_streams_and_exit_code() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), &dir.path().join("hello.txt"));
    let out = forjar(
        dir.path(),
        &[
            "exec",
            "local",
            "-f",
            cfg.to_str().unwrap(),
            "--",
            "sh",
            "-c",
            "echo hi; echo err >&2; exit 3",
        ],
    );
    assert!(
        stdout(&out).contains("hi"),
        "stdout must carry the remote stdout verbatim, got: {:?}",
        stdout(&out)
    );
    assert!(
        stderr(&out).contains("err"),
        "stderr must carry the remote stderr verbatim, got: {:?}",
        stderr(&out)
    );
    assert_eq!(
        out.status.code(),
        Some(3),
        "forjar must exit with the REMOTE exit code, not its own taxonomy"
    );
}

/// (b) An unknown machine names the file it looked in and the machines it found.
#[test]
fn fj446_exec_unknown_machine_names_the_inventory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), &dir.path().join("hello.txt"));
    let out = forjar(
        dir.path(),
        &["exec", "nope", "-f", cfg.to_str().unwrap(), "--", "true"],
    );
    assert!(!out.status.success(), "unknown machine must not exit 0");
    let text = combined(&out);
    assert!(
        text.contains("nope"),
        "must name the machine asked for: {text}"
    );
    assert!(
        text.contains("local"),
        "must list the machines that ARE declared: {text}"
    );
}

/// (c) `--json` is a machine-readable record of the same run.
#[test]
fn fj446_exec_json_carries_exit_code_and_stdout() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), &dir.path().join("hello.txt"));
    let out = forjar(
        dir.path(),
        &[
            "exec",
            "local",
            "-f",
            cfg.to_str().unwrap(),
            "--json",
            "--",
            "printf",
            "x",
        ],
    );
    assert!(out.status.success(), "printf x exits 0: {}", combined(&out));
    let v: serde_json::Value =
        serde_json::from_str(stdout(&out).trim()).unwrap_or_else(|e| panic!("JSON: {e}"));
    assert_eq!(v["machine"], "local");
    assert_eq!(v["exit_code"], 0);
    assert_eq!(v["stdout"], "x");
}

/// (d) `facts` measures the host: identity, PATH, and real filesystems.
#[test]
fn fj446_facts_json_reports_identity_path_and_disks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), &dir.path().join("hello.txt"));
    let out = forjar(
        dir.path(),
        &["facts", "local", "-f", cfg.to_str().unwrap(), "--json"],
    );
    assert!(out.status.success(), "facts failed: {}", combined(&out));
    let v: serde_json::Value =
        serde_json::from_str(stdout(&out).trim()).unwrap_or_else(|e| panic!("JSON: {e}"));

    assert!(
        !v["hostname"].as_str().unwrap_or("").is_empty(),
        "hostname must be measured, got {v:?}"
    );
    assert!(
        !v["path"].as_str().unwrap_or("").is_empty(),
        "the remote PATH is the whole point — it must be reported"
    );
    let disks = v["disks"].as_array().expect("disks array");
    assert!(!disks.is_empty(), "at least one real filesystem: {v:?}");
    assert!(
        disks.iter().all(|d| d["mount"].is_string()),
        "every disk carries its mount point: {disks:?}"
    );

    let whoami = Command::new("id").arg("-un").output().expect("id -un");
    let expected = String::from_utf8_lossy(&whoami.stdout).trim().to_string();
    assert_eq!(
        v["user"].as_str().unwrap_or(""),
        expected,
        "facts must report the identity forjar actually connects as"
    );
}

/// (e) A healthy machine passes, and the report names the two things the ticket
/// asks for by name: the PATH and the disks.
#[test]
fn fj446_doctor_machine_reports_path_and_disk() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), &dir.path().join("hello.txt"));
    let out = forjar(
        dir.path(),
        &["doctor", "--machine", "local", "-f", cfg.to_str().unwrap()],
    );
    let text = combined(&out);
    assert!(out.status.success(), "healthy host must exit 0: {text}");
    assert!(text.contains("PATH"), "must report the remote PATH: {text}");
    assert!(text.contains("disk"), "must report disk headroom: {text}");
}

/// (f) A destination the connecting user cannot write is a FAIL that names the
/// directory and its ownership — the question the operator could not answer.
#[test]
fn fj446_doctor_machine_fails_on_unwritable_destination() {
    if Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
    {
        return; // root writes through any mode; the case is not expressible.
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let locked = dir.path().join("locked");
    std::fs::create_dir(&locked).expect("mkdir");
    let cfg = write_config(dir.path(), &locked.join("hello.txt"));
    set_mode(&locked, 0o555);

    let out = forjar(
        dir.path(),
        &["doctor", "--machine", "local", "-f", cfg.to_str().unwrap()],
    );
    let text = combined(&out);
    set_mode(&locked, 0o755); // so the tempdir can be cleaned up

    assert!(
        !out.status.success(),
        "an unwritable destination must fail the doctor: {text}"
    );
    assert!(
        text.contains(locked.to_str().unwrap()),
        "must name the directory: {text}"
    );
    assert!(
        text.contains("mode 555"),
        "must report the mode it found: {text}"
    );
    assert!(
        text.contains("forjar connects as"),
        "must name the identity forjar connects as: {text}"
    );
}

fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
}

/// The control: if `doctor --machine` were a no-op that always exits 0, this
/// would pass silently. An unknown machine must be refused.
#[test]
fn fj446_doctor_machine_is_not_a_no_op() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = write_config(dir.path(), &dir.path().join("hello.txt"));
    let out = forjar(
        dir.path(),
        &["doctor", "--machine", "ghost", "-f", cfg.to_str().unwrap()],
    );
    assert!(
        !out.status.success(),
        "doctor --machine on an undeclared machine must not exit 0: {}",
        combined(&out)
    );
    assert!(
        combined(&out).contains("ghost"),
        "must name the machine asked for: {}",
        combined(&out)
    );
}

/// `doctor --machine` needs a config to know what the machine is.
#[test]
fn fj446_doctor_machine_requires_a_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = forjar(dir.path(), &["doctor", "--machine", "local"]);
    assert!(!out.status.success(), "must refuse without -f");
    assert!(
        combined(&out).contains("-f"),
        "must say which flag is missing: {}",
        combined(&out)
    );
}
