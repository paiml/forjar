//! `check` and `drift` rendered templates without ever resolving the config's
//! `data:` sources, so a healthy, declared source surfaced as the misleading
//! `unknown data source: <key>`. Both must resolve sources first, as `plan`
//! and `apply` do.

use std::fs;
use std::process::Command;

fn run(verb: &str, data_value: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = dir.path().join("forjar.yaml");
    let out_file = dir.path().join("out.txt");
    fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: ds\nmachines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\n\
data:\n\x20 sha: {{ type: command, value: \"{data_value}\" }}\n\
resources:\n\x20 r: {{ type: file, machine: local, path: {}, content: \"v={{{{data.sha}}}}\\n\" }}\n",
            out_file.display()
        ),
    )
    .unwrap();
    let state = dir.path().join("state");
    let apply = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(["apply", "-f"])
        .arg(&cfg)
        .arg("--state-dir")
        .arg(&state)
        .arg("--yes")
        .output()
        .expect("apply starts");
    let _ = apply;
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .arg(verb)
        .arg("-f")
        .arg(&cfg)
        .arg("--state-dir")
        .arg(&state)
        .output()
        .expect("forjar starts");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    s
}

#[test]
fn check_resolves_a_declared_data_source() {
    let out = run("check", "echo abc123");
    assert!(!out.contains("unknown data source"), "{out}");
}

#[test]
fn drift_resolves_a_declared_data_source() {
    let out = run("drift", "echo abc123");
    assert!(!out.contains("unknown data source"), "{out}");
}

/// A source that really fails is reported as itself, not as "unknown".
#[test]
fn check_names_the_real_failure_of_a_broken_source() {
    let out = run("check", "exit 127");
    assert!(out.contains("127"), "{out}");
    assert!(!out.contains("unknown data source"), "{out}");
}
