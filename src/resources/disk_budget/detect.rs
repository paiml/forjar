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
  # `/proc/*` not `/proc/[0-9]*`: bashrs misparses the bracket glob as a test
  # expression (SC1020/SC1140) and the whole script fails I8 purification.
  # Non-pid entries are filtered by requiring a readable cmdline.
  for p in /proc/*; do
    [ -e "$p/cmdline" ] || continue
    for l in "$p/cwd" "$p/fd"/*; do
      t=$(readlink "$l" 2>/dev/null) || continue
      case "$t" in /*) printf '%s\n' "$t" ;; esac
    done
  done 2> /dev/null | sort -u > "$FB_OPEN"
}

# True when some live process sits inside $1.
fb_in_use() {
  [ -s "$FB_OPEN" ] || return 1
  grep -qF -e "$1" "$FB_OPEN"
}

# Emit "<mtime_epoch>\t<path>" so the caller can sort oldest-first.
fb_stamp() { printf '%s\t%s\n' "$(stat -c %Y "$1" 2>/dev/null || echo 0)" "$1"; }

# SEC011 guard: refuse anything that is not a plausible reclaim target.
# Deletion candidates all come from globs and finds; a glob that matches one
# level too high, or a variable that came back empty, must abort rather than
# delete. Depth >= 3 keeps the reaper away from `/`, `/home`, `/home/noah`,
# `/tmp` and every other top-level directory even if a root is misdeclared.
fb_sweepable() {
  fb_p="$1"
  if [ -z "$fb_p" ]; then fb_log "  REFUSE empty path"; return 1; fi
  case "$fb_p" in
    /*) ;;
    *) fb_log "  REFUSE non-absolute: $fb_p"; return 1 ;;
  esac
  case "$fb_p" in
    *..*) fb_log "  REFUSE traversal: $fb_p"; return 1 ;;
    */) fb_log "  REFUSE trailing slash: $fb_p"; return 1 ;;
  esac
  fb_depth=$(printf '%s' "$fb_p" | tr -cd '/' | wc -c)
  if [ "$fb_depth" -lt 3 ]; then
    fb_log "  REFUSE too shallow (depth $fb_depth): $fb_p"
    return 1
  fi
  [ -e "$fb_p" ] || return 1
  return 0
}
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
/// A directory is a cargo target dir when EITHER:
///   * it holds `.rustc_info.json` — definitive, cargo writes it at the target
///     root; or
///   * it holds `CACHEDIR.TAG` **and** a `debug/` or `release/` subdirectory.
///
/// `CACHEDIR.TAG` alone is not sufficient: `~/.cargo/registry` carries one, and
/// sweeping the registry corrupts it in a way cargo does not notice until a
/// much later build fails on `could not compile cc`. The build-output subdir is
/// what separates the two — the registry's children are `src/`, `cache/`,
/// `index/`.
///
/// Requiring BOTH markers (the original rule) looked safer and was catastrophic:
/// measured on lambda-labs 2026-08-16, **zero** of the 16 marker-bearing
/// directories under a 4.6 TB `targets/` tree carried the pair. Repo roots have
/// `.rustc_info.json` without the tag; per-arch subdirectories have the tag
/// without the info file. The reaper matched nothing at all and reported
/// `health=inert` while the array sat at 94%.
fn cargo_target() -> String {
    r#"
fb_find_cargo_target() {
  for root in "$@"; do
    [ -d "$root" ] || continue
    find "$root" -mindepth 1 -maxdepth 7 -type f \
      \( -name .rustc_info.json -o -name CACHEDIR.TAG \) 2> /dev/null |
    while IFS= read -r fb_marker; do
      dirname "$fb_marker"
    done | sort -u |
    while IFS= read -r d; do
      if [ -f "$d/.rustc_info.json" ]; then
        fb_stamp "$d"
      elif [ -f "$d/CACHEDIR.TAG" ] && { [ -d "$d/debug" ] || [ -d "$d/release" ]; }; then
        # CACHEDIR.TAG alone also matches ~/.cargo/registry, whose children are
        # src/ cache/ index/ — a build-output subdir is what tells them apart.
        fb_stamp "$d"
      fi
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
    for p in /proc/*; do
      [ -e "$p/cmdline" ] || continue
      c=$(tr '\0' '\n' <"$p/cmdline" 2>/dev/null | head -1) || continue
      case "${c##*/}" in claude) ;; *) continue ;; esac
      w=$(readlink "$p/cwd" 2>/dev/null) || continue
      fb_live_slugs="$fb_live_slugs $(printf '%s' "$w" | tr '/' '-')"
    done 2> /dev/null
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
      # Dirty tree (tracked or untracked) => keep. Hoisted out of the `[ -n ... ]`
      # test: bashrs SEC002 cannot see the quoting on `$d` through the nested
      # quotes of a command substitution inside a test.
      dirty=$(git -C "$d" status --porcelain 2>/dev/null)
      [ -n "$dirty" ] && continue
      # No upstream => unpushed work => keep. Fails closed on git error.
      up=$(git -C "$d" rev-parse --abbrev-ref '@{upstream}' 2>/dev/null) || continue
      [ -n "$up" ] || continue
      range="$up..HEAD"
      ahead=$(git -C "$d" rev-list --count "$range" 2>/dev/null) || continue
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

/// Pre-delete hook: capture state that only exists while the candidate does.
///
/// A linked worktree knows where its parent repository is; the directory it
/// sits in does not. `dirname "$cand"` is a worktree POOL (`.claude/worktrees`,
/// `aprender-worktrees/`), which is not a repo — for an in-repo pool git would
/// silently walk up and find the right repo anyway, but for a sibling pool
/// outside any repository the prune simply fails and git's registry keeps
/// growing stale entries (aprender had 223 registered worktrees on 08-15).
/// So resolve the repo from the worktree itself, before it is removed.
pub(super) const fn pre_delete(kind: ReclaimKind) -> &'static str {
    match kind {
        ReclaimKind::AbandonedWorktree => {
            "    FB_REPO_DIR=$(git -C \"$cand\" rev-parse --git-common-dir 2>/dev/null || echo '')\n"
        }
        _ => "",
    }
}

