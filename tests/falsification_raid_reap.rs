//! RAID-REAP Phase C: falsification tests for the declared reaper.
//!
//! The reaper rules (14 d tmp TTL, 3 d build output, 7 d idle worktrees, 70%
//! alarm) live in `examples/raid-reap.yaml`. These tests pin them two ways:
//!
//! 1. Text: the generated reaper carries each constant. A mutated copy of the
//!    declaration (14 d -> 140 d) must make the same check go red, so the check
//!    is shown to be able to fail.
//! 2. Execution: the TTL rule is run against a real aged tree. A 15-day entry
//!    is reaped, a 13-day entry is kept, and a deliverable is never deleted.
//!    With the declared 14 d swapped for 140 d the 15-day entry survives, so
//!    the execution check is shown to be able to fail too.
//!
//! The harness (`tests/common/budget_harness.rs`) stubs `df` through PATH and
//! runs the generated script for real.

#[path = "common/budget_harness.rs"]
mod harness;

use forjar::core::types::{ReclaimKind, ReclaimRule};
use forjar::resources::disk_budget;
use harness::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const EXAMPLE: &str = "examples/raid-reap.yaml";

fn example_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(EXAMPLE)
}

/// Parse a declaration file and return its reaper script.
fn reaper_for(config: &Path) -> String {
    let cfg = forjar::core::parser::parse_and_validate(config)
        .unwrap_or_else(|e| panic!("{} does not validate: {e}", config.display()));
    let r = cfg
        .resources
        .get("raid-reap")
        .expect("raid-reap resource declared");
    disk_budget::reaper_script(r).expect("reaper script generates")
}

/// The constants the RAID-REAP design pins, each as the exact text the
/// generated reaper must contain. Returns the first one that is missing.
fn missing_constant(script: &str) -> Option<&'static str> {
    const REQUIRED: [&str; 5] = [
        // R1 tmp TTL 14 d = 20160 min, passed to the ttl_tree finder.
        "fb_find_ttl_tree 20160 ",
        // R2 build output 3 d = 4320 min idle floor.
        "fb_is_idle \"$cand\" 4320 ",
        // R3 worktree idle 7 d = 10080 min idle floor.
        "fb_is_idle \"$cand\" 10080 ",
        // Alarm / high watermark 70%.
        "FB_HIGH=70\n",
        // Target: 35% free => 65% used.
        "FB_TARGET_USED=65\n",
    ];
    REQUIRED.into_iter().find(|c| !script.contains(c))
}

#[test]
fn declared_reaper_carries_the_phase_c_constants() {
    let script = reaper_for(&example_path());
    assert_eq!(
        missing_constant(&script),
        None,
        "the declared RAID-REAP constants are not all in the generated reaper"
    );
}

