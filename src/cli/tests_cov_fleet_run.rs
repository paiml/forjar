//! Tests: Coverage for cmd_rolling / cmd_canary / cmd_apply_canary_machine
//! happy paths via localhost transport (PMAT-088).

use super::apply_variants::cmd_apply_canary_machine;
use super::fleet_ops::{cmd_canary, cmd_rolling};
use std::path::{Path, PathBuf};

/// Two localhost machines, one file resource each (paths inside `dir`).
fn write_two_machine_cfg(dir: &Path) -> PathBuf {
    let yaml = format!(
        "version: \"1.0\"\nname: fleet\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\n  m2:\n    hostname: m2\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m1\n    path: {a}\n    content: aa\n  b:\n    type: file\n    machine: m2\n    path: {b}\n    content: bb\n",
        a = dir.join("a.txt").display(),
        b = dir.join("b.txt").display()
    );
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

/// Single localhost machine with one file resource.
fn write_one_machine_cfg(dir: &Path) -> PathBuf {
    let yaml = format!(
        "version: \"1.0\"\nname: solo\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m1\n    path: {a}\n    content: aa\n",
        a = dir.join("solo.txt").display()
    );
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

/// Canary machine resource fails (unwritable path); m2 resource is fine.
fn write_failing_canary_cfg(dir: &Path) -> PathBuf {
    let yaml = format!(
        "version: \"1.0\"\nname: failfleet\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\n  m2:\n    hostname: m2\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m1\n    path: /dev/null/forjar-cov/a.txt\n    content: aa\n  b:\n    type: file\n    machine: m2\n    path: {b}\n    content: bb\n",
        b = dir.join("b.txt").display()
    );
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

fn state_dir(dir: &Path) -> PathBuf {
    let sd = dir.join("state");
    std::fs::create_dir_all(&sd).unwrap();
    sd
}

/// Run a fleet op on a 16 MiB stack — cmd_apply's frames exceed the default
/// test-thread stack (same pattern as tests_cov_dispatch_5).
fn on_big_stack<F>(f: F) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String> + Send + 'static,
{
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(f)
        .expect("spawn fleet thread")
        .join()
        .expect("join fleet thread")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── cmd_rolling happy + failure paths ──

    #[test]
    fn rolling_two_machines_batch_one_ok() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_two_machine_cfg(dir.path());
        let sd = state_dir(dir.path());
        let r = on_big_stack(move || cmd_rolling(&cfg, &sd, 1, &[], None));
        assert!(r.is_ok(), "rolling deploy should converge: {r:?}");
        assert!(dir.path().join("a.txt").exists());
        assert!(dir.path().join("b.txt").exists());
    }

    #[test]
    fn rolling_single_batch_ok() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_two_machine_cfg(dir.path());
        let sd = state_dir(dir.path());
        // Batch larger than fleet → one batch covering all machines.
        let r = on_big_stack(move || cmd_rolling(&cfg, &sd, 10, &[], Some(60)));
        assert!(r.is_ok(), "single-batch rolling: {r:?}");
    }

    #[test]
    fn rolling_failing_resource_err() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_failing_canary_cfg(dir.path());
        let sd = state_dir(dir.path());
        let r = on_big_stack(move || cmd_rolling(&cfg, &sd, 1, &[], None));
        assert!(r.is_err(), "rolling must stop on machine failure");
    }

    // ── cmd_canary happy + failure paths ──

    #[test]
    fn canary_single_machine_ok() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_one_machine_cfg(dir.path());
        let sd = state_dir(dir.path());
        // No remaining machines → early "canary deploy complete" path.
        let r = on_big_stack(move || cmd_canary(&cfg, &sd, "m1", false, &[], None));
        assert!(r.is_ok(), "single-machine canary: {r:?}");
        assert!(dir.path().join("solo.txt").exists());
    }

    #[test]
    fn canary_two_machines_manual_proceed_ok() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_two_machine_cfg(dir.path());
        let sd = state_dir(dir.path());
        let r = on_big_stack(move || cmd_canary(&cfg, &sd, "m1", false, &[], None));
        assert!(r.is_ok(), "canary + fleet phase: {r:?}");
        assert!(dir.path().join("b.txt").exists(), "fleet phase ran");
    }

    #[test]
    fn canary_two_machines_auto_proceed_ok() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_two_machine_cfg(dir.path());
        let sd = state_dir(dir.path());
        let r = on_big_stack(move || cmd_canary(&cfg, &sd, "m2", true, &[], Some(60)));
        assert!(r.is_ok(), "auto-proceed canary: {r:?}");
    }

    #[test]
    fn canary_failing_canary_phase_err() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_failing_canary_cfg(dir.path());
        let sd = state_dir(dir.path());
        let r = on_big_stack(move || cmd_canary(&cfg, &sd, "m1", true, &[], None));
        assert!(r.is_err(), "failed canary must abort the fleet");
        // Fleet phase never ran — m2's file should not exist.
        assert!(!dir.path().join("b.txt").exists());
    }

    // ── cmd_apply_canary_machine happy + failure paths ──

    #[test]
    fn apply_canary_machine_single_ok() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_one_machine_cfg(dir.path());
        let sd = state_dir(dir.path());
        let r = on_big_stack(move || cmd_apply_canary_machine(&cfg, &sd, "m1", &[], None));
        assert!(r.is_ok(), "single-machine canary apply: {r:?}");
    }

    #[test]
    fn apply_canary_machine_two_machines_ok() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_two_machine_cfg(dir.path());
        let sd = state_dir(dir.path());
        let r = on_big_stack(move || cmd_apply_canary_machine(&cfg, &sd, "m1", &[], Some(60)));
        assert!(r.is_ok(), "canary + fleet apply: {r:?}");
        assert!(dir.path().join("a.txt").exists());
        assert!(dir.path().join("b.txt").exists());
    }

    #[test]
    fn apply_canary_machine_failing_err() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_failing_canary_cfg(dir.path());
        let sd = state_dir(dir.path());
        let r = on_big_stack(move || cmd_apply_canary_machine(&cfg, &sd, "m1", &[], None));
        assert!(r.is_err(), "failed canary apply must return Err");
    }
}
