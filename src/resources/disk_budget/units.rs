//! FJ-036: systemd units for the disk-budget reaper.

use crate::core::shell_escape::sh_squote;

/// Oneshot service that runs one reclaim pass.
///
/// Deliberately NOT `RemainAfterExit` and NOT `SuccessExitStatus=1`: the
/// reaper's non-zero exit on a missed budget must land in the unit's state,
/// because that is what makes an inert reaper visible to `systemctl` and to
/// `forjar drift`. Masking it would restore exactly the silent-green failure
/// this resource exists to remove.
///
/// #334: `FORJAR_BUDGET_EXECUTE=1` is granted HERE and in `apply_script`, and
/// nowhere else — the same shape `nas_archive` uses for `ARCHIVE_EXECUTE`.
/// Running the reaper by hand is a preview; the scheduled pass is the one that
/// deletes. Removing this line does not make anything safer, it makes every
/// fleet reaper silently inert and the machines slide to 100%.
pub(super) fn service_unit(script_path: &str, path: &str) -> String {
    format!(
        "[Unit]\n\
         Description=forjar disk budget reaper for {path}\n\
         Documentation=https://github.com/paiml/forjar\n\
         After=local-fs.target\n\
         \n\
         [Service]\n\
         Type=oneshot\n\
         # The scheduled pass is the one that reclaims. Without this the reaper\n\
         # only previews — see forjar#334.\n\
         Environment=FORJAR_BUDGET_EXECUTE=1\n\
         ExecStart={script_path}\n\
         # Reclaim is IO-heavy and never urgent; stay out of the way of real work.\n\
         Nice=10\n\
         IOSchedulingClass=idle\n\
         # A wedged find/du must not hold the timer.\n\
         TimeoutStartSec=30min\n\
         StandardOutput=journal\n\
         StandardError=journal\n"
    )
}

/// Timer driving the reaper.
///
/// `Persistent=true` so a pass missed while the machine was off runs at boot —
/// the window right after a long downtime is exactly when a build box is most
/// likely to be over budget. `RandomizedDelaySec` keeps a fleet of machines
/// from all reclaiming on the same tick.
pub(super) fn timer_unit(schedule: &str, path: &str) -> String {
    format!(
        "[Unit]\n\
         Description=forjar disk budget reaper timer for {path}\n\
         \n\
         [Timer]\n\
         OnCalendar={schedule}\n\
         Persistent=true\n\
         RandomizedDelaySec=120\n\
         AccuracySec=1s\n\
         \n\
         [Install]\n\
         WantedBy=timers.target\n"
    )
}

/// Shell that writes a unit file only when its content differs, echoing
/// `changed` so the caller knows whether a daemon-reload is required.
pub(super) fn install_unit(unit_path: &str, content: &str, var: &str) -> String {
    let p = sh_squote(unit_path);
    format!(
        "{var}=0\n\
         NEW=$(cat <<'FORJAR_UNIT_EOF'\n\
         {content}\n\
         FORJAR_UNIT_EOF\n\
         )\n\
         if [ ! -f {p} ] || [ \"$NEW\" != \"$(cat {p} 2>/dev/null)\" ]; then\n\
         \x20 printf '%s\\n' \"$NEW\" >{p}\n\
         \x20 {var}=1\n\
         fi\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_does_not_mask_failure() {
        let s = service_unit("/usr/local/sbin/r.sh", "/");
        // Either of these would hide a missed budget from systemd and drift.
        assert!(!s.contains("SuccessExitStatus"));
        assert!(!s.contains("RemainAfterExit"));
        assert!(s.contains("Type=oneshot"));
    }

    /// #334: the unit is one of exactly two grants of the delete opt-in. If
    /// this assertion ever goes red because someone removed the line, every
    /// timer-driven reaper on the fleet has silently become a preview.
    #[test]
    fn service_grants_execute_exactly_once() {
        let s = service_unit("/usr/local/sbin/r.sh", "/");
        assert_eq!(
            s.matches("Environment=FORJAR_BUDGET_EXECUTE=1").count(),
            1,
            "the scheduled pass must be granted the delete opt-in: {s}"
        );
    }

    #[test]
    fn service_is_bounded_and_deprioritised() {
        let s = service_unit("/usr/local/sbin/r.sh", "/");
        assert!(s.contains("TimeoutStartSec=30min"));
        assert!(s.contains("IOSchedulingClass=idle"));
        assert!(s.contains("Nice=10"));
    }

    #[test]
    fn timer_is_persistent_and_jittered() {
        let t = timer_unit("hourly", "/");
        assert!(t.contains("OnCalendar=hourly"));
        assert!(t.contains("Persistent=true"));
        assert!(t.contains("RandomizedDelaySec=120"));
        assert!(t.contains("WantedBy=timers.target"));
    }

    #[test]
    fn timer_uses_wall_clock_not_relative() {
        // OnUnitActiveSec fires only while the unit keeps re-activating; on a
        // oneshot that has failed it can stop firing entirely.
        assert!(!timer_unit("hourly", "/").contains("OnUnitActiveSec"));
    }

    #[test]
    fn install_is_content_addressed_not_unconditional() {
        let s = install_unit("/etc/systemd/system/x.service", "[Unit]\n", "CHANGED");
        assert!(s.contains("CHANGED=1"));
        assert!(s.contains("!= \"$(cat"), "must compare before writing");
    }
}
