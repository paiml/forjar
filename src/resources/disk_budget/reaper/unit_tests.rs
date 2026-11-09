use super::*;
use crate::core::types::{ReclaimKind, DEFAULT_SCHEDULE};

fn budget(rules: Vec<ReclaimRule>) -> DiskBudget {
    DiskBudget::new("/", 85, 20, 50, DEFAULT_SCHEDULE, rules).unwrap()
}

fn rule() -> ReclaimRule {
    ReclaimRule {
        name: "agent-targets".into(),
        roots: vec!["/home/noah/src/*".into()],
        kind: ReclaimKind::CargoTarget,
        min_idle_minutes: 60,
    }
}

#[test]
fn triggered_pass_that_misses_target_exits_nonzero() {
    let s = script(&budget(vec![rule()]), "/run/x.json", "budget");
    assert!(
        s.contains(r#"[ "$FB_TRIGGERED" = "1" ] && [ "$FB_MET_FINAL" != "1" ]"#),
        "missing the anti-inertness exit"
    );
    assert!(s.contains("exit 1"));
}

#[test]
fn reclaimed_nothing_under_pressure_is_flagged_inert() {
    let s = script(&budget(vec![rule()]), "/run/x.json", "budget");
    assert!(s.contains("FB_HEALTH=inert"));
    assert!(s.contains(r#"[ "$FB_RECLAIMED" -eq 0 ]"#));
}

#[test]
fn reads_df_before_and_after_each_delete() {
    let s = script(&budget(vec![rule()]), "/run/x.json", "budget");
    // once at start, once per candidate, once at the end
    assert!(
        s.matches("fb_read_df").count() >= 3,
        "must re-read df as it deletes"
    );
}

#[test]
fn stops_as_soon_as_target_is_met() {
    let s = script(&budget(vec![rule()]), "/run/x.json", "budget");
    assert!(s.contains(r#"[ "$FB_USED_PCT" -le "$FB_TARGET_USED" ]"#));
    assert!(s.contains("break"));
}

#[test]
fn honours_the_idle_floor_per_rule() {
    let mut r = rule();
    r.min_idle_minutes = 45;
    let s = script(&budget(vec![r]), "/run/x.json", "budget");
    assert!(s.contains("fb_is_idle \"$cand\" 45"));
}

/// forjar#667: ONE in-use check guards every kind, before the idle check and the rm.
/// The match is exhaustive, so a new `ReclaimKind` fails to compile here until added.
#[test]
fn every_kind_is_checked_for_use_before_idle_and_rm() {
    for kind in [
        ReclaimKind::CargoTarget,
        ReclaimKind::ClaudeScratchpad,
        ReclaimKind::AbandonedWorktree,
        ReclaimKind::Glob,
    ] {
        match kind {
            ReclaimKind::CargoTarget
            | ReclaimKind::ClaudeScratchpad
            | ReclaimKind::AbandonedWorktree
            | ReclaimKind::Glob => {}
        }
        let mut r = rule();
        r.kind = kind;
        let s = script(&budget(vec![r]), "/run/x.json", "budget");
        let in_use = s.find("fb_in_use \"$cand\"").expect("central in-use check");
        let idle = s.find("fb_is_idle \"$cand\"").expect("idle check");
        let rm = s.find("rm -rf -- \"$cand\"").expect("delete branch");
        assert!(
            in_use < idle && idle < rm,
            "{kind:?}: in-use must precede idle and rm"
        );
    }
}

/// forjar#667: exe and mapped files count as holding a dir.
#[test]
fn open_path_scan_reads_exe_and_maps() {
    let s = script(&budget(vec![rule()]), "/run/x.json", "budget");
    assert!(s.contains("/proc/*/exe"), "exe link not scanned");
    assert!(s.contains("/proc/*/maps"), "maps not scanned");
}

/// #334: a reaper reached by hand — or by a `sudo`/`ssh` hop that scrubbed
/// the environment — must inspect, not delete.
#[test]
fn deletes_only_on_explicit_opt_in() {
    let s = script(&budget(vec![rule()]), "/run/x.json", "budget");
    let dry = s.find("DRY-RUN would reclaim").expect("dry-run branch");
    let rm = s.find("rm -rf -- \"$cand\"").expect("delete branch");
    assert!(
        dry < rm,
        "dry-run must be the guarded branch, not a fallthrough"
    );
    // The default is DRY. Deleting is an opt-in the environment must grant.
    assert!(s.contains("\nFB_DRY=1\n"), "the default must be dry: {s}");
    assert!(s.contains(r#"if [ "${FORJAR_BUDGET_EXECUTE:-0}" = "1" ]; then FB_DRY=0; fi"#));
    // The documented variable still works, and still wins.
    assert!(s.contains(r#"if [ "${FORJAR_BUDGET_DRY_RUN:-0}" = "1" ]; then FB_DRY=1; fi"#));
    assert!(
        !s.contains(r#"FB_DRY="${FORJAR_BUDGET_DRY_RUN:-0}""#),
        "the fail-dangerous default must be gone"
    );
}

/// The ledger is FREED bytes. A preview that appended to it reported
/// `reclaimed_bytes` > 0 and `health=effective` for deletions that never
/// happened, which is what made a preview and a reclaim byte-identical.
#[test]
fn the_ledger_append_is_inside_the_delete_branch() {
    let s = script(&budget(vec![rule()]), "/run/x.json", "budget");
    let ledger = s
        .find(r#"echo "${sz:-0}" >>"$FB_LEDGER""#)
        .expect("ledger append");
    let close = s
        .find("\n    fi\n  done < \"$FB_CANDS\"")
        .expect("branch end");
    assert!(
        ledger < close,
        "the ledger append must be inside the else branch, not after the fi"
    );
    // The dry branch records what it WOULD free, separately.
    assert!(s.contains(r#"echo "${sz:-0}" >>"$FB_WOULD""#));
}

/// A preview must not stamp a health record for deletions that did not
/// happen: the status file is the heartbeat AND the drift-hashed health.
#[test]
fn a_preview_does_not_write_the_heartbeat_or_fail_the_unit() {
    let s = script(&budget(vec![rule()]), "/run/x.json", "budget");
    assert!(
        s.contains("if [ \"$FB_DRY\" != \"1\" ]; then\ncat >\"$FB_STATUS\""),
        "the status write must be guarded by the mode"
    );
    assert!(
        s.contains(r#"if [ "$FB_DRY" != "1" ] && [ "$FB_TRIGGERED" = "1" ]"#),
        "the anti-inertness exit must exempt a preview"
    );
}

/// A dry run and a reclaim printed identical output. Naming the mode on the
/// start and completion lines is what makes them distinguishable in a log.
#[test]
fn both_log_lines_name_the_mode() {
    let s = script(&budget(vec![rule()]), "/run/x.json", "budget");
    assert!(s.contains(r#"if [ "$FB_DRY" = "1" ]; then FB_MODE=dry-run; else FB_MODE=execute; fi"#));
    assert_eq!(
        s.matches("mode=$FB_MODE").count(),
        2,
        "start and complete must both carry the mode"
    );
    assert!(s.contains("would_reclaim ${FB_WOULD_BYTES} bytes"));
}

#[test]
fn writes_status_json_every_run() {
    let s = script(&budget(vec![rule()]), "/run/budget-root.json", "budget");
    assert!(s.contains("/run/budget-root.json"));
    // No `last_run` epoch: the status file's own mtime IS the heartbeat, so
    // there is no clock arithmetic to keep in sync and no `date` call —
    // which forjar's I8 purification gate rejects as non-deterministic
    // (DET002). state_query reads freshness with `find -mmin` instead.
    assert!(!s.contains("date +%s"), "reaper must not call date");
    for field in [
        "reclaimed_bytes",
        "triggered",
        "target_met",
        "tier",
        "health",
    ] {
        assert!(
            s.contains(&format!("\"{field}\"")),
            "status missing {field}"
        );
    }
}

#[test]
fn target_used_is_derived_from_free_target() {
    // 20% free target => stop at 80% used.
    let s = script(&budget(vec![rule()]), "/run/x.json", "budget");
    assert!(s.contains("FB_TARGET_USED=80"));
    assert!(s.contains("FB_HIGH=85"));
}

#[test]
fn rules_are_emitted_in_declaration_order() {
    let a = ReclaimRule {
        name: "first".into(),
        ..rule()
    };
    let b = ReclaimRule {
        name: "second".into(),
        kind: ReclaimKind::Glob,
        ..rule()
    };
    let s = script(&budget(vec![a, b]), "/run/x.json", "budget");
    assert!(s.find("rule: first").unwrap() < s.find("rule: second").unwrap());
}

#[test]
fn no_rules_still_produces_a_runnable_script() {
    let s = script(&budget(vec![]), "/run/x.json", "budget");
    assert!(s.starts_with("#!/bin/sh"));
    assert!(s.contains("fb_read_df"));
}

#[test]
fn paths_are_shell_quoted() {
    let b = DiskBudget::new("/mnt/a b", 85, 20, 50, "hourly", vec![]).unwrap();
    let s = script(&b, "/run/x.json", "budget");
    assert!(s.contains("'/mnt/a b'"), "path must be quoted");
}

#[test]
fn declares_itself_forjar_managed() {
    let s = script(&budget(vec![]), "/run/x.json", "budget");
    assert!(s.contains("DO NOT EDIT"));
}
