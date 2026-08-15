//! FJ-037: Backup sync types — offsite copy as declared, *verified* state.
//!
//! # Why this type exists
//!
//! lambda-labs held ~2.1 TB of irreplaceable media on a 4-wide RAID0 with no
//! parity, and a job that had been reporting `Backup complete` hourly for
//! months while copying the array onto itself. Measured 2026-08-15: zero bytes
//! existed anywhere off that array.
//!
//! Three independent defects produced that, and each one is something a type
//! can forbid:
//!
//!   1. **The destination was not remote.** `SOURCE=/mnt/nvme-raid0/videos`,
//!      `DEST=/videos`, and `/videos` was a symlink back to the source. rsync
//!      dutifully copied a directory onto itself and exited 0.
//!   2. **The success metric could not fail.** It reported `Files: 0` while 77
//!      matching files sat in that exact directory, because `find "$DEST"` does
//!      not descend a symlink without `-L` or a trailing slash. Not "0 reported
//!      as healthy on an empty set" — structurally 0 on *every* input.
//!   3. **Coverage was never compared to anything.** The job synced a 16 GB
//!      directory and nothing anywhere asserted that the other 2.1 TB was
//!      covered.
//!
//! So: the remote must be syntactically incapable of being a local path, the
//! health signal is a count of files *verified present in the remote by
//! checksum*, and that count is compared against the source. A run that cannot
//! demonstrate coverage fails.

/// Default minimum % of source files that must verify against the remote.
///
/// Not 100: a file written or renamed between the sync and the verify pass is
/// a routine race on a live media box, and a threshold that flags it makes the
/// signal noisy and therefore ignored. 99 tolerates churn while still failing
/// on any real coverage gap.
pub const DEFAULT_VERIFY_PCT: u8 = 99;
/// Default cadence.
pub const DEFAULT_BACKUP_SCHEDULE: &str = "daily";
/// Default per-run upload ceiling, GiB.
///
/// Google Drive rejects uploads past ~750 GB/day/account. Seeding 2.1 TB
/// therefore takes days no matter how fast the link is; the ceiling makes that
/// a designed-for property rather than a nightly burst of 403s.
pub const DEFAULT_DAILY_CAP_GB: u64 = 700;

// Compile-time, not a test: a default that violated these would ship to every
// machine that omits the knobs, which is the common case.
const _: () = assert!(DEFAULT_VERIFY_PCT >= 95 && DEFAULT_VERIFY_PCT <= 100);
const _: () = assert!(
    DEFAULT_DAILY_CAP_GB < 750,
    "must stay under Drive's daily cap"
);

/// Declaration fields for a `backup_sync` resource.
///
/// Grouped into their own struct and `#[serde(flatten)]`-ed into `Resource`:
/// the YAML shape is unchanged (`backup_remote:` stays top level), but the nine
/// fields live with the type that understands them instead of enlarging an
/// already-large kitchen-sink struct.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BackupSpec {
    /// Absolute source paths to protect.
    #[serde(rename = "backup_source", default)]
    pub source: Vec<String>,

    /// Destination in `remote:path` form. A local path is rejected at parse time.
    #[serde(rename = "backup_remote", default)]
    pub remote: Option<String>,

    /// rclone backend for the remote (default "drive").
    #[serde(rename = "backup_remote_type", default)]
    pub remote_type: Option<String>,

    /// Non-secret rclone remote options, written verbatim into rclone.conf.
    #[serde(rename = "backup_remote_config", default)]
    pub remote_config: std::collections::HashMap<String, String>,

    /// OAuth token for the remote. Supply as `{{secrets.NAME}}`.
    #[serde(rename = "backup_token", default)]
    pub token: Option<String>,

    /// systemd `OnCalendar` cadence for the sync (default "daily").
    #[serde(rename = "backup_schedule", default)]
    pub schedule: Option<String>,

    /// Min % of source files that must verify by checksum (default 99).
    #[serde(rename = "backup_verify_pct", default)]
    pub verify_pct: Option<u8>,

    /// Upload ceiling per run in GiB (default 700, under Drive's 750/day).
    #[serde(rename = "backup_daily_cap_gb", default)]
    pub daily_cap_gb: Option<u64>,

    /// Optional rclone bandwidth limit, e.g. "50M".
    #[serde(rename = "backup_bandwidth_limit", default)]
    pub bandwidth_limit: Option<String>,
}

