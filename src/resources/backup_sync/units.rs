//! FJ-037: systemd units for the backup sync.

/// Oneshot service running one sync + verify pass.
///
/// Deliberately carries neither `SuccessExitStatus` nor `RemainAfterExit`: the
/// script exits non-zero when it cannot prove coverage, and that has to land in
/// the unit's state. Masking it would recreate a backup that reports healthy
/// while protecting nothing, which is the failure this resource exists to end.
pub(super) fn service_unit(script_path: &str, remote: &str) -> String {
    format!(
        "[Unit]\n\
         Description=forjar verified backup sync to {remote}\n\
         Documentation=https://github.com/paiml/forjar\n\
         After=network-online.target\n\
         Wants=network-online.target\n\
         \n\
         [Service]\n\
         Type=oneshot\n\
         ExecStart={script_path}\n\
         # Seeding terabytes is long and never urgent; stay behind real work.\n\
         Nice=15\n\
         IOSchedulingClass=idle\n\
         # A multi-terabyte seed legitimately runs for hours. No timeout would\n\
         # let a wedged transfer block the timer forever; too short would kill a\n\
         # healthy seed and never converge.\n\
         TimeoutStartSec=12h\n\
         StandardOutput=journal\n\
         StandardError=journal\n"
    )
}

/// Timer driving the sync.
///
/// `Persistent=true` so a run missed while the machine was off happens at boot
/// — the window after downtime is exactly when a backup is most likely stale.
pub(super) fn timer_unit(schedule: &str, remote: &str) -> String {
    format!(
        "[Unit]\n\
         Description=forjar verified backup sync timer for {remote}\n\
         \n\
         [Timer]\n\
         OnCalendar={schedule}\n\
         Persistent=true\n\
         RandomizedDelaySec=900\n\
         AccuracySec=1min\n\
         \n\
         [Install]\n\
         WantedBy=timers.target\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_does_not_mask_a_failed_verification() {
        let s = service_unit("/usr/local/sbin/b.sh", "gdrive:x");
        assert!(!s.contains("SuccessExitStatus"));
        assert!(!s.contains("RemainAfterExit"));
        assert!(s.contains("Type=oneshot"));
    }

    #[test]
    fn service_allows_a_multi_terabyte_seed_to_finish() {
        // 2.1 TB at ~60 MB/s is ~10h of wall clock even before Drive's
        // 750 GB/day cap splits it across days.
        assert!(service_unit("/x", "r:").contains("TimeoutStartSec=12h"));
    }

    #[test]
    fn service_waits_for_the_network() {
        let s = service_unit("/x", "r:");
        assert!(s.contains("After=network-online.target"));
        assert!(s.contains("Wants=network-online.target"));
    }

    #[test]
    fn service_yields_to_real_work() {
        let s = service_unit("/x", "r:");
        assert!(s.contains("IOSchedulingClass=idle"));
        assert!(s.contains("Nice=15"));
    }

    #[test]
    fn timer_is_persistent_and_jittered() {
        let t = timer_unit("daily", "gdrive:x");
        assert!(t.contains("OnCalendar=daily"));
        assert!(t.contains("Persistent=true"));
        assert!(t.contains("RandomizedDelaySec=900"));
        assert!(t.contains("WantedBy=timers.target"));
    }

    #[test]
    fn timer_uses_wall_clock_not_relative() {
        assert!(!timer_unit("daily", "r:").contains("OnUnitActiveSec"));
    }
}
