//! FJ-038: `nas_archive` — archival as declared state, with the delete gated on proof.
//!
//! # Problem
//!
//! Disk **reclaim** is a first-class forjar resource (`disk_budget`, FJ-036)
//! with watermarks and a policy you can read in the YAML. Disk **archival** —
//! its mirror, and the operation that *deletes originals* — was a shell script
//! with its policy in a string literal:
//!
//! ```sh
//! DIRS="${DIRS:-mac-backup mastering-github-coursera-assets ... RecordedCourses}"
//! ```
//!
//! Three consequences, measured on lambda-labs 2026-08-21:
//!
//!   1. Five directories were never archived — `corpus` (21 G),
//!      `entrenar-checkpoints` (13 G), `hf-coursera-assets` (6.3 G),
//!      `albor-data` (6.1 G), `gemma2-models` (1.6 G) — because their names were
//!      not typed into that string. One held 90 files that existed nowhere but a
//!      4-wide RAID0 with no parity.
//!   2. `forjar drift` was structurally blind: declared state was "a file exists
//!      with this sha", so a *wrong* `DIRS` list was perfectly converged.
//!   3. There was no `plan` before a destructive act — an edit reached
//!      `--execute` within one timer cadence, unreviewed.
//!
//! # Design note: `apply` does not move data
//!
//! Same reasoning as [`crate::resources::backup_sync`], for the same two
//! reasons. Moving 755 G over CIFS takes hours, so an apply that blocked on it
//! would never return; and if the deployer performs the move, the deployer
//! writes the evidence that the *service* ran. `apply` installs the script and
//! arms the timer. The first real pass is systemd's.
//!
//! This differs from `disk_budget`, which does reclaim during apply — the
//! asymmetry is deliberate: reclaim is bounded and non-destructive of anything
//! declared, archival is neither.
//!
//! # YAML
//!
//! ```yaml
//! raid-archive:
//!   type: nas_archive
//!   machine: lambda-labs
//!   path: /mnt/nvme-raid0
//!   archive_destination: /mnt/unas/media
//!   archive_dirs: [corpus, albor-data, entrenar-checkpoints]
//!   archive_max_files: 2000
//!   archive_max_small_file_pct: 50
//!   archive_min_age_days: 30
//!   archive_leave_symlink: true
//!   archive_schedule: daily
//! ```

use crate::core::shell_escape::sh_squote;
use crate::core::types::{
    NasArchive, Resource, DEFAULT_ARCHIVE_SCHEDULE, DEFAULT_MAX_FILES, DEFAULT_MAX_SMALL_FILE_PCT,
    DEFAULT_MIN_AGE_DAYS,
};

mod mover;
mod units;

#[cfg(test)]
mod tests;

pub use mover::archive_script;

/// Resolve a validated [`NasArchive`] from a resource declaration.
///
/// # Errors
///
/// Returns `Err` when the declaration names no directories, has no destination,
/// uses a path where a directory name is required, or places the destination
/// inside the source.
pub fn archive_of(resource: &Resource) -> Result<NasArchive, String> {
    let path = resource
        .path
        .as_deref()
        .ok_or_else(|| "nas_archive requires `path` (the source root)".to_string())?;
    NasArchive::new(
        path,
        resource.archive.destination.as_deref(),
        resource.archive.dirs.clone(),
        resource.archive.max_files.unwrap_or(DEFAULT_MAX_FILES),
        resource
            .archive
            .max_small_file_pct
            .unwrap_or(DEFAULT_MAX_SMALL_FILE_PCT),
        resource
            .archive
            .min_age_days
            .unwrap_or(DEFAULT_MIN_AGE_DAYS),
        resource.archive.leave_symlink.unwrap_or(true),
        resource
            .archive
            .schedule
            .as_deref()
            .unwrap_or(DEFAULT_ARCHIVE_SCHEDULE),
    )
}

pub(crate) fn slug(path: &str) -> String {
    let s: String = path
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let t = s.trim_matches('-').to_string();
    if t.is_empty() {
        "root".to_string()
    } else {
        t
    }
}

pub(crate) fn script_path(path: &str) -> String {
    format!("/usr/local/sbin/forjar-archive-{}.sh", slug(path))
}
pub(crate) fn service_name(path: &str) -> String {
    format!("forjar-archive-{}", slug(path))
}

fn reject(msg: &str) -> String {
    format!("echo {} >&2; exit 1", sh_squote(&format!("ERROR: {msg}")))
}

