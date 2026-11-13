//! forjar#1000: a `github_runner` seat names exactly one machine.
//!
//! The seat's verdict is keyed on the runner `name` alone. With `machine: [a,
//! b]`, `a` registers the name, then `b` finds it listed, is reported
//! `adopted` by apply and `ok` by check, and is recorded converged though
//! nothing was installed on `b`. `forjar validate` must refuse that
//! declaration and name the `machine` field, before any token is requested.
//!
//! A one-entry list is accepted (the control), so the rule is "at most one
//! machine", not "no list".
//!
//! Usage: cargo test --test falsification_github_runner_one_machine

use std::path::Path;
use std::process::Command;

/// One `github_runner` seat whose `machine:` line is `machine`, on a manifest
/// that declares the machines `local` and `peer`. Returns the manifest's path.
fn manifest(dir: &Path, machine: &str) -> String {
    let secrets = dir.join("secrets");
    std::fs::create_dir_all(&secrets).unwrap();
    std::fs::write(secrets.join("runner-app-pem"), "example-marker-key").unwrap();
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: '1.0'\n\
             name: github-runner-seat\n\
             secrets:\n  provider: file\n  path: \"{secrets}\"\n\
             machines:\n\
             \x20 local:\n    hostname: localhost\n    addr: localhost\n    transport: local\n\
             \x20 peer:\n    hostname: peer\n    addr: 127.0.0.2\n\
             resources:\n  gate-seat-1:\n\
             \x20   type: github_runner\n\
             \x20   machine: {machine}\n\
             \x20   scope: org\n\
             \x20   owner: example-org\n\
             \x20   name: host-gate-1\n\
             \x20   labels: [self-hosted, linux, x64, gate]\n\
             \x20   group: default\n\
             \x20   version: 2.330.0\n\
             \x20   sha256: {zeros}\n\
             \x20   work_dir: /var/lib/runner/gate-1\n\
             \x20   user: runner\n\
             \x20   app:\n\
             \x20     app_id: 123456\n\
             \x20     installation_id: 7890123\n\
             \x20     private_key: \"{{{{secrets.runner-app-pem}}}}\"\n",
            secrets = secrets.display(),
            zeros = "0".repeat(64),
        ),
    )
    .unwrap();
    cfg.display().to_string()
}

/// `forjar validate -f <cfg>`: combined stdout and stderr, and whether it
/// exited 0.
fn validate(dir: &Path, cfg: &str) -> (String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(dir)
        .args(["validate", "-f", cfg])
        .output()
        .expect("forjar must run");
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.success(),
    )
}

/// Control: a one-entry machine list is a single seat and is accepted.
#[test]
fn one_machine_control_a_one_entry_list_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = manifest(dir.path(), "[local]");
    let (out, ok) = validate(dir.path(), &cfg);
    assert!(ok, "a seat on one machine must pass validate:\n{out}");
}

/// Falsifier: two machines would make the second a false `adopted`/`ok`.
#[test]
fn one_machine_a_seat_on_two_machines_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = manifest(dir.path(), "[local, peer]");
    let (out, ok) = validate(dir.path(), &cfg);
    assert!(
        !ok,
        "validate accepted a github_runner on two machines:\n{out}"
    );
    for needle in ["gate-seat-1", "github_runner", "`machine`"] {
        assert!(
            out.contains(needle),
            "the refusal must name {needle}:\n{out}"
        );
    }
}

/// A declared `sudo:` would be ignored by this type's executor (the seat
/// script runs its own `sudo -n`), so validate refuses it and names `sudo`.
#[test]
fn one_machine_a_declared_sudo_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = manifest(dir.path(), "local\n    sudo: true");
    let (out, ok) = validate(dir.path(), &cfg);
    assert!(!ok, "validate accepted a github_runner with sudo:\n{out}");
    assert!(
        out.contains("`sudo`"),
        "the refusal must name `sudo`:\n{out}"
    );
}
