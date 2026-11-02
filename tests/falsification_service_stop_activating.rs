//! PMAT-708 (paiml/forjar#708): `state: stopped` must stop a unit that is
//! `activating`, and its check must not call that unit stopped.
//!
//! MEASURED, yoga, 2026-10-14 (paiml/infra#1654). `github-runner-build2` is an
//! ephemeral runner with `Restart=always`, `RestartSec=10`. Between jobs it
//! sits in `activating` (SubState `auto-restart`). The apply script was
//! `if systemctl is-active --quiet U; then systemctl stop U; fi`, and
//! `is-active` is false for `activating`. So the stop was skipped, only the
//! disable ran, the resource was recorded converged, and the unit was active
//! again ten seconds later and took the next job. `--refresh` then reported
//! `unchanged`, because the check asked the same `! is-active` question.
//!
//! These tests EXECUTE the emitted shell against a fake host whose
//! `systemctl` keeps a unit's `ActiveState` in a file and answers `is-active`
//! with systemd's real exit codes (0 only for `active` and `reloading`). The
//! check is judged on its exit code, as `cli::check` judges it.

use forjar::core::types::{MachineTarget, Resource, ResourceType};
use forjar::resources::service::{apply_script, check_script};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

const UNIT: &str = "github-runner-build2";

/// Every `ActiveState` systemd reports for a loaded unit.
const NOT_STOPPED: [&str; 4] = ["activating", "active", "reloading", "deactivating"];
const STOPPED: [&str; 2] = ["inactive", "failed"];

/// A fake host. State lives in files so an apply's effect is visible to the
/// next script run, which is what makes a second apply a real test.
///
/// - `is-active`: prints the state unless `--quiet`; exits 0 only for
///   `active` and `reloading`, as systemd does.
/// - `show -p ActiveState --value`: prints the state.
/// - `stop`: sets `inactive`. `FJ_STOP_STICKS=1` models a unit something
///   re-activates (a socket, a timer); `FJ_NOT_LOADED=1` models a unit with
///   no unit file, where systemd's `stop` exits 5.
/// - every call is appended to `log`.
struct FakeHost {
    dir: tempfile::TempDir,
    bin: PathBuf,
}

impl FakeHost {
    fn new(active_state: &str, enabled: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let systemctl = r#"#!/bin/sh
echo "$*" >> "$FJ_DIR/log"
st=$(cat "$FJ_DIR/state")
case "$1" in
  is-active)
    [ "$2" = "--quiet" ] || echo "$st"
    case "$st" in active|reloading) exit 0 ;; *) exit 3 ;; esac ;;
  show)
    case "$*" in *ActiveState*) echo "$st" ;; *) echo "" ;; esac ;;
  stop)
    if [ "${FJ_NOT_LOADED:-}" = 1 ]; then echo "Failed to stop: Unit not loaded." >&2; exit 5; fi
    [ "${FJ_STOP_STICKS:-}" = 1 ] || echo inactive > "$FJ_DIR/state" ;;
  start) echo active > "$FJ_DIR/state" ;;
  is-enabled) [ -f "$FJ_DIR/enabled" ] ;;
  enable) : > "$FJ_DIR/enabled" ;;
  disable) rm -f "$FJ_DIR/enabled" ;;
  *) exit 0 ;;
esac
"#;
        let p = bin.join("systemctl");
        fs::write(&p, systemctl).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(dir.path().join("state"), format!("{active_state}\n")).unwrap();
        fs::write(dir.path().join("log"), "").unwrap();
        if enabled {
            fs::write(dir.path().join("enabled"), "").unwrap();
        }
        Self { dir, bin }
    }

    fn run(&self, script: &str, env: &[(&str, &str)]) -> Output {
        let mut c = Command::new("bash");
        c.arg("-c")
            .arg(script)
            .env("PATH", format!("{}:/usr/bin:/bin", self.bin.display()))
            .env("FJ_DIR", self.dir.path());
        for (k, v) in env {
            c.env(k, v);
        }
        c.output().expect("bash must run")
    }

    fn state(&self) -> String {
        fs::read_to_string(self.dir.path().join("state"))
            .unwrap()
            .trim()
            .to_string()
    }

    fn log(&self) -> String {
        fs::read_to_string(self.dir.path().join("log")).unwrap()
    }

    fn clear_log(&self) {
        fs::write(self.dir.path().join("log"), "").unwrap();
    }
}

fn stopped_disabled() -> Resource {
    Resource {
        resource_type: ResourceType::Service,
        machine: MachineTarget::Single("yoga".into()),
        name: Some(UNIT.into()),
        state: Some("stopped".into()),
        enabled: Some(false),
        ..Default::default()
    }
}

