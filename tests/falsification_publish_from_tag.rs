//! PMAT-165: `scripts/publish-from-tag.sh` must publish the workspace to
//! crates.io only from a DETACHED worktree of a release tag — never from the
//! ambient working tree, and never with `--allow-dirty`. Publishing from the
//! working tree lets untracked state (agent memory, other worktrees, scratch
//! files) dirty the tree cargo sees, and `--allow-dirty` has been the
//! standing temptation to paper over that.
//!
//! WHY A SHIM `cargo` AND NOT THE REAL ONE. This suite must never touch
//! crates.io, and must prove a negative (no registry token reaches the
//! publish step) that only an in-process observer can attest to. A shim
//! placed first on PATH logs every argv line, its cwd, and whether
//! `CARGO_REGISTRY_TOKEN` was visible, to a file this suite reads back.
//!
//! WHY `--template=` ON `git init`. This machine's global `init.templateDir`
//! installs a pre-commit quality-gate hook into every freshly initialised
//! repository — including this test's throwaway sandbox — and that hook
//! fails outright before the first commit exists. `--template=` (empty)
//! skips it, so the sandbox commits cleanly regardless of the host's global
//! git config.
//!
//! Cases (a)-(e) are the falsification set named in the PMAT-165 brief:
//!   (a) a nonexistent tag exits 2 with an empty shim log
//!   (b) a tag whose Cargo.toml version disagrees with the tag exits 2, empty
//!       shim log
//!   (c) `DRY_RUN=1` on a valid tag: only `publish --dry-run --locked` is
//!       logged, never a plain `publish --locked`; every cargo call ran in a
//!       cwd other than the repo path; no worktree survives afterwards
//!   (d) a dirty AMBIENT checkout (an untracked file in the repo, not the
//!       worktree) still dry-runs clean, because the worktree is fresh
//!   (e) the shim never sees `CARGO_REGISTRY_TOKEN`, even though the test
//!       sets one
//!
//! MUTATION GUARD. Removing `--detach` from the script's `git worktree add`
//! call makes checking out a TAG (not a branch) fail outright — a tag has no
//! branch to attach a worktree to non-detached. So a regression to a
//! non-detached, dirtiable worktree cannot pass case (c) or (d): the script
//! would error before ever reaching the dry-run publish this suite asserts on.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/publish-from-tag.sh")
}

/// Logs `ARGV:`/`CWD:`/`TOKEN:` for every invocation to `$CARGO_SHIM_LOG`,
/// then answers `metadata` and `search` with canned output. `publish` is a
/// no-op success. Never contacts crates.io.
const CARGO_SHIM: &str = r#"#!/usr/bin/env bash
set -eu
LOG="${CARGO_SHIM_LOG:?}"
{
  printf 'ARGV: %s\n' "$*"
  printf 'CWD: %s\n' "$PWD"
  printf 'TOKEN: %s\n' "${CARGO_REGISTRY_TOKEN:-<unset>}"
} >> "$LOG"
case "$1" in
  metadata)
    cat <<'JSON'
{"packages":[{"name":"demo","version":"0.0.1","publish":null,"manifest_path":"/x/Cargo.toml","dependencies":[]}]}
JSON
    ;;
  search)
    printf 'demo = "0.0.0"    # a demo crate\n'
    ;;
  publish)
    :
    ;;
esac
exit 0
"#;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {args:?} failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

struct Sandbox {
    repo: tempfile::TempDir,
    bin: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let repo = tempfile::tempdir().expect("repo tempdir");
        let bin = tempfile::tempdir().expect("bin tempdir");
        let r = repo.path();

        git(r, &["init", "-q", "-b", "main", "--template=", "."]);
        git(r, &["config", "user.email", "test@example.com"]);
        git(r, &["config", "user.name", "test"]);

        fs::write(
            r.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.0.1\"\nedition = \"2021\"\n\n\
             [workspace]\nmembers = [\".\"]\n",
        )
        .expect("write Cargo.toml");
        fs::create_dir_all(r.join("src")).expect("create src dir");
        fs::write(r.join("src/main.rs"), "fn main() {}\n").expect("write main.rs");

        git(r, &["add", "-A"]);
        git(r, &["commit", "-q", "-m", "init"]);
        git(r, &["tag", "v0.0.1"]);
        // Points at itself: `git fetch origin main` inside the script then
        // has a real `origin/main` to check ancestry against.
        git(r, &["remote", "add", "origin", &r.display().to_string()]);

