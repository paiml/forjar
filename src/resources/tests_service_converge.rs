//! #663: a service resource's `apply_script` reaches exactly what its
//! `check_script` asserts, for every `state:`, from every starting condition.
//!
//! The scripts are EXECUTED, not read: a fake `systemctl` placed first on PATH
//! keeps is-active / is-enabled as files in a temp dir and records every call.
//! The assertions are about the fake's resulting state, the check's exit code,
//! and which mutating calls a second apply makes.
//!
//! Semantics (docs/book/src/03-resources.md, "Service States"):
//!   running  — start; enablement from `enabled:` (default true)
//!   stopped  — stop;  enablement from `enabled:` (default true)
//!   enabled  — `systemctl enable`, activity left as found
//!   disabled — `systemctl disable`, activity left as found

use super::service::{apply_script, check_script};
use crate::core::types::{MachineTarget, Resource, ResourceType};
use std::path::Path;
use std::process::Command;

const FAKE_SYSTEMCTL: &str = r#"#!/bin/sh
d="$FAKE_SYSTEMD_DIR"
echo "$*" >> "$d/calls"
verb="$1"
shift
quiet=0
for a in "$@"; do [ "$a" = "--quiet" ] && quiet=1; done
case "$verb" in
  is-active)
    if [ -f "$d/active" ]; then [ $quiet = 1 ] || echo active; exit 0; fi
    [ $quiet = 1 ] || echo inactive; exit 3 ;;
  is-enabled)
    if [ -f "$d/enabled" ]; then [ $quiet = 1 ] || echo enabled; exit 0; fi
    [ $quiet = 1 ] || echo disabled; exit 1 ;;
  start|restart|reload-or-restart) : > "$d/active" ;;
  stop) rm -f "$d/active" ;;
  enable) : > "$d/enabled" ;;
  disable) rm -f "$d/enabled" ;;
  *) echo "fake systemctl: unsupported verb $verb" >&2; exit 64 ;;
esac
"#;

const MUTATING: &[&str] = &[
    "start",
    "stop",
    "restart",
    "reload-or-restart",
    "enable",
    "disable",
];

/// (active, enabled) — the four starting conditions.
const STARTS: [(bool, bool); 4] = [(true, true), (true, false), (false, true), (false, false)];

struct FakeHost {
    dir: tempfile::TempDir,
}

impl FakeHost {
    fn new(active: bool, enabled: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        let sc = bin.join("systemctl");
        std::fs::write(&sc, FAKE_SYSTEMCTL).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&sc, std::fs::Permissions::from_mode(0o755)).unwrap();
        let host = Self { dir };
        host.set(active, enabled);
        host
    }

    fn set(&self, active: bool, enabled: bool) {
        for (flag, on) in [("active", active), ("enabled", enabled)] {
            let p = self.dir.path().join(flag);
            if on {
                std::fs::write(&p, "").unwrap();
            } else {
                let _ = std::fs::remove_file(&p);
            }
        }
    }

    fn state(&self) -> (bool, bool) {
        let p = self.dir.path();
        (p.join("active").exists(), p.join("enabled").exists())
    }

    /// Run `script` under bash with the fake first on PATH. Returns the exit
    /// code and the mutating systemctl calls the run made.
    fn run(&self, script: &str) -> (i32, Vec<String>) {
        let calls = self.dir.path().join("calls");
        let _ = std::fs::remove_file(&calls);
        let path = format!(
            "{}:{}",
            self.dir.path().join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let out = Command::new("bash")
            .arg("-c")
            .arg(script)
            .env("PATH", path)
            .env("FAKE_SYSTEMD_DIR", self.dir.path())
            .output()
            .unwrap();
        (out.status.code().unwrap_or(-1), mutating_calls(&calls))
    }
}

fn mutating_calls(calls: &Path) -> Vec<String> {
    std::fs::read_to_string(calls)
        .unwrap_or_default()
        .lines()
        .filter(|l| MUTATING.contains(&l.split_whitespace().next().unwrap_or("")))
        .map(str::to_string)
        .collect()
}

fn svc(state: &str, enabled: Option<bool>) -> Resource {
    Resource {
        resource_type: ResourceType::Service,
        machine: MachineTarget::Single("m1".to_string()),
        name: Some("demo".to_string()),
        state: Some(state.to_string()),
        enabled,
        ..Default::default()
    }
}

/// What the unit must look like after apply: (active, enabled), where an
/// activity of `None` means "as it was before apply".
fn expected(state: &str, enabled: Option<bool>) -> (Option<bool>, bool) {
    match state {
        "running" => (Some(true), enabled.unwrap_or(true)),
        "stopped" => (Some(false), enabled.unwrap_or(true)),
        "enabled" => (None, true),
        "disabled" => (None, false),
        other => panic!("no expectation for state {other}"),
    }
}

