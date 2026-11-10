//! forjar#845 unit tests: the checkout-HEAD policy against a scripted git.

use super::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;

/// A scripted git: each joined argv maps to a result; unscripted calls fail.
#[derive(Default)]
struct FakeGit {
    answers: HashMap<String, Result<String, String>>,
    calls: RefCell<Vec<String>>,
}

impl FakeGit {
    fn with(mut self, args: &str, out: Result<&str, &str>) -> Self {
        let out = out.map(str::to_string).map_err(str::to_string);
        self.answers.insert(args.to_string(), out);
        self
    }
    fn called(&self, args: &str) -> bool {
        self.calls.borrow().iter().any(|c| c == args)
    }
}

impl GitProbe for FakeGit {
    fn run(&self, _dir: &Path, args: &[&str]) -> Result<String, String> {
        let key = args.join(" ");
        self.calls.borrow_mut().push(key.clone());
        self.answers
            .get(&key)
            .cloned()
            .unwrap_or_else(|| Err(format!("unscripted: {key}")))
    }
}

const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// A temp dir that looks like a checkout (has `.git`), with a config inside.
fn checkout() -> (tempfile::TempDir, PathBuf) {
    let d = tempfile::tempdir().unwrap();
    fs::create_dir(d.path().join(".git")).unwrap();
    fs::create_dir(d.path().join("cfg")).unwrap();
    let cfg = d.path().join("cfg/forjar.yaml");
    fs::write(&cfg, "version: \"1.0\"\n").unwrap();
    (d, cfg)
}

fn origin_git() -> FakeGit {
    FakeGit::default()
        .with("remote", Ok("origin\n"))
        .with("fetch --quiet origin main", Ok(""))
        .with("rev-parse --verify refs/remotes/origin/main", Ok(SHA_A))
}

#[test]
fn not_a_checkout_is_not_guarded() {
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("forjar.yaml");
    let git = FakeGit::default();
    assert_eq!(check_checkout_head(&cfg, None, &git), Ok(None));
    assert!(
        git.calls.borrow().is_empty(),
        "no git call outside a checkout"
    );
}

#[test]
fn checkout_root_is_found_from_a_nested_config() {
    let (d, cfg) = checkout();
    let root = find_checkout_root(&cfg).unwrap();
    assert_eq!(root, d.path().canonicalize().unwrap());
}

#[test]
fn bare_relative_config_resolves_from_cwd() {
    // "forjar.yaml" has an empty parent; it must resolve as ".", not panic.
    let _ = find_checkout_root(Path::new("forjar.yaml"));
}

#[test]
fn a_repo_without_origin_is_not_a_clone() {
    let (_d, cfg) = checkout();
    let git = FakeGit::default().with("remote", Ok("upstream"));
    assert_eq!(check_checkout_head(&cfg, None, &git), Ok(None));
    assert!(!git.called("fetch --quiet origin main"));
}

#[test]
fn head_at_origin_main_is_allowed_after_a_fetch() {
    let (_d, cfg) = checkout();
    let git = origin_git().with("rev-parse --verify HEAD", Ok(SHA_A));
    assert_eq!(check_checkout_head(&cfg, None, &git), Ok(None));
    assert!(git.called("fetch --quiet origin main"), "must fetch first");
}

#[test]
fn head_behind_origin_main_is_refused_with_both_shas_and_counts() {
    let (_d, cfg) = checkout();
    let git = origin_git()
        .with("rev-parse --verify HEAD", Ok(SHA_B))
        .with("rev-list --count HEAD..refs/remotes/origin/main", Ok("1"))
        .with("rev-list --count refs/remotes/origin/main..HEAD", Ok("0"));
    let err = check_checkout_head(&cfg, None, &git).unwrap_err();
    assert!(err.contains("bbbbbbbbbbbb"), "{err}");
    assert!(err.contains("aaaaaaaaaaaa"), "{err}");
    assert!(err.contains("1 commit(s) behind, 0 ahead"), "{err}");
    assert!(err.contains(PR_WORKTREE_ENV), "{err}");
}

#[test]
fn a_failed_fetch_is_a_refusal_not_a_pass() {
    let (_d, cfg) = checkout();
    let git = FakeGit::default()
        .with("remote", Ok("origin"))
        .with("fetch --quiet origin main", Err("network down"));
    let err = check_checkout_head(&cfg, None, &git).unwrap_err();
    assert!(
        err.contains("network down") && err.contains("unmeasured"),
        "{err}"
    );
}

#[test]
fn an_unreadable_remote_list_is_a_refusal() {
    let (_d, cfg) = checkout();
    let git = FakeGit::default().with("remote", Err("corrupt repo"));
    assert!(check_checkout_head(&cfg, None, &git).is_err());
}

