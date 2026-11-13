//! forjar#1010: `doctor --machine` dir probe must read owner:group:mode on BSD
//! stat (macOS), where `stat -c` does not exist, not report `?:?:?`.

use super::doctor_machine::{dir_probe_script, parse_dir_stat};
use std::os::unix::fs::PermissionsExt;

/// Run the probe with a BSD-style `stat` first on PATH: it rejects `-c` and
/// answers `-f` the way macOS does.
fn run_with_bsd_stat(dir: &str) -> String {
    let bin = tempfile::tempdir().unwrap();
    let stub = bin.path().join("stat");
    std::fs::write(
        &stub,
        "#!/bin/sh\ncase \"$1\" in\n  -c) echo 'stat: illegal option -- c' >&2; exit 1;;\n  -f) echo 'alice:staff:755'; exit 0;;\nesac\nexit 1\n",
    )
    .unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap()
    );
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(dir_probe_script(&[dir.to_string()]))
        .env("PATH", path)
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn dir_probe_reads_ownership_from_bsd_stat() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run_with_bsd_stat(tmp.path().to_str().unwrap());
    let st = out.lines().find_map(parse_dir_stat).expect("dirstat line");
    assert_eq!(
        (st.owner.as_str(), st.group.as_str(), st.mode.as_str()),
        ("alice", "staff", "755"),
        "{out}"
    );
}

#[test]
fn dir_probe_still_reads_gnu_stat() {
    let tmp = tempfile::tempdir().unwrap();
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(dir_probe_script(&[tmp
            .path()
            .to_str()
            .unwrap()
            .to_string()]))
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let st = text.lines().find_map(parse_dir_stat).expect("dirstat line");
    assert_ne!(st.owner, "?", "{text}");
}
