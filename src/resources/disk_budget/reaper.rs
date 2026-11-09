//! FJ-036: the reclaim pass — a watermark-driven loop, not a TTL sweep.
//!
//! The loop deletes oldest-first through the declared rules and stops the
//! instant the target watermark is met. Two properties matter more than the
//! deletion itself:
//!
//!   1. **It is driven by observed free space.** It reads `df` before it starts
//!      and again after every delete. A pass that never looks at the resource
//!      it is defending cannot know whether it worked.
//!   2. **A triggered pass that misses its target exits non-zero.** That is the
//!      anti-inertness clause. The predecessor exited 0 whether it reclaimed
//!      250G or nothing, so a reaper that had stopped working looked exactly
//!      like a machine that had nothing to reclaim — for a month, across two
//!      100%-full events. Absence of reclaim under pressure is now a failure.

use super::detect;
use crate::core::shell_escape::sh_squote;
use crate::core::types::{DiskBudget, ReclaimRule};

/// Emit the `df`-reading preamble: current used-%, free bytes, free GiB.
fn read_df(path: &str) -> String {
    let p = sh_squote(path);
    format!(
        r#"
fb_read_df() {{
  # POSIX `df -P` keeps the record on one line even for long device names.
  set -- $(df -P -k {p} 2>/dev/null | awk 'NR==2{{gsub(/%/,"",$5); print $5, $4, $2}}')
  FB_USED_PCT="${{1:-0}}"
  FB_FREE_KB="${{2:-0}}"
  FB_TOTAL_KB="${{3:-0}}"
  FB_FREE_GB=$((FB_FREE_KB / 1024 / 1024))
}}
"#
    )
}

