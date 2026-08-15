//! FJ-036: candidate-detection shell for the disk-budget reaper.
//!
//! Each `ReclaimKind` contributes one shell function that prints candidate
//! paths on stdout, **oldest first**, one per line. The reaper consumes that
//! stream and deletes until the target watermark is met.
//!
//! Everything here is written to be safe against the two ways a reaper of this
//! shape historically goes wrong:
//!
//!   * **Deleting the wrong thing.** Detection never keys on a directory's
//!     name. Cargo build dirs are found by the markers cargo writes, git
//!     worktrees by asking git, scratchpads by their structural position.
//!   * **Deleting live work.** Every kind has an explicit protection predicate,
//!     and a candidate that cannot be proven disposable is skipped, not swept.

use crate::core::types::ReclaimKind;

/// Shared helpers: idle test, size accounting, and the /proc liveness scan.
pub(super) fn prelude() -> String {
    // `fb_` prefix = forjar budget, to avoid colliding with anything the host's
    // shell profile defines.
    r#"
# --- shared helpers -------------------------------------------------------
# A path is idle if no regular file under it was modified within N minutes.
# This is a floor that protects in-flight builds, not the reclaim policy.
fb_is_idle() {
  [ -e "$1" ] || return 1
  ! find "$1" -type f -mmin "-$2" -print -quit 2>/dev/null | grep -q .
}

fb_bytes() { du -sx --block-size=1 "$1" 2>/dev/null | cut -f1; }

# Absolute paths currently held open by ANY process (cwd or fd). One pass over
# /proc; the caller greps this file rather than re-scanning per candidate.
fb_scan_open_paths() {
  : >"$FB_OPEN"
  for p in /proc/[0-9]*; do
    for l in "$p/cwd" "$p/fd"/*; do
      t=$(readlink "$l" 2>/dev/null) || continue
      case "$t" in /*) printf '%s\n' "$t" ;; esac
    done
  done 2>/dev/null | sort -u >"$FB_OPEN"
}

# True when some live process sits inside $1.
fb_in_use() {
  [ -s "$FB_OPEN" ] || return 1
  grep -qF -e "$1" "$FB_OPEN"
}

# Emit "<mtime_epoch>\t<path>" so the caller can sort oldest-first.
fb_stamp() { printf '%s\t%s\n' "$(stat -c %Y "$1" 2>/dev/null || echo 0)" "$1"; }
"#
    .to_string()
}

/// Shell function name that enumerates candidates for `kind`.
pub(super) const fn fn_name(kind: ReclaimKind) -> &'static str {
    match kind {
        ReclaimKind::CargoTarget => "fb_find_cargo_target",
        ReclaimKind::ClaudeScratchpad => "fb_find_claude_scratchpad",
        ReclaimKind::AbandonedWorktree => "fb_find_abandoned_worktree",
        ReclaimKind::Glob => "fb_find_glob",
    }
}

/// Cargo build directories, identified by cargo's own markers.
///
/// Requires **both** `CACHEDIR.TAG` and `.rustc_info.json`. That conjunction is
/// load-bearing, not belt-and-braces: `~/.cargo/registry/` carries a
/// `CACHEDIR.TAG` and no `.rustc_info.json`, so requiring both is precisely
/// what keeps the reaper out of the registry. Verified on lambda-labs
/// 2026-08-15 — the registry has the tag, lacks the info file, and no directory
/// beneath it carries the pair.
fn cargo_target() -> String {
    r#"
fb_find_cargo_target() {
  for root in "$@"; do
    [ -d "$root" ] || continue
    find "$root" -mindepth 1 -maxdepth 7 -type f -name CACHEDIR.TAG 2>/dev/null | while read -r tag; do
      d=$(dirname "$tag")
      # BOTH markers required — CACHEDIR.TAG alone matches the cargo registry.
      [ -f "$d/.rustc_info.json" ] || continue
      fb_stamp "$d"
    done
  done | sort -n | cut -f2-
}
"#
    .to_string()
}

/// Claude Code agent scratchpads: `<root>/<project>/<session>/scratchpad`.
///
/// Skipped whenever the session is live. Liveness is the union of two signals,
/// because either alone under-detects: a process holding a cwd/fd inside the
/// session dir (catches a session mid-tool-call), and a live `claude` process
/// whose cwd maps to the project slug (catches an idle session waiting on the
/// user, which holds nothing open and would otherwise look abandoned).
fn claude_scratchpad() -> String {
    r#"
fb_find_claude_scratchpad() {
  for root in "$@"; do
    [ -d "$root" ] || continue
    # Sessions whose project has a live `claude` process (slug = cwd with / -> -).
    fb_live_slugs=""
    for p in /proc/[0-9]*; do
      c=$(tr '\0' '\n' <"$p/cmdline" 2>/dev/null | head -1) || continue
      case "${c##*/}" in claude) ;; *) continue ;; esac
      w=$(readlink "$p/cwd" 2>/dev/null) || continue
      fb_live_slugs="$fb_live_slugs $(printf '%s' "$w" | tr '/' '-')"
    done 2>/dev/null
    for s in "$root"/*/*/scratchpad; do
      [ -d "$s" ] || continue
      sess=$(dirname "$s"); proj=$(basename "$(dirname "$sess")")
      fb_in_use "$sess" && continue
      case " $fb_live_slugs " in *" $proj "*) continue ;; esac
      fb_stamp "$s"
    done
  done | sort -n | cut -f2-
}
"#
    .to_string()
}

