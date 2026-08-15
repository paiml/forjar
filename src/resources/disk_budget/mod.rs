//! FJ-036: `disk_budget` resource handler — free space as declared state.
//!
//! # Problem
//!
//! Disk exhaustion on a build machine is not an incident that announces itself.
//! On lambda-labs (2026-08-15) `/` reached 100% with 1.2G free while a reaper
//! ran on schedule every night, exited 0 every night, and reclaimed 1.6G in a
//! month. It was deployed. It was enabled. `systemctl` said active. It had a
//! fixed 7-day idle TTL, so on a box whose build trees turn over in two days
//! every candidate it examined was legitimately "recent" and it correctly
//! declined to delete anything, all the way to 100%. It also matched build
//! directories by name (`target`, `target-local`, `target-private`) and so
//! never saw the `.target` dirs holding 189G.
//!
//! Nothing in that system was in a position to notice. The reaper did not read
//! `df`; the machine had no declared budget to be measured against; a
//! successful-but-useless run was indistinguishable from a healthy one.
//!
//! # Model
//!
//! `disk_budget` makes free space a first-class declared property of a machine,
//! the way a package version or a mount point already is:
//!
//! ```yaml
//! root-budget:
//!   type: disk_budget
//!   machine: lambda-labs
//!   path: /
//!   sudo: true
//!   budget_high_watermark_pct: 85    # reclaim triggers here
//!   budget_target_free_pct: 20       # ...and runs until this much is free
//!   budget_critical_free_gb: 50      # below this, drift fails hard
//!   budget_reclaim:
//!     - name: dead-agent-scratchpads
//!       kind: claude_scratchpad
//!       roots: ["/tmp/claude-1000"]
//!     - name: abandoned-agent-worktrees
//!       kind: abandoned_worktree
//!       roots: ["/home/noah/src/aprender/.claude/worktrees"]
//!     - name: idle-build-dirs
//!       kind: cargo_target
//!       roots: ["/home/noah/src"]
//! ```
//!
//! `apply` installs a generated reaper plus its timer and runs one pass.
//! `state_query` publishes the health CLASS — not the raw byte counts, which
//! change every second and would make every machine permanently drifted — so
//! `forjar drift` fails when a machine is over budget, when its reaper has gone
//! stale, or when a triggered pass reclaimed nothing.

use crate::core::shell_escape::sh_squote;
use crate::core::types::{DiskBudget, Resource};
use crate::core::types::{
    DEFAULT_CRITICAL_FREE_GB, DEFAULT_HIGH_WATERMARK_PCT, DEFAULT_SCHEDULE, DEFAULT_TARGET_FREE_PCT,
};

mod detect;
mod reaper;
mod units;

#[cfg(test)]
mod tests;

/// Heartbeat freshness window, as a multiple of the timer period. A reaper that
/// has not written status in this long is `stale` — the silent-desync detector.
const STALE_AFTER_MISSED_RUNS: u64 = 3;

/// Resolve a validated [`DiskBudget`] from a resource's declared fields.
///
/// # Errors
///
/// Returns `Err` when `path` is absent or the watermark pair lacks hysteresis.
pub fn budget_of(resource: &Resource) -> Result<DiskBudget, String> {
    let path = resource
        .path
        .as_deref()
        .ok_or_else(|| "disk_budget requires `path` (the filesystem to budget)".to_string())?;
    DiskBudget::new(
        path,
        resource
            .budget_high_watermark_pct
            .unwrap_or(DEFAULT_HIGH_WATERMARK_PCT),
        resource
            .budget_target_free_pct
            .unwrap_or(DEFAULT_TARGET_FREE_PCT),
        resource
            .budget_critical_free_gb
            .unwrap_or(DEFAULT_CRITICAL_FREE_GB),
        resource
            .budget_schedule
            .as_deref()
            .unwrap_or(DEFAULT_SCHEDULE),
        resource.budget_reclaim.clone(),
    )
}

/// Filesystem-safe slug for a mount path (`/` -> `root`, `/mnt/x` -> `mnt-x`).
fn slug(path: &str) -> String {
    let s: String = path
        .trim_matches('/')
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    if s.is_empty() {
        "root".to_string()
    } else {
        s
    }
}

fn script_path(path: &str) -> String {
    format!("/usr/local/sbin/forjar-disk-budget-{}.sh", slug(path))
}
fn service_name(path: &str) -> String {
    format!("forjar-disk-budget-{}", slug(path))
}
fn service_path(path: &str) -> String {
    format!("/etc/systemd/system/{}.service", service_name(path))
}
fn timer_path(path: &str) -> String {
    format!("/etc/systemd/system/{}.timer", service_name(path))
}
fn status_json(path: &str) -> String {
    format!("/run/forjar-disk-budget-{}.json", slug(path))
}

