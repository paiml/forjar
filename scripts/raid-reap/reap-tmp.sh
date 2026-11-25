#!/usr/bin/env bash
# reap-tmp.sh -- RAID-REAP rules R1 tmp TTL (14 d), R2 build output (3 d), R4 burst/submit result trees.
# Dry-run unless FORJAR_BUDGET_EXECUTE=1. Prints one "REAP tmp PASS|FAIL ..." line; rc 1 if anything unmeasured.
# mutation: change REAP_TTL_DAYS default 14 to 140 and the 15-day fixture stops being reaped (test-reap.sh red).
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
. "$here/lib.sh"
root=${REAP_TMP_ROOT:-/mnt/nvme-raid0/tmp}
ttl=${REAP_TTL_DAYS:-14}
bttl=${REAP_BUILD_DAYS:-3}
reap_init tmp
case $root in /|"") reap_fail "refusing root '$root'" ;; /*) ;; *) reap_fail "relative root '$root'" ;; esac
[ -d "$root" ] || reap_fail "unreadable root $root"
rootdev=$(stat -c %d "$root")
export REAP_RESCUE_DIR=${REAP_RESCUE_DIR:-/mnt/nvme-raid0/rescue}

# Snapshot R1 ages first: removing a child bumps the parent's mtime and would hide a stale tree.
declare -A age
while IFS= read -r -d '' e; do age[$e]=$(newest_mtime "$e"); done < <(find "$root" -mindepth 1 -maxdepth 1 -print0)

has_record() { # a result record carrying a sha256 of the output
  local f
  for f in "$1"/result.json "$1"/receipt*; do
    [ -f "$f" ] && grep -Eq '[0-9a-f]{64}' "$f" && return 0
  done
  return 1
}
is_resulttree() { case $(basename "$1") in burst-*-out | submit-*) return 0 ;; esac; return 1; }
has_deliverable() {
  [ -n "$(find "$1" \( -iname '*.mp4' -o -iname '*.mov' -o -iname '*.wav' -o -iname '*.mp3' -o -iname '*.srt' \
    -o -iname '*.vtt' -o -iname '*.apr' -o -iname '*.gguf' -o -iname '*.safetensors' -o -iname '*.onnx' \
    -o -path '*/receipts/*' -o -path '*/evidence/*' \) -print -quit)" ]
}
unrecorded_inside() { # a nested result tree with no record blocks deleting the parent
  local d
  while IFS= read -r -d '' d; do has_record "$d" || return 0; done \
    < <(find "$1" -type d \( -name 'burst-*-out' -o -name 'submit-*' \) -print0)
  return 1
}

# R2 build output idle >= 3 d (result trees belong to R4)
while IFS= read -r -d '' d; do
  over_cap && break
  is_resulttree "$d" && continue
  [ "$(basename "$d")" = target ] && [ ! -e "$d/CACHEDIR.TAG" ] && [ ! -e "$d/.rustc_info.json" ] && continue
  older_than "$d" "$bttl" "$(newest_mtime "$d")" || continue
  do_remove "$d" R2-build
done < <(find "$root" -maxdepth 5 -type d \( -name target -o -name incremental -o -name .fingerprint \
  -o -name node_modules -o -name sccache \) -prune -print0)

# R4 burst/submit trees: delete the output only when the record exists, else KEEP + not_measured
while IFS= read -r -d '' d; do
  over_cap && break
  has_record "$d" || { note_nm "$d" "no result record"; continue; }
  has_open "$d" && { note_keep "$d" "open file"; continue; }
  while IFS= read -r -d '' f; do
    case $(basename "$f") in result.json | receipt*) continue ;; esac
    do_remove "$f" R4-burst
  done < <(find "$d" -mindepth 1 -maxdepth 1 -print0)
done < <(find "$root" -maxdepth 2 -type d \( -name 'burst-*-out' -o -name 'submit-*' \) -print0)

# R1 tmp TTL: top-level entries untouched >= ttl days; deliverables are MOVED, never deleted
for e in "${!age[@]}"; do
  over_cap && break
  [ -e "$e" ] || continue
  [ -L "$e" ] && { note_keep "$e" "symlink"; continue; }
  is_resulttree "$e" && continue
  older_than "$e" "$ttl" "${age[$e]}" || continue
  [ "$(stat -c %d "$e")" = "$rootdev" ] || { note_keep "$e" "other device"; continue; }
  if unrecorded_inside "$e"; then note_nm "$e" "nested result tree without record"; continue; fi
  if has_deliverable "$e"; then do_move "$e" R1-tmp; else do_remove "$e" R1-tmp; fi
done
finish
