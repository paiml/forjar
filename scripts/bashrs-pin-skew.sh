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
#
# Usage: bashrs-pin-skew.sh --latest X.Y.Z --machines DIR [--cargo Cargo.toml]
# Output: one TSV row per pin: source  form  version  ok|SKEW, then a summary.
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

# Prints "form<TAB>version" for every bashrs pin in one manifest.
extract_pins() {
  awk '
    function ver(s) { if (match(s, /[0-9]+\.[0-9]+\.[0-9]+/)) return substr(s, RSTART, RLENGTH); return "" }
    /^[[:space:]]*#/ { next }
    incontent {
      if ($0 ~ /^[[:space:]]*$/) next
      v = ver($0); if (v != "") print "pin-file\t" v
      incontent = 0
    }
    /^[^[:space:]]/ { key = "" }
    /^[[:space:]]*bashrs_version:[[:space:]]*"?[0-9]/ { print "param\t" ver($0); next }
    /^  [A-Za-z0-9_.-]+:[[:space:]]*(#.*)?$/ { key = $1; sub(/:$/, "", key); next }
    key == "stack-tool-bashrs" && /^    version:/ { v = ver($0); if (v != "") print "stack-tool\t" v; next }
    key == "fleet-bashrs-pin" && /^    content:[[:space:]]*[|>]/ { incontent = 1; next }
    /bashrs[[:space:]]+--version[[:space:]]+[0-9]|bashrs@[0-9]/ {
      s = $0; sub(/.*bashrs([[:space:]]+--version[[:space:]]+|@)/, "", s)
      v = ver(s); if (v != "") print "install\t" v
    }
  ' "$1"
}

total=0 skew=0
row() { # source form version
  total=$((total + 1))
  if [ "$3" = "$latest" ]; then
    printf '%s\t%s\t%s\tok\n' "$1" "$2" "$3"
  else
    skew=$((skew + 1)); printf '%s\t%s\t%s\tSKEW\n' "$1" "$2" "$3"
  fi
}

printf 'source\tform\tversion\tstatus\n'
creq=$(grep -E '^bashrs[[:space:]]*=' "$cargo" | head -n 1 | grep -Eo '[0-9]+\.[0-9]+\.[0-9]+' | head -n 1 || true)
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
echo "SUMMARY latest=$latest pins=$total skewed=$skew" >&2
[ "$skew" -eq 0 ] || exit 1
exit 0
