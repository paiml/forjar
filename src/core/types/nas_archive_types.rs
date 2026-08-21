//! FJ-038: NAS archive types — archival as declared, *verified* state.
//!
//! # Why this type exists
//!
//! Disk **reclaim** is a forjar resource with declared policy (`disk_budget`,
//! FJ-036). Disk **archival** — the mirror operation, and the destructive one —
//! was a shell script with its policy hardcoded in a string literal:
//!
//! ```sh
//! DIRS="${DIRS:-mac-backup mastering-github-coursera-assets ... RecordedCourses}"
//! ```
//!
//! That asymmetry cost three specific things, and each is something a type can
//! fix:
//!
//!   1. **Five directories were never archived** — `corpus` (21 G),
//!      `entrenar-checkpoints` (13 G), `hf-coursera-assets` (6.3 G),
//!      `albor-data` (6.1 G), `gemma2-models` (1.6 G) — not because they were
//!      unsuitable, but because their names were not typed into that string.
//!      One held 90 files that existed nowhere but a 4-wide RAID0 with no
//!      parity.
//!   2. **`forjar drift` could not see it.** Declared state was "a file exists
//!      with this sha", so a wrong `DIRS` list was perfectly converged. Drift
//!      detection was structurally blind to the thing that mattered.
//!   3. **No `plan` before a destructive act.** An edit to `DIRS` reached
//!      `--execute` within one timer cadence, unreviewed, for an operation that
//!      deletes originals.
//!
//! # What this type forbids outright
//!
//! The predecessor script had six defects found by inspection, four of which
//! were in the *verify-then-delete* path. A declaration cannot fix an imperative
//! bug, but it can make the dangerous configurations unrepresentable:
//!
//!   * A destination inside the source — the "copy the array onto itself" shape
//!     that made `backup_sync`'s predecessor report success over zero bytes.
//!   * An empty `archive_dirs` — an archive that moves nothing and reports
//!     converged is the unconditional-success bug in miniature.
//!   * A path component in a directory name, so a declaration can never reach
//!     outside the source root.
//!   * Small-file trees, which collapse to ~7.9 MB/s over CIFS against
//!     ~350 MB/s for large files — measured on this fleet, a 45x difference that
//!     turns a bounded move into an unbounded one.

/// Default ceiling on files in one archived directory.
///
/// Measured on this fleet: CIFS large-file throughput is ~350 MB/s and
/// small-file throughput ~7.9 MB/s. A tree of thousands of small files is not
/// slightly slower, it is a different operation — and the move deletes the
/// source at the end, so an operation that cannot finish is an operation that
/// holds a delete open indefinitely.
pub const DEFAULT_MAX_FILES: u64 = 2000;

/// Default ceiling on the share of files under 64 KiB, as a percentage.
pub const DEFAULT_MAX_SMALL_FILE_PCT: u8 = 50;

/// Below this many files, the percentage is not applied.
///
/// A directory of 8 files that are 87% small is `entrenar-checkpoints` — a
/// perfectly ordinary archive target. The percentage is a statement about
/// *aggregate* transfer shape, and on a handful of files it says nothing. The
/// first cut applied it at any size and refused exactly that directory.
pub const SMALL_FILE_PCT_MIN_FILES: u64 = 500;

/// Default minimum age before a directory may be archived, in days.
///
/// Archival deletes the source. Data still being written is data whose owner
/// has not finished with it, and `mtime` is the cheapest available proxy for
/// that. Zero is permitted but must be written down.
pub const DEFAULT_MIN_AGE_DAYS: u64 = 30;

/// Default cadence.
pub const DEFAULT_ARCHIVE_SCHEDULE: &str = "daily";

// Compile-time, not tests: a bad default ships to every machine that omits the
// knob, which is the common case.
const _: () = assert!(DEFAULT_MAX_SMALL_FILE_PCT <= 100);
const _: () = assert!(DEFAULT_MAX_FILES > 0, "a zero ceiling archives nothing");
// The percentage guard must not fire on a small directory. The first cut
// applied it at any size and refused `entrenar-checkpoints` — 8 files, 87%
// under 64 KiB, and a perfectly ordinary archive target. A floor below 100
// would bring that back, so it fails the BUILD rather than a test.
const _: () = assert!(
    SMALL_FILE_PCT_MIN_FILES >= 100,
    "the small-file percentage needs a floor, or a handful of files is refused"
);

