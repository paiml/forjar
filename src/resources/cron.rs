//! FJ-033: Cron resource handler.
//!
//! Manages scheduled tasks via crontab entries.

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;
use crate::resources::verdict;

/// The two lines that decide whether this script needs `sudo`.
///
/// forjar#348: only `apply_script` carried them. The apply wrote into root's
/// crontab under sudo while `check_script` read the invoking SSH user's and
/// reported the job missing forever — a resource that was correctly installed
/// and permanently unconvergeable, blocking every dependent.
///
/// `crontab -u <user>` refuses EVERY non-root caller — cronie exits with
/// "must be privileged to use -u" even for the caller's own username — so
/// READING another user's crontab is exactly as privileged as writing it.
/// One copy, three call sites.
const SUDO_PREAMBLE: &str = "SUDO=\"\"\n[ \"$(id -u)\" -ne 0 ] && SUDO=\"sudo\"";

/// Refuse to answer when the crontab cannot be read at all.
///
/// Modelled on `service::SYSTEMD_CHECK_GUARD`. `crontab -l` exits 1 for BOTH
/// "no crontab for user" and EPERM, so the honest signal has to be taken
/// BEFORE the read; without it a host with no passwordless sudo just moves the
/// false `missing:` one step later. `cli::check` maps exit 2 to SKIP —
/// forjar cannot observe another user's crontab without privilege, and that is
/// neither a pass nor a failure.
const CRONTAB_CHECK_GUARD: &str = "\
if [ \"$(id -u)\" -ne 0 ] && ! sudo -n true >/dev/null 2>&1; then\n  \
  echo 'FORJAR_SKIP: sudo unavailable - crontab state is not observable here'\n  \
  exit 2\n\
fi";

/// The crontab this resource owns, escaped for the shell.
///
/// apply, check and state_query each defaulted to `root` independently, which
/// is how they were able to disagree. They delegate here instead.
fn crontab_user(resource: &Resource) -> String {
    sh_squote(resource.owner.as_deref().unwrap_or("root"))
}

/// Generate shell script to check if a cron job exists.
pub fn check_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let u = crontab_user(resource);
    let marker = sh_squote(&format!("# forjar:{name}"));
    let verdict = verdict::single(
        &format!("$SUDO crontab -u {u} -l 2>/dev/null | grep -qF {marker}"),
        &format!("exists:{name}"),
        &format!("missing:{name}"),
    );
    format!("{CRONTAB_CHECK_GUARD}\n{SUDO_PREAMBLE}\n{verdict}")
}

/// Generate shell script to add/remove a cron entry.
pub fn apply_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let state = resource.state.as_deref().unwrap_or("present");

    let u = crontab_user(resource);
    let marker = sh_squote(&format!("# forjar:{name}"));
    let cmd_marker = sh_squote(&format!("# forjar-cmd:{name}"));

    match state {
        "absent" => format!(
            "set -euo pipefail\n\
             {SUDO_PREAMBLE}\n\
             EXISTING=$($SUDO crontab -u {u} -l 2>/dev/null || true)\n\
             echo \"$EXISTING\" | grep -v {marker} | grep -v {cmd_marker} | $SUDO crontab -u {u} -"
        ),
        _ => {
            let schedule = resource.schedule.as_deref().unwrap_or("* * * * *");
            let command = resource.command.as_deref().unwrap_or("true");
            // The crontab content line must stay literal in the user's
            // crontab; escape the whole `schedule command` string as one word.
            let entry = sh_squote(&format!("{schedule} {command}"));

            format!(
                "set -euo pipefail\n\
                 {SUDO_PREAMBLE}\n\
                 EXISTING=$($SUDO crontab -u {u} -l 2>/dev/null | grep -v {marker} | grep -v {cmd_marker} || true)\n\
                 {{\n\
                   echo \"$EXISTING\"\n\
                   echo {marker}\n\
                   echo {cmd_marker}  \n\
                   echo {entry}\n\
                 }} | $SUDO crontab -u {u} -"
            )
        }
    }
}

/// Generate shell to query cron state (for BLAKE3 hashing).
///
/// forjar#348 applies here too, and this half is the more expensive one: an
/// unprivileged read made the observable `cron=MISSING:<name>` for a job that
/// exists, so the lock recorded "absent" as the observed state and drift
/// detection was wrong in the same direction as the check.
pub fn state_query_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let u = crontab_user(resource);
    let marker = sh_squote(&format!("# forjar:{name}"));
    format!(
        "{SUDO_PREAMBLE}\n\
         $SUDO crontab -u {u} -l 2>/dev/null | grep -A1 {marker} || echo {}",
        sh_squote(&format!("cron=MISSING:{name}"))
    )
}