fn show(o: &Output) -> String {
    format!(
        "exit={:?}\nstdout:\n{}\nstderr:\n{}",
        o.status.code(),
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

#[test]
fn a_restart_always_unit_caught_mid_restart_ends_inactive() {
    // THE REGRESSION: activating + enabled, exactly yoga's build2 at 10:52Z.
    let host = FakeHost::new("activating", true);
    let r = stopped_disabled();
    let out = host.run(&apply_script(&r), &[]);
    assert_eq!(out.status.code(), Some(0), "{}", show(&out));
    assert_eq!(
        host.state(),
        "inactive",
        "apply left an auto-restarting unit {} — it disabled it and recorded \
         converged while the unit was still coming back.\nlog:\n{}\nscript:\n{}",
        host.state(),
        host.log(),
        apply_script(&r)
    );
    assert!(
        !host.dir.path().join("enabled").exists(),
        "disable must still run"
    );
}

#[test]
fn every_not_stopped_active_state_is_stopped_by_apply() {
    for st in NOT_STOPPED {
        let host = FakeHost::new(st, false);
        let out = host.run(&apply_script(&stopped_disabled()), &[]);
        assert_eq!(out.status.code(), Some(0), "{st}: {}", show(&out));
        assert_eq!(host.state(), "inactive", "{st}: log:\n{}", host.log());
    }
}

#[test]
fn the_check_does_not_call_a_not_stopped_unit_stopped() {
    let r = stopped_disabled();
    for st in NOT_STOPPED {
        let host = FakeHost::new(st, false);
        let out = host.run(&check_script(&r), &[]);
        assert_ne!(
            out.status.code(),
            Some(0),
            "check passed `state: stopped` over ActiveState={st}.\n{}\nscript:\n{}",
            show(&out),
            check_script(&r)
        );
    }
}

#[test]
fn the_check_passes_inactive_and_failed() {
    // The other direction: a check that fails everything detects nothing.
    let r = stopped_disabled();
    for st in STOPPED {
        let host = FakeHost::new(st, false);
        let out = host.run(&check_script(&r), &[]);
        assert_eq!(out.status.code(), Some(0), "{st}: {}", show(&out));
    }
}

#[test]
fn a_second_apply_is_a_no_op_success_and_the_check_then_passes() {
    // forjar's dependency propagation re-runs an applied resource, so the
    // script must survive a second run on the state its first run produced.
    let host = FakeHost::new("activating", true);
    let r = stopped_disabled();
    let first = host.run(&apply_script(&r), &[]);
    assert_eq!(first.status.code(), Some(0), "{}", show(&first));
    host.clear_log();
    let second = host.run(&apply_script(&r), &[]);
    assert_eq!(second.status.code(), Some(0), "{}", show(&second));
    assert!(
        !host.log().lines().any(|l| l.starts_with("stop ")),
        "a second apply on an inactive unit must not stop it again:\n{}",
        host.log()
    );
    let check = host.run(&check_script(&r), &[]);
    assert_eq!(check.status.code(), Some(0), "{}", show(&check));
}

#[test]
fn an_unloaded_unit_is_already_stopped() {
    // systemd reports ActiveState=inactive for a unit with no unit file, and
    // `systemctl stop` on it exits 5. Declaring such a unit stopped was a
    // converged no-op before this fix and must stay one.
    let host = FakeHost::new("inactive", false);
    let out = host.run(
        &apply_script(&stopped_disabled()),
        &[("FJ_NOT_LOADED", "1")],
    );
    assert_eq!(out.status.code(), Some(0), "{}", show(&out));
}

#[test]
fn a_stop_that_does_not_take_fails_the_apply() {
    // A unit something re-activates is not stopped. Recording it converged
    // is the defect this file exists for, so the apply must fail.
    let host = FakeHost::new("activating", false);
    let out = host.run(
        &apply_script(&stopped_disabled()),
        &[("FJ_STOP_STICKS", "1")],
    );
    assert_ne!(out.status.code(), Some(0), "{}", show(&out));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("activating"),
        "the failure must name the state the unit is in:\n{}",
        show(&out)
    );
}

#[test]
fn the_rendered_scripts_survive_forjars_own_bashrs_gate() {
    let r = stopped_disabled();
    for script in [apply_script(&r), check_script(&r)] {
        forjar::core::purifier::validate_script(&script)
            .unwrap_or_else(|e| panic!("bashrs refused a generated service script: {e}\n{script}"));
    }
}
