//! FJ-LOCK-001 C3/C4/C5 (#923, #924, #925), through the real `forjar apply`
//! dispatch: one hung machine does not block another, and the new flags parse.

use super::commands::*;
use super::dispatch_apply_b::dispatch_apply_cmd;
use std::path::{Path, PathBuf};

#[derive(clap::Parser)]
#[command(name = "forjar")]
struct TestCli {
    #[command(subcommand)]
    cmd: Commands,
}

fn parse(extra: &[&str]) -> Commands {
    let mut argv = vec!["forjar", "apply"];
    argv.extend_from_slice(extra);
    <TestCli as clap::Parser>::try_parse_from(argv)
        .expect("clap parse")
        .cmd
}

fn run_apply(extra: &[&str]) -> Result<(), String> {
    let argv: Vec<String> = extra.iter().map(|s| s.to_string()).collect();
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
            dispatch_apply_cmd(parse(&argv), false)
        })
        .expect("spawn")
        .join()
        .expect("join")
}

/// Two localhost machines, each with one file resource under `dir`.
fn two_machine_config(dir: &Path) -> PathBuf {
    let yaml = format!(
        "version: \"1.0\"\nname: two\nmachines:\n  intel:\n    hostname: intel\n    addr: 127.0.0.1\n  lambda:\n    hostname: lambda\n    addr: 127.0.0.1\nresources:\n  fi:\n    type: file\n    machine: intel\n    path: {i}\n    content: i\n  fl:\n    type: file\n    machine: lambda\n    path: {l}\n    content: l\n",
        i = dir.join("intel.txt").display(),
        l = dir.join("lambda.txt").display()
    );
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

/// A lock a live process other than us (pid 1) holds on `machine`.
fn hang_machine(state_dir: &Path, machine: &str, id: &str) -> PathBuf {
    let lock = state_dir.join(machine).join(".forjar.lock");
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    let info = crate::core::state::LockInfo {
        id: id.into(),
        pid: 1,
        starttime: String::new(),
        command: "forjar apply -m intel".into(),
        ..crate::core::state::LockInfo::current()
    };
    std::fs::write(&lock, info.render()).unwrap();
    lock
}

fn s(p: &Path) -> String {
    p.display().to_string()
}

/// The #918 incident, end to end: a hung apply on intel (lock held by a live
/// PID) must not stop an apply that only declares lambda; an apply that
/// declares intel is still refused, and the refusal names the holder.
#[test]
fn falsify_c4_hung_intel_apply_does_not_block_lambda() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = two_machine_config(dir.path());
    let sd = dir.path().join("state");
    let lock = hang_machine(&sd, "intel", "hung-intel-id");

    let ok = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "-m", "lambda", "--yes"]);
    assert!(ok.is_ok(), "lambda blocked by a hung intel apply: {ok:?}");
    assert!(dir.path().join("lambda.txt").exists(), "lambda not converged");
    assert!(!dir.path().join("intel.txt").exists(), "intel was touched");

    let refused = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "-m", "intel", "--yes"])
        .unwrap_err();
    assert!(refused.contains("locked by PID 1"), "{refused}");
    assert!(refused.contains("id=hung-intel-id"), "LockInfo missing: {refused}");
    assert!(lock.exists(), "the refused apply removed the live lock");
    assert!(!dir.path().join("intel.txt").exists());
}

/// `--force-unlock <ID>` on a live local holder is refused through the CLI.
#[test]
fn falsify_c3_cli_force_unlock_id_refuses_live_holder() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = two_machine_config(dir.path());
    let sd = dir.path().join("state");
    let lock = hang_machine(&sd, "intel", "live-id");
    let err = run_apply(&[
        "-f", &s(&cfg), "--state-dir", &s(&sd), "-m", "intel", "--yes", "--force-unlock", "live-id",
    ])
    .unwrap_err();
    assert!(err.contains("holder is alive"), "{err}");
    assert!(lock.exists());
    // The bare flag is refused for a live holder too.
    let bare = run_apply(&[
        "-f", &s(&cfg), "--state-dir", &s(&sd), "-m", "intel", "--yes", "--force-unlock",
    ])
    .unwrap_err();
    assert!(bare.contains("holder is alive"), "{bare}");
    assert!(lock.exists());
}

#[test]
fn c3_c5_flags_parse() {
    let got = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let pick = |extra: &[&str]| match parse(extra) {
                Commands::Apply(a) => (a.lock.force_unlock, a.lock.lock_timeout),
                _ => panic!("not apply"),
            };
            [
                pick(&["-f", "x.yaml", "--force-unlock", "abc", "--lock-timeout", "5s"]),
                pick(&["-f", "x.yaml", "--force-unlock"]),
                pick(&["-f", "x.yaml"]),
            ]
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(got[0], (Some("abc".to_string()), Some("5s".to_string())));
    assert_eq!(got[1], (Some(String::new()), None), "bare flag");
    assert_eq!(got[2], (None, None));
}

#[test]
fn c5_invalid_lock_timeout_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = two_machine_config(dir.path());
    let sd = dir.path().join("state");
    let err = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--yes", "--lock-timeout", "soon"])
        .unwrap_err();
    assert!(err.contains("invalid --lock-timeout"), "{err}");
}
