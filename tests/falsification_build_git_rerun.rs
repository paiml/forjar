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
    assert!(
        paths.iter().any(|p| p.ends_with("packed-refs")),
        "{paths:?}"
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
