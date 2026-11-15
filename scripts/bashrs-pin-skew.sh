#!/usr/bin/env bash
# bashrs pin-skew check (PMAT-1015). CHECK CLASS: LAB.
#
# LAB means: this check can never block a merge or a release. It runs at night
# on an idle machine, never inside a PR, merge-queue or release job. A red opens
# or updates one ticket with one owner. tests/bashrs-pin-skew.sh fails if any
# workflow under .github/workflows names this script.
#
# What it measures: every bashrs version pinned in a tree of fleet manifests
# (<dir>/<machine>/forjar.yaml) and forjar's own Cargo.toml requirement, against
# the newest published bashrs tag. Four pin forms are read (comment lines are
# skipped):
#   param      `bashrs_version: "X"`
#   stack-tool `version: "X"` inside the `stack-tool-bashrs:` resource
#   pin-file   the first content line of the `fleet-bashrs-pin:` resource
#   install    `bashrs --version X` or `bashrs@X` in a command
# A pin may carry a comparator (= ^ ~ > <), a `v` prefix, single or double quotes,
# a pre-release or build suffix, and an inline comment; the version is read from the
# pin itself, never from a trailing comment. A pin that is not one X.Y.Z (a word, a
# $VAR, a range) is reported CANNOT-PARSE and counts as skewed. Nothing is skipped.
#
# Usage: bashrs-pin-skew.sh --latest X.Y.Z --machines DIR [--cargo Cargo.toml]
# Output: one TSV row per pin: source  form  version  ok|SKEW|CANNOT-PARSE, then a summary.
# Exit: 0 every pin equals --latest; 1 at least one pin is skewed;
#       2 not measured (bad arguments, missing input, or no pin found at all).
#       A not-measured run is never a pass.
set -euo pipefail

usage() { echo "usage: $0 --latest X.Y.Z --machines DIR [--cargo PATH]" >&2; exit 2; }

latest="" machines="" cargo="$(cd "$(dirname "$0")/.." && pwd)/Cargo.toml"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --latest) [ "$#" -ge 2 ] || usage; latest="${2#v}"; shift 2 ;;
    --machines) [ "$#" -ge 2 ] || usage; machines="$2"; shift 2 ;;
    --cargo) [ "$#" -ge 2 ] || usage; cargo="$2"; shift 2 ;;
    *) usage ;;
  esac
done
if [ -z "$latest" ] || [ -z "$machines" ]; then usage; fi
if ! printf '%s\n' "$latest" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "NOT MEASURED: --latest '$latest' is not X.Y.Z" >&2; exit 2
fi
if [ ! -d "$machines" ]; then
  echo "NOT MEASURED: machines dir '$machines' is missing" >&2; exit 2
fi
if [ ! -f "$cargo" ]; then
  echo "NOT MEASURED: Cargo.toml '$cargo' is missing" >&2; exit 2
fi

# Shared awk: parse() reduces one raw pin value to its exact version. It drops an
# inline `# comment`, surrounding quotes (single or double), a leading comparator
# (= ^ ~ > <) and a `v` prefix, then requires a whole X.Y.Z[-pre][+build]. Anything
# else (a word, a $VAR, a range) is "UNPARSEABLE": reported, never skipped.
AWK_PARSE='
function parse(s) {
  sub(/[[:space:]]+#.*$/, "", s)
  gsub(/^[[:space:]]+|[[:space:]]+$/, "", s)
  gsub("^[\"" q "]+|[\"" q "]+$", "", s)
  sub(/^[=^~<>]+[[:space:]]*/, "", s)
  sub(/^v/, "", s)
  if (s ~ /^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/) return s
  return "UNPARSEABLE"
}'

# Prints "form<TAB>version" for every bashrs pin in one manifest.
extract_pins() {
  awk -v q="'" "$AWK_PARSE"'
    /^[[:space:]]*#/ { next }
    incontent {
      if ($0 ~ /^[[:space:]]*$/) next
      print "pin-file\t" parse($0)
      incontent = 0
    }
    /^[^[:space:]]/ { key = "" }
    /^[[:space:]]*bashrs_version:/ { v = $0; sub(/^[[:space:]]*bashrs_version:/, "", v); print "param\t" parse(v); next }
    /^  [A-Za-z0-9_.-]+:[[:space:]]*(#.*)?$/ { key = $1; sub(/:$/, "", key); next }
    key == "stack-tool-bashrs" && /^    version:/ { v = $0; sub(/^    version:/, "", v); print "stack-tool\t" parse(v); next }
    key == "fleet-bashrs-pin" && /^    content:[[:space:]]*[|>]/ { incontent = 1; next }
    {
      s = $0; sub(/[[:space:]]+#.*$/, "", s)
      while (match(s, /bashrs[[:space:]]+--version(=|[[:space:]]+)[^[:space:]]+|bashrs@[^[:space:]]+/)) {
        m = substr(s, RSTART, RLENGTH); s = substr(s, RSTART + RLENGTH)
        sub(/^bashrs([[:space:]]+--version(=|[[:space:]]+)|@)/, "", m)
        sub(/[;&|)]+$/, "", m)
        if (m != "") print "install\t" parse(m)
      }
    }
  ' "$1"
}

total=0 skew=0 unparse=0
row() { # source form version
  total=$((total + 1))
  if [ "$3" = "UNPARSEABLE" ]; then
    skew=$((skew + 1)); unparse=$((unparse + 1)); printf '%s\t%s\t%s\tCANNOT-PARSE\n' "$1" "$2" "$3"
  elif [ "$3" = "$latest" ]; then
    printf '%s\t%s\t%s\tok\n' "$1" "$2" "$3"
  else
    skew=$((skew + 1)); printf '%s\t%s\t%s\tSKEW\n' "$1" "$2" "$3"
  fi
}

printf 'source\tform\tversion\tstatus\n'
# The requirement is the `version = "..."` of a table form, else the bare string.
creq=$(awk -v q="'" "$AWK_PARSE"'
  /^bashrs[[:space:]]*=/ {
    s = $0; sub(/^bashrs[[:space:]]*=[[:space:]]*/, "", s); sub(/[[:space:]]+#.*$/, "", s)
    if (s ~ /^\{/) {
      if (match(s, /version[[:space:]]*=[[:space:]]*("[^"]*"|\047[^\047]*\047)/)) {
        s = substr(s, RSTART, RLENGTH); sub(/^version[[:space:]]*=[[:space:]]*/, "", s)
      } else exit
    }
    print parse(s); exit
  }' "$cargo")
if [ -z "$creq" ]; then
  echo "NOT MEASURED: no bashrs requirement in $cargo" >&2; exit 2
fi
row "forjar:Cargo.toml" "cargo-req" "$creq"

for f in "$machines"/*/forjar.yaml; do
  [ -f "$f" ] || continue
  m=$(basename "$(dirname "$f")")
  while IFS=$'\t' read -r form v; do
    row "$m" "$form" "$v"
  done < <(extract_pins "$f")
done

if [ "$total" -le 1 ]; then
  echo "NOT MEASURED: no bashrs pin found under $machines" >&2; exit 2
fi
echo "SUMMARY latest=$latest pins=$total skewed=$skew unparseable=$unparse" >&2
[ "$skew" -eq 0 ] || exit 1
exit 0
