//! Unit tests for the detector prelude (split from detect.rs for the 500-line cap).

use super::*;

/// Run the real `fb_sweepable` from the emitted prelude under `sh`.
fn sweepable(cand: &str, prefixes: &[&str]) -> bool {
    let script = format!("fb_log() {{ :; }}\n{}\nfb_sweepable \"$@\"", prelude());
    std::process::Command::new("sh")
        .arg("-c")
        .arg(script)
        .arg("sh")
        .arg(cand)
        .args(prefixes)
        .status()
        .expect("sh runs")
        .success()
}

#[test]
fn sweepable_accepts_a_direct_child_of_tmp_under_a_tmp_root() {
    // forjar#627: a rule rooted at `/tmp/*-target` matches depth-2 paths,
    // and the old depth >= 3 floor refused every one of them, so the
    // rule validated, ran, and reclaimed nothing.
    let dir = std::path::Path::new("/tmp").join(format!("fj627-{}-target", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.to_str().unwrap().to_string();
    let ok = sweepable(&p, &["/tmp"]);
    std::fs::remove_dir_all(&dir).ok();
    assert!(ok, "{p} under declared prefix /tmp must be sweepable");
}

#[test]
fn sweepable_refuses_a_candidate_outside_every_declared_root() {
    // The comment in the reaper claimed this re-check existed; it did not.
    let here = env!("CARGO_MANIFEST_DIR");
    assert!(!sweepable(here, &["/tmp"]), "{here} is not under /tmp");
    assert!(!sweepable(here, &[]), "no declared root means no deletion");
}

#[test]
fn sweepable_refuses_the_root_itself_and_top_level_dirs() {
    assert!(
        !sweepable("/tmp", &["/tmp"]),
        "a root is not its own descendant"
    );
    assert!(!sweepable("/home", &["/"]), "`/` is never a usable prefix");
    let home = std::env::var("HOME").unwrap();
    assert!(!sweepable(&home, &["/home"]), "$HOME itself is never swept");
}

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
