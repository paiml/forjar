//! #639: `build.rs` must re-run when the commit moves, including on an attached
//! branch (HEAD file unchanged) and in a linked worktree (`.git` is a file).

#[path = "../build_git.rs"]
mod build_git;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let st = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .status()
        .expect("run git");
    assert!(st.success(), "git {args:?} failed");
}

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("fj639-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn norm(p: &str) -> std::path::PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.into())
}

#[test]
fn attached_branch_watches_the_branch_ref() {
    let d = tmp("attached");
    git(&d, &["init", "-q", "-b", "work"]);
    git(&d, &["commit", "-q", "--allow-empty", "-m", "a"]);
    let paths = build_git::rerun_paths(&d);
    let want = norm(d.join(".git/refs/heads/work").to_str().unwrap());
    assert!(
        paths.iter().any(|p| norm(p) == want),
        "branch ref not watched: {paths:?}"
    );
    assert!(paths.iter().any(|p| p.ends_with("HEAD")), "{paths:?}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn linked_worktree_watches_real_git_dir() {
    let d = tmp("wt-main");
    git(&d, &["init", "-q", "-b", "main"]);
    git(&d, &["commit", "-q", "--allow-empty", "-m", "a"]);
    let wt = tmp("wt-linked");
    let wt_path = wt.join("w");
    git(
        &d,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "feat",
            wt_path.to_str().unwrap(),
        ],
    );
    assert!(
        wt_path.join(".git").is_file(),
        "worktree .git must be a file"
    );
    let paths = build_git::rerun_paths(&wt_path);
    assert!(!paths.is_empty(), "no paths for a worktree");
    for p in &paths {
        assert!(
            !p.contains("/w/.git/"),
            "path goes through the .git file: {p}"
        );
    }
    let head = paths
        .iter()
        .find(|p| p.ends_with("HEAD"))
        .expect("HEAD path");
    assert!(Path::new(head).exists(), "HEAD path does not exist: {head}");
    assert!(
        paths.iter().any(|p| p.ends_with("refs/heads/feat")),
        "{paths:?}"
    );
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::remove_dir_all(&wt);
}

#[test]
fn outside_git_is_empty() {
    let d = tmp("nogit");
    let enclosed = Command::new("git")
        .current_dir(&d)
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if enclosed {
        return;
    }
    assert!(build_git::rerun_paths(&d).is_empty());
    let _ = std::fs::remove_dir_all(&d);
}

/// Cargo treats a MISSING `rerun-if-changed` path as changed on every
/// invocation, so one missing path recompiles the root crate each time cargo
/// runs. A fresh clone has no `packed-refs` — the state of every CI checkout.
#[test]
fn fresh_repo_names_no_missing_path() {
    let d = tmp("fresh");
    git(&d, &["init", "-q", "-b", "work"]);
    git(&d, &["commit", "-q", "--allow-empty", "-m", "a"]);
    assert!(
        !d.join(".git/packed-refs").exists(),
        "fixture: a fresh repo must have no packed-refs"
    );
    let paths = build_git::rerun_paths(&d);
    assert!(paths.len() >= 2, "HEAD and the branch ref: {paths:?}");
    for p in &paths {
        assert!(
            Path::new(p).exists(),
            "watched path does not exist, cargo would rebuild on every call: {p}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// The mirror state: after `git pack-refs` the loose ref is gone. Nothing
/// missing may be named, and the NEXT commit — which creates the loose file
/// and leaves `packed-refs` untouched — must still land inside a watched path.
#[test]
fn packed_ref_names_no_missing_path_and_still_sees_the_next_commit() {
    let d = tmp("packed");
    git(&d, &["init", "-q", "-b", "fix/topic"]);
    git(&d, &["commit", "-q", "--allow-empty", "-m", "a"]);
    git(&d, &["pack-refs", "--all"]);
    let loose = d.join(".git/refs/heads/fix/topic");
    assert!(
        !loose.exists(),
        "fixture: the loose ref must be packed away"
    );
    let paths = build_git::rerun_paths(&d);
    for p in &paths {
        assert!(
            Path::new(p).exists(),
            "watched path does not exist, cargo would rebuild on every call: {p}"
        );
    }
    assert!(
        paths.iter().any(|p| p.ends_with("packed-refs")),
        "{paths:?}"
    );
    let git_dir = norm(d.join(".git").to_str().unwrap());
    let dirs: Vec<_> = paths
        .iter()
        .map(|p| norm(p))
        .filter(|p| p.is_dir())
        .collect();
    assert!(!dirs.is_empty(), "no directory watched: {paths:?}");
    for w in &dirs {
        assert!(
            w.starts_with(git_dir.join("refs")),
            "watched directory is outside refs/ (cargo scans it recursively): {w:?}"
        );
    }
    git(&d, &["commit", "-q", "--allow-empty", "-m", "b"]);
    let loose = norm(loose.to_str().unwrap());
    assert!(
        loose.exists(),
        "fixture: the commit must create the loose ref"
    );
    assert!(
        dirs.iter().any(|w| loose.starts_with(w)),
        "the new loose ref {loose:?} is under no watched directory: {dirs:?}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