#[test]
fn unresolvable_origin_main_is_a_refusal() {
    let (_d, cfg) = checkout();
    let git = FakeGit::default()
        .with("remote", Ok("origin"))
        .with("fetch --quiet origin main", Ok(""))
        .with("rev-parse --verify HEAD", Ok(SHA_B));
    assert!(check_checkout_head(&cfg, None, &git).is_err());
}

#[test]
fn a_failed_rev_list_is_a_refusal() {
    let (_d, cfg) = checkout();
    let git = origin_git().with("rev-parse --verify HEAD", Ok(SHA_B));
    assert!(check_checkout_head(&cfg, None, &git).is_err());
}

#[test]
fn a_pr_number_names_the_worktree_and_skips_the_fetch() {
    let (_d, cfg) = checkout();
    let git = origin_git().with("rev-parse --verify HEAD", Ok(SHA_B));
    for name in ["845", "#845"] {
        let note = check_checkout_head(&cfg, Some(name), &git)
            .unwrap()
            .unwrap();
        assert!(
            note.contains(name) && note.contains("bbbbbbbbbbbb"),
            "{note}"
        );
    }
    assert!(!git.called("fetch --quiet origin main"));
}

#[test]
fn the_named_branch_must_be_the_branch_head_is_on() {
    let (_d, cfg) = checkout();
    let git = origin_git()
        .with("rev-parse --verify HEAD", Ok(SHA_B))
        .with("rev-parse --abbrev-ref HEAD", Ok("fix/x"));
    let note = check_checkout_head(&cfg, Some("fix/x"), &git).unwrap();
    assert!(note.unwrap().contains("PR worktree fix/x"));
    let err = check_checkout_head(&cfg, Some("other"), &git).unwrap_err();
    assert!(err.contains("is on 'fix/x'"), "{err}");
    // "#" alone is not a PR number: it is a branch name that does not match.
    assert!(check_checkout_head(&cfg, Some("#"), &git).is_err());
}

#[test]
fn an_unreadable_branch_is_a_refusal() {
    let (_d, cfg) = checkout();
    let git = origin_git().with("rev-parse --verify HEAD", Ok(SHA_B));
    assert!(check_checkout_head(&cfg, Some("fix/x"), &git).is_err());
}

#[test]
fn a_blank_name_is_no_exception() {
    let (_d, cfg) = checkout();
    let git = origin_git()
        .with("rev-parse --verify HEAD", Ok(SHA_B))
        .with("rev-list --count HEAD..refs/remotes/origin/main", Ok("2"))
        .with("rev-list --count refs/remotes/origin/main..HEAD", Ok("0"));
    assert!(check_checkout_head(&cfg, Some("  "), &git).is_err());
}

#[test]
fn short_tolerates_a_short_sha() {
    assert_eq!(short("abc"), "abc");
    assert_eq!(short(SHA_A), "aaaaaaaaaaaa");
}

#[test]
fn system_git_reports_success_and_failure() {
    let d = tempfile::tempdir().unwrap();
    let git = SystemGit { program: "git" };
    let ok = git.run(d.path(), &["--version"]).unwrap();
    assert!(ok.starts_with("git version"), "{ok}");
    let err = git
        .run(d.path(), &["rev-parse", "--verify", "HEAD"])
        .unwrap_err();
    assert!(err.starts_with("git rev-parse --verify HEAD:"), "{err}");
    let missing = git
        .run(&d.path().join("no-such-dir"), &["status"])
        .unwrap_err();
    assert!(missing.starts_with("git status"), "{missing}");
}

#[test]
fn entry_point_allows_a_config_outside_any_checkout() {
    let d = tempfile::tempdir().unwrap();
    assert_eq!(check_apply_checkout(&d.path().join("f.yaml")), Ok(()));
}

#[test]
fn a_missing_git_binary_is_an_error_not_a_pass() {
    let d = tempfile::tempdir().unwrap();
    let git = SystemGit {
        program: "forjar-845-no-such-git",
    };
    let err = git.run(d.path(), &["remote"]).unwrap_err();
    assert!(err.starts_with("git remote:"), "{err}");
}

#[test]
fn a_config_under_a_missing_dir_is_not_in_a_checkout() {
    let cfg = Path::new("/nonexistent-forjar-845/cfg/forjar.yaml");
    assert_eq!(find_checkout_root(cfg), None);
}

#[test]
fn an_unresolvable_head_is_a_refusal() {
    let (_d, cfg) = checkout();
    assert!(check_checkout_head(&cfg, None, &origin_git()).is_err());
    assert!(check_checkout_head(&cfg, Some("845"), &origin_git()).is_err());
}

#[test]
fn a_failed_ahead_count_is_a_refusal() {
    let (_d, cfg) = checkout();
    let git = origin_git()
        .with("rev-parse --verify HEAD", Ok(SHA_B))
        .with("rev-list --count HEAD..refs/remotes/origin/main", Ok("1"));
    assert!(check_checkout_head(&cfg, None, &git).is_err());
}
