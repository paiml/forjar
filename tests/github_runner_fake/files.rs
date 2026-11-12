//! Builders for the rig: a throwaway RSA key, a fake runner tarball, and a
//! `bash` shim. Split from `mod.rs` to keep each file under the repo limit.

use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Command;

pub(super) fn generate_key(root: &Path) -> String {
    let path = root.join("app-key.pem");
    let out = Command::new("openssl")
        .args(["genrsa", "-traditional", "-out"])
        .arg(&path)
        .arg("2048")
        .output()
        .expect("these falsifiers need `openssl` on PATH to make a throwaway test key");
    assert!(out.status.success(), "openssl genrsa failed");
    std::fs::read_to_string(&path).unwrap()
}

/// A runner tarball whose `config.sh` records its argv and the token env var.
pub(super) fn make_tarball(root: &Path) -> String {
    let src = root.join("runner-src");
    let log = root.join("seatlog");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&log).unwrap();
    let config = format!(
        "#!/bin/sh\necho \"$*\" >> {log}/config-calls\nprintf '%s' \"$ACTIONS_RUNNER_INPUT_TOKEN\" > {log}/token-seen\n",
        log = log.display()
    );
    let svc = format!(
        "#!/bin/sh\necho \"$*\" >> {log}/svc-calls\n",
        log = log.display()
    );
    for (name, body) in [("config.sh", config), ("svc.sh", svc)] {
        let f = src.join(name);
        std::fs::write(&f, body).unwrap();
        Command::new("chmod").arg("755").arg(&f).status().unwrap();
    }
    let dl = root.join("dl/v2.330.0");
    std::fs::create_dir_all(&dl).unwrap();
    let tarball = dl.join("actions-runner-linux-x64-2.330.0.tar.gz");
    let st = Command::new("tar")
        .arg("czf")
        .arg(&tarball)
        .arg("-C")
        .arg(&src)
        .arg(".")
        .status()
        .unwrap();
    assert!(st.success());
    Sha256::digest(std::fs::read(&tarball).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A `bash` earlier on PATH than the real one: it files the script it was handed
/// on stdin (so the test can read what the seat received) and then runs it.
pub(super) fn make_shim(root: &Path) {
    let shim = root.join("shim");
    std::fs::create_dir_all(&shim).unwrap();
    let real = std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .map(|d| Path::new(d).join("bash"))
        .find(|p| p.is_file())
        .expect("bash on PATH");
    let log = root.join("seatlog");
    let script = format!(
        "#!/bin/sh\n[ \"$#\" -gt 0 ] && exec {real} \"$@\"\nf=\"{log}/script.$(date +%s%N)\"\ncat > \"$f\"\nexec {real} \"$@\" < \"$f\"\n",
        log = log.display(),
        real = real.display()
    );
    let f = shim.join("bash");
    std::fs::write(&f, script).unwrap();
    Command::new("chmod").arg("755").arg(&f).status().unwrap();
}