/// Emit an error script for an invalid declaration.
fn reject(msg: &str) -> String {
    format!("echo {} >&2; exit 1", sh_squote(&format!("ERROR: {msg}")))
}

/// Seconds after which the heartbeat is considered stale, from the cadence.
fn stale_secs(schedule: &str) -> u64 {
    let period = match schedule {
        "minutely" => 60,
        "hourly" => 3600,
        "daily" => 86400,
        "weekly" => 604_800,
        // Unknown/explicit OnCalendar expression: assume hourly for freshness.
        _ => 3600,
    };
    period * STALE_AFTER_MISSED_RUNS
}

/// Check script: report whether the budget is currently satisfied.
pub fn check_script(resource: &Resource) -> String {
    let budget = match budget_of(resource) {
        Ok(b) => b,
        Err(e) => return reject(&e),
    };
    let p = sh_squote(&budget.path);
    let svc = service_path(&budget.path);
    let scr = script_path(&budget.path);
    // The health boundary is the TRIGGER, not the target. Between the two lies
    // the hysteresis band, which is where a healthy machine spends most of its
    // life: reclaim has run, brought usage below the target, and usage has
    // since crept back up without yet being due for another pass. Judging
    // health against the target would report a permanently over-budget fleet
    // and train everyone to ignore it.
    let high = budget.high_watermark_pct;
    format!(
        "set -u\n\
         if [ ! -f {scr} ] || [ ! -f {svc} ]; then echo 'absent'; exit 0; fi\n\
         set -- $(df -P -k {p} 2>/dev/null | awk 'NR==2{{gsub(/%/,\"\",$5); print $5, $4}}')\n\
         USED=\"${{1:-0}}\"; FREEGB=$((${{2:-0}} / 1024 / 1024))\n\
         if [ \"$USED\" -lt {high} ]; then echo 'present'; else echo 'over-budget'; fi\n\
         echo \"used_pct=$USED free_gb=$FREEGB\" >&2\n"
    )
}

/// Apply script: install the reaper + timer, then run one reclaim pass.
pub fn apply_script(resource: &Resource) -> String {
    let budget = match budget_of(resource) {
        Ok(b) => b,
        Err(e) => return reject(&e),
    };
    if resource.state.as_deref() == Some("absent") {
        return remove_script(&budget);
    }

    let scr = script_path(&budget.path);
    let scr_q = sh_squote(&scr);
    let name = service_name(&budget.path);
    let body = reaper::script(&budget, &status_json(&budget.path), &name);
    let svc = units::service_unit(&scr, &budget.path);
    let tmr = units::timer_unit(&budget.schedule, &budget.path);

    format!(
        "set -eu\n\
         # -- reaper script --\n\
         mkdir -p /usr/local/sbin\n\
         NEW_SCRIPT=$(cat <<'FORJAR_REAPER_EOF'\n\
         {body}\n\
         FORJAR_REAPER_EOF\n\
         )\n\
         if [ ! -f {scr_q} ] || [ \"$NEW_SCRIPT\" != \"$(cat {scr_q} 2>/dev/null)\" ]; then\n\
         \x20 printf '%s\\n' \"$NEW_SCRIPT\" >{scr_q}\n\
         fi\n\
         chmod 0755 {scr_q}\n\
         # -- units --\n\
         {svc_install}{tmr_install}\n\
         if [ \"$SVC_CHANGED\" = \"1\" ] || [ \"$TMR_CHANGED\" = \"1\" ]; then\n\
         \x20 systemctl daemon-reload\n\
         fi\n\
         systemctl enable {name}.timer >/dev/null 2>&1 || true\n\
         # Restart (not start) so a unit-content change actually takes effect.\n\
         systemctl restart {name}.timer\n\
         # Run one pass now so `apply` converges the budget instead of merely\n\
         # scheduling it. A missed budget surfaces here, at apply time.\n\
         {scr_q}\n",
        svc_install = units::install_unit(&service_path(&budget.path), &svc, "SVC_CHANGED"),
        tmr_install = units::install_unit(&timer_path(&budget.path), &tmr, "TMR_CHANGED"),
    )
}

/// Removal: stop the timer and delete the generated artifacts.
fn remove_script(budget: &DiskBudget) -> String {
    let name = service_name(&budget.path);
    let scr = sh_squote(&script_path(&budget.path));
    let svc = sh_squote(&service_path(&budget.path));
    let tmr = sh_squote(&timer_path(&budget.path));
    format!(
        "set -u\n\
         systemctl disable --now {name}.timer >/dev/null 2>&1 || true\n\
         rm -f {scr} {svc} {tmr}\n\
         systemctl daemon-reload || true\n"
    )
}

