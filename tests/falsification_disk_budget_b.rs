//! FJ-036: falsification tests for the disk-budget reaper (part 2 of 2).
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
use std::process::Command;

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
            max_age: None,
            idle_days: None,
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
            max_age: None,
            idle_days: None,
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
            max_age: None,
            idle_days: None,
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