/// Declaration fields for a `nas_archive` resource.
///
/// `#[serde(flatten)]`-ed into `Resource`, so the YAML shape stays flat
/// (`archive_dirs:` is top level) while the fields live with the type that
/// understands them.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArchiveSpec {
    /// Directory names under `path` to archive. Names only — never paths.
    #[serde(rename = "archive_dirs", default)]
    pub dirs: Vec<String>,

    /// Absolute destination root on the NAS.
    #[serde(rename = "archive_destination", default)]
    pub destination: Option<String>,

    /// Refuse a directory with more than this many files.
    #[serde(rename = "archive_max_files", default)]
    pub max_files: Option<u64>,

    /// Refuse a directory where more than this % of files are under 64 KiB.
    #[serde(rename = "archive_max_small_file_pct", default)]
    pub max_small_file_pct: Option<u8>,

    /// Refuse a directory modified more recently than this many days ago.
    #[serde(rename = "archive_min_age_days", default)]
    pub min_age_days: Option<u64>,

    /// Leave a symlink at the old location pointing at the NAS copy.
    #[serde(rename = "archive_leave_symlink", default)]
    pub leave_symlink: Option<bool>,

    /// Cadence for the archive timer.
    #[serde(rename = "archive_schedule", default)]
    pub schedule: Option<String>,
}

/// Why a `nas_archive` declaration was refused.
///
/// A closed enum rather than strings: the Kani harness enumerates it, and a new
/// rejection reason that forgets a proof obligation fails to compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveRejection {
    /// `archive_dirs` was empty — nothing would move, and it would report success.
    NoDirs,
    /// A directory name contained a path separator or `..`.
    NameIsAPath,
    /// A directory name was empty.
    EmptyName,
    /// The same directory was declared twice.
    DuplicateName,
    /// `path` was not absolute.
    SourceNotAbsolute,
    /// `archive_destination` was missing.
    NoDestination,
    /// `archive_destination` was not absolute.
    DestinationNotAbsolute,
    /// The destination is the source, or lies inside it.
    DestinationInsideSource,
    /// The source lies inside the destination.
    SourceInsideDestination,
    /// A percentage above 100.
    PercentOutOfRange,
    /// A ceiling of zero files, which can never admit anything.
    ZeroMaxFiles,
}

/// A validated `nas_archive` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NasArchive {
    /// Absolute source root.
    pub path: String,
    /// Absolute destination root on the NAS.
    pub destination: String,
    /// Directory names under `path`, validated as names.
    pub dirs: Vec<String>,
    /// Ceiling on files per directory.
    pub max_files: u64,
    /// Ceiling on the share of files under 64 KiB.
    pub max_small_file_pct: u8,
    /// Minimum age before a directory may be archived.
    pub min_age_days: u64,
    /// Leave a symlink behind.
    pub leave_symlink: bool,
    /// Timer cadence.
    pub schedule: String,
}

/// Normalise a path for containment tests: strip trailing slashes, keeping `/`.
fn norm(p: &str) -> &str {
    let t = p.trim_end_matches('/');
    if t.is_empty() {
        "/"
    } else {
        t
    }
}

/// Does `inner` lie at or inside `outer`?
///
/// Compares whole components, so `/mnt/unas-old` is NOT inside `/mnt/unas` —
/// a prefix test on raw strings would say otherwise and refuse a valid
/// declaration.
pub(crate) fn contains_path(outer: &str, inner: &str) -> bool {
    let (o, i) = (norm(outer), norm(inner));
    if o == i {
        return true;
    }
    if o == "/" {
        return i.starts_with('/');
    }
    i.starts_with(o) && i.as_bytes().get(o.len()) == Some(&b'/')
}

/// Classify a declaration, returning the first reason it cannot be accepted.
///
/// Split from the error-message construction so Kani can enumerate outcomes
/// without modelling `format!`, whose allocations dominate CBMC time on the
/// error path (a lesson from an earlier harness that ran 117 minutes).
pub(crate) fn classify_declaration(
    path: &str,
    destination: Option<&str>,
    dirs: &[String],
    max_files: u64,
    max_small_file_pct: u8,
) -> Option<ArchiveRejection> {
    if !path.starts_with('/') {
        return Some(ArchiveRejection::SourceNotAbsolute);
    }
    let Some(dest) = destination else {
        return Some(ArchiveRejection::NoDestination);
    };
    if !dest.starts_with('/') {
        return Some(ArchiveRejection::DestinationNotAbsolute);
    }
    // Checked in this order so the more specific "destination inside source"
    // wins for equal paths, which is the shape that copies a tree onto itself.
    if contains_path(path, dest) {
        return Some(ArchiveRejection::DestinationInsideSource);
    }
    if contains_path(dest, path) {
        return Some(ArchiveRejection::SourceInsideDestination);
    }
    if dirs.is_empty() {
        return Some(ArchiveRejection::NoDirs);
    }
    if max_files == 0 {
        return Some(ArchiveRejection::ZeroMaxFiles);
    }
    if max_small_file_pct > 100 {
        return Some(ArchiveRejection::PercentOutOfRange);
    }
    for (i, d) in dirs.iter().enumerate() {
        if d.is_empty() {
            return Some(ArchiveRejection::EmptyName);
        }
        if d.contains('/') || d == ".." || d == "." {
            return Some(ArchiveRejection::NameIsAPath);
        }
        if dirs.iter().skip(i + 1).any(|o| o == d) {
            return Some(ArchiveRejection::DuplicateName);
        }
    }
    None
}

