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
    // #334: and it must SAY the pass deletes, and grant the opt-in explicitly.
    // The reaper previews by default; a bare invocation here would install the
    // budget and then converge nothing.
    assert!(
        s.contains("FORJAR_BUDGET_EXECUTE=1 '/usr/local/sbin/forjar-disk-budget-root.sh'"),
        "apply's pass must grant the reclaim opt-in: {s}"
    );
    assert!(s.contains("EXECUTE mode (this deletes)"));
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

/// FJ-036: every script this handler emits must survive forjar's own I8
/// purification gate.
///
/// This is the test that was missing when `disk_budget` first shipped. All 53
/// unit tests and 9 falsification tests passed while `forjar apply` rejected
/// the resource outright with `I8 violation — script failed bashrs validation`:
/// nothing in the suite ran the generated shell through the same gate that
/// production does. Six distinct violations were only visible at apply time —
/// a missing `rm -rf` guard (SEC011), `continue` inside a piped `while` that
/// bashrs cannot see as a loop (SC2242), `date +%s` (DET002), git args whose
/// quoting is invisible through a nested command substitution (SEC002), a
/// `/proc/[0-9]*` glob read as a test expression, and the literal words `[tag]`
/// and `done:` inside log strings.
#[test]
fn every_emitted_script_passes_the_purifier() {
    use crate::core::purifier::validate_script;

    let mut r = res();
    r.budget_reclaim = vec![
        ReclaimRule {
            name: "scratchpads".into(),
            roots: vec!["/tmp/claude-1000".into()],
            kind: ReclaimKind::ClaudeScratchpad,
            min_idle_minutes: 180,
        },
        ReclaimRule {
            name: "fixtures".into(),
            roots: vec!["/tmp/.tmp*".into()],
            kind: ReclaimKind::Glob,
            min_idle_minutes: 1440,
        },
        ReclaimRule {
            name: "worktrees".into(),
            roots: vec!["/home/x/src/repo/.claude/worktrees".into()],
            kind: ReclaimKind::AbandonedWorktree,
            min_idle_minutes: 720,
        },
        ReclaimRule {
            name: "targets".into(),
            roots: vec!["/home/x/src".into()],
            kind: ReclaimKind::CargoTarget,
            min_idle_minutes: 240,
        },
    ];

    for (what, script) in [
        ("apply", apply_script(&r)),
        ("check", check_script(&r)),
        ("state_query", state_query_script(&r)),
    ] {
        assert!(
            validate_script(&script).is_ok(),
            "{what}_script fails I8 purification, so `forjar apply` will reject \
             this resource on every machine:\n{:?}",
            validate_script(&script).unwrap_err()
        );
    }

    // ...including the removal path.
    let absent = Resource {
        state: Some("absent".into()),
        ..r
    };
    assert!(validate_script(&apply_script(&absent)).is_ok());
}

#[test]
fn status_file_is_valid_json() {
    // The path reaches the status body inside a JSON string, so it needs JSON
    // quoting, not shell quoting. Emitting `{"path":'/'}` produced a status
    // file no JSON parser could read — invisible to the reaper's own sed-based
    // reads, and to every text-matching test.
    let s = apply_script(&res());
    assert!(
        s.contains(r#""path":"/""#),
        "status path must be JSON-quoted, not shell-quoted"
    );
    assert!(!s.contains(r#""path":'/'"#));
}

#[test]
fn state_query_hashes_the_deployed_artifacts() {
    // A regenerated reaper (forjar upgrade, edited reclaim rule) must show up
    // as drift. Without the script sha in the state query, the hash is built
    // only from runtime classes, so `apply` reports "unchanged" and the machine
    // keeps running the OLD reaper indefinitely — the silent desync this whole
    // resource exists to remove, reintroduced one level up.
    let s = state_query_script(&res());
    for k in [
        "disk_budget_script_sha=",
        "disk_budget_unit_sha=",
        "disk_budget_timer_sha=",
    ] {
        assert!(s.contains(k), "state query must hash the deployed {k}");
    }
    assert!(s.contains("sha256sum /usr/local/sbin/forjar-disk-budget-root.sh"));
}

#[test]
fn health_is_judged_against_the_trigger_not_the_target() {
    // Between target (80% used) and trigger (85%) lies the hysteresis band —
    // where a healthy machine spends most of its life: reclaim ran, took usage
    // under the target, and usage has crept back without being due again.
    // Judging health against the TARGET reports a permanently over-budget
    // fleet, and a permanently-red signal is one everyone learns to ignore.
    let c = check_script(&res());
    assert!(
        c.contains(r#"[ "$USED" -lt 85 ]"#),
        "check must use the trigger"
    );
    assert!(!c.contains("-le 80"), "check must not use the target");

    let q = state_query_script(&res());
    assert!(
        q.contains(r#"[ "$USED" -ge 85 ]"#),
        "state_query tier must use the trigger"
    );
}

#[test]
fn state_query_and_reaper_agree_on_tier() {
    // Same machine, same numbers, two code paths: if they diverge, `forjar
    // drift` and the status file contradict each other and neither is trusted.
    let r = res();
    let q = state_query_script(&r);
    let a = apply_script(&r);
    let high = budget_of(&r).unwrap().high_watermark_pct;
    let crit = budget_of(&r).unwrap().critical_free_gb;
    // Both classify `critical` on free-GiB against the same threshold...
    assert!(q.contains(&format!("-lt {crit}")));
    assert!(a.contains(&format!("FB_CRIT_GB={crit}")));
    // ...and `pressure` at the same trigger.
    assert!(q.contains(&format!("-ge {high}")));
    assert!(a.contains(&format!("FB_HIGH={high}")));
}

#[test]
fn systemctl_probes_emit_exactly_one_line_each() {
    // `systemctl is-active`/`is-failed` print a state AND exit non-zero for most
    // states, so `$(cmd || echo unknown)` captures both and injects a bare
    // `unknown` line into the drift-hashed stdout. Every emitted key must be
    // exactly one `key=value` line.
    let s = state_query_script(&res());
    // Line-scoped, and comments excluded: the pattern being banned is also the
    // thing the comment above it describes.
    for line in s.lines().filter(|l| !l.trim_start().starts_with('#')) {
        assert!(
            !(line.contains("systemctl is-") && line.contains("|| echo")),
            "systemctl probe appends a fallback to captured output, which emits \
             a second bare line into the drift-hashed stdout:\n  {line}"
        );
    }
    assert!(s.contains("| head -1"));
}
