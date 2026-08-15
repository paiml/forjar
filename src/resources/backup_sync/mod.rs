//! FJ-037: `backup_sync` — an offsite copy that has to prove it exists.
//!
//! # Problem
//!
//! Measured on lambda-labs, 2026-08-15: ~2.1 TB of irreplaceable media on a
//! 4-wide RAID0 with no parity, and **zero bytes** of it anywhere off that
//! array. An hourly job had been reporting `Backup complete` for months. It was
//! deployed, enabled, and exiting 0 the whole time.
//!
//! It failed in three ways at once, and each is now unrepresentable:
//!
//!   1. Its destination was local — `/videos`, a symlink back to its own
//!      source — so rsync copied a directory onto itself. `backup_remote` is
//!      rejected at parse time unless it is an rclone `remote:path`.
//!   2. Its success metric could not fail: `find "$DEST"` on a symlink returns
//!      nothing without `-L`, so it printed `Files: 0` while 77 matching files
//!      sat in that exact directory. Health here is a count of files verified
//!      present in the remote **by checksum**, and zero examined files is an
//!      error rather than a pass.
//!   3. Nothing compared coverage to the source. Coverage is now a percentage
//!      of source files proven present, and a run below the declared threshold
//!      exits non-zero.
//!
//! # Design note: `apply` does not run the sync
//!
//! [`crate::resources::disk_budget`] deliberately runs its reaper during apply,
//! because a budget should converge immediately. This resource deliberately
//! does **not**. Two reasons, one practical and one about evidence:
//!
//!   * Seeding terabytes takes days (Google Drive caps uploads at ~750 GB per
//!     day per account); an apply that blocks on it would never return.
//!   * If the deployer runs the job, the deployer writes the status file that
//!     is supposed to be evidence the *service* ran. The observability signal
//!     would be manufactured by the thing being observed. `apply` installs and
//!     arms the timer; the first real pass is systemd's, and `state_query`
//!     reads the journal to confirm that.
//!
//! # YAML
//!
//! ```yaml
//! media-backup:
//!   type: backup_sync
//!   machine: lambda-labs
//!   remote: "gdrive:lambda-labs-media"
//!   remote_type: drive
//!   remote_config:
//!     scope: drive.file
//!   token: "{{secrets.rclone-gdrive-token}}"
//!   schedule: daily
//!   verify_pct: 99
//!   source:
//!     - /mnt/nvme-raid0/RecordedCourses
//!     - /mnt/nvme-raid0/home-Videos
//! ```

use crate::core::shell_escape::sh_squote;
use crate::core::types::{BackupSync, Resource};
use crate::core::types::{DEFAULT_BACKUP_SCHEDULE, DEFAULT_DAILY_CAP_GB, DEFAULT_VERIFY_PCT};

mod config;
mod preflight;
mod sync;
mod units;

#[cfg(test)]
mod tests;

/// Heartbeat window as a multiple of the cadence.
const STALE_AFTER_MISSED_RUNS: u64 = 3;

/// Resolve a validated [`BackupSync`] from a resource declaration.
///
/// # Errors
///
/// Returns `Err` when sources are missing/relative/overlapping, or the remote
/// is not a valid `remote:path`.
pub fn backup_of(resource: &Resource) -> Result<BackupSync, String> {
    let remote = resource.backup.remote.as_deref().ok_or_else(|| {
        "backup_sync requires `backup_remote` (an rclone remote:path)".to_string()
    })?;
    BackupSync::new(
        resource.backup.source.clone(),
        remote,
        resource
            .backup
            .schedule
            .as_deref()
            .unwrap_or(DEFAULT_BACKUP_SCHEDULE),
        resource.backup.verify_pct.unwrap_or(DEFAULT_VERIFY_PCT),
        resource.backup.daily_cap_gb.unwrap_or(DEFAULT_DAILY_CAP_GB),
        resource.backup.bandwidth_limit.clone(),
    )
}