/// State query: publish drift-visible health classes.
///
/// Only CLASSES go to stdout (which is what gets hashed into drift): raw free
/// bytes move constantly, so hashing them would report every machine as drifted
/// on every run and train everyone to ignore drift. The classes are stable
/// while the machine is healthy and flip exactly when it stops being healthy.
/// Raw numbers go to stderr for operators.
pub fn state_query_script(resource: &Resource) -> String {
    let budget = match budget_of(resource) {
        Ok(b) => b,
        Err(e) => return reject(&e),
    };
    let p = sh_squote(&budget.path);
    let status = sh_squote(&status_json(&budget.path));
    let name = service_name(&budget.path);
    // Must match the reaper's own tier logic exactly, or `forjar drift` and the
    // status file disagree about the same machine in the hysteresis band.
    let high = budget.high_watermark_pct;
    let crit = budget.critical_free_gb;
    let stale_min = stale_secs(&budget.schedule) / 60;
    let scr = script_path(&budget.path);
    let svc = service_path(&budget.path);
    let tmr = timer_path(&budget.path);

    format!(
        "set -u\n\
         set -- $(df -P -k {p} 2>/dev/null | awk 'NR==2{{gsub(/%/,\"\",$5); print $5, $4}}')\n\
         USED=\"${{1:-0}}\"; FREEGB=$((${{2:-0}} / 1024 / 1024))\n\
         if [ \"$FREEGB\" -lt {crit} ]; then echo 'disk_budget_tier=critical'\n\
         elif [ \"$USED\" -ge {high} ]; then echo 'disk_budget_tier=pressure'\n\
         else echo 'disk_budget_tier=ok'; fi\n\
         echo \"disk_budget_installed=$([ -f {scr} ] && echo yes || echo no)\"\n\
         # Hash the DEPLOYED reaper. Without this, the state hash is computed\n\
         # only from runtime classes, so regenerating the script (a forjar\n\
         # upgrade, an edited reclaim rule) is invisible: `apply` reports\n\
         # \"unchanged\" and the machine keeps running the OLD reaper forever.\n\
         # That is the same silent-desync this resource exists to eliminate.\n\
         echo \"disk_budget_script_sha=$( (sha256sum {scr} 2>/dev/null || echo missing) | awk '{{print $1}}')\"\n\
         echo \"disk_budget_unit_sha=$( (sha256sum {svc} 2>/dev/null || echo missing) | awk '{{print $1}}')\"\n\
         echo \"disk_budget_timer_sha=$( (sha256sum {tmr} 2>/dev/null || echo missing) | awk '{{print $1}}')\"\n\
         # `systemctl is-active`/`is-failed` PRINT a state and still exit non-zero\n\
         # for most states, so `$(... || echo unknown)` captures BOTH and emits a\n\
         # stray second line into the drift-hashed output. Take the first line\n\
         # and default only when it is genuinely empty.\n\
         TMR_STATE=\"$(systemctl is-active {name}.timer 2>/dev/null | head -1)\"\n\
         UNIT_STATE=\"$(systemctl is-failed {name}.service 2>/dev/null | head -1)\"\n\
         echo \"disk_budget_timer=${{TMR_STATE:-unknown}}\"\n\
         echo \"disk_budget_unit=${{UNIT_STATE:-unknown}}\"\n\
         HE=\"$(sed -n 's/.*\"health\":\"\\([a-z][a-z]*\\)\".*/\\1/p' {status} 2>/dev/null | head -1)\"\n\
         RB=\"$(sed -n 's/.*\"reclaimed_bytes\":\\([0-9][0-9]*\\).*/\\1/p' {status} 2>/dev/null | head -1)\"\n\
         AGED=\"$(find {status} -mmin +{stale_min} 2>/dev/null)\"\n\
         if [ ! -f {status} ]; then echo 'disk_budget_heartbeat=missing'\n\
         elif [ -n \"$AGED\" ]; then echo 'disk_budget_heartbeat=stale'\n\
         else echo 'disk_budget_heartbeat=fresh'; fi\n\
         echo \"disk_budget_health=${{HE:-unknown}}\"\n\
         # raw, volatile values -> stderr only (never drift-hashed)\n\
         echo \"disk_budget_used_pct=$USED disk_budget_free_gb=$FREEGB disk_budget_last_reclaimed=${{RB:-0}}\" >&2\n"
    )
}
