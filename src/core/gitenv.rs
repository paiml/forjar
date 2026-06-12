//! Git subprocess construction immune to hook-injected repository targeting.
//!
//! Git exports `GIT_DIR` (and related variables) to processes it spawns from
//! hooks. A forjar invocation inside a pre-push/post-commit hook would
//! therefore run its own `git` subprocesses against the *hook's* repository
//! instead of the stack's — `current_dir()` is ignored once `GIT_DIR` is set
//! (GH-134). Every production `git` invocation must go through these
//! constructors.

use std::path::Path;
use std::process::Command;

/// Environment variables through which git redirects repository discovery.
const GIT_REPO_ENV: [&str; 9] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
    "GIT_PREFIX",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CEILING_DIRECTORIES",
];

/// A `git` command whose repository is resolved solely from the process cwd.
pub fn git() -> Command {
    let mut cmd = Command::new("git");
    for var in GIT_REPO_ENV {
        cmd.env_remove(var);
    }
    cmd
}

/// A `git` command whose repository is resolved solely from `dir`.
pub fn git_in(dir: &Path) -> Command {
    let mut cmd = git();
    cmd.current_dir(dir);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_scrubs_all_repo_env_vars() {
        let cmd = git();
        let removed: Vec<_> = cmd
            .get_envs()
            .filter(|(_, v)| v.is_none())
            .map(|(k, _)| k.to_os_string())
            .collect();
        for var in GIT_REPO_ENV {
            assert!(
                removed.iter().any(|k| k == var),
                "expected {var} to be scrubbed"
            );
        }
    }

    #[test]
    fn git_in_sets_cwd_and_scrubs() {
        let dir = tempfile::tempdir().unwrap();
        let cmd = git_in(dir.path());
        assert_eq!(cmd.get_current_dir(), Some(dir.path()));
        assert!(cmd.get_envs().any(|(k, v)| k == "GIT_DIR" && v.is_none()));
    }

    /// Demonstrates the GH-134 failure mode at the Command level: an
    /// explicit GIT_DIR overrides current_dir for repository targeting,
    /// while the scrubbed constructor resolves the cwd repository.
    #[test]
    fn git_dir_env_overrides_cwd_but_scrubbed_command_is_immune() {
        let repo = tempfile::tempdir().unwrap();
        let decoy = tempfile::tempdir().unwrap();
        for dir in [repo.path(), decoy.path()] {
            let ok = git_in(dir).args(["init", "-q"]).status().unwrap();
            assert!(ok.success());
        }

        // Unscrubbed + GIT_DIR (as inside a git hook): the decoy wins.
        let out = Command::new("git")
            .current_dir(repo.path())
            .env("GIT_DIR", decoy.path().join(".git"))
            .args(["rev-parse", "--absolute-git-dir"])
            .output()
            .unwrap();
        let hijacked = String::from_utf8_lossy(&out.stdout);
        assert!(
            hijacked
                .trim()
                .starts_with(&*decoy.path().to_string_lossy()),
            "expected GIT_DIR to hijack discovery, got: {hijacked}"
        );

        // Scrubbed constructor: cwd repository wins. (The scrub list is
        // applied by git_in; the decoy var cannot be re-injected here
        // without defeating the test, so this asserts the cwd resolution.)
        let out = git_in(repo.path())
            .args(["rev-parse", "--absolute-git-dir"])
            .output()
            .unwrap();
        let resolved = String::from_utf8_lossy(&out.stdout);
        let canon = repo.path().canonicalize().unwrap();
        let canon_resolved = Path::new(resolved.trim())
            .canonicalize()
            .unwrap_or_default();
        assert!(
            canon_resolved.starts_with(&canon),
            "expected {canon:?} prefix, got {canon_resolved:?}"
        );
    }
}