fn slug(remote: &str) -> String {
    let s: String = remote
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let t = s.trim_matches('-').to_string();
    if t.is_empty() {
        "backup".to_string()
    } else {
        t
    }
}

fn script_path(remote: &str) -> String {
    format!("/usr/local/sbin/forjar-backup-{}.sh", slug(remote))
}
fn service_name(remote: &str) -> String {
    format!("forjar-backup-{}", slug(remote))
}
fn service_path(remote: &str) -> String {
    format!("/etc/systemd/system/{}.service", service_name(remote))
}
fn timer_path(remote: &str) -> String {
    format!("/etc/systemd/system/{}.timer", service_name(remote))
}
fn status_json(remote: &str) -> String {
    format!("/run/forjar-backup-{}.json", slug(remote))
}

fn home_of(resource: &Resource) -> String {
    resource.home.clone().unwrap_or_else(|| "/root".to_string())
}

fn reject(msg: &str) -> String {
    format!("echo {} >&2; exit 1", sh_squote(&format!("ERROR: {msg}")))
}

fn stale_secs(schedule: &str) -> u64 {
    let period = match schedule {
        "hourly" => 3600,
        "daily" => 86400,
        "weekly" => 604_800,
        _ => 86400,
    };
    period * STALE_AFTER_MISSED_RUNS
}

/// Check: is a verified backup currently in place?
pub fn check_script(resource: &Resource) -> String {
    let cfg = match backup_of(resource) {
        Ok(c) => c,
        Err(e) => return reject(&e),
    };
    let scr = script_path(&cfg.remote);
    let svc = service_path(&cfg.remote);
    let status = sh_squote(&status_json(&cfg.remote));
    format!(
        "set -u\n\
         if [ ! -f {scr} ] || [ ! -f {svc} ]; then echo 'absent'; exit 0; fi\n\
         HE=\"$(sed -n 's/.*\"health\":\"\\([a-z][a-z]*\\)\".*/\\1/p' {status} 2>/dev/null | head -1)\"\n\
         if [ \"${{HE:-none}}\" = \"verified\" ]; then echo 'present'; else echo 'unverified'; fi\n"
    )
}

/// Apply: install config + script + units, arm the timer. Does NOT sync.
pub fn apply_script(resource: &Resource) -> String {
    let cfg = match backup_of(resource) {
        Ok(c) => c,
        Err(e) => return reject(&e),
    };
    if resource.state.as_deref() == Some("absent") {
        return remove_script(&cfg);
    }
    if let Some(t) = resource.backup.token.as_deref() {
        if config::is_unresolved(t) {
            return reject(&format!(
                "backup_token is still an unresolved template ({t}) — the secrets provider \
                 did not supply it. Writing this would set the remote's credential to that \
                 literal string and fail later as an auth error."
            ));
        }
    }

    let home = home_of(resource);
    let name = service_name(&cfg.remote);
    let scr = script_path(&cfg.remote);
    let scr_q = sh_squote(&scr);
    let body = sync::script(&cfg, &status_json(&cfg.remote), &name);
    let stanza = config::stanza(
        resource,
        cfg.remote_name(),
        resource.backup.token.as_deref(),
    );
    let svc = units::service_unit(&scr, &cfg.remote);
    let tmr = units::timer_unit(&cfg.schedule, &cfg.remote);
    let svc_q = sh_squote(&service_path(&cfg.remote));
    let tmr_q = sh_squote(&timer_path(&cfg.remote));

    format!(
        r#"set -eu
{conf_install}
mkdir -p /usr/local/sbin
cat > {scr_q} <<'FORJAR_BACKUP_EOF'
{body}
FORJAR_BACKUP_EOF
chmod 0755 {scr_q}

cat > {svc_q} <<'FORJAR_BACKUP_SVC'
{svc}
FORJAR_BACKUP_SVC

cat > {tmr_q} <<'FORJAR_BACKUP_TMR'
{tmr}
FORJAR_BACKUP_TMR

systemctl daemon-reload
systemctl enable {name}.timer >/dev/null 2>&1 || true
systemctl restart {name}.timer
# The sync is NOT run here. Seeding takes days, and a deployer that runs the
# job also writes the status file that is supposed to prove the SERVICE ran.
echo 'backup_sync armed; first pass runs on the timer'
"#,
        conf_install = config::install(&home, &stanza),
    )
}