/// Every divergence of one declaration across the four starting conditions.
fn converge_failures(state: &str, enabled: Option<bool>) -> Vec<String> {
    let r = svc(state, enabled);
    let (apply, check) = (apply_script(&r), check_script(&r));
    let (want_active, want_enabled) = expected(state, enabled);
    let mut failures = Vec::new();
    for (a0, e0) in STARTS {
        let host = FakeHost::new(a0, e0);
        let tag = format!("state={state} enabled={enabled:?} start=(active={a0}, enabled={e0})");
        let want = (want_active.unwrap_or(a0), want_enabled);
        let (pre_check, _) = host.run(&check);
        let (code, first_calls) = host.run(&apply);
        if code != 0 {
            failures.push(format!("{tag}: apply exited {code}"));
        }
        if host.state() != want {
            failures.push(format!(
                "{tag}: apply left {:?}, want {want:?}",
                host.state()
            ));
        }
        if pre_check == 0 && !first_calls.is_empty() {
            failures.push(format!(
                "{tag}: check passed before apply, yet apply ran {first_calls:?}"
            ));
        }
        if (a0, e0) != want && pre_check == 0 {
            failures.push(format!(
                "{tag}: check passed on a host that is not converged"
            ));
        }
        let (post_check, _) = host.run(&check);
        if post_check != 0 {
            failures.push(format!("{tag}: check exited {post_check} after apply"));
        }
        let (code2, second_calls) = host.run(&apply);
        if post_check == 0 && (code2 != 0 || !second_calls.is_empty()) {
            failures.push(format!(
                "{tag}: second apply exited {code2}, ran {second_calls:?}"
            ));
        }
    }
    failures
}

fn assert_converges(state: &str, variants: &[Option<bool>]) {
    let f: Vec<String> = variants
        .iter()
        .flat_map(|e| converge_failures(state, *e))
        .collect();
    assert!(f.is_empty(), "{} divergence(s):\n{}", f.len(), f.join("\n"));
}

#[test]
fn test_663_running_converges() {
    assert_converges("running", &[None, Some(false)]);
}

#[test]
fn test_663_stopped_converges() {
    assert_converges("stopped", &[None, Some(false)]);
}

#[test]
fn test_663_enabled_converges() {
    assert_converges("enabled", &[None, Some(true)]);
}

#[test]
fn test_663_disabled_converges() {
    assert_converges("disabled", &[None, Some(false)]);
}

/// The fake answers what the scripts ask; a typo in a verb would otherwise
/// read as "inactive" and every row could pass for the wrong reason.
#[test]
fn test_663_fake_systemctl_discriminates() {
    let host = FakeHost::new(false, false);
    assert_eq!(host.run("systemctl is-active --quiet demo").0, 3);
    assert_eq!(host.run("systemctl is-enabled --quiet demo").0, 1);
    let (code, calls) = host.run("systemctl start demo && systemctl enable demo");
    assert_eq!((code, calls.len()), (0, 2));
    assert_eq!(host.state(), (true, true));
    assert_eq!(host.run("systemctl frobnicate demo").0, 64);
}

fn validate(state: &str, enabled: Option<&str>) -> Vec<String> {
    let enabled_line = enabled
        .map(|e| format!("    enabled: {e}\n"))
        .unwrap_or_default();
    let yaml = format!(
        "version: \"1.0\"\nname: t\nmachines:\n  m1:\n    hostname: box\n    addr: 1.2.3.4\n\
         resources:\n  svc:\n    type: service\n    machine: m1\n    name: demo\n    \
         state: {state}\n{enabled_line}"
    );
    let config = crate::core::parser::parse_config(&yaml).unwrap();
    crate::core::parser::validate_config(&config)
        .into_iter()
        .map(|e| e.message)
        .collect()
}

#[test]
fn test_663_contradictory_enabled_is_a_validation_error() {
    for (state, enabled) in [("enabled", "false"), ("disabled", "true")] {
        let errors = validate(state, Some(enabled));
        assert!(
            errors
                .iter()
                .any(|e| e.contains("'svc'") && e.contains("contradicts")),
            "state: {state} with enabled: {enabled} must be refused, got {errors:?}"
        );
    }
}

#[test]
fn test_663_agreeing_or_absent_enabled_validates() {
    for (state, enabled) in [
        ("enabled", Some("true")),
        ("enabled", None),
        ("disabled", Some("false")),
        ("disabled", None),
        ("running", Some("false")),
        ("stopped", Some("true")),
    ] {
        let errors = validate(state, enabled);
        assert!(
            !errors.iter().any(|e| e.contains("contradicts")),
            "state: {state} enabled: {enabled:?} must validate, got {errors:?}"
        );
    }
}
