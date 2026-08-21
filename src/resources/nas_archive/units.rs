//! systemd units for a `nas_archive` resource.

use crate::core::types::NasArchive;

/// The oneshot service that runs the archive pass.
///
/// `ARCHIVE_EXECUTE=1` is set here and nowhere else: running the script by hand
/// is a dry run unless the operator opts in, so an accidental invocation
/// inspects rather than deletes.
pub fn service_unit(a: &NasArchive, script: &str) -> String {
    format!(
        "[Unit]\n\
         Description=forjar NAS archive for {path}\n\
         After=network-online.target remote-fs.target\n\
         Wants=remote-fs.target\n\
         \n\
         [Service]\n\
         Type=oneshot\n\
         Environment=ARCHIVE_EXECUTE=1\n\
         ExecStart={script}\n\
         Nice=10\n\
         IOSchedulingClass=idle\n",
        path = a.path,
        script = script,
    )
}

/// The timer.
///
/// `Persistent=true` so a machine that was off over its window archives on the
/// next boot instead of silently skipping a cadence — absence of a run is this
/// fleet's proven silent-green failure mode.
pub fn timer_unit(a: &NasArchive) -> String {
    format!(
        "[Unit]\n\
         Description=forjar NAS archive timer for {path}\n\
         \n\
         [Timer]\n\
         OnCalendar={schedule}\n\
         Persistent=true\n\
         RandomizedDelaySec=900\n\
         \n\
         [Install]\n\
         WantedBy=timers.target\n",
        path = a.path,
        schedule = a.schedule,
    )
}