/// Script that reports whether the declaration is converged.
///
/// Convergence has two halves, and asserting only the first is how the
/// predecessor stayed green with a wrong `DIRS` list:
///
///   1. The machinery is installed — script, service, timer, timer live.
///   2. Every declared directory is accounted for: archived (a symlink into the
///      destination, with the destination present) or still pending.
///
/// A directory that is neither — a symlink pointing somewhere else, or a
/// symlink whose target is missing — is divergence, because that is a *broken*
/// archive rather than an unfinished one.
pub fn check_script(resource: &Resource) -> String {
    let a = match archive_of(resource) {
        Ok(a) => a,
        Err(e) => return reject(&e),
    };
    let script = script_path(&a.path);
    let unit = service_name(&a.path);

    let mut assertions = vec![
        crate::resources::verdict::assert_that(
            &format!("test -x {}", sh_squote(&script)),
            &format!("archive-script-present:{unit}"),
            &format!("archive-script-missing:{unit}"),
        ),
        crate::resources::verdict::assert_that(
            &format!(
                "systemctl is-enabled {} >/dev/null 2>&1",
                sh_squote(&format!("{unit}.timer"))
            ),
            &format!("archive-timer-enabled:{unit}"),
            &format!("archive-timer-disabled:{unit}"),
        ),
        crate::resources::verdict::assert_that(
            &format!("test -d {}", sh_squote(&a.destination)),
            &format!("archive-destination-present:{}", a.destination),
            &format!("archive-destination-missing:{}", a.destination),
        ),
    ];

    // Per-directory: archived, pending, or broken. Only broken is divergence —
    // a pending directory is the ordinary state between the declaration landing
    // and the timer's next pass, and failing on it would make every fresh
    // declaration red for up to a cadence.
    for d in &a.dirs {
        let src = sh_squote(&a.source_of(d));
        let dest = sh_squote(&a.dest_of(d));
        assertions.push(crate::resources::verdict::assert_block(
            &format!("test -L {src}"),
            // Archived: the symlink must resolve INTO the declared destination.
            &format!(
                "if\n    [ \"$(readlink -f {src})\" = \"$(readlink -f {dest} 2>/dev/null)\" ] && [ -d {dest} ]\n  \
                 then\n    echo 'archived:{d}'\n  else\n    echo 'archive-broken:{d}'; __fj_diverged=1\n  fi",
                d = d
            ),
            &format!("echo 'archive-pending:{d}'"),
        ));
    }

    crate::resources::verdict::check_script_from(&assertions)
}

/// Script that installs the archive script, service and timer.
pub fn apply_script(resource: &Resource) -> String {
    let a = match archive_of(resource) {
        Ok(a) => a,
        Err(e) => return reject(&e),
    };
    let script = script_path(&a.path);
    let unit = service_name(&a.path);

    format!(
        "set -e\n\
         mkdir -p {script_dir}\n\
         cat > {script_q} <<'FORJAR_ARCHIVE_EOF'\n{body}\nFORJAR_ARCHIVE_EOF\n\
         chmod 0755 {script_q}\n\
         cat > {svc} <<'FORJAR_SVC_EOF'\n{service}\nFORJAR_SVC_EOF\n\
         cat > {tmr} <<'FORJAR_TMR_EOF'\n{timer}\nFORJAR_TMR_EOF\n\
         systemctl daemon-reload\n\
         systemctl enable --now {unit}.timer\n",
        script_dir = sh_squote("/usr/local/sbin"),
        script_q = sh_squote(&script),
        body = archive_script(&a),
        svc = sh_squote(&format!("/etc/systemd/system/{unit}.service")),
        service = units::service_unit(&a, &script),
        tmr = sh_squote(&format!("/etc/systemd/system/{unit}.timer")),
        timer = units::timer_unit(&a),
        unit = unit,
    )
}

/// Script that reports observed state for `plan` and `drift`.
///
/// This is what makes a wrong `archive_dirs` visible: it names every declared
/// directory and what it currently is, so a directory that was never archived
/// shows up as `pending` rather than being invisible.
pub fn state_query_script(resource: &Resource) -> String {
    let a = match archive_of(resource) {
        Ok(a) => a,
        Err(e) => return reject(&e),
    };
    let mut out = String::from("set -u\n");
    for d in &a.dirs {
        let src = sh_squote(&a.source_of(d));
        out.push_str(&format!(
            "if [ -L {src} ]; then echo 'archived {d}'; \
             elif [ -d {src} ]; then echo \"pending {d} $(du -sh {src} 2>/dev/null | cut -f1)\"; \
             else echo 'absent {d}'; fi\n",
            src = src,
            d = d
        ));
    }
    out
}
