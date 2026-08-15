//! FJ-037: preflight — refuse to run without a real, configured remote.
//!
//! The credential itself is deliberately NOT managed by forjar: an OAuth token
//! is a secret and does not belong in a git repo. But "not managed" must never
//! degrade into "not checked". A backup that runs with an unconfigured remote
//! is the failure this resource exists to prevent, so every precondition is
//! asserted before a single byte moves, and a missing one is a hard exit.

use crate::core::shell_escape::sh_squote;
use crate::core::types::BackupSync;

/// Preflight assertions, emitted once at the top of the sync script.
pub(super) fn block(cfg: &BackupSync) -> String {
    let remote_name = sh_squote(cfg.remote_name());
    let remote_disp = cfg.remote_name();
    let sources: Vec<String> = cfg.sources.iter().map(|s| sh_squote(s)).collect();
    let source_checks: String = sources
        .iter()
        .map(|s| {
            format!(
                "if [ ! -d {s} ]; then\n  \
                 bs_log \"FATAL: source {s} does not exist\"\n  \
                 exit 1\nfi\n"
            )
        })
        .collect();

    format!(
        r#"
# --- preflight ---------------------------------------------------------
if ! command -v rclone >/dev/null 2>&1; then
  bs_log "FATAL: rclone is not installed"
  exit 1
fi

# The remote must be CONFIGURED, not merely named. An unconfigured remote makes
# rclone treat the destination as a local path, which is exactly how a backup
# silently becomes a copy onto the same disk.
if ! rclone listremotes 2>/dev/null | grep -qx {remote_name}:; then
  bs_log "FATAL: rclone remote {remote_disp}: is not configured on this host."
  bs_log "       Run: rclone config   (or install the credential out-of-band)."
  bs_log "       forjar does not manage the OAuth token - it is a secret - but it"
  bs_log "       refuses to run a backup that cannot reach its destination."
  exit 1
fi

# Prove the remote actually answers before claiming to protect anything.
if ! rclone about {remote_name}: >/dev/null 2>&1; then
  if ! rclone lsd {remote_name}: >/dev/null 2>&1; then
    bs_log "FATAL: remote {remote_disp}: is configured but unreachable (auth expired?)"
    exit 1
  fi
fi

{source_checks}bs_log "preflight ok: rclone present, remote {remote_disp}: reachable"
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> BackupSync {
        BackupSync::new(
            vec!["/mnt/media".into()],
            "gdrive:bk",
            "daily",
            99,
            700,
            None,
        )
        .unwrap()
    }

    #[test]
    fn requires_rclone_to_be_installed() {
        let s = block(&cfg());
        assert!(s.contains("command -v rclone"));
        assert!(s.contains("FATAL: rclone is not installed"));
    }

    #[test]
    fn requires_the_remote_to_be_configured_not_just_named() {
        // An unconfigured remote makes rclone fall back to a LOCAL path — the
        // precise mechanism behind the self-referential predecessor.
        let s = block(&cfg());
        assert!(s.contains("rclone listremotes"));
        assert!(s.contains("grep -qx 'gdrive':"));
        assert!(s.contains("is not configured"));
    }

    #[test]
    fn proves_the_remote_answers_before_syncing() {
        let s = block(&cfg());
        assert!(s.contains("rclone about") || s.contains("rclone lsd"));
        assert!(s.contains("unreachable"));
    }

    #[test]
    fn every_source_must_exist() {
        let c = BackupSync::new(
            vec!["/mnt/a".into(), "/mnt/b".into()],
            "gdrive:bk",
            "daily",
            99,
            700,
            None,
        )
        .unwrap();
        let s = block(&c);
        assert!(s.contains("[ ! -d '/mnt/a' ]"));
        assert!(s.contains("[ ! -d '/mnt/b' ]"));
        assert_eq!(s.matches("does not exist").count(), 2);
    }

    #[test]
    fn every_precondition_hard_exits() {
        // No `|| true`, no warn-and-continue: a failed precondition must stop
        // the run, or the backup reports success having done nothing.
        let s = block(&cfg());
        assert_eq!(s.matches("exit 1").count(), 4);
    }

    #[test]
    fn explains_that_the_token_is_deliberately_unmanaged() {
        assert!(block(&cfg()).contains("does not manage the OAuth token"));
    }
}
