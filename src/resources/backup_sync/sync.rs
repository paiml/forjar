//! FJ-037: the sync + verify pass.
//!
//! The pass does two separable things, and the second is the one that matters:
//! it copies, and then it *proves* the copy exists by comparing checksums
//! against the remote. The predecessor did only the first, and reported
//! success from rsync's exit code — which is 0 when you copy a directory onto
//! itself.
//!
//! Verification uses `rclone check --combined`, which emits one line per file
//! with a status character, so coverage is a count of files the remote
//! actually holds with a matching hash — not a log line saying "complete".
//!
//! `-` means present locally and MISSING from the remote. That is the number
//! that was silently 2.1 TB.

use super::preflight;
use crate::core::shell_escape::sh_squote;
use crate::core::types::BackupSync;

/// Per-source sync + verify block.
fn source_block(src: &str, remote: &str, cap_gb: u64, bwlimit: &str) -> String {
    let src_q = sh_squote(src);
    // Each source lands under its own basename in the remote so two sources
    // can never collide into one directory.
    let leaf = src
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("root");
    let dest = format!("{remote}/{leaf}");
    let dest_q = sh_squote(&dest);
    let leaf_q = sh_squote(leaf);

    format!(
        r#"
# -- source: {leaf} --
bs_log "sync {leaf}: starting"
rclone sync {src_q} {dest_q} \
  --checksum \
  --max-transfer {cap_gb}G \
  --cutoff-mode soft \
  {bwlimit}--transfers 4 \
  --retries 3 \
  --low-level-retries 10 \
  --stats-one-line \
  --stats 5m 2>&1 | tail -20 || bs_log "sync {leaf}: rclone exited nonzero (may be transfer cap)"

# Verify by CHECKSUM, one line per file. Never trust the sync's exit code:
# rsync exits 0 having copied a directory onto itself, which is the whole
# reason this resource exists.
rclone check {src_q} {dest_q} --checksum --combined "$BS_TMP/check-{leaf}" >/dev/null 2>&1 || true
if [ -f "$BS_TMP/check-{leaf}" ]; then
  bs_m=$(bs_count '^= ' "$BS_TMP/check-{leaf}")
  bs_x=$(bs_count '^\* ' "$BS_TMP/check-{leaf}")
  bs_o=$(bs_count '^- ' "$BS_TMP/check-{leaf}")
  bs_e=$(bs_count '^! ' "$BS_TMP/check-{leaf}")
else
  bs_m=0; bs_x=0; bs_o=0; bs_e=1
  bs_log "verify {leaf}: rclone check produced NO output - treating as unverified"
fi
BS_MATCH=$((BS_MATCH + bs_m))
BS_DIFFER=$((BS_DIFFER + bs_x))
BS_MISSING=$((BS_MISSING + bs_o))
BS_ERROR=$((BS_ERROR + bs_e))
bs_log "verify {leaf_q}: matched=$bs_m differing=$bs_x missing=$bs_o errors=$bs_e"
"#
    )
}