        let cargo_path = bin.path().join("cargo");
        fs::write(&cargo_path, CARGO_SHIM).expect("write cargo shim");
        let mut perms = fs::metadata(&cargo_path).expect("stat shim").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&cargo_path, perms).expect("chmod shim");

        Sandbox { repo, bin }
    }

    fn path(&self) -> &Path {
        self.repo.path()
    }

    /// Tag the current HEAD under a name whose version does not match
    /// Cargo.toml (which still says 0.0.1).
    fn tag_mismatched(&self, tag: &str) {
        git(self.path(), &["tag", tag]);
    }

    fn run(&self, tag: &str, dry_run: bool, log: &Path) -> Output {
        fs::write(log, "").expect("truncate shim log");
        let path_var = format!(
            "{}:{}",
            self.bin.path().display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut cmd = Command::new("bash");
        cmd.arg(script())
            .arg(tag)
            .current_dir(self.path())
            .env("PATH", path_var)
            .env("CARGO_SHIM_LOG", log)
            .env("CARGO_REGISTRY_TOKEN", "super-secret-test-token");
        if dry_run {
            cmd.env("DRY_RUN", "1");
        } else {
            cmd.env_remove("DRY_RUN");
        }
        cmd.output().expect("run publish-from-tag.sh")
    }
}

fn log_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn worktree_count(repo: &Path) -> usize {
    let out = Command::new("git")
        .args(["worktree", "list"])
        .current_dir(repo)
        .output()
        .expect("git worktree list");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count()
}

#[test]
fn a_nonexistent_tag_refuses_before_any_cargo_call() {
    let sb = Sandbox::new();
    let log = sb.bin.path().join("log-a.txt");
    let out = sb.run("v9.9.9", false, &log);
    assert_eq!(
        out.status.code(),
        Some(2),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        log_text(&log).is_empty(),
        "the shim was called for a tag that does not exist"
    );
}

#[test]
fn b_a_version_mismatch_refuses_before_any_cargo_call() {
    let sb = Sandbox::new();
    sb.tag_mismatched("v0.0.2");
    let log = sb.bin.path().join("log-b.txt");
    let out = sb.run("v0.0.2", false, &log);
    assert_eq!(
        out.status.code(),
        Some(2),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        log_text(&log).is_empty(),
        "the shim was called for a tag whose Cargo.toml version disagrees with the tag"
    );
}

#[test]
fn c_dry_run_never_publishes_and_leaves_no_worktree_behind() {
    let sb = Sandbox::new();
    let log = sb.bin.path().join("log-c.txt");
    let out = sb.run("v0.0.1", true, &log);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = log_text(&log);
    assert!(
        text.contains("ARGV: publish --dry-run --locked -p demo"),
        "no dry-run publish was logged:\n{text}"
    );
    assert!(
        !text.contains("ARGV: publish --locked -p demo"),
        "a real (non-dry-run) publish ran under DRY_RUN=1:\n{text}"
    );

    let repo_path = sb.path().display().to_string();
    for line in text.lines().filter(|l| l.starts_with("CWD: ")) {
        let cwd = line.trim_start_matches("CWD: ");
        assert_ne!(
            cwd, repo_path,
            "a cargo call ran in the repo path instead of a detached worktree:\n{text}"
        );
    }
    assert_eq!(
        worktree_count(sb.path()),
        1,
        "a worktree was left behind after the script exited:\n{text}"
    );
}

#[test]
fn d_a_dirty_ambient_checkout_still_dry_runs_clean_from_the_worktree() {
    let sb = Sandbox::new();
    fs::write(sb.path().join("scratch.txt"), "untracked scratch\n").expect("untracked file");
    let log = sb.bin.path().join("log-d.txt");
    let out = sb.run("v0.0.1", true, &log);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        log_text(&log).contains("ARGV: publish --dry-run --locked -p demo"),
        "the dirty ambient checkout stopped the worktree from dry-running clean"
    );
}

#[test]
fn e_the_registry_token_never_reaches_the_shim() {
    let sb = Sandbox::new();
    let log = sb.bin.path().join("log-e.txt");
    let out = sb.run("v0.0.1", false, &log);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = log_text(&log);
    let lines: Vec<&str> = text.lines().collect();
    let mut publish_token_lines = 0;
    for w in lines.windows(3) {
        if w[0].starts_with("ARGV: publish") {
            publish_token_lines += 1;
            assert_eq!(
                w[2], "TOKEN: <unset>",
                "CARGO_REGISTRY_TOKEN reached the publish step:\n{text}"
            );
        }
    }
    assert!(
        publish_token_lines > 0,
        "no publish calls were logged at all:\n{text}"
    );
}
