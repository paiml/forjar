//! PMAT-165: `scripts/publish-from-tag.sh` must publish the workspace to
//! crates.io only from a DETACHED worktree of a release tag — never from the
//! ambient working tree, and never with `--allow-dirty`. Publishing from the
//! working tree lets untracked state (agent memory, other worktrees, scratch
//! files) dirty the tree cargo sees, and `--allow-dirty` has been the
//! standing temptation to paper over that.
//!
//! WHY A SHIM `cargo` AND NOT THE REAL ONE. This suite must never touch
//! crates.io, and must prove negatives (no registry token reaches the publish
//! step; no scratch file exists in the worktree cargo publishes from) that
//! only an in-process observer can attest to. A shim first on PATH logs, per
//! invocation: argv, cwd, whether `CARGO_REGISTRY_TOKEN` was visible,
//! `git rev-parse --git-common-dir`, the live worktree count,
//! `git status --porcelain` line count, and `ls -a` of its cwd.
//!
//! WHY `--template=` ON `git init`. This machine's global `init.templateDir`
//! installs a pre-commit quality-gate hook into every freshly initialised
//! repository — including this test's throwaway sandbox — and that hook
//! fails outright before the first commit exists. `--template=` (empty)
//! skips it. `TMPDIR` is likewise per sandbox, so a crashed test leaks no
//! worktree into the shared `/tmp`.
//!
//! MUTATION GUARD (PMAT-187). The guard is not `--detach`: `git worktree add`
//! detaches at a tag anyway. The guard is that cargo runs inside a WORKTREE
//! OF THIS REPO. Replacing `git worktree add` with `git clone` or `cp -r`
//! yields a cwd that still looks tag-shaped and still dry-runs clean, so case
//! (c) pins the two facts only a worktree can satisfy: the shim's
//! `git rev-parse --git-common-dir` resolves to the sandbox repo's own
//! `.git`, and `git worktree list` shows exactly two entries WHILE cargo runs
//! (one after the script exits). A clone reports its own `.git` and one
//! worktree; a `cp -r` of the repo reports the copy's `.git`. Both go RED.

// The sandbox repo and the shim `cargo` that observes it live in the harness
// module; only the falsification cases live here.
#[path = "publish_from_tag_harness/mod.rs"]
mod harness;

use harness::{field, stderr, worktree_count, Sandbox, META_CHAIN, META_DEV_BACK_EDGE};
use std::fs;

#[test]
fn a_nonexistent_tag_refuses_before_any_cargo_call() {
    let sb = Sandbox::new();
    let (out, text) = sb.run("v9.9.9").go("a");
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    assert!(
        text.is_empty(),
        "the shim was called for a tag that does not exist"
    );
}

#[test]
fn b_a_version_mismatch_refuses_before_any_cargo_call() {
    let sb = Sandbox::new();
    sb.tag_mismatched("v0.0.2");
    let (out, text) = sb.run("v0.0.2").go("b");
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    assert!(
        text.is_empty(),
        "the shim was called for a tag whose Cargo.toml version disagrees with the tag"
    );
}

