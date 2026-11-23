#!/usr/bin/env bash
# reap-worktrees.sh -- RAID-REAP rule R3: remove a clean, fully pushed worktree whose PR is merged/closed
# or that idled >= 7 d. Only `git worktree remove` + prune; dirty or unpushed worktrees are KEPT and listed.
# PR state failure is not_measured (no action). Dry-run unless FORJAR_BUDGET_EXECUTE=1.
# mutation: change REAP_WT_IDLE_DAYS default 7 to 70 and the idle-8d fixture stops being removed.
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
. "$here/lib.sh"
idle=${REAP_WT_IDLE_DAYS:-7}
repos=${REAP_REPOS?space-separated list of main checkouts (may be empty)}
roots=${REAP_WT_ROOTS:?colon-separated worktree roots}
prcmd=${REAP_PR_CMD:-$here/pr-state.sh}
reap_init worktrees

under_roots() { local r IFS=:; for r in $roots; do case $1 in "$r"/*) return 0 ;; esac; done; return 1; }

pushed() { # pushed <wt> <branch> : tip is on origin/<branch> or already inside origin/main
  local wt=$1 b=$2
  if [ "$b" = - ]; then git -C "$wt" merge-base --is-ancestor HEAD origin/main 2>/dev/null; return; fi
  if git -C "$wt" rev-parse --verify -q "origin/$b" >/dev/null; then
    [ -z "$(git -C "$wt" rev-list "origin/$b..$b")" ]
  else
    git -C "$wt" merge-base --is-ancestor "$b" origin/main 2>/dev/null
  fi
}

for repo in $repos; do
  while IFS=$'\t' read -r wt br; do
    under_roots "$wt" || continue
    over_cap && break 2
    [ -d "$wt" ] || continue
    if [ -n "$(git -C "$wt" status --porcelain)" ]; then note_keep "$wt" "dirty"; continue; fi
    if ! pushed "$wt" "$br"; then note_keep "$wt" "unpushed"; continue; fi
    has_open "$wt" && { note_keep "$wt" "open file"; continue; }
    why=""
    if older_than "$wt" "$idle" "$(newest_mtime "$wt")"; then why=idle
    elif [ "$br" = - ]; then note_keep "$wt" "detached, not idle"; continue
    else
      if ! st=$("$prcmd" "$repo" "$br" 2>/dev/null); then note_nm "$wt" "pr state unavailable"; continue; fi
      if [ "$st" = "done" ]; then why=pr-closed; else note_keep "$wt" "pr $st, not idle"; continue; fi
    fi
    b=$(bytes_of "$wt")
    if is_exec; then
      if git -C "$repo" worktree remove "$wt"; then
        git -C "$repo" worktree prune
        ledger R3-worktree "$wt" "$b" "removed:$why"
        RECLAIMED=$((RECLAIMED + b))
      else note_keep "$wt" "worktree remove refused"; fi
    else
      echo "WOULD-REMOVE R3-worktree $wt $b ($why)"
      RECLAIMED=$((RECLAIMED + b))
    fi
  done < <(git -C "$repo" worktree list --porcelain | awk -v RS= -F'\n' 'NR>1 {p=""; b="-"; for(i=1;i<=NF;i++){ if($i ~ /^worktree /) p=substr($i,10); if($i ~ /^branch refs\/heads\//) b=substr($i,19)} print p "\t" b}')
done
finish
