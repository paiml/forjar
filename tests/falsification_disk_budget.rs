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

use forjar::core::types::{MachineTarget, ReclaimKind, ReclaimRule, Resource, ResourceType};
use forjar::resources::disk_budget;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Unique temp dir (no external tempfile dep in this test target).
fn tmpdir(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "forjar-budget-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&base).unwrap();
    base
}

/// A directory that looks exactly like a cargo target dir to the detector.
fn mk_cargo_target(p: &Path) {
    fs::create_dir_all(p.join("debug")).unwrap();
    fs::write(p.join("CACHEDIR.TAG"), "Signature: 8a477f597d28d172").unwrap();
    fs::write(p.join(".rustc_info.json"), "{}").unwrap();
    fs::write(p.join("debug/blob"), vec![0u8; 4096]).unwrap();
}

/// Install a `df` shim that pops one "used_pct free_kb" line per invocation.
fn install_df_stub(bin: &Path, sequence: &[(u32, u64)]) {
    fs::create_dir_all(bin).unwrap();
    let state = bin.join("df.state");
    let mut s = String::new();
    for (used, free_kb) in sequence {
        s.push_str(&format!("{used} {free_kb}\n"));
    }
    fs::write(&state, s).unwrap();
    let shim = format!(
        "#!/bin/sh\n\
         S={state:?}\n\
         L=$(head -1 \"$S\")\n\
         if [ \"$(wc -l <\"$S\")\" -gt 1 ]; then tail -n +2 \"$S\" >\"$S.tmp\" && mv \"$S.tmp\" \"$S\"; fi\n\
         set -- $L\n\
         echo 'Filesystem 1024-blocks Used Available Capacity Mounted on'\n\
         echo \"/dev/stub 1000000000 1 $2 $1% /\"\n"
    );
    let p = bin.join("df");
    fs::write(&p, shim).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
}

fn budget_resource(path: &Path, rules: Vec<ReclaimRule>) -> Resource {
    Resource {
        resource_type: ResourceType::DiskBudget,
        machine: MachineTarget::Single("test".into()),
        path: Some(path.to_string_lossy().into_owned()),
        budget_high_watermark_pct: Some(85),
        budget_target_free_pct: Some(20),
        budget_reclaim: rules,
        ..Default::default()
    }
}

/// Extract the reaper body from the apply script and run it directly.
struct RunResult {
    code: i32,
    stdout: String,
}

/// Pull the reaper body out of the apply script's heredoc.
fn reaper_body(res: &Resource) -> String {
    const OPEN: &str = "<<'FORJAR_REAPER_EOF'\n";
    const CLOSE: &str = "\nFORJAR_REAPER_EOF\n";
    let apply = disk_budget::apply_script(res);
    let start = apply.find(OPEN).expect("reaper heredoc open") + OPEN.len();
    let end = apply[start..].find(CLOSE).expect("reaper heredoc close") + start;
    apply[start..end].to_string()
}

fn run_reaper(res: &Resource, bin: &Path, work: &Path) -> RunResult {
    let body = reaper_body(res);

    let script = work.join("reaper.sh");
    fs::write(&script, &body).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let path_env = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("/bin/sh")
        .arg(&script)
        .env("PATH", path_env)
        .env("FORJAR_BUDGET_STATUS", work.join("status.json"))
        .output()
        .expect("run reaper");
    RunResult {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
    }
}

/// Backdate everything under `p` so the idle floor is satisfied.
fn backdate(p: &Path) {
    let _ = Command::new("find")
        .args([
            p.to_str().unwrap(),
            "-exec",
            "touch",
            "-d",
            "3 days ago",
            "{}",
            "+",
        ])
        .output();
}

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
