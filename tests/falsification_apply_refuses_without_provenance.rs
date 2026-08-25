//! forjar#266: `ensure_event_log_writable` was written to stop an apply that
//! cannot record what it did — and had **zero callers**. Its own doc comment
//! said "Call this in the apply preflight"; nothing did.
//!
//! So a full disk, a read-only state dir or a bad permission produced an apply
//! that MUTATED THE HOST and recorded nothing, behind a stderr warning.
//!
//! An absent event is indistinguishable from an apply that never ran. That
//! ambiguity is what left paiml/infra#208 unattributable across three toolchain
//! deletions in one day.
//!
//! Every assertion here is on WHETHER THE HOST CHANGED — never on an exit code
//! alone, because an exit code cannot distinguish "refused before touching
//! anything" from "failed after mutating".

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

struct Sb {
    dir: tempfile::TempDir,
}

impl Sb {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("src.txt"), "DECLARED\n").unwrap();
        Self { dir }
    }
    fn p(&self, r: &str) -> std::path::PathBuf {
        self.dir.path().join(r)
    }
    fn cfg(&self) -> std::path::PathBuf {
        let c = self.p("f.yaml");
        fs::write(
            &c,
            format!(
                "version: \"1.0\"\nname: t\nmachines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\nresources:\n  f: {{ type: file, machine: local, path: {}, source: {}, mode: \"0644\" }}\n",
                self.p("target.txt").display(),
                self.p("src.txt").display()
            ),
        )
        .unwrap();
        c
    }
    fn apply(&self, extra: &[&str]) -> i32 {
        // Bind the paths first: inlining `self.p(..).to_str()` borrows a
        // temporary that is dropped at the end of the statement.
        let cfg = self.p("f.yaml");
        let state = self.p("state");
        let mut args = vec![
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            state.to_str().unwrap(),
            "--yes",
        ];
        args.extend_from_slice(extra);
        Command::new(forjar())
            .args(&args)
            .output()
            .expect("forjar failed to start")
            .status
            .code()
            .unwrap_or(-1)
    }
}

#[test]
fn apply_refuses_when_it_cannot_record_what_it_did() {
    let sb = Sb::new();
    sb.cfg();

    // Make the per-machine state dir unwritable, so the event log cannot be
    // appended. Create it first — an absent dir is creatable and is a different
    // condition from an unwritable one.
    let md = sb.p("state/local");
    fs::create_dir_all(&md).unwrap();
    fs::set_permissions(&md, fs::Permissions::from_mode(0o500)).unwrap();

    let rc = sb.apply(&[]);

    // THE ASSERTION THAT MATTERS: the host must be untouched.
    assert!(
        !sb.p("target.txt").exists(),
        "apply mutated the host while unable to write its provenance log — an \
         absent event is indistinguishable from an apply that never ran"
    );
    assert_ne!(
        rc, 0,
        "apply reported success while refusing to do the work"
    );

    // Restore so the tempdir can be cleaned up.
    fs::set_permissions(&md, fs::Permissions::from_mode(0o700)).unwrap();
}

#[test]
fn a_writable_state_dir_still_applies() {
    // THE CONTROL. Without it, "refuse always" passes the test above, which
    // would be a far worse defect than the one being fixed.
    let sb = Sb::new();
    sb.cfg();
    let rc = sb.apply(&[]);
    assert_eq!(rc, 0, "a normal apply must still succeed");
    assert_eq!(
        fs::read_to_string(sb.p("target.txt")).unwrap(),
        "DECLARED\n",
        "a normal apply must still converge the file"
    );
}

#[test]
fn dry_run_does_not_require_a_writable_log() {
    // `--dry-run` mutates nothing, so an unwritable log costs nothing. Failing
    // it would make the read-only inspection path depend on write access —
    // exactly when an operator is most likely to be diagnosing a full disk.
    let sb = Sb::new();
    sb.cfg();
    let md = sb.p("state/local");
    fs::create_dir_all(&md).unwrap();
    fs::set_permissions(&md, fs::Permissions::from_mode(0o500)).unwrap();

    let rc = sb.apply(&["--dry-run"]);
    fs::set_permissions(&md, fs::Permissions::from_mode(0o700)).unwrap();

    assert_eq!(rc, 0, "--dry-run must not require a writable event log");
    assert!(!sb.p("target.txt").exists(), "--dry-run mutated the host");
}
