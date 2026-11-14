#!/usr/bin/env bash
# Assemble per-PR fragments into the hot files (BUILD-FIRST R3).
#   changelog.d/<PMAT-id>[-slug].md            -> CHANGELOG.md, end of [Unreleased]
#   docs/roadmaps/fragments/<PMAT-id>.yaml     -> docs/roadmaps/roadmap.yaml rows
# A PR adds ONE fragment of each kind and never edits the hot files, so two PRs
# never touch the same line. Run at release time (make assemble-fragments).
#   --check     validate only, write nothing (exit 1 on a duplicate id or bad shape)
#   --consume   delete the fragments after assembling (release cut)
#   --root DIR  repo root (default: the script's parent)
# Idempotent: a changelog fragment whose text is already in [Unreleased] is not
# appended again, and a roadmap row with the fragment's id is replaced, so a run
# without --consume can be repeated. Ids compare case-insensitively (pmat-1 is
# PMAT-1). Rewritten files are mode 644, never mktemp's 600.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"; check=0; consume=0
while [ $# -gt 0 ]; do
  case "$1" in
    --check) check=1 ;; --consume) consume=1 ;;
    --root) root="$2"; shift ;;
    *) echo "usage: $0 [--check] [--consume] [--root DIR]" >&2; exit 2 ;;
  esac; shift
done
cd "$root"
fail=0; err() { echo "assemble-changelog: $*" >&2; fail=1; }
trim() { awk '{l[NR]=$0} END{n=NR; while(n>0&&l[n]=="")n--; for(i=1;i<=n;i++)print l[i]}' "$@"; }

# ---- changelog fragments ----------------------------------------------------
cl_files=(); declare -A seen_cl=()
shopt -s nullglob
for f in changelog.d/*.md; do
  b="$(basename "$f")"; [ "$b" = README.md ] && continue
  id="$(printf '%s' "$b" | sed -nE 's/^([A-Za-z]+-[0-9]+)(-.*)?\.md$/\1/p')"
  if [ -z "$id" ]; then err "$f: name must be <PMAT-id>[-slug].md"; continue; fi
  key="${id^^}"
  if [ -n "${seen_cl[$key]:-}" ]; then err "duplicate changelog id $key: ${seen_cl[$key]} and $f"; continue; fi
  seen_cl[$key]="$f"
  [ -s "$f" ] || { err "$f is empty"; continue; }
  cl_files+=("$f")
done

# ---- roadmap fragments ------------------------------------------------------
rm_files=(); declare -A seen_rm=()
for f in docs/roadmaps/fragments/*.yaml; do
  id="$(sed -nE '1s/^- id: *([A-Za-z]+-[0-9]+) *$/\1/p' "$f")"
  if [ -z "$id" ]; then err "$f: first line must be '- id: <PMAT-id>' at column 0"; continue; fi
  if [ "$(basename "$f" .yaml)" != "$id" ]; then err "$f: file name must be $id.yaml"; continue; fi
  key="${id^^}"
  if [ -n "${seen_rm[$key]:-}" ]; then err "duplicate roadmap id $key: ${seen_rm[$key]} and $f"; continue; fi
  if [ "$(grep -c '^- id:' "$f")" != 1 ]; then err "$f: exactly one row per fragment"; continue; fi
  seen_rm[$key]="$f"; rm_files+=("$f")
done
[ "$fail" = 0 ] || exit 1
if [ "$check" = 1 ]; then
  echo "assemble-changelog: ${#cl_files[@]} changelog + ${#rm_files[@]} roadmap fragments OK"; exit 0
fi

# ---- write CHANGELOG.md -----------------------------------------------------
if [ "${#cl_files[@]}" -gt 0 ]; then
  u="$(grep -n '^## \[Unreleased\]' CHANGELOG.md | head -1 | cut -d: -f1)"
  [ -n "$u" ] || { echo "assemble-changelog: no [Unreleased] heading" >&2; exit 1; }
  m="$(awk -v u="$u" 'NR>u && /^## \[/ {print NR; exit}' CHANGELOG.md)"
  # The [Unreleased] section as it stands; a fragment already in it is skipped.
  if [ -n "$m" ]; then sec="$(sed -n "${u},$((m-1))p" CHANGELOG.md)"; else sec="$(tail -n +"$u" CHANGELOG.md)"; fi
  new=()
  for f in "${cl_files[@]}"; do
    frag="$(trim "$f")"
    [[ "$sec" == *"$frag"* ]] || new+=("$f")
  done
  if [ "${#new[@]}" -gt 0 ]; then
    tmp="$(mktemp)"
    {
      if [ -n "$m" ]; then head -n $((m-1)) CHANGELOG.md; else cat CHANGELOG.md; fi | trim
      for f in "${new[@]}"; do echo; trim "$f"; done
      if [ -n "$m" ]; then echo; tail -n +"$m" CHANGELOG.md; fi
    } > "$tmp"
    chmod 644 "$tmp"; mv "$tmp" CHANGELOG.md
  fi
fi

# ---- write roadmap.yaml: replace a row with the same id, else append --------
for f in "${rm_files[@]}"; do
  id="$(sed -nE '1s/^- id: *(.*[^ ]) *$/\1/p' "$f")"
  tmp="$(mktemp)"
  awk -v id="$id" '
    /^- id:/ { skip = (tolower($0) ~ "^- id: *" tolower(id) " *$") }
    !skip { print }' docs/roadmaps/roadmap.yaml > "$tmp"
  cat "$f" >> "$tmp"; chmod 644 "$tmp"; mv "$tmp" docs/roadmaps/roadmap.yaml
done

if [ "$consume" = 1 ]; then
  for f in "${cl_files[@]}" "${rm_files[@]}"; do rm -f "${f:?}"; done
fi
echo "assemble-changelog: assembled ${#cl_files[@]} changelog + ${#rm_files[@]} roadmap fragments"