#[test]
fn mutated_declaration_goes_red_on_the_constant_check() {
    // The falsifier: 14 d -> 140 d in the declaration must fail the check.
    let text = fs::read_to_string(example_path()).expect("read example");
    assert_eq!(
        text.matches("max_age: 14d").count(),
        1,
        "example must declare exactly one 14d TTL"
    );
    let mutant = text.replace("max_age: 14d", "max_age: 140d");
    let root = tmpdir("raid-reap-mutant");
    let cfg = root.join("raid-reap.yaml");
    fs::write(&cfg, mutant).unwrap();
    let script = reaper_for(&cfg);
    let missing = missing_constant(&script);
    assert_eq!(
        missing,
        Some("fb_find_ttl_tree 20160 "),
        "the 140d mutant must be caught by the 14d TTL constant"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn worktree_rule_declares_idle_days_not_a_bare_floor() {
    let cfg = forjar::core::parser::parse_and_validate(&example_path()).expect("validate");
    let r = cfg.resources.get("raid-reap").expect("resource");
    let wt = r
        .budget
        .reclaim
        .iter()
        .find(|x| x.kind == ReclaimKind::AbandonedWorktree)
        .expect("worktree rule");
    assert_eq!(wt.idle_days, Some(7));
    assert!(wt.max_age.is_none());
}

#[test]
fn ttl_rule_without_max_age_is_refused_at_budget_time() {
    let res = harness::budget_resource(
        Path::new("/mnt/nvme-raid0"),
        vec![ReclaimRule {
            name: "tmp-ttl".into(),
            roots: vec!["/mnt/nvme-raid0/tmp".into()],
            kind: ReclaimKind::TtlTree,
            min_idle_minutes: 60,
            max_age: None,
            idle_days: None,
        }],
    );
    let err = disk_budget::reaper_script(&res).expect_err("no max_age must be refused");
    assert!(err.contains("needs max_age"), "{err}");
}

/// Backdate every entry under `p` by `days`, so content looks that old.
fn age(p: &Path, days: u32) {
    let st = Command::new("find")
        .arg(p)
        .args(["-exec", "touch", "-h", "-d"])
        .arg(format!("{days} days ago"))
        .args(["{}", "+"])
        .status()
        .expect("run find/touch");
    assert!(st.success(), "backdating {} failed", p.display());
}

/// Build a tmp tree with a 15-day entry, a 13-day entry and a 20-day entry
/// holding a deliverable, run the TTL rule with `max_age`, and report which
/// entries survive: (old15, new13, deliverable).
fn run_ttl_fixture(max_age: &str) -> (bool, bool, bool, i32) {
    let root = tmpdir(&format!("raid-reap-ttl-{}", max_age.replace(' ', "")));
    let bin = root.join("bin");
    let tmp = root.join("tmp");
    let old = tmp.join("old15");
    let new = tmp.join("new13");
    let deliv = tmp.join("deliv");
    for d in [&old, &new] {
        fs::create_dir_all(d).unwrap();
        fs::write(d.join("blob"), vec![0u8; 16 * 1024]).unwrap();
    }
    fs::create_dir_all(deliv.join("receipts")).unwrap();
    fs::write(deliv.join("clip.mp4"), vec![0u8; 16 * 1024]).unwrap();
    age(&old, 15);
    age(&new, 13);
    age(&deliv, 20);
    // Above the 85% default trigger for the stubbed filesystem, then clear it.
    install_df_stub(&bin, &[(95, 1_000), (95, 1_000), (50, 900_000_000)]);

    let res = budget_resource(
        &root,
        vec![ReclaimRule {
            name: "tmp-ttl".into(),
            roots: vec![tmp.to_string_lossy().into_owned()],
            kind: ReclaimKind::TtlTree,
            min_idle_minutes: 60,
            max_age: Some(max_age.into()),
            idle_days: None,
        }],
    );
    let r = run_reaper(&res, &bin, &root);
    let out = (
        old.exists(),
        new.exists(),
        deliv.join("clip.mp4").exists(),
        r.code,
    );
    fs::remove_dir_all(&root).ok();
    out
}

#[test]
fn ttl_rule_reaps_15_day_entry_keeps_13_day_and_deliverable() {
    let (old15, new13, deliv, code) = run_ttl_fixture("14d");
    assert_eq!(code, 0, "a triggered pass that reaches target must exit 0");
    assert!(!old15, "a 15-day entry must be reaped under a 14d TTL");
    assert!(new13, "a 13-day entry must be kept under a 14d TTL");
    assert!(
        deliv,
        "an entry holding a deliverable must never be deleted"
    );
}

#[test]
fn mutated_ttl_keeps_the_15_day_entry_so_the_execution_check_is_red() {
    // With 14d raised to 140d the same fixture keeps the 15-day entry. The
    // assertion above is therefore sensitive to the declared TTL.
    let (old15, _, deliv, code) = run_ttl_fixture("140d");
    assert_eq!(
        code, 1,
        "140d leaves the pass unreachable: anti-inertness must exit 1"
    );
    assert!(old15, "140d TTL must keep the 15-day entry");
    assert!(deliv);
}