/// Rule name reduced to a shell-comment-safe label.
///
/// The name reaches the generated script in two places with different escaping
/// needs: a `#` comment marker (which must not be able to carry a newline out
/// of the comment) and a `fb_log` argument (which is quoted). Sanitising here
/// keeps the marker readable — `rule: agent-targets`, not `rule: 'agent-targets'`.
fn label(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// forjar#669: the bytes this rule could free if EVERY eligible candidate went.
/// Same eligibility as the delete loop (idle, not in use, strictly under a
/// declared root), summed into FB_REACH_KB, and nothing is removed.
fn rule_preflight(rule: &ReclaimRule) -> String {
    let finder = detect::fn_name(rule.kind);
    let idle = rule.min_idle_minutes;
    let roots: Vec<String> = rule.roots.iter().map(|r| sh_squote(r)).collect();
    let roots = roots.join(" ");
    let mut prefixes: Vec<String> = rule
        .roots
        .iter()
        .map(|r| ReclaimRule::root_prefix(r))
        .collect();
    prefixes.dedup();
    let prefixes: Vec<String> = prefixes.iter().map(|p| sh_squote(p)).collect();
    let prefixes = prefixes.join(" ");
    format!(
        r#"
: >"$FB_UNMLOG"
{finder} {roots} >"$FB_CANDS" 2>/dev/null || true
# An unfinished walk is not a measurement: leave the verdict to the rule block,
# which logs NOT MEASURED, rather than calling the target unreachable.
if [ -s "$FB_UNMLOG" ]; then
  FB_REACH_UNK=1
  : >"$FB_UNMLOG"
fi
while IFS= read -r cand; do
  [ -n "$cand" ] || continue
  fb_in_use "$cand" && continue
  fb_is_idle "$cand" {idle} || continue
  fb_sweepable "$cand" {prefixes} || continue
  sz=$(fb_bytes "$cand")
  FB_REACH_KB=$((FB_REACH_KB + ${{sz:-0}} / 1024))
done < "$FB_CANDS"
"#
    )
}

/// The per-rule reclaim block.
fn rule_block(rule: &ReclaimRule) -> String {
    let name = label(&rule.name);
    let log_msg = sh_squote(&format!("rule {}: scanning", label(&rule.name)));
    let finder = detect::fn_name(rule.kind);
    let idle = rule.min_idle_minutes;
    let roots: Vec<String> = rule.roots.iter().map(|r| sh_squote(r)).collect();
    let roots = roots.join(" ");
    let mut prefixes: Vec<String> = rule
        .roots
        .iter()
        .map(|r| ReclaimRule::root_prefix(r))
        .collect();
    prefixes.dedup();
    let prefixes: Vec<String> = prefixes.iter().map(|p| sh_squote(p)).collect();
    let prefixes = prefixes.join(" ");
    let pre = detect::pre_delete(rule.kind);
    let post = detect::post_delete(rule.kind);

    // Candidates go to a file and the loop reads from it, rather than
    // `finder | while read`. Two reasons: a piped `while` runs in a SUBSHELL, so
    // `FB_MET` set inside it is lost (which is why an earlier revision needed a
    // flag file); and bashrs cannot see the loop through the pipe, so every
    // `continue` in the body trips SC2242 and the whole script fails forjar's
    // I8 purification gate.
    format!(
        r#"
# -- rule: {name} ({kind}) --
# FAIL SAFE: without a completed /proc scan nothing is PROVEN dead, so the rule
# deletes nothing (forjar#727: a timed-out or empty scan is not "no one is
# using it").
if [ "$FB_MET" != "1" ] && [ "$FB_OPEN_OK" != "1" ]; then
  fb_log "rule {name}: open-path scan not measured - nothing proven dead, deleting nothing"
fi
if [ "$FB_MET" != "1" ] && [ "$FB_OPEN_OK" = "1" ] && [ "$FB_UNREACHABLE" != "1" ]; then
  fb_log {log_msg}
  {finder} {roots} >"$FB_CANDS" 2>/dev/null || true
  if [ -s "$FB_UNMLOG" ]; then
    FB_UNMEASURED=1
    fb_log "  NOT MEASURED, no candidates from the unfinished walk: $(tr '\n' ';' <"$FB_UNMLOG")"
    : >"$FB_UNMLOG"
  fi
  while IFS= read -r cand; do
    [ -n "$cand" ] || continue
    fb_read_df
    if [ "$FB_USED_PCT" -le "$FB_TARGET_USED" ]; then
      FB_MET=1
      break
    fi
    # forjar#667: ONE in-use check for every kind, before the idle check and the rm.
    fb_in_use "$cand" && {{ fb_log "  keep (in use): $cand"; continue; }}
    fb_is_idle "$cand" {idle} || {{ fb_log "  keep (active <{idle}m): $cand"; continue; }}
    # SEC011: last line of defence for a script whose job is `rm -rf`. Every
    # candidate arrives from a glob or a find; if one ever resolves a level too
    # high, or to empty, abort on it rather than delete. `fb_sweepable` also
    # requires it to sit strictly under the literal prefix of a declared root.
    fb_sweepable "$cand" {prefixes} || continue
    sz=$(fb_bytes "$cand")
{pre}    if [ "$FB_DRY" = "1" ]; then
      fb_log "  DRY-RUN would reclaim ${{sz:-0}} bytes: $cand"
      echo "${{sz:-0}}" >>"$FB_WOULD"
    else
      # SEC011: re-assert immediately adjacent to the rm. `fb_sweepable` above
      # is the real check; this is the one a reader — and the linter — sees
      # without following a call.
      if [ -z "$cand" ] || [ "$cand" = "/" ]; then
        fb_log "  REFUSE to remove: $cand"
        continue
      fi
      rm -rf -- "$cand" || {{ fb_log "  FAILED to remove: $cand"; continue; }}
{post}      fb_log "  reclaimed ${{sz:-0}} bytes: $cand"
      # The ledger is FREED bytes, so the append lives inside the delete
      # branch. It used to sit after the `fi`, so a preview accumulated bytes
      # it had not freed: FB_RECLAIMED > 0, health=effective, and a
      # `reclaimed_bytes` figure in the status file for deletions that never
      # happened. A dry run was indistinguishable from a reclaim.
      echo "${{sz:-0}}" >>"$FB_LEDGER"
    fi
  done < "$FB_CANDS"
fi
"#,
        kind = rule.kind,
    )
}

/// Generate the complete reaper script for a budget.
pub(super) fn script(budget: &DiskBudget, status_json: &str, log_tag: &str) -> String {
    let path_q = sh_squote(&budget.path);
    // Shell-quoting is for the SHELL; inside the JSON body the path needs JSON
    // quoting, or the status file is not parseable ({{"path":'/'}} is not JSON).
    let path_json = budget.path.replace('\\', "\\\\").replace('"', "\\\"");
    let tag = sh_squote(log_tag);
    let high = budget.high_watermark_pct;
    let target_used = budget.target_used_pct();
    let crit = budget.critical_free_gb;
    let rules: String = budget.reclaim.iter().map(rule_block).collect();
    let preflight: String = budget.reclaim.iter().map(rule_preflight).collect();

    format!(
        r#"#!/bin/sh
# forjar-managed disk-budget reaper for {path_q}. DO NOT EDIT — regenerate with
# `forjar apply`. Hand edits are reverted on the next apply and will not survive.
set -u

FB_TARGET_USED={target_used}
FB_HIGH={high}
FB_CRIT_GB={crit}
# DELETING IS THE OPT-IN, NOT THE DEFAULT.
#
# This used to default FB_DRY from FORJAR_BUDGET_DRY_RUN, i.e. delete unless
# the operator's shell said otherwise. That variable is read HERE, at the far
# end of a chain that strips it: `sudo bash <<'FORJAR_SUDO'` resets the
# environment and `ssh host bash` never carries it, so the documented preview
# reached this line with nothing set and reclaimed 1.5 TB while reporting
# `1 converged` (#334). Nothing forjar can do makes an ambient variable survive
# that, so the default is inverted instead: a reaper run by hand INSPECTS.
# Deleting requires FORJAR_BUDGET_EXECUTE=1, which only the systemd unit and
# `forjar apply` grant. FORJAR_BUDGET_DRY_RUN still works and still wins, so
# the variable the fleet notes document stops being a lie.
FB_DRY=1
if [ "${{FORJAR_BUDGET_EXECUTE:-0}}" = "1" ]; then FB_DRY=0; fi
if [ "${{FORJAR_BUDGET_DRY_RUN:-0}}" = "1" ]; then FB_DRY=1; fi
if [ "$FB_DRY" = "1" ]; then FB_MODE=dry-run; else FB_MODE=execute; fi
FB_STATUS="${{FORJAR_BUDGET_STATUS:-{status_json}}}"
FB_LEDGER=$(mktemp) || exit 1
FB_WOULD=$(mktemp) || exit 1
FB_CANDS=$(mktemp) || exit 1
FB_OPEN=$(mktemp) || exit 1
FB_PROCRAW=$(mktemp) || exit 1
FB_MARKERS=$(mktemp) || exit 1
FB_UNMLOG=$(mktemp) || exit 1
FB_MET=0
FB_UNREACHABLE=0
trap 'rm -f "$FB_LEDGER" "$FB_WOULD" "$FB_CANDS" "$FB_OPEN" "$FB_PROCRAW" "$FB_MARKERS" "$FB_UNMLOG"' EXIT INT TERM

# Tag carries no square brackets: bashrs parses `[...]` inside the string
# as a test expression (SC1140) and rejects the script.
fb_log() {{ echo "{tag}: $*"; }}
{df_reader}{prelude}{detectors}
fb_read_df
FB_USED_BEFORE="$FB_USED_PCT"
FB_FREE_GB_BEFORE="$FB_FREE_GB"
fb_log "start: {path_q} at ${{FB_USED_PCT}}% used, ${{FB_FREE_GB}}G free (trigger ${{FB_HIGH}}%, target ${{FB_TARGET_USED}}%) mode=$FB_MODE"

if [ "$FB_USED_PCT" -lt "$FB_HIGH" ]; then
  fb_log "under watermark - no reclaim needed"
  FB_TRIGGERED=0
else
  FB_TRIGGERED=1
  fb_scan_open_paths
  # forjar#669: can freeing every eligible candidate reach the target at all? If
  # not, say so and delete NOTHING: a missed target must never escalate into a
  # time-to-live sweep of every idle candidate.
  FB_REACH_KB=0
  FB_REACH_UNK=0
  if [ "$FB_OPEN_OK" = "1" ]; then
{preflight}  fi
  FB_NEED_KB=$((FB_TOTAL_KB / 100 * (FB_USED_PCT - FB_TARGET_USED)))
  if [ "$FB_OPEN_OK" = "1" ] && [ "$FB_REACH_UNK" != "1" ] && [ "$FB_REACH_KB" -lt "$FB_NEED_KB" ]; then
    FB_UNREACHABLE=1
    fb_log "target unreachable: need ${{FB_NEED_KB}}K, every eligible candidate offers ${{FB_REACH_KB}}K (shortfall $((FB_NEED_KB - FB_REACH_KB))K) - deleting nothing"
  fi
{rules}fi

fb_read_df
FB_RECLAIMED=$(awk '{{s+=$1}} END{{print s+0}}' "$FB_LEDGER" 2>/dev/null || echo 0)
FB_WOULD_BYTES=$(awk '{{s+=$1}} END{{print s+0}}' "$FB_WOULD" 2>/dev/null || echo 0)
FB_MET_FINAL=0
[ "$FB_USED_PCT" -le "$FB_TARGET_USED" ] && FB_MET_FINAL=1

if [ "$FB_FREE_GB" -lt "$FB_CRIT_GB" ]; then FB_TIER=critical
elif [ "$FB_USED_PCT" -ge "$FB_HIGH" ]; then FB_TIER=pressure
else FB_TIER=ok
fi

# A triggered pass that reclaimed nothing is the inertness signature.
if [ "$FB_TRIGGERED" = "1" ] && [ "$FB_RECLAIMED" -eq 0 ]; then
  FB_HEALTH=inert
elif [ "$FB_TRIGGERED" = "1" ]; then
  FB_HEALTH=effective
else
  FB_HEALTH=idle
fi

# A PREVIEW MUST NOT REWRITE THE HEARTBEAT. This file is both the freshness
# heartbeat (`disk_budget_heartbeat`) and the drift-hashed `disk_budget_health`
# that state_query reads. A dry pass that overwrote it would stamp a
# health/tier/reclaimed record for deletions that never happened onto the
# machine's own health record, which is the thing drift trusts.
if [ "$FB_DRY" != "1" ]; then
cat >"$FB_STATUS" <<EOF
{{"path":"{path_json}","used_pct_before":$FB_USED_BEFORE,"used_pct_after":$FB_USED_PCT,"free_gb_before":$FB_FREE_GB_BEFORE,"free_gb_after":$FB_FREE_GB,"reclaimed_bytes":$FB_RECLAIMED,"triggered":$FB_TRIGGERED,"target_met":$FB_MET_FINAL,"tier":"$FB_TIER","health":"$FB_HEALTH","dry_run":0}}
EOF
else
  fb_log "preview: heartbeat not written; would reclaim ${{FB_WOULD_BYTES}} bytes"
fi

# "complete", not "done": bashrs reads a leading `done` inside the string
# as the loop keyword (SC1035) and rejects the script.
fb_log "complete: ${{FB_USED_PCT}}% used, ${{FB_FREE_GB}}G free, unmeasured=$FB_UNMEASURED, reclaimed ${{FB_RECLAIMED}} bytes, would_reclaim ${{FB_WOULD_BYTES}} bytes, tier=$FB_TIER health=$FB_HEALTH mode=$FB_MODE"

# Exit non-zero when a triggered pass failed to reach target. systemd marks the
# unit failed, `forjar drift` sees it, and an inert reaper stops being invisible.
#
# A PREVIEW IS EXEMPT. The clause exists to catch a reclaim that achieved
# nothing; a preview achieving nothing is the correct outcome, and failing it
# would make `sh /usr/local/sbin/forjar-disk-budget-*.sh` exit 1 on every
# healthy-but-over-watermark machine an operator inspected.
if [ "$FB_DRY" != "1" ] && [ "$FB_TRIGGERED" = "1" ] && [ "$FB_MET_FINAL" != "1" ]; then
  fb_log "FAILED: still ${{FB_USED_PCT}}% used after reclaim; budget target ${{FB_TARGET_USED}}% not met"
  exit 1
fi
exit 0
"#,
        df_reader = read_df(&budget.path),
        prelude = detect::prelude(),
        detectors = detect::all_detectors(),
    )
}

#[cfg(test)]
mod unit_tests;