/// Resolved, validated backup declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupSync {
    /// Absolute source paths to protect.
    pub sources: Vec<String>,
    /// Destination in `remote:path` form (an rclone remote).
    pub remote: String,
    /// systemd `OnCalendar` cadence.
    pub schedule: String,
    /// Minimum % of source files that must verify by checksum.
    pub verify_pct: u8,
    /// Upload ceiling per run, GiB.
    pub daily_cap_gb: u64,
    /// Optional rclone bandwidth limit (e.g. "50M").
    pub bandwidth_limit: Option<String>,
}

impl BackupSync {
    /// Build a declaration, rejecting anything that cannot be a real backup.
    ///
    /// # Errors
    ///
    /// Returns `Err` when there are no sources, a source is not absolute, the
    /// remote is not in `remote:path` form, sources overlap, or `verify_pct` is
    /// out of range.
    pub fn new(
        sources: Vec<String>,
        remote: &str,
        schedule: &str,
        verify_pct: u8,
        daily_cap_gb: u64,
        bandwidth_limit: Option<String>,
    ) -> Result<Self, String> {
        if sources.is_empty() {
            return Err(
                "backup_sync requires at least one `backup_source` — a backup of \
                        nothing verifies trivially and protects nothing"
                    .to_string(),
            );
        }
        for s in &sources {
            if !s.starts_with('/') {
                return Err(format!("backup_sync source '{s}' is not an absolute path"));
            }
        }
        validate_no_overlap(&sources)?;
        validate_remote(remote)?;
        if verify_pct == 0 || verify_pct > 100 {
            return Err(format!(
                "backup_sync verify_pct must be in 1..=100, got {verify_pct}"
            ));
        }
        Ok(Self {
            sources,
            remote: remote.to_string(),
            schedule: schedule.to_string(),
            verify_pct,
            daily_cap_gb,
            bandwidth_limit,
        })
    }

    /// The remote's name (the part before `:`).
    pub fn remote_name(&self) -> &str {
        self.remote.split(':').next().unwrap_or("")
    }
}

/// A destination must be an rclone remote, never a local path.
///
/// This is the single most load-bearing check in the type. The job this
/// replaces had a *local* destination that symlinked back to its own source,
/// so it copied the array onto itself and exited 0 for months. `remote:path`
/// cannot name a local directory, and rejecting everything else makes the
/// self-referential backup unrepresentable rather than merely discouraged.
///
/// # Errors
///
/// Returns `Err` for absolute paths, relative paths, bare names with no colon,
/// and empty remote names.
fn validate_remote(remote: &str) -> Result<(), String> {
    if remote.starts_with('/') || remote.starts_with('.') || remote.starts_with('~') {
        return Err(format!(
            "backup_sync remote '{remote}' is a LOCAL path. The destination must be an \
             rclone remote in `remote:path` form — a local destination is how the \
             predecessor came to copy the array onto itself and report success."
        ));
    }
    let Some((name, _path)) = remote.split_once(':') else {
        return Err(format!(
            "backup_sync remote '{remote}' has no `remote:` prefix; expected `remote:path`"
        ));
    };
    if name.is_empty() {
        return Err(format!(
            "backup_sync remote '{remote}' has an empty remote name"
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "backup_sync remote name '{name}' must be alphanumeric/-/_ (rclone remote name)"
        ));
    }
    Ok(())
}

