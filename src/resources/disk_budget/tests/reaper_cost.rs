use super::*;

// ---------------------------------------------------------------------------
// forjar#727 (PMAT-727): the reaper's own cost.
//
// Measured on lambda-labs 2026-10-20: ONE pass of the 15-minute timer took 84 s
// of CPU and 2 min 46 s wall, for a unit that is Nice=10 / idle-IO. The cost was
// structural: a `readlink` launched per /proc entry and per fd, a `tr|head`
// launched per pid, an unbounded `find` over the whole scan root, and a
// 15-minute cadence declared by the host. These tests read the generated script
// STRUCTURALLY, so each one goes red when its old shape comes back.
// ---------------------------------------------------------------------------

fn reaper_with_all_kinds() -> String {
    let mut r = res();
    r.budget_reclaim = vec![
        ReclaimRule {
            name: "targets".into(),
            roots: vec!["/home/x/src".into()],
            kind: ReclaimKind::CargoTarget,
            min_idle_minutes: 60,
        },
        ReclaimRule {
            name: "scratch".into(),
            roots: vec!["/tmp/claude-1000".into()],
            kind: ReclaimKind::ClaudeScratchpad,
            min_idle_minutes: 60,
        },
    ];
    reaper_script(&r).expect("reaper script")
}

/// The body of shell function `name` (from its definition to the first line
/// that is exactly `}`).
fn shell_fn<'a>(script: &'a str, name: &str) -> &'a str {
    let start = script
        .find(&format!("\n{name}() {{"))
        .unwrap_or_else(|| panic!("function {name} not emitted"));
    let rest = &script[start + 1..];
    let end = rest.find("\n}\n").expect("function end");
    &rest[..end]
}

fn code_lines(s: &str) -> Vec<&str> {
    s.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .collect()
}

#[test]
fn proc_scan_launches_no_process_per_pid_or_fd() {
    let s = reaper_with_all_kinds();
    let scan = shell_fn(&s, "fb_scan_open_paths");
    let body = code_lines(scan).join("\n");
    // The old shape: a loop over /proc/* that ran readlink once per entry.
    assert!(
        !s.contains("readlink"),
        "a per-entry readlink is the old O(pids x fds) process-launch cost"
    );
    assert!(
        !code_lines(&s)
            .iter()
            .any(|l| l.starts_with("for p in /proc")),
        "no shell loop may walk /proc entry by entry"
    );
    assert!(!s.contains("tr '\\0'"), "no per-pid tr over cmdline");
    // One pass, one tool.
    assert_eq!(
        body.matches("find /proc/*/cwd /proc/*/fd").count(),
        1,
        "the whole scan is ONE find over /proc: {body}"
    );
    assert!(body.contains("-printf '%p\\t%l\\n'"));
}

#[test]
fn claude_session_scan_reuses_the_single_proc_pass() {
    let s = reaper_with_all_kinds();
    let finder = shell_fn(&s, "fb_find_claude_scratchpad");
    assert!(
        !finder.contains("/proc"),
        "the scratchpad finder must not re-walk /proc: {finder}"
    );
    let scan = shell_fn(&s, "fb_scan_open_paths");
    // comm is read with the `read` builtin, never cat/tr/head.
    assert!(scan.contains("<\"${fb_src%/cwd}/comm\""));
}

#[test]
fn every_find_is_bounded_and_stays_on_the_filesystem() {
    let s = reaper_with_all_kinds();
    let bounded = shell_fn(&s, "fb_bounded");
    assert!(bounded.contains("timeout -k 5 \"$FB_FIND_TIMEOUT\""));
    assert!(bounded.contains("nice -n 19"));
    assert!(bounded.contains("ionice -c3"));
    assert!(s.contains("FB_FIND_TIMEOUT=\"${FORJAR_BUDGET_FIND_TIMEOUT:-120}\""));
    for l in code_lines(&s) {
        // A bare `find` statement is unbounded. (`fb_bounded find` and the
        // /proc scan, which is itself fb_bounded, do not start with `find`.)
        assert!(
            !l.starts_with("find "),
            "unbounded find, wrap it in fb_bounded: {l}"
        );
    }
    let cargo = shell_fn(&s, "fb_find_cargo_target");
    assert!(cargo.contains("fb_bounded find \"$root\" -xdev"), "{cargo}");
    let idle = shell_fn(&s, "fb_is_idle");
    assert!(idle.contains("fb_bounded find \"$1\" -xdev"), "{idle}");
}

#[test]
fn a_timeout_means_not_measured_and_nothing_is_deleted() {
    let s = reaper_with_all_kinds();
    // (1) find over a root: timeout discards the walk.
    let cargo = shell_fn(&s, "fb_find_cargo_target");
    assert!(cargo.contains("fb_timed_out \"$fb_rc\""));
    assert!(cargo.contains("FB_UNMLOG"));
    // (2) idle check: timeout => not idle => keep.
    let idle = shell_fn(&s, "fb_is_idle");
    assert!(idle.contains("NOT MEASURED, keeping"));
    assert!(idle.contains("return 1"));
    // (3) /proc scan: timeout or empty => FB_OPEN_OK stays 0, in_use answers yes.
    let scan = shell_fn(&s, "fb_scan_open_paths");
    assert!(scan.contains("NOT MEASURED, nothing proven dead"));
    let in_use = shell_fn(&s, "fb_in_use");
    assert!(in_use.contains("[ \"$FB_OPEN_OK\" = \"1\" ] || return 0"));
    // (4) the rule loop only runs on a completed scan.
    assert!(s.contains("if [ \"$FB_MET\" != \"1\" ] && [ \"$FB_OPEN_OK\" = \"1\" ]; then"));
    assert!(s.contains("nothing proven dead, deleting nothing"));
}

#[test]
fn default_cadence_is_hourly_and_not_every_fifteen_minutes() {
    let r = res();
    let apply = apply_script(&r);
    assert!(
        apply.contains("OnCalendar=hourly"),
        "default must be hourly"
    );
    assert!(
        !apply.contains("*:0/15"),
        "the 15-minute cadence must be an explicit declaration, never the default"
    );
    assert_eq!(crate::core::types::DEFAULT_SCHEDULE, "hourly");
    // The old value stays reachable by declaration.
    let mut fast = res();
    fast.budget_schedule = Some("*:0/15".into());
    assert!(apply_script(&fast).contains("OnCalendar=*:0/15"));
}
