//! forjar#845: REFUSE TO APPLY FROM A STALE CHECKOUT, BEFORE TOUCHING ANYTHING.
//!
//! `forjar apply` converges hosts to the config it reads from the working
//! tree. A checkout that is behind `origin/main` converges them to an OLD
//! config. This gate fetches and compares HEAD to `origin/main` before any
//! connect, plan-execute or state write, and refuses with the two shas and the
//! number of commits behind.
//!
//! Exemption: `FORJAR_PR_WORKTREE=<reason>` names the checkout as a PR
//! worktree on purpose; the exemption is printed so it lands in the apply log.
//! Read-only verbs (`plan`, `drift`) and `--dry-run` never reach this gate.
//! A directory that is not a git checkout, or has no `origin/main`, is not
//! this gate's business and passes.

use std::path::Path;
use std::process::Command;

/// Env var that names a checkout as a deliberate PR worktree.
pub(super) const PR_WORKTREE_ENV: &str = "FORJAR_PR_WORKTREE";

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Pure decision over measured facts: `Err(message)` when behind.
pub(super) fn judge(head: &str, origin: &str, behind: u64) -> Result<(), String> {
    if head == origin || behind == 0 {
        return Ok(());
    }
    Err(format!(
        "apply refused: this checkout is stale. HEAD {} != origin/main {} ({} commit(s) behind). \
         Pull first, or set {}=<reason> to apply from a PR worktree on purpose.",
        &head[..head.len().min(12)],
        &origin[..origin.len().min(12)],
        behind,
        PR_WORKTREE_ENV
    ))
}

/// The gate. `config_file` locates the checkout; `exempt` is the PR-worktree
/// reason, if any.
pub(super) fn checkout_fresh_gate(config_file: &Path, exempt: Option<&str>) -> Result<(), String> {
    let dir = config_file
        .canonicalize()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| Path::new(".").to_path_buf());
    if git(&dir, &["rev-parse", "--is-inside-work-tree"]).as_deref() != Some("true") {
        return Ok(());
    }
    if let Some(reason) = exempt.filter(|r| !r.is_empty()) {
        eprintln!("apply: checkout-freshness gate exempted ({PR_WORKTREE_ENV}={reason})");
        return Ok(());
    }
    // A failed fetch is not measured: do not refuse on it.
    let _ = git(&dir, &["fetch", "--quiet", "origin", "main"]);
    let (Some(head), Some(origin)) = (
        git(&dir, &["rev-parse", "HEAD"]),
        git(&dir, &["rev-parse", "--verify", "-q", "origin/main"]),
    ) else {
        return Ok(());
    };
    let behind = git(&dir, &["rev-list", "--count", "HEAD..origin/main"])
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    judge(&head, &origin, behind)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(dir: &Path, args: &[&str]) {
        let s = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .unwrap();
        assert!(s.success(), "git {args:?}");
    }

    /// origin (bare) + a clone one commit BEHIND origin/main.
    fn stale_clone(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("fj845-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (origin, work, other) = (root.join("o.git"), root.join("w"), root.join("x"));
        std::fs::create_dir_all(&root).unwrap();
        sh(&root, &["init", "-q", "--bare", "-b", "main", "o.git"]);
        for d in [&work, &other] {
            sh(
                &root,
                &[
                    "clone",
                    "-q",
                    origin.to_str().unwrap(),
                    d.file_name().unwrap().to_str().unwrap(),
                ],
            );
            sh(d, &["config", "user.email", "t@t"]);
            sh(d, &["config", "user.name", "t"]);
        }
        std::fs::write(work.join("forjar.yaml"), "a: 1\n").unwrap();
        sh(&work, &["add", "."]);
        sh(&work, &["commit", "-q", "-m", "one"]);
        sh(&work, &["push", "-q", "origin", "HEAD:main"]);
        sh(&other, &["pull", "-q", "origin", "main"]);
        std::fs::write(other.join("forjar.yaml"), "a: 2\n").unwrap();
        sh(&other, &["commit", "-q", "-am", "two"]);
        sh(&other, &["push", "-q", "origin", "HEAD:main"]);
        (root, work)
    }

    #[test]
    fn falsifier_stale_checkout_is_refused_and_names_shas() {
        let (root, work) = stale_clone("stale");
        let err = checkout_fresh_gate(&work.join("forjar.yaml"), None).unwrap_err();
        assert!(err.contains("1 commit(s) behind"), "{err}");
        assert!(err.contains("origin/main"), "{err}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn fresh_checkout_passes() {
        let (root, work) = stale_clone("fresh");
        sh(&work, &["pull", "-q", "origin", "main"]);
        checkout_fresh_gate(&work.join("forjar.yaml"), None).unwrap();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn pr_worktree_exemption_passes() {
        let (root, work) = stale_clone("exempt");
        checkout_fresh_gate(&work.join("forjar.yaml"), Some("PR #1")).unwrap();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn non_git_dir_passes() {
        let d = std::env::temp_dir().join(format!("fj845-nogit-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("f.yaml"), "").unwrap();
        checkout_fresh_gate(&d.join("f.yaml"), None).unwrap();
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn judge_equal_and_behind() {
        judge("abc", "abc", 0).unwrap();
        assert!(judge("abc", "def", 3).unwrap_err().contains("3 commit(s)"));
    }
}
