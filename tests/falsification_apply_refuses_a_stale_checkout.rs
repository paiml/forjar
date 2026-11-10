//! forjar#845: `forjar apply` refuses a config checkout whose HEAD is not
//! `origin/main`, unless the checkout is named as a PR worktree.
//!
//! An intel `pv` apply ran from a forjar checkout 73 files behind origin/main
//! and nothing refused it. The falsifier builds a real clone one commit behind
//! its (local, bare) origin and asserts on WHETHER THE TARGET CHANGED and
//! WHETHER STATE WAS WRITTEN, never on the exit code alone.
//!
//! Mutant (recorded in the PR's falsification receipt): make
//! `check_apply_checkout` return `Ok(())` without looking. The apply then
//! converges the target from the stale checkout and
//! `apply_from_a_checkout_one_commit_behind_origin_main_touches_nothing` is RED.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

/// git with the inherited environment scrubbed (see e04: a hook's GIT_DIR
/// once let a test commit into the developer's own repository) and with no
/// global/system config, hooks or signing in play.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "fj845")
        .env("GIT_AUTHOR_EMAIL", "fj845@example.invalid")
        .env("GIT_COMMITTER_NAME", "fj845")
        .env("GIT_COMMITTER_EMAIL", "fj845@example.invalid")
        .output()
        .expect("git failed to start");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

struct Sb {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Sb {
    /// A bare `origin`, a clone `work` holding the config, and a target dir
    /// OUTSIDE the clone. `origin/main` is one commit ahead of nothing yet.
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().canonicalize().unwrap();
        let sb = Self { _dir: dir, root };
        fs::create_dir_all(sb.p("target")).unwrap();
        git(
            &sb.root,
            &["init", "-q", "--bare", "-b", "main", "origin.git"],
        );
        git(&sb.root, &["init", "-q", "-b", "main", "work"]);
        git(
            &sb.work(),
            &[
                "remote",
                "add",
                "origin",
                sb.p("origin.git").to_str().unwrap(),
            ],
        );
        fs::write(sb.work().join("src.txt"), "DECLARED\n").unwrap();
        fs::write(
            sb.cfg(),
            format!(
                "version: \"1.0\"\nname: t\nmachines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\nresources:\n  f: {{ type: file, machine: local, path: {}, source: {}, mode: \"0644\" }}\n",
                sb.target().display(),
                sb.work().join("src.txt").display()
            ),
        )
        .unwrap();
        git(&sb.work(), &["add", "-A"]);
        git(&sb.work(), &["commit", "-q", "--no-verify", "-m", "c1"]);
        git(&sb.work(), &["push", "-q", "origin", "main"]);
        sb
    }
    fn p(&self, r: &str) -> PathBuf {
        self.root.join(r)
    }
    fn work(&self) -> PathBuf {
        self.p("work")
    }
    fn cfg(&self) -> PathBuf {
        self.work().join("forjar.yaml")
    }
    fn target(&self) -> PathBuf {
        self.p("target/out.txt")
    }
    fn state(&self) -> PathBuf {
        self.p("state")
    }
    /// Push a second commit to origin, then move the clone back one: HEAD is
    /// exactly one commit behind origin/main.
    fn fall_one_behind(&self) {
        fs::write(self.work().join("NOTES"), "c2\n").unwrap();
        git(&self.work(), &["add", "-A"]);
        git(&self.work(), &["commit", "-q", "--no-verify", "-m", "c2"]);
        git(&self.work(), &["push", "-q", "origin", "main"]);
        git(&self.work(), &["reset", "-q", "--hard", "HEAD~1"]);
    }
    fn apply(&self, pr_worktree: Option<&str>) -> Output {
        let mut cmd = Command::new(forjar());
        cmd.args(["apply", "-f"])
            .arg(self.cfg())
            .arg("--state-dir")
            .arg(self.state())
            .arg("--yes")
            .env_remove("FORJAR_PR_WORKTREE")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null");
        if let Some(name) = pr_worktree {
            cmd.env("FORJAR_PR_WORKTREE", name);
        }
        cmd.output().expect("forjar failed to start")
    }
    fn state_is_empty(&self) -> bool {
        fs::read_dir(self.state()).map_or(true, |mut d| d.next().is_none())
    }
}

fn log(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn apply_from_a_checkout_one_commit_behind_origin_main_touches_nothing() {
    let sb = Sb::new();
    sb.fall_one_behind();
    let head = git(&sb.work(), &["rev-parse", "HEAD"]);

    let out = sb.apply(None);
    let log = log(&out);
    assert_ne!(
        out.status.code(),
        Some(0),
        "stale checkout must be refused:\n{log}"
    );
    assert!(!sb.target().exists(), "target was touched:\n{log}");
    assert!(sb.state_is_empty(), "state was written:\n{log}");
    assert!(log.contains("forjar#845"), "{log}");
    assert!(log.contains(&head[..12]), "message names HEAD:\n{log}");
    assert!(log.contains("1 commit(s) behind"), "{log}");
}

/// CONTROL: the same sandbox at origin/main converges. Without it, "target
/// untouched" above would also pass if this config could never apply.
#[test]
fn apply_from_a_checkout_at_origin_main_converges() {
    let sb = Sb::new();
    let out = sb.apply(None);
    assert_eq!(out.status.code(), Some(0), "{}", log(&out));
    assert_eq!(fs::read_to_string(sb.target()).unwrap(), "DECLARED\n");
}

/// The fetch is real: a clone whose remote-tracking ref is stale (origin moved
/// on after the clone last fetched) is still refused.
#[test]
fn apply_fetches_before_comparing() {
    let sb = Sb::new();
    let other = sb.p("other");
    git(
        &sb.root,
        &["clone", "-q", sb.p("origin.git").to_str().unwrap(), "other"],
    );
    fs::write(other.join("NOTES"), "moved\n").unwrap();
    git(&other, &["add", "-A"]);
    git(&other, &["commit", "-q", "--no-verify", "-m", "c2"]);
    git(&other, &["push", "-q", "origin", "main"]);

    let out = sb.apply(None);
    assert_ne!(out.status.code(), Some(0), "{}", log(&out));
    assert!(!sb.target().exists());
}

#[test]
fn a_named_pr_worktree_is_the_exception_and_is_logged() {
    let sb = Sb::new();
    sb.fall_one_behind();
    git(&sb.work(), &["checkout", "-q", "-b", "fix/845"]);

    let out = sb.apply(Some("fix/845"));
    let log = log(&out);
    assert_eq!(out.status.code(), Some(0), "{log}");
    assert!(sb.target().exists(), "{log}");
    assert!(log.contains("PR worktree fix/845"), "{log}");
}

#[test]
fn a_misnamed_pr_worktree_is_refused() {
    let sb = Sb::new();
    sb.fall_one_behind();
    let out = sb.apply(Some("not-this-branch"));
    assert_ne!(out.status.code(), Some(0), "{}", log(&out));
    assert!(!sb.target().exists());
    assert!(sb.state_is_empty());
}

/// Read-only verbs are out of scope: `plan` on the stale checkout still runs.
#[test]
fn plan_is_not_guarded() {
    let sb = Sb::new();
    sb.fall_one_behind();
    let out = Command::new(forjar())
        .args(["plan", "-f"])
        .arg(sb.cfg())
        .arg("--state-dir")
        .arg(sb.state())
        .env_remove("FORJAR_PR_WORKTREE")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", log(&out));
    assert!(!log(&out).contains("forjar#845"));
}
