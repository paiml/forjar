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
///
/// EVERY PATH RETURNED EXISTS. Cargo reads a `rerun-if-changed` path that is
/// missing as "changed", on every invocation, for ever: the build script
/// re-runs and the whole root crate recompiles each time `cargo` is called.
/// A fresh clone (and every CI checkout) has a loose `refs/heads/<branch>`
/// and NO `packed-refs`; after `git pack-refs` it is the other way round. So
/// naming both unconditionally made one of them missing in every repo state.
/// Measured in CI run 45183144895: `cargo build --release` twice in one job,
/// 10m55s and then 9m08s, with nothing changed between them.
///
/// What is watched instead, and why no commit is missed:
///   - `HEAD` always (checkout, detached commits).
///   - loose ref present: watch it. A commit rewrites it; `git pack-refs`
///     deletes it, and a watched path that disappears also re-runs.
///   - loose ref absent (packed): watch `packed-refs` if present AND the
///     nearest existing directory above the loose ref, because the next
///     commit CREATES the loose file and leaves `packed-refs` untouched.
#[allow(dead_code)]
pub fn rerun_paths(dir: &Path) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    paths.extend(git_path(dir, "HEAD"));
    if let Some(sym) = git_out(dir, &["symbolic-ref", "-q", "HEAD"]) {
        if let Some(loose) = git_path(dir, &sym) {
            if Path::new(&loose).exists() {
                paths.push(loose);
            } else {
                paths.extend(git_path(dir, "packed-refs"));
                paths.extend(nearest_existing_dir(&loose, sym.matches('/').count()));
            }
        }
    }
    paths.retain(|p| Path::new(p).exists());
    paths
}

/// The nearest existing directory above `path`, climbing at most `max_up`
/// levels — the number of `/` in the ref name, so the climb stops at `refs/`
/// and can never reach the git dir itself (cargo scans a watched directory
/// recursively, and the git dir changes on every `git status`).
fn nearest_existing_dir(path: &str, max_up: usize) -> Option<String> {
    let mut cur = Path::new(path);
    for _ in 0..max_up {
        cur = cur.parent()?;
        if cur.is_dir() {
            return Some(cur.to_string_lossy().into_owned());
        }
    }
    None
}