/// Overlapping sources double-count coverage and make the percentage a lie.
fn validate_no_overlap(sources: &[String]) -> Result<(), String> {
    for (i, a) in sources.iter().enumerate() {
        for b in sources.iter().skip(i + 1) {
            let (outer, inner) = if a.len() <= b.len() { (a, b) } else { (b, a) };
            let prefix = outer.trim_end_matches('/');
            if inner == outer || inner.starts_with(&format!("{prefix}/")) {
                return Err(format!(
                    "backup_sync sources overlap ('{outer}' contains '{inner}'), which \
                     double-counts files and makes verify_pct meaningless"
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_sources() -> Vec<String> {
        vec!["/mnt/a".into(), "/mnt/b".into()]
    }

    fn build(remote: &str) -> Result<BackupSync, String> {
        BackupSync::new(ok_sources(), remote, "daily", 99, 700, None)
    }

    #[test]
    fn accepts_a_real_remote() {
        let b = build("gdrive:lambda-labs-media").unwrap();
        assert_eq!(b.remote_name(), "gdrive");
    }

    #[test]
    fn rejects_an_absolute_local_destination() {
        // THE bug: DEST=/videos, a symlink back to SOURCE.
        let e = build("/videos").unwrap_err();
        assert!(e.contains("LOCAL path"), "{e}");
        assert!(e.contains("copy the array onto itself"), "{e}");
    }

    #[test]
    fn rejects_relative_and_home_destinations() {
        assert!(build("./backup").is_err());
        assert!(build("../backup").is_err());
        assert!(build("~/backup").is_err());
    }

    #[test]
    fn rejects_a_bare_name_with_no_colon() {
        assert!(build("gdrive").is_err());
    }

    #[test]
    fn rejects_an_empty_remote_name() {
        assert!(build(":path").is_err());
    }

    #[test]
    fn rejects_a_remote_name_with_path_separators() {
        // `mnt/nvme-raid0:x` would be a directory, not a remote.
        assert!(build("mnt/nvme-raid0:x").is_err());
    }

    #[test]
    fn rejects_an_empty_source_list() {
        let e = BackupSync::new(vec![], "gdrive:x", "daily", 99, 700, None).unwrap_err();
        assert!(e.contains("at least one"), "{e}");
        assert!(e.contains("verifies trivially"), "{e}");
    }

    #[test]
    fn rejects_a_relative_source() {
        assert!(BackupSync::new(vec!["media".into()], "gdrive:x", "daily", 99, 700, None).is_err());
    }

    #[test]
    fn rejects_overlapping_sources() {
        let e = BackupSync::new(
            vec!["/mnt/media".into(), "/mnt/media/courses".into()],
            "gdrive:x",
            "daily",
            99,
            700,
            None,
        )
        .unwrap_err();
        assert!(e.contains("overlap"), "{e}");
    }

    #[test]
    fn sibling_sources_sharing_a_prefix_are_fine() {
        // /mnt/media-a is NOT inside /mnt/media.
        assert!(BackupSync::new(
            vec!["/mnt/media".into(), "/mnt/media-a".into()],
            "gdrive:x",
            "daily",
            99,
            700,
            None
        )
        .is_ok());
    }

    #[test]
    fn rejects_duplicate_sources() {
        assert!(BackupSync::new(
            vec!["/mnt/a".into(), "/mnt/a".into()],
            "gdrive:x",
            "daily",
            99,
            700,
            None
        )
        .is_err());
    }

    #[test]
    fn rejects_out_of_range_verify_pct() {
        assert!(BackupSync::new(ok_sources(), "gdrive:x", "daily", 0, 700, None).is_err());
        assert!(BackupSync::new(ok_sources(), "gdrive:x", "daily", 101, 700, None).is_err());
        assert!(BackupSync::new(ok_sources(), "gdrive:x", "daily", 100, 700, None).is_ok());
    }

    #[test]
    fn defaults_are_sane() {
        // The numeric bounds are compile-time assertions above; this pins the
        // cadence, which cannot be checked in a const context.
        assert_eq!(DEFAULT_BACKUP_SCHEDULE, "daily");
    }
}
