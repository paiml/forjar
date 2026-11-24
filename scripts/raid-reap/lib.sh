#!/usr/bin/env bash
# lib.sh -- shared helpers for the RAID reaper (forjar RAID-REAP Phase C, paiml/infra#5264).
# Sourced, never run. Dry-run unless FORJAR_BUDGET_EXECUTE=1. Bash, find, awk, git, df, du only.
# mutation: delete the has_open check in do_remove and the open-file-hold fixture goes red.
set -euo pipefail

REAP_NOW=${REAP_NOW:-$(date +%s)}
REAP_LEDGER=${REAP_LEDGER:-/mnt/nvme-raid0/reaper-ledger.tsv}
REAP_STATE_DIR=${REAP_STATE_DIR:-/mnt/nvme-raid0/reaper-state}
REAP_MAX_BYTES=${REAP_MAX_BYTES:-0} # per-pass cap; 0 = none
RECLAIMED=0 KEPT=0 NM=0
REAP_RULE=${REAP_RULE:-reap}

is_exec() { [ "${FORJAR_BUDGET_EXECUTE:-0}" = 1 ]; }
reap_fail() { echo "REAP $REAP_RULE FAIL $*" >&2; exit 2; }

# reap_init <rule> : name the pass, snapshot every path a live process holds open.
reap_init() {
  REAP_RULE=$1
  mkdir -p "$REAP_STATE_DIR"
  : >"$REAP_STATE_DIR/nm-$REAP_RULE.tsv"
  : >"$REAP_STATE_DIR/keep-$REAP_RULE.tsv"
  REAP_OPEN=$(mktemp)
  trap 'rm -f "${REAP_OPEN:?}"' EXIT
  # find prints symlink targets without a fork per fd; a process vanishing mid-scan makes find exit 1,
  # which is not a measurement failure, but an empty snapshot is (any live system holds files open).
  find /proc/[0-9]*/fd /proc/[0-9]*/cwd -maxdepth 1 -type l -printf "%l\n" 2>/dev/null >"$REAP_OPEN" || [ -s "$REAP_OPEN" ]
  [ -s "$REAP_OPEN" ] || reap_fail "open-file snapshot empty"
}

# has_open <path> : true when any process holds the path (or something under it) open or as cwd.
has_open() {
  awk -v p="$1" 'index($0,p)==1 && (length($0)==length(p) || substr($0,length(p)+1,1)=="/") {f=1; exit} END{exit !f}' "$REAP_OPEN"
}

# newest_mtime <path> : epoch seconds of the newest file in the tree (unreadable tree = hard fail).
newest_mtime() {
  local out
  out=$(find "$1" -printf '%T@\n') || reap_fail "unreadable $1"
  printf '%s\n' "$out" | sort -n | tail -n 1 | cut -d. -f1
}

# older_than <path> <days> <mtime> : true when mtime is at least <days> days before now.
older_than() { [ $((REAP_NOW - $3)) -ge $(($2 * 86400)) ]; }

bytes_of() { du -sb -- "$1" | cut -f1; }
note_keep() { KEPT=$((KEPT + 1)); echo "KEEP $REAP_RULE $1 ($2)"; printf '%s\t%s\t%s\n' "$REAP_RULE" "$1" "$2" >>"$REAP_STATE_DIR/keep-$REAP_RULE.tsv"; }
note_nm() { NM=$((NM + 1)); echo "NOT_MEASURED $REAP_RULE $1 ($2)"; printf '%s\t%s\n' "$1" "$2" >>"$REAP_STATE_DIR/nm-$REAP_RULE.tsv"; }
over_cap() { [ "$REAP_MAX_BYTES" -gt 0 ] && [ "$RECLAIMED" -ge "$REAP_MAX_BYTES" ]; }

ledger() { # ledger <rule> <path> <bytes> <action>
  printf '%s\t%s\t%s\t%s\t%s\n' "$REAP_NOW" "$1" "$2" "$3" "$4" >>"$REAP_LEDGER"
}

# do_remove <path> <label> : remove a build/tmp tree (never a worktree) unless a process holds it.
do_remove() {
  local p=$1 b
  if has_open "$p"; then note_keep "$p" "open file"; return 0; fi
  b=$(bytes_of "$p")
  if is_exec; then ledger "$2" "$p" "$b" removed; rm -rf -- "${p:?}"; else echo "WOULD-REMOVE $2 $p $b"; fi
  RECLAIMED=$((RECLAIMED + b))
}

# do_move <path> <label> : move a tree holding deliverables to the rescue dir, never delete it.
do_move() {
  local p=$1 b dest
  if has_open "$p"; then note_keep "$p" "open file"; return 0; fi
  b=$(bytes_of "$p")
  dest="${REAP_RESCUE_DIR:?}/$(date -u -d "@$REAP_NOW" +%F)/$(basename "$p")"
  [ -e "$dest" ] && dest="$dest.$REAP_NOW"
  if is_exec; then
    mkdir -p "$(dirname "$dest")"
    ledger "$2" "$p" "$b" "moved:$dest"
    mv -- "$p" "$dest"
  else echo "WOULD-MOVE $2 $p -> $dest"; fi
  # moved bytes stay on the same disk: they are kept, not reclaimed
  KEPT=$((KEPT + 1))
}

# finish : print the single GATE-style line; rc 1 on any unmeasured item.
finish() {
  local gb verdict=PASS rc=0
  gb=$(awk -v b="$RECLAIMED" 'BEGIN{printf "%.3f", b/1e9}')
  if [ "$NM" -gt 0 ]; then verdict=FAIL; rc=1; fi
  echo "REAP $REAP_RULE $verdict reclaimed=$gb kept=$KEPT not_measured=$NM"
  exit "$rc"
}

# disk_stat : "<used-pct> <free-GB>" for the RAID root; REAP_DF_CMD / FORJAR_DF_CMD stubs it in tests.
disk_stat() {
  local cmd=${FORJAR_DF_CMD:-${REAP_DF_CMD:-}}
  if [ -n "$cmd" ]; then "$cmd"; return; fi
  df -BG --output=pcent,avail "${REAP_RAID_ROOT:-/mnt/nvme-raid0}" | tail -n 1 | tr -dc '0-9 \n'
}
