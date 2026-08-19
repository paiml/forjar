//! FJ-036: falsification tests for the disk-budget reaper.
//!
//! These tests EXECUTE the generated shell against a real directory tree and
//! assert on what is left on disk afterwards. They exist because the failure
//! this resource replaces was invisible to exactly the kind of test that greps
//! generated text: the old reaper's script was *correct-looking* and shipped
//! green for months while reclaiming nothing.
//!
//! `df` is stubbed through PATH so a test can drive the watermark logic to any
//! pressure level deterministically. Everything else — `find`, `du`, `stat`,
//! `git`, `rm` — is the real thing, so what is under test is the actual
//! detection and deletion behaviour, not a description of it.
//!
//! The harness lives in `tests/common/budget_harness.rs` (file-health limit).

#[path = "common/budget_harness.rs"]
mod harness;

use forjar::core::types::{ReclaimKind, ReclaimRule};
use harness::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

#[test]
fn reclaims_dot_prefixed_target_dirs() {
    // The 189G case: `.target`, invisible to any name-based sweep.
    let root = tmpdir("dot-target");
    let bin = root.join("bin");
    let src = root.join("src/wt");
    mk_cargo_target(&src.join(".target"));
    backdate(&src);
    install_df_stub(&bin, &[(95, 1_000), (95, 1_000), (50, 900_000_000)]);

    let res = budget_resource(
        &root,
        vec![ReclaimRule {
            name: "targets".into(),
            roots: vec![root.join("src").to_string_lossy().into_owned()],
            kind: ReclaimKind::CargoTarget,
            min_idle_minutes: 60,
        }],
    );
    let r = run_reaper(&res, &bin, &root);
    assert!(
        !src.join(".target").exists(),
        "dot-prefixed .target must be reclaimed\n{}",
        r.stdout
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn never_deletes_a_source_dir_named_target() {
    // The `cc` crate shape: src/target/ with no cargo markers. A name-based
    // sweep eats this and corrupts the cargo registry.
    let root = tmpdir("decoy");
    let bin = root.join("bin");
    let decoy = root.join("src/cc-1.0/src/target");
    fs::create_dir_all(&decoy).unwrap();
    fs::write(decoy.join("mod.rs"), "// source, not build output").unwrap();
    // A registry-shaped dir: CACHEDIR.TAG but no .rustc_info.json.
    let registry = root.join("src/registry");
    fs::create_dir_all(&registry).unwrap();
    fs::write(registry.join("CACHEDIR.TAG"), "Signature: 8a477f597d28d172").unwrap();
    fs::write(registry.join("crate.rs"), "// vendored source").unwrap();
    backdate(&root.join("src"));
    // Stay under pressure the whole run so it tries as hard as it can.
    install_df_stub(&bin, &[(99, 100)]);

    let res = budget_resource(
        &root,
        vec![ReclaimRule {
            name: "targets".into(),
            roots: vec![root.join("src").to_string_lossy().into_owned()],
            kind: ReclaimKind::CargoTarget,
            min_idle_minutes: 60,
        }],
    );
    let r = run_reaper(&res, &bin, &root);
    assert!(
        decoy.join("mod.rs").exists(),
        "a source dir named `target` must never be reclaimed\n{}",
        r.stdout
    );
    assert!(
        registry.join("crate.rs").exists(),
        "CACHEDIR.TAG alone must not make a dir reclaimable (that is the registry)\n{}",
        r.stdout
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn does_nothing_when_under_the_watermark() {
    let root = tmpdir("under");
    let bin = root.join("bin");
    let t = root.join("src/wt/target");
    mk_cargo_target(&t);
    backdate(&root.join("src"));
    install_df_stub(&bin, &[(40, 900_000_000)]);

    let res = budget_resource(
        &root,
        vec![ReclaimRule {
            name: "targets".into(),
            roots: vec![root.join("src").to_string_lossy().into_owned()],
            kind: ReclaimKind::CargoTarget,
            min_idle_minutes: 60,
        }],
    );
    let r = run_reaper(&res, &bin, &root);
    assert_eq!(r.code, 0, "healthy run must exit 0\n{}", r.stdout);
    assert!(
        t.exists(),
        "must not reclaim below the trigger\n{}",
        r.stdout
    );
    assert!(r.stdout.contains("under watermark"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn respects_the_idle_floor() {
    // A target being written right now must survive even under pressure.
    let root = tmpdir("idle");
    let bin = root.join("bin");
    let t = root.join("src/wt/target");
    mk_cargo_target(&t); // fresh mtime, not backdated
    install_df_stub(&bin, &[(99, 100)]);

    let res = budget_resource(
        &root,
        vec![ReclaimRule {
            name: "targets".into(),
            roots: vec![root.join("src").to_string_lossy().into_owned()],
            kind: ReclaimKind::CargoTarget,
            min_idle_minutes: 60,
        }],
    );
    let r = run_reaper(&res, &bin, &root);
    assert!(
        t.exists(),
        "an active build must not be reclaimed\n{}",
        r.stdout
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn triggered_pass_that_cannot_reach_target_exits_nonzero() {
    // THE anti-inertness test. Pressure, nothing reclaimable => must FAIL.
    // The predecessor exited 0 here, every night, for a month.
    let root = tmpdir("inert");
    let bin = root.join("bin");
    fs::create_dir_all(root.join("src")).unwrap();
    install_df_stub(&bin, &[(99, 100)]);

    let res = budget_resource(
        &root,
        vec![ReclaimRule {
            name: "targets".into(),
            roots: vec![root.join("src").to_string_lossy().into_owned()],
            kind: ReclaimKind::CargoTarget,
            min_idle_minutes: 60,
        }],
    );
    let r = run_reaper(&res, &bin, &root);
    assert_eq!(
        r.code, 1,
        "a triggered pass that misses its budget must fail loudly\n{}",
        r.stdout
    );
    assert!(r.stdout.contains("health=inert"), "{}", r.stdout);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn stops_as_soon_as_the_target_watermark_is_met() {
    // Two candidates, but df reports the target met after the first delete:
    // the second must survive. This is the low-watermark stop, and it is what
    // keeps the reaper from deleting the whole box under transient pressure.
    let root = tmpdir("stop");
    let bin = root.join("bin");
    let older = root.join("src/a/target");
    let newer = root.join("src/b/target");
    mk_cargo_target(&older);
    mk_cargo_target(&newer);
    backdate(&root.join("src"));
    // Make `older` strictly older so ordering is deterministic.
    let _ = Command::new("touch")
        .args(["-d", "10 days ago", older.to_str().unwrap()])
        .output();
    // start: 95%, first candidate check: 95%, after first delete: 50% => met.
    install_df_stub(
        &bin,
        &[
            (95, 1_000),
            (95, 1_000),
            (50, 900_000_000),
            (50, 900_000_000),
        ],
    );

    let res = budget_resource(
        &root,
        vec![ReclaimRule {
            name: "targets".into(),
            roots: vec![root.join("src").to_string_lossy().into_owned()],
            kind: ReclaimKind::CargoTarget,
            min_idle_minutes: 60,
        }],
    );
    let r = run_reaper(&res, &bin, &root);
    assert!(!older.exists(), "oldest must go first\n{}", r.stdout);
    assert!(
        newer.exists(),
        "must stop once the target watermark is met\n{}",
        r.stdout
    );
    assert_eq!(r.code, 0, "reaching target must exit 0\n{}", r.stdout);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn abandoned_worktree_with_unpushed_work_is_never_removed() {
    // The one rule that can destroy unrecoverable work. It must fail closed.
    let root = tmpdir("worktree");
    let bin = root.join("bin");
    let repo = root.join("repo");
    fs::create_dir_all(&repo).unwrap();
    let git = |args: &[&str], dir: &Path| {
        Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .expect("git")
    };
    git(&["init", "-q", "-b", "main"], &repo);
    fs::write(repo.join("f.txt"), "hello").unwrap();
    git(&["add", "."], &repo);
    git(&["commit", "-qm", "init"], &repo);

    let pool = root.join("pool");
    fs::create_dir_all(&pool).unwrap();
    let wt = pool.join("feature");
    git(
        &[
            "worktree",
            "add",
            "-q",
            wt.to_str().unwrap(),
            "-b",
            "feature",
        ],
        &repo,
    );
    // `feature` has no upstream => unpushed work => must be protected.
    backdate(&pool);
    install_df_stub(&bin, &[(99, 100)]);

    let res = budget_resource(
        &root,
        vec![ReclaimRule {
            name: "worktrees".into(),
            roots: vec![pool.to_string_lossy().into_owned()],
            kind: ReclaimKind::AbandonedWorktree,
            min_idle_minutes: 60,
        }],
    );
    let r = run_reaper(&res, &bin, &root);
    assert!(
        wt.join("f.txt").exists(),
        "a worktree with no upstream is unpushed work and must be kept\n{}",
        r.stdout
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn abandoned_worktree_is_removed_and_pruned_from_a_sibling_pool() {
    // A pool OUTSIDE the repository (~/src/aprender-worktrees is the real
    // shape). Pruning from `dirname "$cand"` cannot work here — that directory
    // is not a repo and git has nothing to walk up to. If the prune is wrong,
    // the tree is deleted but git keeps a stale registration forever.
    let root = tmpdir("prune");
    let bin = root.join("bin");
    let repo = root.join("repo");
    fs::create_dir_all(&repo).unwrap();
    let git = |args: &[&str], dir: &Path| {
        Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .expect("git")
    };
    // A bare "remote" so the worktree branch can have a real upstream.
    let remote = root.join("remote.git");
    git(&["init", "-q", "--bare", remote.to_str().unwrap()], &root);
    git(&["init", "-q", "-b", "main"], &repo);
    fs::write(repo.join("f.txt"), "hello").unwrap();
    git(&["add", "."], &repo);
    git(&["commit", "-qm", "init"], &repo);
    git(
        &["remote", "add", "origin", remote.to_str().unwrap()],
        &repo,
    );
    git(&["push", "-q", "-u", "origin", "main"], &repo);

    // Sibling pool, deliberately NOT under `repo/`.
    let pool = root.join("repo-worktrees");
    fs::create_dir_all(&pool).unwrap();
    let wt = pool.join("done");
    // A branch tracking origin/main and level with it: clean, has upstream,
    // zero commits ahead — i.e. genuinely abandoned. (`worktree add <p> main`
    // would fail: main is already checked out in the primary worktree.)
    let add = git(
        &[
            "worktree",
            "add",
            "-q",
            "--track",
            "-b",
            "done",
            wt.to_str().unwrap(),
            "origin/main",
        ],
        &repo,
    );
    assert!(
        wt.exists(),
        "setup: worktree add failed: {}",
        String::from_utf8_lossy(&add.stderr)
    );

    assert_eq!(
        String::from_utf8_lossy(&git(&["worktree", "list"], &repo).stdout)
            .lines()
            .count(),
        2,
        "setup: repo + one linked worktree"
    );

    backdate(&pool);
    install_df_stub(&bin, &[(95, 1_000), (95, 1_000), (50, 900_000_000)]);

    let res = budget_resource(
        &root,
        vec![ReclaimRule {
            name: "worktrees".into(),
            roots: vec![pool.to_string_lossy().into_owned()],
            kind: ReclaimKind::AbandonedWorktree,
            min_idle_minutes: 60,
        }],
    );
    let r = run_reaper(&res, &bin, &root);

    assert!(
        !wt.exists(),
        "abandoned worktree must be removed\n{}",
        r.stdout
    );
    let listed = String::from_utf8_lossy(&git(&["worktree", "list"], &repo).stdout).to_string();
    assert!(
        !listed.contains("done"),
        "git's worktree registry must be pruned, not left stale:\n{listed}\n{}",
        r.stdout
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn dry_run_reclaims_nothing() {
    let root = tmpdir("dry");
    let bin = root.join("bin");
    let t = root.join("src/wt/target");
    mk_cargo_target(&t);
    backdate(&root.join("src"));
    install_df_stub(&bin, &[(99, 100)]);

    let res = budget_resource(
        &root,
        vec![ReclaimRule {
            name: "targets".into(),
            roots: vec![root.join("src").to_string_lossy().into_owned()],
            kind: ReclaimKind::CargoTarget,
            min_idle_minutes: 60,
        }],
    );
    let script = root.join("reaper.sh");
    fs::write(&script, reaper_body(&res)).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let out = Command::new("/bin/sh")
        .arg(&script)
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("FORJAR_BUDGET_DRY_RUN", "1")
        .env("FORJAR_BUDGET_STATUS", root.join("status.json"))
        .output()
        .unwrap();
    assert!(
        t.exists(),
        "dry-run must not delete\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn reclaims_a_per_arch_target_dir_that_has_only_cachedir_tag() {
    // The layout that made the reaper inert on a 4.6 TB tree: per-arch
    // subdirectories carry CACHEDIR.TAG and no .rustc_info.json, so a
    // both-markers rule matched nothing at all.
    let root = tmpdir("archdir");
    let bin = root.join("bin");
    let arch = root.join("src/targets/aprender/wasm32-unknown-unknown");
    mk_cargo_target_arch(&arch);
    backdate(&root.join("src"));
    install_df_stub(&bin, &[(95, 1_000), (95, 1_000), (50, 900_000_000)]);

    let res = budget_resource(
        &root,
        vec![ReclaimRule {
            name: "targets".into(),
            roots: vec![root.join("src").to_string_lossy().into_owned()],
            kind: ReclaimKind::CargoTarget,
            min_idle_minutes: 60,
        }],
    );
    let r = run_reaper(&res, &bin, &root);
    assert!(
        !arch.exists(),
        "a per-arch target dir with only CACHEDIR.TAG must be reclaimed\n{}",
        r.stdout
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn never_reclaims_a_cargo_registry() {
    // The registry has CACHEDIR.TAG and no .rustc_info.json — the same marker
    // set as a per-arch target dir. Only the absence of debug/ or release/
    // separates them, so this is the test standing between the reaper and a
    // corrupted registry.
    let root = tmpdir("registry");
    let bin = root.join("bin");
    let reg = root.join("src/registry");
    mk_cargo_registry(&reg);
    backdate(&root.join("src"));
    install_df_stub(&bin, &[(99, 100)]);

    let res = budget_resource(
        &root,
        vec![ReclaimRule {
            name: "targets".into(),
            roots: vec![root.join("src").to_string_lossy().into_owned()],
            kind: ReclaimKind::CargoTarget,
            min_idle_minutes: 60,
        }],
    );
    let r = run_reaper(&res, &bin, &root);
    assert!(
        reg.join("src/lib.rs").exists(),
        "the cargo registry must never be reclaimed\n{}",
        r.stdout
    );
    fs::remove_dir_all(&root).ok();
}
