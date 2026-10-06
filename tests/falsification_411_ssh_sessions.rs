//! forjar#411 (CRUX audit E08), slice 1: the instrument.
//!
//! The issue's success criterion is a count — SSH sessions for a converged
//! apply, at most one per resource plus one per machine — and nothing in the
//! repo measured it. This harness does: a fake `ssh` first on PATH records
//! every invocation by kind and runs the session's script with local `bash`,
//! so a machine at a non-local address goes through the real SSH transport
//! end to end, and the count comes from the process table rather than from
//! reading the code.
//!
//! The ceiling below is the MEASURED baseline, not the target. It is a ratchet:
//! the fix lowers it, and anything that adds a session per resource fails here.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Resources in the fixture. Large enough that per-resource cost dominates the
/// per-machine cost, small enough to run in a unit-test budget.
const RESOURCES: usize = 6;

const FAKE_SSH: &str = r#"#!/bin/bash
# forjar#411 fake ssh: log the invocation's kind, then behave like a reachable host.
log="$FORJAR_411_SSH_LOG"
case " $* " in
  *" -O "*) echo control >> "$log"; exit 255 ;;
  *" -N "*) echo master >> "$log"; exit 0 ;;
esac
last="${!#}"
if [ "$last" = bash ]; then
  echo session >> "$log"
  exec bash
fi
echo "other $*" >> "$log"
exit 255
"#;

struct Sandbox {
    dir: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let s = Self {
            dir: tempfile::tempdir().expect("tempdir"),
        };
        let bin = s.path("bin");
        fs::create_dir_all(&bin).unwrap();
        let ssh = bin.join("ssh");
        fs::write(&ssh, FAKE_SSH).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&ssh, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let mut yaml = String::from(
            "version: \"1.0\"\nname: e08\nmachines:\n  web:\n    hostname: web\n    addr: forjar-411.invalid\n    user: root\nresources:\n",
        );
        for i in 0..RESOURCES {
            yaml.push_str(&format!(
                "  f{i}:\n    type: file\n    machine: web\n    path: {}\n    content: \"declared {i}\\n\"\n    mode: \"0644\"\n",
                s.target(i).display()
            ));
        }
        fs::write(s.config(), yaml).unwrap();
        s
    }
    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }
    fn config(&self) -> PathBuf {
        self.path("forjar.yaml")
    }
    fn target(&self, i: usize) -> PathBuf {
        self.path(&format!("target-{i}.txt"))
    }
    fn log(&self) -> PathBuf {
        self.path("ssh.log")
    }
    /// Run forjar with the fake ssh first on PATH; return the invocation log
    /// this run produced.
    fn run(&self, verb: &str, extra: &[&str]) -> (i32, String, Vec<String>) {
        let _ = fs::remove_file(self.log());
        let path = format!(
            "{}:{}",
            self.path("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
            .arg(verb)
            .args(["-f", self.config().to_str().unwrap()])
            .args(["--state-dir", self.path("state").to_str().unwrap()])
            .args(extra)
            .env("PATH", path)
            .env("FORJAR_411_SSH_LOG", self.log())
            .output()
            .expect("forjar failed to start");
        let log = fs::read_to_string(self.log()).unwrap_or_default();
        (
            out.status.code().unwrap_or(-1),
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
            log.lines().map(str::to_owned).collect(),
        )
    }
}

fn count(log: &[String], kind: &str) -> usize {
    log.iter().filter(|l| l.as_str() == kind).count()
}

fn converged() -> Sandbox {
    let s = Sandbox::new();
    let (code, out, log) = s.run("apply", &["--yes"]);
    assert_eq!(code, 0, "first apply failed: {out}");
    // The files exist only because the fake ssh ran the scripts: proof the
    // remote transport, not the local one, carried this apply.
    for i in 0..RESOURCES {
        assert_eq!(
            fs::read_to_string(s.target(i)).unwrap_or_default(),
            format!("declared {i}\n"),
            "{out}"
        );
    }
    assert!(count(&log, "session") >= RESOURCES, "{log:?}\n{out}");
    s
}

#[test]
fn falsify_411_harness_sees_every_ssh_session() {
    // Discrimination: the instrument must be able to read a nonzero count, and
    // must classify every invocation it sees.
    let s = converged();
    let (code, out, log) = s.run("apply", &["--yes"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        log.iter()
            .all(|l| matches!(l.as_str(), "session" | "master" | "control")),
        "unclassified ssh invocation: {log:?}"
    );
    assert!(
        count(&log, "session") > 0,
        "converged apply ran no ssh: {out}"
    );
}

/// Sessions a converged apply of the fixture opens, MEASURED on main at
/// 61d5a703: 12 for 6 resources and 6 for 3, so two per converged resource and
/// none per machine (the issue estimated three). The target (forjar#411) is
/// RESOURCES + 1. Master opens are not sessions and are reported, not capped:
/// the fake opens no socket, so a run that asks twice opens twice.
const CONVERGED_SESSION_CEILING: usize = 2 * RESOURCES;

#[test]
fn falsify_411_converged_apply_session_count_does_not_grow() {
    let s = converged();
    let (code, out, log) = s.run("apply", &["--yes"]);
    assert_eq!(code, 0, "{out}");
    let sessions = count(&log, "session");
    eprintln!(
        "forjar#411: converged apply of {RESOURCES} file resources on 1 machine: \
         {sessions} ssh session(s), {} master open(s), {} control check(s); target <= {}",
        count(&log, "master"),
        count(&log, "control"),
        RESOURCES + 1
    );
    assert!(
        sessions <= CONVERGED_SESSION_CEILING,
        "converged apply opened {sessions} sessions, ceiling {CONVERGED_SESSION_CEILING}: {log:?}"
    );
}