/// Generate the complete sync script.
pub(super) fn script(cfg: &BackupSync, status_json: &str, log_tag: &str) -> String {
    let tag = sh_squote(log_tag);
    let remote = cfg.remote.trim_end_matches('/');
    let verify_pct = cfg.verify_pct;
    let bwlimit = cfg
        .bandwidth_limit
        .as_ref()
        .map(|b| format!("--bwlimit {} ", sh_squote(b)))
        .unwrap_or_default();
    let blocks: String = cfg
        .sources
        .iter()
        .map(|s| source_block(s, remote, cfg.daily_cap_gb, &bwlimit))
        .collect();

    format!(
        r#"#!/bin/sh
# forjar-managed backup sync. DO NOT EDIT - regenerate with `forjar apply`.
set -u

BS_STATUS="${{FORJAR_BACKUP_STATUS:-{status_json}}}"
BS_TMP=$(mktemp -d) || exit 1
# SEC011: guard the teardown. An unset or truncated BS_TMP would make the trap
# an `rm -rf /`, and a trap fires on paths that never reach normal cleanup.
bs_cleanup() {{
  [ -n "${{BS_TMP:-}}" ] || return 0
  [ "$BS_TMP" != "/" ] || return 0
  # Deliberately not `rm -rf`: `find -delete` cannot recurse outside the tree it
  # was given, and `rmdir` only removes an EMPTY directory. A wrong or truncated
  # BS_TMP therefore fails harmlessly instead of destroying whatever it names.
  find "$BS_TMP" -mindepth 1 -delete 2>/dev/null || true
  rmdir "$BS_TMP" 2>/dev/null || true
}}
trap bs_cleanup EXIT INT TERM

BS_MATCH=0
BS_DIFFER=0
BS_MISSING=0
BS_ERROR=0
BS_VERIFY_PCT={verify_pct}

bs_log() {{ echo "{tag}: $*"; }}

# `grep -c` PRINTS a count and still exits 1 when that count is zero. A
# `$(grep -c ... || echo 0)` therefore captures BOTH the real "0" and the
# fallback "0" as two lines, and the later $(( )) dies with "Illegal number".
# Same shape as `systemctl is-failed`, which also prints a state and exits
# non-zero: take the first line, default only when genuinely empty.
bs_count() {{
  bs_c=$(grep -c "$1" "$2" 2>/dev/null | head -1)
  echo "${{bs_c:-0}}"
}}
{preflight}
{blocks}
BS_TOTAL=$((BS_MATCH + BS_DIFFER + BS_MISSING + BS_ERROR))
if [ "$BS_TOTAL" -gt 0 ]; then
  BS_COVERAGE=$((BS_MATCH * 100 / BS_TOTAL))
else
  BS_COVERAGE=0
fi

# Fail closed on an empty result set.
#
# The predecessor's metric reported "Files: 0" and called it "Backup complete"
# - it could not distinguish "nothing to protect" from "protected nothing".
# Zero files examined is never healthy when sources are declared: it means the
# check did not run, not that the backup is fine.
if [ "$BS_TOTAL" -eq 0 ]; then
  BS_HEALTH=unverified
  bs_log "FAILED: verification examined 0 files. Sources are declared, so this is a"
  bs_log "        broken check, not an empty backup."
elif [ "$BS_MATCH" -eq 0 ]; then
  BS_HEALTH=unverified
  bs_log "FAILED: 0 of $BS_TOTAL files verified present in the remote."
elif [ "$BS_COVERAGE" -lt "$BS_VERIFY_PCT" ]; then
  BS_HEALTH=partial
else
  BS_HEALTH=verified
fi

cat > "$BS_STATUS" <<EOF
{{"remote":"{remote}","matched":$BS_MATCH,"differing":$BS_DIFFER,"missing":$BS_MISSING,"errors":$BS_ERROR,"total":$BS_TOTAL,"coverage_pct":$BS_COVERAGE,"verify_pct":$BS_VERIFY_PCT,"health":"$BS_HEALTH"}}
EOF

bs_log "complete: coverage=${{BS_COVERAGE}}% (matched=$BS_MATCH missing=$BS_MISSING differing=$BS_DIFFER errors=$BS_ERROR) health=$BS_HEALTH"

# A backup that cannot prove coverage is a failed backup. systemd records it,
# `forjar drift` sees it, and it stops looking like a healthy no-op.
if [ "$BS_HEALTH" != "verified" ]; then
  bs_log "FAILED: coverage ${{BS_COVERAGE}}% is below the required ${{BS_VERIFY_PCT}}%"
  exit 1
fi
exit 0
"#,
        preflight = preflight::block(cfg),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> BackupSync {
        BackupSync::new(
            vec!["/mnt/nvme-raid0/RecordedCourses".into()],
            "gdrive:lambda-labs-media",
            "daily",
            99,
            700,
            Some("50M".into()),
        )
        .unwrap()
    }

    #[test]
    fn verifies_by_checksum_not_by_exit_code() {
        let s = script(&cfg(), "/run/x.json", "backup");
        assert!(s.contains("rclone check"));
        assert!(s.contains("--checksum"));
        assert!(s.contains("--combined"));
        // The sync's own exit status must NOT be the health signal.
        assert!(s.contains("Never trust the sync's exit code"));
    }

    #[test]
    fn counts_files_missing_from_the_remote() {
        // `-` = present locally, absent remotely. This is the number that was
        // silently 2.1 TB on lambda-labs.
        let s = script(&cfg(), "/run/x.json", "backup");
        assert!(s.contains(r"bs_count '^- '"));
        assert!(s.contains("BS_MISSING"));
    }

    #[test]
    fn zero_examined_files_is_a_failure_not_a_success() {
        let s = script(&cfg(), "/run/x.json", "backup");
        assert!(s.contains(r#"[ "$BS_TOTAL" -eq 0 ]"#));
        assert!(s.contains("broken check, not an empty backup"));
        // ...and it must reach the nonzero exit.
        assert!(s.contains("BS_HEALTH=unverified"));
    }

    #[test]
    fn zero_matched_files_is_a_failure() {
        let s = script(&cfg(), "/run/x.json", "backup");
        assert!(s.contains(r#"[ "$BS_MATCH" -eq 0 ]"#));
    }

    #[test]
    fn unverified_coverage_exits_nonzero() {
        let s = script(&cfg(), "/run/x.json", "backup");
        assert!(s.contains(r#"[ "$BS_HEALTH" != "verified" ]"#));
        assert!(s.contains("exit 1"));
    }

    #[test]
    fn missing_check_output_counts_as_an_error_not_a_pass() {
        let s = script(&cfg(), "/run/x.json", "backup");
        assert!(s.contains("rclone check produced NO output"));
        assert!(s.contains("bs_e=1"));
    }

    #[test]
    fn caps_transfer_under_the_drive_daily_limit() {
        let s = script(&cfg(), "/run/x.json", "backup");
        assert!(s.contains("--max-transfer 700G"));
        assert!(s.contains("--cutoff-mode soft"));
    }

    #[test]
    fn applies_bandwidth_limit_when_declared() {
        assert!(script(&cfg(), "/run/x.json", "backup").contains("--bwlimit '50M'"));
        let no_bw =
            BackupSync::new(vec!["/mnt/a".into()], "gdrive:x", "daily", 99, 700, None).unwrap();
        assert!(!script(&no_bw, "/run/x.json", "backup").contains("--bwlimit"));
    }

    #[test]
    fn each_source_lands_under_its_own_leaf() {
        let c = BackupSync::new(
            vec!["/mnt/a/media".into(), "/mnt/b/media2".into()],
            "gdrive:bk",
            "daily",
            99,
            700,
            None,
        )
        .unwrap();
        let s = script(&c, "/run/x.json", "backup");
        assert!(s.contains("'gdrive:bk/media'"));
        assert!(s.contains("'gdrive:bk/media2'"));
    }

    #[test]
    fn status_has_no_timestamp_field() {
        // DET002: no `date` anywhere. The status file's mtime is the heartbeat.
        let s = script(&cfg(), "/run/x.json", "backup");
        assert!(!s.contains("date +%s"));
    }

    #[test]
    fn counters_never_capture_a_double_line() {
        // `grep -c` prints "0" and exits 1 on no match; `|| echo 0` appends a
        // second line and the arithmetic expansion then fails with
        // "Illegal number", aborting the run before it can report health.
        let s = script(&cfg(), "/run/x.json", "backup");
        // Comment-excluded: the rationale comment above the helper necessarily
        // quotes the very construct it bans, so a whole-string `contains` here
        // matches the explanation rather than any real code.
        for line in s.lines().filter(|l| !l.trim_start().starts_with('#')) {
            assert!(
                !line.contains("|| echo 0"),
                "counter must not use a || fallback: {line}"
            );
        }
        assert!(s.contains("bs_count() {"));
        assert!(s.contains("| head -1"));
    }

    #[test]
    fn temp_teardown_is_guarded() {
        // SEC011: an unset BS_TMP turns the EXIT trap into `rm -rf /`.
        let s = script(&cfg(), "/run/x.json", "backup");
        assert!(s.contains(r#"[ -n "${BS_TMP:-}" ] || return 0"#));
        assert!(s.contains(r#"[ "$BS_TMP" != "/" ] || return 0"#));
        // No `rm -rf` in the teardown at all: find/rmdir bound the blast radius.
        // Line-scoped and comment-excluded — the rationale comment names the
        // very construct it is banning.
        for line in s.lines().filter(|l| !l.trim_start().starts_with('#')) {
            assert!(
                !line.contains("rm -rf"),
                "teardown must not use rm -rf: {line}"
            );
        }
    }

    #[test]
    fn log_tag_carries_no_square_brackets() {
        // SC1140: bashrs parses `[...]` inside a string as a test expression.
        let s = script(&cfg(), "/run/x.json", "backup");
        assert!(!s.contains("[backup]"));
    }
}
