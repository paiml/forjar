//! Tests for the disk-budget detection shell.

use super::*;

#[test]
fn cargo_detector_accepts_either_real_layout() {
    // Measured on lambda-labs 2026-08-16 across a 4.6 TB targets/ tree:
    //   targets/<repo>                  .rustc_info.json, NO CACHEDIR.TAG
    //   targets/<repo>/<arch-triple>    CACHEDIR.TAG, NO .rustc_info.json
    //   BOTH markers: 0 of 16
    // Requiring the conjunction matched nothing and the reaper reported
    // health=inert while the array sat at 94%.
    let s = cargo_target();
    assert!(
        s.contains(r#"if [ -f "$d/.rustc_info.json" ]; then"#),
        ".rustc_info.json alone must be sufficient"
    );
    assert!(
        s.contains(r#"[ -f "$d/CACHEDIR.TAG" ] && { [ -d "$d/debug" ] || [ -d "$d/release" ]; }"#),
        "CACHEDIR.TAG must be accepted when a build-output subdir is present"
    );
}

#[test]
fn cargo_detector_still_excludes_the_registry() {
    // ~/.cargo/registry carries CACHEDIR.TAG and no .rustc_info.json, and
    // its children are src/ cache/ index/ — no debug/ or release/. The
    // build-output requirement is the only thing keeping the reaper out of
    // it now that CACHEDIR.TAG alone can qualify a directory.
    let s = cargo_target();
    let tag_branch = s
        .find(r#"[ -f "$d/CACHEDIR.TAG" ]"#)
        .expect("tag branch present");
    let rest = &s[tag_branch..];
    assert!(
        rest.starts_with(r#"[ -f "$d/CACHEDIR.TAG" ] && {"#),
        "CACHEDIR.TAG must never qualify a directory on its own"
    );
    assert!(rest.contains(r#"-d "$d/debug""#) && rest.contains(r#"-d "$d/release""#));
}

#[test]
fn cargo_detector_never_matches_by_name() {
    // Name-matching is the bug this replaces: it missed `.target` (189G)
    // and would eat the `cc` crate's source `src/target/`.
    let s = cargo_target();
    assert!(
        !s.contains("-name target"),
        "detection must not key on names"
    );
    assert!(!s.contains("-name '.target'"));
}

#[test]
fn worktree_detector_fails_closed_without_upstream() {
    let s = abandoned_worktree();
    assert!(s.contains("@{upstream}"));
    assert!(s.contains("rev-list --count"));
    assert!(s.contains("status --porcelain"));
    // Every guard must `continue` (keep), never fall through to delete.
    assert!(s.contains("|| continue"));
}

#[test]
fn worktree_detector_skips_primary_checkout() {
    // A primary checkout has a .git DIRECTORY; only linked worktrees (.git
    // file) are ever candidates.
    assert!(abandoned_worktree().contains("[ -f \"$d/.git\" ] || continue"));
}

#[test]
fn scratchpad_detector_has_both_liveness_signals() {
    let s = claude_scratchpad();
    assert!(s.contains("fb_in_use"), "open-handle signal missing");
    assert!(s.contains("fb_live_slugs"), "project-slug signal missing");
}

#[test]
fn every_kind_has_a_detector_definition() {
    let all = all_detectors();
    for k in [
        ReclaimKind::CargoTarget,
        ReclaimKind::ClaudeScratchpad,
        ReclaimKind::AbandonedWorktree,
        ReclaimKind::Glob,
    ] {
        assert!(
            all.contains(&format!("{}()", fn_name(k))),
            "no detector emitted for {k}"
        );
    }
}

#[test]
fn only_worktrees_prune_after_delete() {
    assert!(post_delete(ReclaimKind::AbandonedWorktree).contains("worktree prune"));
    assert_eq!(post_delete(ReclaimKind::CargoTarget), "");
    assert_eq!(post_delete(ReclaimKind::Glob), "");
    assert_eq!(post_delete(ReclaimKind::ClaudeScratchpad), "");
}

#[test]
fn prune_targets_the_repo_not_the_pool_directory() {
    // `dirname "$cand"` is the worktree POOL, which is not a repository.
    // For an in-repo pool git walks up and it accidentally works; for a
    // sibling pool (~/src/aprender-worktrees) it fails and git's registry
    // accumulates stale entries forever.
    let post = post_delete(ReclaimKind::AbandonedWorktree);
    assert!(
        !post.contains("dirname"),
        "prune must not be run from the pool directory: {post}"
    );
    assert!(post.contains("--git-dir \"$FB_REPO_DIR\""));
}

#[test]
fn repo_dir_is_captured_before_the_worktree_is_deleted() {
    // The path can only be resolved while the worktree still exists.
    let pre = pre_delete(ReclaimKind::AbandonedWorktree);
    assert!(pre.contains("--git-common-dir"));
    assert!(pre.contains("FB_REPO_DIR="));
    // Other kinds need no capture.
    assert_eq!(pre_delete(ReclaimKind::CargoTarget), "");
    assert_eq!(pre_delete(ReclaimKind::Glob), "");
    assert_eq!(pre_delete(ReclaimKind::ClaudeScratchpad), "");
}

#[test]
fn candidates_are_emitted_oldest_first() {
    // Each detector stamps mtime then `sort -n | cut -f2-`.
    for s in [
        cargo_target(),
        claude_scratchpad(),
        abandoned_worktree(),
        glob(),
    ] {
        assert!(s.contains("sort -n | cut -f2-"), "not oldest-first: {s}");
    }
}

#[test]
fn prelude_defines_every_helper_the_detectors_call() {
    let p = prelude();
    for helper in [
        "fb_is_idle",
        "fb_bytes",
        "fb_in_use",
        "fb_stamp",
        "fb_scan_open_paths",
    ] {
        assert!(
            p.contains(&format!("{helper}()")),
            "missing helper {helper}"
        );
    }
}

// fb_sweepable guard tests live in their own file (500-line cap).
#[path = "detect_sweepable_tests.rs"]
mod sweepable_tests;
