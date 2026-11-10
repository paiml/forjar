//! forjar#845: `apply` refuses to run from a checkout whose HEAD is not origin/main.
//!
//! A stale (or off-branch) checkout converges hosts to an old declaration and
//! reports success. The gate runs first in `cmd_apply_scoped`, before any
//! connect, plan-execute or state write. `FORJAR_APPLY_PR_WORKTREE=<name>`
//! names a PR worktree and is recorded on stderr (the apply log).

use std::path::Path;
use std::process::Command;

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

/// Pure decision: `Err` names HEAD, origin/main and the distance behind.
pub(super) fn judge(head: &str, main: &str, behind: &str, pr: Option<&str>) -> Result<(), String> {
    if head == main {
        return Ok(());
    }
    if let Some(name) = pr.filter(|n| !n.is_empty()) {
        eprintln!("apply: PR worktree '{name}' allowed (HEAD {head}, origin/main {main})");
        return Ok(());
    }
    Err(format!(
        "refusing to apply: checkout HEAD {head} is not origin/main {main} ({behind} commit(s) behind); \
         update the checkout or set FORJAR_APPLY_PR_WORKTREE=<name> for a PR worktree (forjar#845)"
    ))
}

/// Dry runs and configs outside a git checkout are exempt.
pub(super) fn fresh_checkout_gate(file: &Path, dry_run: bool) -> Result<(), String> {
    if dry_run {
        return Ok(());
    }
    let dir = file
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if git(dir, &["rev-parse", "--show-toplevel"]).is_none() {
        return Ok(());
    }
    if git(dir, &["fetch", "--quiet", "origin", "main"]).is_none() {
        eprintln!("warning: could not fetch origin/main; judging against the local ref");
    }
    let (Some(head), Some(main)) = (
        git(dir, &["rev-parse", "HEAD"]),
        git(dir, &["rev-parse", "refs/remotes/origin/main"]),
    ) else {
        return Ok(()); // no origin/main to compare: not measured, not a refusal
    };
    let behind = git(
        dir,
        &["rev-list", "--count", "HEAD..refs/remotes/origin/main"],
    )
    .unwrap_or_else(|| "?".into());
    judge(
        &head,
        &main,
        &behind,
        std::env::var("FORJAR_APPLY_PR_WORKTREE").ok().as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(dir: &Path, args: &[&str]) {
        let ok = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["-c", "user.email=t@t", "-c", "user.name=t"])
            .args(args)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?}");
    }

    /// origin with 2 commits; clone checked out at the first.
    fn behind_clone() -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        let (o, c) = (t.path().join("o"), t.path().join("c"));
        std::fs::create_dir(&o).unwrap();
        run(&o, &["init", "-q", "-b", "main"]);
        run(&o, &["commit", "-q", "--allow-empty", "-m", "one"]);
        run(t.path(), &["clone", "-q", o.to_str().unwrap(), "c"]);
        run(&o, &["commit", "-q", "--allow-empty", "-m", "two"]);
        run(&c, &["fetch", "-q", "origin", "main"]); // c stays at "one"
        std::fs::write(c.join("forjar.yaml"), "x").unwrap();
        t
    }

    #[test]
    fn behind_checkout_is_refused_with_shas_and_count() {
        let t = behind_clone();
        let e = fresh_checkout_gate(&t.path().join("c/forjar.yaml"), false).unwrap_err();
        assert!(
            e.contains("1 commit(s) behind") && e.contains("origin/main"),
            "{e}"
        );
    }

    #[test]
    fn dry_run_is_exempt() {
        let t = behind_clone();
        assert!(fresh_checkout_gate(&t.path().join("c/forjar.yaml"), true).is_ok());
    }

    #[test]
    fn current_checkout_passes() {
        let t = behind_clone();
        let c = t.path().join("c");
        run(&c, &["merge", "-q", "--ff-only", "origin/main"]);
        assert!(fresh_checkout_gate(&c.join("forjar.yaml"), false).is_ok());
    }

    #[test]
    fn pr_worktree_name_allows_a_behind_checkout() {
        assert!(judge("a", "b", "1", Some("pr-845")).is_ok());
        assert!(judge("a", "b", "1", Some("")).is_err());
        assert!(judge("a", "b", "1", None).is_err());
    }

    #[test]
    fn non_git_config_dir_passes() {
        let t = tempfile::tempdir().unwrap();
        assert!(fresh_checkout_gate(&t.path().join("forjar.yaml"), false).is_ok());
    }
}
