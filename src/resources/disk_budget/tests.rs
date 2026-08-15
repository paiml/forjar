//! FJ-036: disk_budget handler tests.

use super::*;
use crate::core::types::{MachineTarget, ReclaimKind, ReclaimRule, ResourceType};

fn res() -> Resource {
    Resource {
        resource_type: ResourceType::DiskBudget,
        machine: MachineTarget::Single("lambda-labs".into()),
        path: Some("/".into()),
        budget_reclaim: vec![ReclaimRule {
            name: "idle-build-dirs".into(),
            roots: vec!["/home/noah/src".into()],
            kind: ReclaimKind::CargoTarget,
            min_idle_minutes: 60,
        }],
        ..Default::default()
    }
}

#[test]
fn budget_uses_documented_defaults() {
    let b = budget_of(&res()).unwrap();
    assert_eq!(b.high_watermark_pct, 85);
    assert_eq!(b.target_free_pct, 20);
    assert_eq!(b.critical_free_gb, 50);
    assert_eq!(b.schedule, "hourly");
}

#[test]
fn defaults_are_hourly_not_daily() {
    // A daily pass cannot defend a box that can burn 250G/day — which is
    // exactly how lambda-labs went 370G-free to 1.2G-free between two runs.
    assert_eq!(budget_of(&res()).unwrap().schedule, "hourly");
}

#[test]
fn missing_path_is_rejected_not_defaulted() {
    let r = Resource {
        path: None,
        ..res()
    };
    assert!(budget_of(&r).is_err());
    // ...and the emitted script refuses loudly rather than doing something.
    let s = apply_script(&r);
    assert!(s.contains("ERROR"));
    assert!(s.contains("exit 1"));
}

#[test]
fn incoherent_watermarks_are_rejected_at_codegen() {
    let r = Resource {
        budget_high_watermark_pct: Some(85),
        budget_target_free_pct: Some(10), // => 90% used, above trigger
        ..res()
    };
    assert!(budget_of(&r).is_err());
    assert!(apply_script(&r).contains("hysteresis"));
}

#[test]
fn slug_is_filesystem_safe() {
    assert_eq!(slug("/"), "root");
    assert_eq!(slug("/mnt/nvme-raid0"), "mnt-nvme-raid0");
    assert_eq!(slug("/home/noah"), "home-noah");
}

#[test]
fn artifact_paths_are_namespaced_per_filesystem() {
    // Two budgets on one machine must not collide.
    assert_ne!(script_path("/"), script_path("/mnt/nvme-raid0"));
    assert_ne!(service_path("/"), service_path("/mnt/nvme-raid0"));
    assert_ne!(status_json("/"), status_json("/mnt/nvme-raid0"));
}

#[test]
fn apply_installs_script_units_and_runs_a_pass() {
    let s = apply_script(&res());
    assert!(s.contains("/usr/local/sbin/forjar-disk-budget-root.sh"));
    assert!(s.contains("/etc/systemd/system/forjar-disk-budget-root.service"));
    assert!(s.contains("/etc/systemd/system/forjar-disk-budget-root.timer"));
    assert!(s.contains("systemctl enable forjar-disk-budget-root.timer"));
    // apply must CONVERGE the budget, not merely schedule a future attempt
    assert!(
        s.trim_end().ends_with("forjar-disk-budget-root.sh'"),
        "apply must run one pass: {}",
        s.lines().last().unwrap_or("")
    );
}

#[test]
fn apply_restarts_timer_on_content_change() {
    let s = apply_script(&res());
    assert!(s.contains("systemctl restart"));
    assert!(s.contains("daemon-reload"));
}

#[test]
fn absent_state_removes_units_and_never_runs_reclaim() {
    let r = Resource {
        state: Some("absent".into()),
        ..res()
    };
    let s = apply_script(&r);
    assert!(s.contains("systemctl disable --now"));
    assert!(s.contains("rm -f"));
    // Removing a budget must not trigger a deletion pass as a side effect.
    assert!(!s.contains("fb_read_df"));
}

#[test]
fn state_query_publishes_classes_not_raw_bytes() {
    let s = state_query_script(&res());
    // Classes on stdout => drift-hashed.
    for k in [
        "disk_budget_tier=",
        "disk_budget_heartbeat=",
        "disk_budget_health=",
        "disk_budget_timer=",
    ] {
        assert!(s.contains(k), "missing drift-visible class {k}");
    }
    // Raw volatile numbers must be stderr-only or every machine drifts hourly.
    let raw = s.find("disk_budget_used_pct=").expect("raw line");
    assert!(
        s[raw..].contains(">&2"),
        "raw byte counts must go to stderr, not into the drift hash"
    );
}

#[test]
fn state_query_surfaces_a_failed_unit() {
    assert!(state_query_script(&res()).contains("is-failed"));
}

#[test]
fn heartbeat_window_scales_with_cadence() {
    assert_eq!(stale_secs("hourly"), 3600 * 3);
    assert_eq!(stale_secs("daily"), 86400 * 3);
    assert_eq!(stale_secs("minutely"), 60 * 3);
    // Unknown OnCalendar expressions fall back to hourly, never to zero (which
    // would report every host permanently stale).
    assert_eq!(stale_secs("Mon *-*-* 04:00:00"), 3600 * 3);
}

#[test]
fn check_reports_over_budget_distinctly_from_absent() {
    let s = check_script(&res());
    assert!(s.contains("'absent'"));
    assert!(s.contains("'present'"));
    assert!(s.contains("'over-budget'"));
}

#[test]
fn generated_reaper_is_embedded_in_apply() {
    // The reaper is generated, not shipped as a repo file that can desync from
    // the declaration — the exact failure mode that preceded this resource.
    let s = apply_script(&res());
    assert!(s.contains("FB_TARGET_USED=80"));
    assert!(s.contains("fb_find_cargo_target"));
}

#[test]
fn every_declared_rule_reaches_the_reaper() {
    let r = Resource {
        budget_reclaim: vec![
            ReclaimRule {
                name: "scratch".into(),
                roots: vec!["/tmp/claude-1000".into()],
                kind: ReclaimKind::ClaudeScratchpad,
                min_idle_minutes: 30,
            },
            ReclaimRule {
                name: "worktrees".into(),
                roots: vec!["/home/noah/src/aprender/.claude/worktrees".into()],
                kind: ReclaimKind::AbandonedWorktree,
                min_idle_minutes: 120,
            },
        ],
        ..res()
    };
    let s = apply_script(&r);
    assert!(s.contains("rule: scratch"));
    assert!(s.contains("rule: worktrees"));
    assert!(s.contains("fb_find_claude_scratchpad"));
    assert!(s.contains("fb_find_abandoned_worktree"));
}