/// Post-delete hook for a kind (e.g. pruning git's worktree registry).
pub(super) const fn post_delete(kind: ReclaimKind) -> &'static str {
    match kind {
        ReclaimKind::AbandonedWorktree => {
            "      [ -n \"$FB_REPO_DIR\" ] && git --git-dir \"$FB_REPO_DIR\" worktree prune 2>/dev/null\n"
        }
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_detector_accepts_either_real_layout() {
        // Measured on lambda-labs 2026-08-16 across a 4.6 TB targets/ tree:
        //   targets/<repo>                  .rustc_info.json, NO CACHEDIR.TAG
        //   targets/<repo>/<arch-triple>    CACHEDIR.TAG, NO .rustc_info.json
        //   BOTH markers: 0 of 16
        // Requiring the conjunction matched nothing and the reaper reported
        // health=inert while the array sat at 94%.
        let s = cargo_target();
        assert!(
            s.contains(r#"if [ -f "$d/.rustc_info.json" ]; then"#),
            ".rustc_info.json alone must be sufficient"
        );
        assert!(
            s.contains(
                r#"[ -f "$d/CACHEDIR.TAG" ] && { [ -d "$d/debug" ] || [ -d "$d/release" ]; }"#
            ),
            "CACHEDIR.TAG must be accepted when a build-output subdir is present"
        );
    }

    #[test]
    fn cargo_detector_still_excludes_the_registry() {
        // ~/.cargo/registry carries CACHEDIR.TAG and no .rustc_info.json, and
        // its children are src/ cache/ index/ — no debug/ or release/. The
        // build-output requirement is the only thing keeping the reaper out of
        // it now that CACHEDIR.TAG alone can qualify a directory.
        let s = cargo_target();
        let tag_branch = s
            .find(r#"[ -f "$d/CACHEDIR.TAG" ]"#)
            .expect("tag branch present");
        let rest = &s[tag_branch..];
        assert!(
            rest.starts_with(r#"[ -f "$d/CACHEDIR.TAG" ] && {"#),
            "CACHEDIR.TAG must never qualify a directory on its own"
        );
        assert!(rest.contains(r#"-d "$d/debug""#) && rest.contains(r#"-d "$d/release""#));
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
    fn prune_targets_the_repo_not_the_pool_directory() {
        // `dirname "$cand"` is the worktree POOL, which is not a repository.
        // For an in-repo pool git walks up and it accidentally works; for a
        // sibling pool (~/src/aprender-worktrees) it fails and git's registry
        // accumulates stale entries forever.
        let post = post_delete(ReclaimKind::AbandonedWorktree);
        assert!(
            !post.contains("dirname"),
            "prune must not be run from the pool directory: {post}"
        );
        assert!(post.contains("--git-dir \"$FB_REPO_DIR\""));
    }

    #[test]
    fn repo_dir_is_captured_before_the_worktree_is_deleted() {
        // The path can only be resolved while the worktree still exists.
        let pre = pre_delete(ReclaimKind::AbandonedWorktree);
        assert!(pre.contains("--git-common-dir"));
        assert!(pre.contains("FB_REPO_DIR="));
        // Other kinds need no capture.
        assert_eq!(pre_delete(ReclaimKind::CargoTarget), "");
        assert_eq!(pre_delete(ReclaimKind::Glob), "");
        assert_eq!(pre_delete(ReclaimKind::ClaudeScratchpad), "");
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