/// Abandoned git worktrees — removed whole, then pruned from the registry.
///
/// A worktree is abandoned only when git itself says it holds nothing: clean
/// status, an upstream, and zero commits ahead of that upstream. A worktree
/// with no upstream is NOT abandoned — it is unpushed work, and this is the one
/// rule in the reaper that can destroy something a rebuild cannot recreate, so
/// it fails closed on every uncertainty (git errors included).
fn abandoned_worktree() -> String {
    r#"
fb_find_abandoned_worktree() {
  for root in "$@"; do
    [ -d "$root" ] || continue
    for d in "$root"/*; do
      [ -e "$d/.git" ] || continue
      # A linked worktree has a .git FILE ("gitdir: ..."), not a directory;
      # never offer the primary checkout as a candidate.
      [ -f "$d/.git" ] || continue
      fb_in_use "$d" && continue
      git -C "$d" rev-parse --git-dir >/dev/null 2>&1 || continue
      # Dirty tree (tracked or untracked) => keep.
      [ -n "$(git -C "$d" status --porcelain 2>/dev/null)" ] && continue
      # No upstream => unpushed work => keep. Fails closed on git error.
      up=$(git -C "$d" rev-parse --abbrev-ref '@{upstream}' 2>/dev/null) || continue
      [ -n "$up" ] || continue
      ahead=$(git -C "$d" rev-list --count "$up..HEAD" 2>/dev/null) || continue
      [ "$ahead" = "0" ] || continue
      fb_stamp "$d"
    done
  done | sort -n | cut -f2-
}
"#
    .to_string()
}

/// Literal glob paths — leaked test fixtures and similar debris.
fn glob() -> String {
    r#"
fb_find_glob() {
  for pat in "$@"; do
    for d in $pat; do
      [ -e "$d" ] || continue
      fb_in_use "$d" && continue
      fb_stamp "$d"
    done
  done | sort -n | cut -f2-
}
"#
    .to_string()
}

/// All detector function definitions, emitted once into the reaper.
pub(super) fn all_detectors() -> String {
    format!(
        "{}{}{}{}",
        cargo_target(),
        claude_scratchpad(),
        abandoned_worktree(),
        glob()
    )
}

/// Post-delete hook for a kind (e.g. pruning git's worktree registry).
pub(super) const fn post_delete(kind: ReclaimKind) -> &'static str {
    match kind {
        ReclaimKind::AbandonedWorktree => {
            "  git -C \"$(dirname \"$cand\")\" worktree prune 2>/dev/null || true\n"
        }
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_detector_requires_both_markers() {
        let s = cargo_target();
        assert!(s.contains("CACHEDIR.TAG"));
        assert!(s.contains(".rustc_info.json"));
        // The conjunction is what excludes ~/.cargo/registry. If this `continue`
        // guard is ever dropped, the reaper eats the registry.
        assert!(
            s.contains("[ -f \"$d/.rustc_info.json\" ] || continue"),
            "cargo target detection must require BOTH markers"
        );
    }

    #[test]
    fn cargo_detector_never_matches_by_name() {
        // Name-matching is the bug this replaces: it missed `.target` (189G)
        // and would eat the `cc` crate's source `src/target/`.
        let s = cargo_target();
        assert!(
            !s.contains("-name target"),
            "detection must not key on names"
        );
        assert!(!s.contains("-name '.target'"));
    }

    #[test]
    fn worktree_detector_fails_closed_without_upstream() {
        let s = abandoned_worktree();
        assert!(s.contains("@{upstream}"));
        assert!(s.contains("rev-list --count"));
        assert!(s.contains("status --porcelain"));
        // Every guard must `continue` (keep), never fall through to delete.
        assert!(s.contains("|| continue"));
    }

    #[test]
    fn worktree_detector_skips_primary_checkout() {
        // A primary checkout has a .git DIRECTORY; only linked worktrees (.git
        // file) are ever candidates.
        assert!(abandoned_worktree().contains("[ -f \"$d/.git\" ] || continue"));
    }

    #[test]
    fn scratchpad_detector_has_both_liveness_signals() {
        let s = claude_scratchpad();
        assert!(s.contains("fb_in_use"), "open-handle signal missing");
        assert!(s.contains("fb_live_slugs"), "project-slug signal missing");
    }

    #[test]
    fn every_kind_has_a_detector_definition() {
        let all = all_detectors();
        for k in [
            ReclaimKind::CargoTarget,
            ReclaimKind::ClaudeScratchpad,
            ReclaimKind::AbandonedWorktree,
            ReclaimKind::Glob,
        ] {
            assert!(
                all.contains(&format!("{}()", fn_name(k))),
                "no detector emitted for {k}"
            );
        }
    }

    #[test]
    fn only_worktrees_prune_after_delete() {
        assert!(post_delete(ReclaimKind::AbandonedWorktree).contains("worktree prune"));
        assert_eq!(post_delete(ReclaimKind::CargoTarget), "");
        assert_eq!(post_delete(ReclaimKind::Glob), "");
        assert_eq!(post_delete(ReclaimKind::ClaudeScratchpad), "");
    }

    #[test]
    fn candidates_are_emitted_oldest_first() {
        // Each detector stamps mtime then `sort -n | cut -f2-`.
        for s in [
            cargo_target(),
            claude_scratchpad(),
            abandoned_worktree(),
            glob(),
        ] {
            assert!(s.contains("sort -n | cut -f2-"), "not oldest-first: {s}");
        }
    }

    #[test]
    fn prelude_defines_every_helper_the_detectors_call() {
        let p = prelude();
        for helper in [
            "fb_is_idle",
            "fb_bytes",
            "fb_in_use",
            "fb_stamp",
            "fb_scan_open_paths",
        ] {
            assert!(
                p.contains(&format!("{helper}()")),
                "missing helper {helper}"
            );
        }
    }
}