fn remove_script(cfg: &BackupSync) -> String {
    let name = service_name(&cfg.remote);
    let scr = sh_squote(&script_path(&cfg.remote));
    let svc = sh_squote(&service_path(&cfg.remote));
    let tmr = sh_squote(&timer_path(&cfg.remote));
    // Stop and disable BEFORE removing the files (PMAT-219): deleting a unit
    // file out from under a loaded unit leaves it Active: failed with
    // "Unit to trigger vanished", invisible to a converged-looking apply.
    format!(
        "set -u\n\
         systemctl stop {name}.timer {name}.service >/dev/null 2>&1 || true\n\
         systemctl disable {name}.timer >/dev/null 2>&1 || true\n\
         rm -f {scr} {svc} {tmr}\n\
         systemctl daemon-reload || true\n\
         systemctl reset-failed {name}.timer {name}.service >/dev/null 2>&1 || true\n"
    )
}

/// State query: drift-visible health classes, verified against the journal.
pub fn state_query_script(resource: &Resource) -> String {
    let cfg = match backup_of(resource) {
        Ok(c) => c,
        Err(e) => return reject(&e),
    };
    let name = service_name(&cfg.remote);
    let scr = script_path(&cfg.remote);
    let status = sh_squote(&status_json(&cfg.remote));
    let stale_min = stale_secs(&cfg.schedule) / 60;
    let conf = sh_squote(&config::conf_path(&home_of(resource)));

    format!(
        "set -u\n\
         echo \"backup_installed=$([ -f {scr} ] && echo yes || echo no)\"\n\
         echo \"backup_conf_sha=$( (sha256sum {conf} 2>/dev/null || echo missing) | awk '{{print $1}}')\"\n\
         TMR_STATE=\"$(systemctl is-active {name}.timer 2>/dev/null | head -1)\"\n\
         UNIT_STATE=\"$(systemctl is-failed {name}.service 2>/dev/null | head -1)\"\n\
         echo \"backup_timer=${{TMR_STATE:-unknown}}\"\n\
         echo \"backup_unit=${{UNIT_STATE:-unknown}}\"\n\
         # Execution evidence comes from the JOURNAL, not from a status file the\n\
         # deployer could have written. No journal entries => never ran.\n\
         if journalctl -u {name}.service -n 1 --no-pager 2>/dev/null | grep -q .; then\n\
         \x20 echo 'backup_ever_ran=yes'\n\
         else\n\
         \x20 echo 'backup_ever_ran=no'\n\
         fi\n\
         HE=\"$(sed -n 's/.*\"health\":\"\\([a-z][a-z]*\\)\".*/\\1/p' {status} 2>/dev/null | head -1)\"\n\
         CV=\"$(sed -n 's/.*\"coverage_pct\":\\([0-9][0-9]*\\).*/\\1/p' {status} 2>/dev/null | head -1)\"\n\
         MI=\"$(sed -n 's/.*\"missing\":\\([0-9][0-9]*\\).*/\\1/p' {status} 2>/dev/null | head -1)\"\n\
         AGED=\"$(find {status} -mmin +{stale_min} 2>/dev/null)\"\n\
         if [ ! -f {status} ]; then echo 'backup_heartbeat=missing'\n\
         elif [ -n \"$AGED\" ]; then echo 'backup_heartbeat=stale'\n\
         else echo 'backup_heartbeat=fresh'; fi\n\
         echo \"backup_health=${{HE:-unknown}}\"\n\
         # Volatile counters -> stderr only, never into the drift hash.\n\
         echo \"backup_coverage_pct=${{CV:-0}} backup_missing=${{MI:-0}}\" >&2\n"
    )
}
