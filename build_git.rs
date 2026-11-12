//! #639: the git paths whose change must re-run `build.rs`, so the SHA baked
//! into `forjar --version` never goes stale. Shared by `build.rs` (via
//! `#[path]`) and `tests/falsification_build_git_rerun.rs`.
//!
//! `.git/HEAD` alone is not enough. On an attached branch a commit moves
//! `refs/heads/<branch>` (or `packed-refs`) and leaves HEAD untouched. In a
//! linked worktree `.git` is a FILE, so `.git/HEAD` does not exist; the real
//! paths come from `git rev-parse --git-path`.

use std::path::Path;
use std::process::Command;

fn git_out(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn git_path(dir: &Path, name: &str) -> Option<String> {
    let p = git_out(dir, &["rev-parse", "--git-path", name])?;
    let p = Path::new(&p);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        dir.join(p)
    };
    Some(abs.to_string_lossy().into_owned())
}

/// Paths to hand to `cargo::rerun-if-changed=`; empty outside a git tree.
#[allow(dead_code)]
pub fn rerun_paths(dir: &Path) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    paths.extend(git_path(dir, "HEAD"));
    if let Some(sym) = git_out(dir, &["symbolic-ref", "-q", "HEAD"]) {
        paths.extend(git_path(dir, &sym));
        paths.extend(git_path(dir, "packed-refs"));
    }
    paths
}
