#!/usr/bin/env bash
# Spot (burst) labels: a job that can resolve to one must (a) do so only on a pull_request event,
# (b) hold exactly `permissions: contents: read`, (c) name no secrets and no id-token. The
# reusable-workflow `runs_on` input (its provenance job signs) must stay clean-room, and
# pool-canary.yml must be one job, workflow_dispatch only, no secrets or id-token.
# Usage: pr-spot-labels-guard.sh [workflows-dir]   A missing or empty dir is a failure.
set -euo pipefail
dir="${1:-$(dirname "$0")/../.github/workflows}"
n=0; bad=0
for f in "$dir"/*.yml; do
  [ -f "$f" ] || continue
  n=$((n+1))
  out=$(awk '
    function flush() {
      if (job == "") return
      if (spot) {
        if (sign) { print FILENAME ": job " job " has id-token and can resolve to a spot label"; bad=1 }
        if (secr) { print FILENAME ": job " job " names secrets and can resolve to a spot label"; bad=1 }
        if (!pronly) { print FILENAME ": job " job " can resolve to a spot label outside a pull_request event"; bad=1 }
        if (!cread || other || wall) { print FILENAME ": job " job " can resolve to a spot label without exactly permissions: contents: read"; bad=1 }
      }
    }
    function reset() { sign=0; spot=0; secr=0; pronly=0; cread=0; other=0; wall=0; inperm=0 }
    /^[[:space:]]*#/ { next }
    /^jobs:/ { injobs=1; next }
    injobs && /^  [A-Za-z0-9_-]+:[[:space:]]*$/ { flush(); job=$1; reset(); next }
    !injobs && /id-token:[[:space:]]*write/ { topsign=1 }
    injobs && job != "" {
      if ($0 ~ /id-token/ || topsign) sign=1
      if ($0 ~ /secrets[.:]/) secr=1
      if ($0 ~ /^    permissions:/) {
        inperm=1
        if ($0 ~ /write-all|write/) wall=1
        if ($0 ~ /read-all/) other=1
        next
      }
      if (inperm) {
        if ($0 ~ /^      [A-Za-z-]+:/) {
          if ($0 ~ /^      contents:[[:space:]]*read[[:space:]]*$/) cread=1; else other=1
          next
        }
        inperm=0
      }
      if ($0 ~ /^    runs-on:/ && $0 ~ /burst/) {
        spot=1
        if ($0 ~ /github\.event_name == .pull_request. &&/) pronly=1
      }
    }
    /burst/ && /^[[:space:]]*runs_on:/ { print FILENAME ": reusable runs_on input names a spot label"; bad=1 }
    END { flush(); exit bad }
  ' "$f") || { echo "FAIL $out"; bad=1; continue; }
done
# The pool canary workflow: one job, no secrets or id-token, only workflow_dispatch.
c="$dir/pool-canary.yml"
if [ -f "$c" ]; then
  body=$(grep -v -E '^[[:space:]]*#' "$c")
  if printf '%s\n' "$body" | grep -q -E 'secrets|id-token'; then echo "FAIL pool-canary.yml names secrets or id-token"; bad=1; fi
  trig=$(printf '%s\n' "$body" | awk '/^on:/{o=1;next} o&&/^[^[:space:]]/{o=0} o&&/^  [A-Za-z_]+:/{sub(/:.*/,"");gsub(/ /,"");print}')
  [ "$trig" = workflow_dispatch ] || { echo "FAIL pool-canary.yml triggers are '$(echo $trig)', not only workflow_dispatch"; bad=1; }
  printf '%s\n' "$body" | grep -q 'runs-on: \[self-hosted, pool-canary\]' || { echo "FAIL pool-canary.yml runs-on is not [self-hosted, pool-canary]"; bad=1; }
  jobs=$(printf '%s\n' "$body" | awk '/^jobs:/{j=1;next} j&&/^[^[:space:]]/{j=0} j&&/^  [A-Za-z0-9_-]+:[[:space:]]*$/{c++} END{print c+0}')
  [ "$jobs" -eq 1 ] || { echo "FAIL pool-canary.yml has $jobs jobs, not exactly 1"; bad=1; }
fi
if [ "$n" -eq 0 ]; then echo "FAIL no workflow files in $dir, nothing was measured"; exit 1; fi
[ "$bad" -eq 0 ] && echo "ok $n workflow file(s): spot-eligible jobs are PR-only, contents: read, no secrets or id-token"
exit "$bad"
