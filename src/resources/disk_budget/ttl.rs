//! RAID-REAP Phase C: the TTL reclaim kind (`ttl_tree`) and the day-based idle
//! floor for `abandoned_worktree`.
//!
//! The watermark engine decides WHEN to reclaim; these rules decide WHAT is old
//! enough to be disposable. Both are declared per rule and validated before any
//! script is generated, so a bad declaration fails `forjar apply`, not the
//! reaper at 3am.

use crate::core::types::{ReclaimKind, ReclaimRule};
use crate::resources::tmp_reaper::parse_age;

/// Minutes in a day, for the `idle_days` to minutes mapping.
const MINUTES_PER_DAY: u64 = 1440;

/// Reject a rule whose TTL or idle fields do not fit its kind.
///
/// # Errors
///
/// Returns `Err` naming the rule when `max_age` is missing for `ttl_tree` or
/// malformed, when `max_age` is set on another kind, or when `idle_days` is set
/// on a kind other than `abandoned_worktree` or is zero.
pub(super) fn validate_rule(rule: &ReclaimRule) -> Result<(), String> {
    let bad = |why: &str| format!("disk_budget rule '{}': {why}", rule.name);
    match rule.kind {
        ReclaimKind::TtlTree => {
            let age = rule
                .max_age
                .as_deref()
                .ok_or_else(|| bad("kind ttl_tree needs max_age (e.g. 14d)"))?;
            parse_age(age).map_err(|e| bad(&e))?;
        }
        _ if rule.max_age.is_some() => {
            return Err(bad("max_age applies only to kind ttl_tree"));
        }
        _ => {}
    }
    match (rule.kind, rule.idle_days) {
        (ReclaimKind::AbandonedWorktree, Some(0)) => {
            return Err(bad("idle_days must be at least 1"));
        }
        (_, Some(_)) if rule.kind != ReclaimKind::AbandonedWorktree => {
            return Err(bad("idle_days applies only to kind abandoned_worktree"));
        }
        _ => {}
    }
    Ok(())
}

/// The idle floor in minutes the reaper applies to each candidate. `idle_days`
/// wins over `min_idle_minutes` when set (RAID-REAP R3: 7 days).
pub(super) fn idle_minutes(rule: &ReclaimRule) -> u64 {
    rule.idle_days
        .map_or(rule.min_idle_minutes, |d| d.saturating_mul(MINUTES_PER_DAY))
}

/// Arguments spliced before the roots in the finder call. A `ttl_tree` finder
/// takes its TTL in minutes first; every other finder takes roots only.
///
/// A TTL that does not parse yields the empty string, which the detector reads
/// as "no TTL": it emits nothing (fail closed). `validate_rule` rejects that
/// case before a script is ever generated, so this path is defence in depth.
pub(super) fn finder_args(rule: &ReclaimRule) -> String {
    match (rule.kind, rule.max_age.as_deref()) {
        (ReclaimKind::TtlTree, Some(age)) => {
            parse_age(age).map(|m| format!("{m} ")).unwrap_or_default()
        }
        _ => String::new(),
    }
}

/// Shell detector for `ttl_tree`: top-level entries under each root whose
/// content has not changed within the TTL, oldest first.
///
/// Two guards keep an entry out of the candidate list even when it is old:
/// a deliverable anywhere under it (media, `receipts/`, `evidence/`) keeps the
/// whole entry, and a timed-out walk is recorded as NOT MEASURED and skipped.
pub(super) fn detector() -> String {
    r#"
fb_find_ttl_tree() {
  fb_ttl_min="$1"
  shift
  case "$fb_ttl_min" in
    ''|*[!0-9]*) return 0 ;;
  esac
  [ "$fb_ttl_min" -gt 0 ] || return 0
  for root in "$@"; do
    [ -d "$root" ] || continue
    for d in "$root"/*; do
      [ -e "$d" ] || continue
      fb_in_use "$d" && continue
      fb_hit=$(fb_bounded find "$d" -xdev \( -type f \( -name '*.mp4' -o -name '*.mov' -o -name '*.wav' -o -name '*.mp3' -o -name '*.srt' -o -name '*.vtt' -o -name '*.apr' -o -name '*.gguf' -o -name '*.safetensors' -o -name '*.onnx' \) -o -type d \( -name receipts -o -name evidence \) \) -print -quit 2>/dev/null)
      fb_rc=$?
      if fb_timed_out "$fb_rc"; then echo "ttl scan of $d timed out" >>"$FB_UNMLOG"; continue; fi
      [ -z "$fb_hit" ] || continue
      fb_hit=$(fb_bounded find "$d" -xdev -mmin "-$fb_ttl_min" -print -quit 2>/dev/null)
      fb_rc=$?
      if fb_timed_out "$fb_rc"; then echo "ttl age of $d timed out" >>"$FB_UNMLOG"; continue; fi
      [ -z "$fb_hit" ] || continue
      fb_stamp "$d"
    done
  done | sort -n | cut -f2-
}
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(kind: ReclaimKind) -> ReclaimRule {
        ReclaimRule {
            name: "r".into(),
            roots: vec!["/mnt/nvme-raid0/tmp".into()],
            kind,
            min_idle_minutes: 60,
            max_age: None,
            idle_days: None,
        }
    }

    #[test]
    fn ttl_tree_requires_a_parseable_max_age() {
        assert!(validate_rule(&rule(ReclaimKind::TtlTree)).is_err());
        let mut r = rule(ReclaimKind::TtlTree);
        r.max_age = Some("14d".into());
        assert!(validate_rule(&r).is_ok());
        r.max_age = Some("fourteen".into());
        assert!(validate_rule(&r).is_err());
    }

    #[test]
    fn max_age_rejected_on_other_kinds() {
        let mut r = rule(ReclaimKind::CargoTarget);
        r.max_age = Some("14d".into());
        let err = validate_rule(&r).unwrap_err();
        assert!(err.contains("only to kind ttl_tree"), "{err}");
    }

    #[test]
    fn idle_days_only_on_abandoned_worktree_and_positive() {
        let mut r = rule(ReclaimKind::AbandonedWorktree);
        r.idle_days = Some(7);
        assert!(validate_rule(&r).is_ok());
        r.idle_days = Some(0);
        assert!(validate_rule(&r).is_err());
        let mut t = rule(ReclaimKind::TtlTree);
        t.max_age = Some("3d".into());
        t.idle_days = Some(7);
        assert!(validate_rule(&t).is_err());
    }

    #[test]
    fn idle_days_maps_to_minutes_and_overrides_floor() {
        let mut r = rule(ReclaimKind::AbandonedWorktree);
        assert_eq!(idle_minutes(&r), 60);
        r.idle_days = Some(7);
        assert_eq!(idle_minutes(&r), 10_080);
    }

    #[test]
    fn finder_args_carry_ttl_minutes_only_for_ttl_tree() {
        let mut r = rule(ReclaimKind::TtlTree);
        r.max_age = Some("14d".into());
        assert_eq!(finder_args(&r), "20160 ");
        assert_eq!(finder_args(&rule(ReclaimKind::CargoTarget)), "");
    }

    #[test]
    fn detector_fails_closed_on_a_non_numeric_ttl() {
        let d = detector();
        assert!(d.contains("''|*[!0-9]*) return 0"), "{d}");
        assert!(d.contains("-name '*.mp4'"), "deliverable guard missing");
    }
}
