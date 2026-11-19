#!/usr/bin/env bash
# A job that signs (id-token: write) must never resolve to a spot (burst) label, and the
# reusable-workflow `runs_on` input (which its provenance job runs on) must stay clean-room.
# Usage: pr-spot-labels-guard.sh [workflows-dir]   A missing or empty dir is a failure.
set -euo pipefail
dir="${1:-$(dirname "$0")/../.github/workflows}"
n=0; bad=0
for f in "$dir"/*.yml; do
  [ -f "$f" ] || continue
  n=$((n+1))
  out=$(awk '
    function flush() { if (job != "" && sign && spot) { print FILENAME ": job " job " has id-token: write and a spot label"; bad=1 } }
    /^[[:space:]]*#/ { next }
    /^permissions:/ { top=1 }
    /^jobs:/ { injobs=1; top=0; next }
    injobs && /^  [A-Za-z0-9_-]+:[[:space:]]*$/ { flush(); job=$1; sign=0; spot=0; if (topsign) sign=1; next }
    !injobs && /id-token:[[:space:]]*write/ { topsign=1 }
    /id-token:[[:space:]]*write/ { sign=1 }
    /burst/ { spot=1; if ($0 ~ /^[[:space:]]*runs_on:/) { print FILENAME ": reusable runs_on input names a spot label"; bad=1 } }
    END { flush(); exit bad }
  ' "$f") || { echo "FAIL $out"; bad=1; continue; }
done
if [ "$n" -eq 0 ]; then echo "FAIL no workflow files in $dir, nothing was measured"; exit 1; fi
[ "$bad" -eq 0 ] && echo "ok $n workflow file(s): no signing job or reusable runs_on resolves to a spot label"
exit "$bad"
