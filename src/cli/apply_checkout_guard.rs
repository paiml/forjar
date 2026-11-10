//! forjar#845: `apply` refuses a config checkout whose HEAD is not `origin/main`.
//!
//! An intel `pv` apply ran from a forjar checkout that was 73 files behind
//! origin/main, and nothing refused it. The config an apply reads IS the
//! desired state; a stale checkout converges a host to a state the repository
//! has already moved past.
//!
//! # The rule
//!
//! Before any host is touched, the checkout holding the config file is
//! fetched (`git fetch origin main`) and its HEAD must equal `origin/main`.
//! Otherwise the apply exits non-zero naming the HEAD sha, the origin/main sha
//! and how far behind (and ahead) HEAD is.
//!
//! The one exception is a checkout explicitly named as a PR worktree:
//! `FORJAR_PR_WORKTREE=<branch>` (must be the branch HEAD is on) or
//! `FORJAR_PR_WORKTREE=<N>` / `#<N>` (a PR number). The name and HEAD sha are
//! printed to the apply's stderr so the run says what it applied.
//!
//! # What is not a checkout
//!
//! - A config with no `.git` in any ancestor directory is not in a checkout;
//!   there is no origin to be behind, so the guard does not apply.
//! - A checkout with no `origin` remote is a local repository (for example a
//!   state repo used with `--auto-commit`), not a clone of anything.
//!
//! Everything else that cannot be measured — a fetch that fails, an
//! `origin/main` that does not resolve — is a refusal: an unmeasured check is
//! a failing check.
//!
//! Read-only verbs (`plan`, `drift`, and the read-only apply modes) never call
//! this; the call site sits beside the operator gate in `dispatch_apply_cmd`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The env var naming a PR worktree as the one allowed exception.
pub(crate) const PR_WORKTREE_ENV: &str = "FORJAR_PR_WORKTREE";

/// Seam for every git call, so the policy is unit-testable without a repo.
pub(crate) trait GitProbe {
    /// Run `git -C <dir> <args>`; `Ok(trimmed stdout)` on exit 0, else the error.
    fn run(&self, dir: &Path, args: &[&str]) -> Result<String, String>;
}

/// The real git binary (`program` is a seam so a missing binary is testable).
pub(crate) struct SystemGit {
    pub program: &'static str,
}

impl GitProbe for SystemGit {
    fn run(&self, dir: &Path, args: &[&str]) -> Result<String, String> {
        let out = Command::new(self.program)
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            // Measure the checkout at `dir`, never one an inherited GIT_DIR
            // (a hook environment) points at.
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .map_err(|e| format!("git {}: {e}", args.join(" ")))?;
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
        }
        Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// The nearest ancestor of the config's directory that holds `.git`.
pub(crate) fn find_checkout_root(config: &Path) -> Option<PathBuf> {
    let parent = config.parent().filter(|p| !p.as_os_str().is_empty());
    let dir = parent.unwrap_or(Path::new("."));
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    dir.ancestors()
        .find(|d| d.join(".git").exists())
        .map(Path::to_path_buf)
}

/// Entry point used by `dispatch_apply_cmd`: real git, exception from the env.
pub(super) fn check_apply_checkout(config: &Path) -> Result<(), String> {
    let named = std::env::var(PR_WORKTREE_ENV).ok();
    if let Some(note) =
        check_checkout_head(config, named.as_deref(), &SystemGit { program: "git" })?
    {
        eprintln!("{note}");
    }
    Ok(())
}

/// The policy. `Ok(None)`: allowed silently. `Ok(Some(note))`: allowed as a
/// named PR worktree, and `note` must be logged. `Err`: refused.
pub(crate) fn check_checkout_head(
    config: &Path,
    pr_worktree: Option<&str>,
    git: &dyn GitProbe,
) -> Result<Option<String>, String> {
    let Some(top) = find_checkout_root(config) else {
        return Ok(None);
    };
    let remotes = git.run(&top, &["remote"]).map_err(|e| refusal(&top, &e))?;
    if !remotes.lines().any(|r| r.trim() == "origin") {
        return Ok(None);
    }
    if let Some(name) = pr_worktree.map(str::trim).filter(|n| !n.is_empty()) {
        return named_worktree(&top, name, git).map(Some);
    }
    git.run(&top, &["fetch", "--quiet", "origin", "main"])
        .map_err(|e| refusal(&top, &e))?;
    let head = rev(git, &top, "HEAD")?;
    let main = rev(git, &top, "refs/remotes/origin/main")?;
    if head == main {
        return Ok(None);
    }
    let behind = count(git, &top, "HEAD..refs/remotes/origin/main")?;
    let ahead = count(git, &top, "refs/remotes/origin/main..HEAD")?;
    Err(format!(
        "forjar#845: refusing to apply: config checkout {} has HEAD {} but origin/main is {} \
         ({behind} commit(s) behind, {ahead} ahead). Update it (`git -C {} pull --ff-only`) \
         or name it as a PR worktree with {PR_WORKTREE_ENV}=<branch|PR number>.",
        top.display(),
        short(&head),
        short(&main),
        top.display()
    ))
}

/// The exception: a PR number, or the branch HEAD is actually on.
fn named_worktree(top: &Path, name: &str, git: &dyn GitProbe) -> Result<String, String> {
    let head = rev(git, top, "HEAD")?;
    let digits = name.strip_prefix('#').unwrap_or(name);
    let is_pr = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit());
    if !is_pr {
        let branch = git
            .run(top, &["rev-parse", "--abbrev-ref", "HEAD"])
            .map_err(|e| refusal(top, &e))?;
        if branch != name {
            return Err(format!(
                "forjar#845: refusing to apply: {PR_WORKTREE_ENV}={name} but checkout {} \
                 is on '{branch}'; name the branch HEAD is on, or a PR number.",
                top.display()
            ));
        }
    }
    Ok(format!(
        "forjar#845: applying from PR worktree {name} (checkout {}, HEAD {}), not origin/main",
        top.display(),
        short(&head)
    ))
}

fn rev(git: &dyn GitProbe, top: &Path, what: &str) -> Result<String, String> {
    git.run(top, &["rev-parse", "--verify", what])
        .map_err(|e| refusal(top, &e))
}

fn count(git: &dyn GitProbe, top: &Path, range: &str) -> Result<String, String> {
    git.run(top, &["rev-list", "--count", range])
        .map_err(|e| refusal(top, &e))
}

fn short(sha: &str) -> &str {
    sha.get(..12).unwrap_or(sha)
}

fn refusal(top: &Path, err: &str) -> String {
    format!(
        "forjar#845: refusing to apply: cannot measure checkout {} against origin/main ({err}); \
         an unmeasured checkout is refused. Name a PR worktree with {PR_WORKTREE_ENV}=<branch|PR number>.",
        top.display()
    )
}

#[cfg(test)]
#[path = "tests_apply_checkout_guard.rs"]
mod tests;