#[test]
fn c_dry_run_publishes_only_from_a_worktree_of_this_repo() {
    let sb = Sandbox::new();
    let (out, text) = sb.run("v0.0.1").dry_run().go("c");
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    assert!(
        text.contains("ARGV: publish --dry-run --locked -p demo"),
        "no dry-run publish was logged:\n{text}"
    );
    assert!(
        !text.contains("ARGV: publish --locked -p demo"),
        "a real (non-dry-run) publish ran under DRY_RUN=1:\n{text}"
    );

    let repo_path = sb.path().display().to_string();
    for cwd in field(&text, "CWD") {
        assert_ne!(
            cwd, repo_path,
            "a cargo call ran in the repo path instead of a worktree:\n{text}"
        );
    }

    // PMAT-187: a clone or a `cp -r` would give cargo a cwd that is not a
    // worktree of THIS repo. Both facts below are unforgeable by either.
    let git_dir = fs::canonicalize(sb.path().join(".git"))
        .expect("canonicalize .git")
        .display()
        .to_string();
    let common_dirs = field(&text, "GITCOMMONDIR");
    assert!(!common_dirs.is_empty(), "the shim logged nothing:\n{text}");
    for seen in &common_dirs {
        assert_eq!(
            seen, &git_dir,
            "cargo ran outside a worktree of the sandbox repo:\n{text}"
        );
    }
    for seen in field(&text, "WORKTREES") {
        assert_eq!(
            seen, "2",
            "the tag checkout was not a second worktree of this repo while cargo ran:\n{text}"
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
    let (out, text) = sb.run("v0.0.1").dry_run().go("d");
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(
        text.contains("ARGV: publish --dry-run --locked -p demo"),
        "the dirty ambient checkout stopped the worktree from dry-running clean"
    );
}

#[test]
fn e_the_registry_token_never_reaches_the_shim() {
    let sb = Sandbox::new();
    let (out, text) = sb.run("v0.0.1").go("e");
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    let lines: Vec<&str> = text.lines().collect();
    let mut publish_calls = 0;
    for w in lines.windows(3) {
        if w[0].starts_with("ARGV: publish") {
            publish_calls += 1;
            assert_eq!(
                w[2], "TOKEN: <unset>",
                "CARGO_REGISTRY_TOKEN reached the publish step:\n{text}"
            );
        }
    }
    assert!(publish_calls > 0, "no publish calls were logged:\n{text}");
}

#[test]
fn f_no_scratch_file_ever_lands_in_the_publish_worktree() {
    // PMAT-184: the script's own metadata/order scratch files must live
    // OUTSIDE the tag worktree. Written inside it they are untracked files in
    // the tree `cargo publish` inspects, and cargo refuses to package an
    // uncommitted tree without `--allow-dirty` — the exact flag this whole
    // pattern exists to avoid.
    let sb = Sandbox::new();
    let (out, text) = sb.run("v0.0.1").dry_run().go("f");
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    let listings = field(&text, "LSCWD");
    assert!(!listings.is_empty(), "the shim logged no directory listing");
    for entry in &listings {
        assert!(
            !entry.contains(".publish-"),
            "a scratch file was present in the publish worktree:\n{text}"
        );
    }
    for status in field(&text, "STATUS") {
        assert_eq!(
            status, "0",
            "the worktree was dirty while cargo ran — cargo publish would refuse it:\n{text}"
        );
    }
}

#[test]
fn g_the_index_poll_waits_until_cargo_info_confirms_the_exact_version() {
    // PMAT-185: two failing `cargo info` answers for alpha@0.1.0 must not let
    // beta (which depends on alpha) publish early; the third, succeeding
    // answer releases it.
    let sb = Sandbox::new();
    let (out, text) = sb
        .run("v0.0.1")
        .metadata(META_CHAIN)
        .info_mode("after:2")
        .go("g");
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    let alpha_polls = text
        .lines()
        .filter(|l| l.starts_with("ARGV: info alpha@0.1.0"))
        .count();
    assert_eq!(
        alpha_polls, 3,
        "the poll did not retry exactly twice before the index answered:\n{text}"
    );
    assert!(
        text.contains("ARGV: publish --locked -p alpha"),
        "alpha never published:\n{text}"
    );
    assert!(
        text.contains("ARGV: publish --locked -p beta"),
        "beta never published after the index caught up:\n{text}"
    );
}

#[test]
fn h_an_index_that_never_catches_up_exits_2_before_the_dependent() {
    // PMAT-185: the poll is bounded. When the bound is hit the script exits 2
    // naming the crate and version, and the dependent is never published.
    let sb = Sandbox::new();
    let (out, text) = sb
        .run("v0.0.1")
        .metadata(META_CHAIN)
        .info_mode("never")
        .go("h");
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    assert!(
        stderr(&out).contains("alpha 0.1.0"),
        "the refusal did not name the crate and version:\n{}",
        stderr(&out)
    );
    assert!(
        text.contains("ARGV: publish --locked -p alpha"),
        "alpha never published:\n{text}"
    );
    assert!(
        !text.contains("-p beta"),
        "beta was published (or dry-run) although alpha never reached the index:\n{text}"
    );
}

#[test]
fn i_the_cargo_search_fallback_matches_the_exact_name_not_a_substring() {
    // PMAT-185: where `cargo info` does not exist, the fallback parses
    // `cargo search`. An unanchored substring match treats a DIFFERENT
    // crate's line (`other-demo = "0.0.1"`, which contains `demo = "0.0.1"`)
    // as proof that demo 0.0.1 is already published, and silently skips it.
    let sb = Sandbox::new();
    let (out, text) = sb
        .run("v0.0.1")
        .dry_run()
        .info_mode("absent")
        .search("other-demo = \"0.0.1\"    # a decoy")
        .go("i");
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(
        text.contains("ARGV: search demo --limit 1"),
        "the script did not fall back to cargo search:\n{text}"
    );
    assert!(
        !text.contains("ARGV: info demo@"),
        "the script queried `cargo info` although this cargo has no such command:\n{text}"
    );
    assert!(
        text.contains("ARGV: publish --dry-run --locked -p demo"),
        "another crate's search line was mistaken for demo 0.0.1:\n{text}"
    );
}

#[test]
fn j_a_dev_dependency_back_edge_is_not_a_cycle() {
    // PMAT-186: `a_app` depends on `b_core`; `b_core` DEV-depends back on
    // `a_app`. Dev-dependencies are not publish-order edges (cargo resolves
    // them only for `cargo test`), so the order is b_core then a_app — not a
    // refusal.
    let sb = Sandbox::new();
    let (out, text) = sb
        .run("v0.0.1")
        .dry_run()
        .metadata(META_DEV_BACK_EDGE)
        .go("j");
    assert!(
        out.status.success(),
        "a dev-dependency back-edge was reported as a cycle: {}",
        stderr(&out)
    );
    let order: Vec<String> = text
        .lines()
        .filter_map(|l| l.strip_prefix("ARGV: publish --dry-run --locked -p "))
        .map(str::to_string)
        .collect();
    assert_eq!(
        order,
        vec!["b_core".to_string(), "a_app".to_string()],
        "the dependency-only publish order is wrong:\n{text}"
    );
}