impl NasArchive {
    /// Validate a declaration.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        path: &str,
        destination: Option<&str>,
        dirs: Vec<String>,
        max_files: u64,
        max_small_file_pct: u8,
        min_age_days: u64,
        leave_symlink: bool,
        schedule: &str,
    ) -> Result<Self, String> {
        if let Some(reason) =
            classify_declaration(path, destination, &dirs, max_files, max_small_file_pct)
        {
            return Err(explain(reason, path, destination, &dirs));
        }
        Ok(Self {
            path: norm(path).to_string(),
            destination: norm(destination.unwrap_or_default()).to_string(),
            dirs,
            max_files,
            max_small_file_pct,
            min_age_days,
            leave_symlink,
            schedule: schedule.to_string(),
        })
    }

    /// Absolute source path of one declared directory.
    pub fn source_of(&self, dir: &str) -> String {
        format!("{}/{}", self.path, dir)
    }

    /// Absolute destination path of one declared directory.
    pub fn dest_of(&self, dir: &str) -> String {
        format!("{}/{}", self.destination, dir)
    }
}

/// Turn a rejection into an operator-actionable message.
fn explain(
    reason: ArchiveRejection,
    path: &str,
    destination: Option<&str>,
    dirs: &[String],
) -> String {
    let dest = destination.unwrap_or("<unset>");
    match reason {
        ArchiveRejection::NoDirs => format!(
            "nas_archive at '{path}' declares no archive_dirs. An archive that moves nothing \
             and reports converged is the unconditional-success bug: name the directories, or \
             remove the resource."
        ),
        ArchiveRejection::NameIsAPath => {
            let bad = dirs
                .iter()
                .find(|d| d.contains('/') || *d == ".." || *d == ".")
                .map(String::as_str)
                .unwrap_or("?");
            format!(
                "nas_archive entry '{bad}' is a path, not a directory name. Entries name direct \
                 children of '{path}' so a declaration can never reach outside the source root."
            )
        }
        ArchiveRejection::EmptyName => {
            format!("nas_archive at '{path}' has an empty entry in archive_dirs")
        }
        ArchiveRejection::DuplicateName => {
            let bad = dirs
                .iter()
                .enumerate()
                .find(|(i, d)| dirs.iter().skip(i + 1).any(|o| &o == d))
                .map(|(_, d)| d.as_str())
                .unwrap_or("?");
            format!(
                "nas_archive at '{path}' declares '{bad}' twice; a directory would be moved, \
                 then moved again from a location that no longer exists"
            )
        }
        ArchiveRejection::SourceNotAbsolute => {
            format!("nas_archive path '{path}' must be absolute")
        }
        ArchiveRejection::NoDestination => format!(
            "nas_archive at '{path}' has no archive_destination; there is nowhere for the data \
             to go and the originals are deleted at the end"
        ),
        ArchiveRejection::DestinationNotAbsolute => {
            format!("nas_archive destination '{dest}' must be absolute")
        }
        ArchiveRejection::DestinationInsideSource => format!(
            "nas_archive destination '{dest}' is inside (or equal to) source '{path}'. That is a \
             move onto itself — the shape that let this fleet's previous backup job report \
             success for months while zero bytes existed off the array."
        ),
        ArchiveRejection::SourceInsideDestination => format!(
            "nas_archive source '{path}' is inside destination '{dest}'; the move would \
             enclose its own source"
        ),
        ArchiveRejection::PercentOutOfRange => format!(
            "nas_archive archive_max_small_file_pct at '{path}' exceeds 100; it is a percentage"
        ),
        ArchiveRejection::ZeroMaxFiles => format!(
            "nas_archive archive_max_files at '{path}' is 0, which admits no directory at all"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn build(path: &str, dest: Option<&str>, d: &[&str]) -> Result<NasArchive, String> {
        NasArchive::new(
            path,
            dest,
            dirs(d),
            DEFAULT_MAX_FILES,
            DEFAULT_MAX_SMALL_FILE_PCT,
            DEFAULT_MIN_AGE_DAYS,
            true,
            DEFAULT_ARCHIVE_SCHEDULE,
        )
    }

    #[test]
    fn accepts_the_lambda_labs_declaration() {
        let a = build(
            "/mnt/nvme-raid0",
            Some("/mnt/unas/media"),
            &["corpus", "albor-data", "entrenar-checkpoints"],
        )
        .unwrap();
        assert_eq!(a.source_of("corpus"), "/mnt/nvme-raid0/corpus");
        assert_eq!(a.dest_of("corpus"), "/mnt/unas/media/corpus");
    }

    #[test]
    fn rejects_an_empty_dir_list() {
        // An archive that moves nothing and reports converged.
        let e = build("/mnt/nvme-raid0", Some("/mnt/unas/media"), &[]).unwrap_err();
        assert!(e.contains("unconditional-success"), "{e}");
    }

    #[test]
    fn rejects_a_destination_inside_the_source() {
        // THE shape that copied the array onto itself in backup_sync's predecessor.
        let e = build("/mnt/nvme-raid0", Some("/mnt/nvme-raid0/archive"), &["a"]).unwrap_err();
        assert!(e.contains("move onto itself"), "{e}");
    }

    #[test]
    fn rejects_a_destination_equal_to_the_source() {
        let e = build("/mnt/nvme-raid0", Some("/mnt/nvme-raid0"), &["a"]).unwrap_err();
        assert!(e.contains("move onto itself"), "{e}");
    }

    #[test]
    fn rejects_a_source_inside_the_destination() {
        let e = build("/mnt/unas/media/sub", Some("/mnt/unas/media"), &["a"]).unwrap_err();
        assert!(e.contains("enclose its own source"), "{e}");
    }

    #[test]
    fn a_sibling_with_a_shared_prefix_is_not_containment() {
        // `/mnt/unas-old` must NOT read as inside `/mnt/unas`; a raw string
        // prefix test would refuse this valid declaration.
        assert!(build("/mnt/unas-old", Some("/mnt/unas"), &["a"]).is_ok());
        assert!(build("/mnt/unas", Some("/mnt/unas-old"), &["a"]).is_ok());
    }

    #[test]
    fn trailing_slashes_do_not_defeat_the_containment_check() {
        let e = build("/mnt/nvme-raid0/", Some("/mnt/nvme-raid0"), &["a"]).unwrap_err();
        assert!(e.contains("move onto itself"), "{e}");
    }

    #[test]
    fn rejects_a_path_in_place_of_a_name() {
        for bad in ["../etc", "a/b", "..", "."] {
            let e = build("/mnt/nvme-raid0", Some("/mnt/unas/media"), &[bad]).unwrap_err();
            assert!(e.contains("is a path, not a directory name"), "{bad}: {e}");
        }
    }

    #[test]
    fn rejects_a_duplicate_directory() {
        let e = build("/mnt/nvme-raid0", Some("/mnt/unas/media"), &["a", "b", "a"]).unwrap_err();
        assert!(e.contains("twice"), "{e}");
    }

    #[test]
    fn rejects_a_relative_source_or_destination() {
        assert!(build("mnt/x", Some("/mnt/unas"), &["a"]).is_err());
        assert!(build("/mnt/x", Some("mnt/unas"), &["a"]).is_err());
    }

    #[test]
    fn rejects_a_missing_destination() {
        let e = build("/mnt/nvme-raid0", None, &["a"]).unwrap_err();
        assert!(e.contains("nowhere for the data to go"), "{e}");
    }

    #[test]
    fn rejects_degenerate_ceilings() {
        let zero = NasArchive::new(
            "/mnt/x",
            Some("/mnt/unas"),
            dirs(&["a"]),
            0,
            50,
            30,
            true,
            "daily",
        );
        assert!(zero.unwrap_err().contains("admits no directory"));

        let pct = NasArchive::new(
            "/mnt/x",
            Some("/mnt/unas"),
            dirs(&["a"]),
            10,
            101,
            30,
            true,
            "daily",
        );
        assert!(pct.unwrap_err().contains("percentage"));
    }

    #[test]
    fn contains_path_is_component_wise() {
        assert!(contains_path("/mnt/unas", "/mnt/unas"));
        assert!(contains_path("/mnt/unas", "/mnt/unas/media"));
        assert!(!contains_path("/mnt/unas", "/mnt/unas-old"));
        assert!(!contains_path("/mnt/unas/media", "/mnt/unas"));
        assert!(contains_path("/", "/anything"));
    }
}
