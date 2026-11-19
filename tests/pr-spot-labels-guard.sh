#!/usr/bin/env bash
# Spot (burst) labels: an ALLOWLIST, not a blocklist. Cannot judge = red.
#  (a) Only the jobs named below, in ci.yml, may carry an opt-in runs-on, and each has exactly ONE
#      allowed runs-on line (byte-compared after trimming; 4-space indent).
#  (b) In EVERY file under the workflows dir (.yml and .yaml), any non-comment line mentioning
#      'burst', PR_SPOT_OPTIN or BURST_OPTIN, other than those exact lines in those exact jobs, fails.
#      A runs-on / runs_on line that is not a plain label list, a matrix reference or the canonical
#      line (vars., inputs., needs., env., secrets, fromJSON, an empty value) fails.
#  (c) Each allowlisted job has exactly 'permissions:' with only 'contents: read', no id-token and no
#      secrets; a file with an allowlisted job has no workflow-level secrets, id-token or write scope.
#  (d) pool-canary.yml must be byte-identical to tests/pool-canary.expected.yml.
# Usage: pr-spot-labels-guard.sh [workflows-dir]   A missing or empty dir is a failure.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
dir="${1:-$here/../.github/workflows}"
IFS= read -r PS <<'EOF'
runs-on: ${{ (github.event_name == 'pull_request' && vars.PR_SPOT_OPTIN == '1') && fromJSON('["self-hosted","burst-x64","burst-public"]') || fromJSON('["self-hosted","clean-room","X64"]') }}
EOF
IFS= read -r BU <<'EOF'
runs-on: ${{ (github.event_name == 'pull_request' && vars.BURST_OPTIN == '1') && fromJSON('["self-hosted","burst-x64"]') || fromJSON('["self-hosted","clean-room","X64"]') }}
EOF
n=0; bad=0
for f in "$dir"/*.yml "$dir"/*.yaml; do
  [ -f "$f" ] || continue
  n=$((n+1))
  base="$(basename "$f")"
  if [ "$base" = pool-canary.yml ]; then
    if [ ! -f "$here/pool-canary.expected.yml" ]; then echo "FAIL $base: no committed expected copy, cannot judge"; bad=1
    elif ! cmp -s "$f" "$here/pool-canary.expected.yml"; then echo "FAIL $base: not byte-identical to tests/pool-canary.expected.yml"; bad=1; fi
    continue
  fi
  out=$(awk -v base="$base" -v ps="$PS" -v bu="$BU" '
    function trim(s) { gsub(/^[ \t\r]+|[ \t\r]+$/, "", s); return s }
    function fail(m) { print base ": " m; bad=1 }
    function roval_ok(v,   w) {
      w = v; sub(/[ \t]+#.*$/, "", w); w = trim(w)
      if (w ~ /^\[[A-Za-z0-9_, -]*\]$/) return 1
      if (w ~ /^\047\[[A-Za-z0-9_", -]*\]\047$/) return 1
      if (w ~ /^\$\{\{ matrix\.[A-Za-z_]+ \}\}$/) return 1
      if (w ~ /^[A-Za-z0-9_.-]+$/) return 1
      return 0
    }
    BEGIN {
      if (base == "ci.yml") {
        allow["lockfile"] = ps; allow["examples-validate"] = ps; allow["dogfood-surface"] = ps
        allow["no-default-features"] = bu; allow["doctests"] = bu; allow["dogfood-guards"] = bu
      }
    }
    { sub(/\r$/, "") }
    /^[ \t]*#/ || /^[ \t]*$/ { next }
    {
      raw = $0; t = trim(raw); low = tolower(raw); canon_line = 0
      if (raw ~ /^jobs:[ \t]*$/) { injobs = 1; job = ""; inperm = 0; next }
      if (raw ~ /^jobs:/) noncanon = "jobs section is not in block form"
      if (raw ~ /^[^ \t]/) { injobs = 0; job = ""; inperm = 0 }
      if (injobs && raw ~ /^  [A-Za-z0-9_-]+:[ \t]*$/) {
        job = t; sub(/:$/, "", job); inperm = 0
        if (seen[job]++) noncanon = "job " job " declared twice"
        next
      }
      if (injobs && raw ~ /^  [^ ]/) { noncanon = "unparseable job header: " t; next }
      if (low ~ /burst|pr_spot_optin|burst_optin/) {
        mentions++
        if (job != "" && (job in allow) && t == allow[job] && raw ~ /^    [^ ]/) { canon[job]++; canon_line = 1 }
        else fail("line mentioning burst/opt-in outside the allowlist" (job != "" ? " in job " job : " at workflow level") ": " t)
      }
      if (!canon_line && low ~ /(runs-on|runs_on)[ \t]*:/) {
        v = t; sub(/^-[ \t]+/, "", v); sub(/^(runs-on|runs_on)[ \t]*:[ \t]*/, "", v)
        if (!roval_ok(v)) fail("runs-on/runs_on not a plain label list or matrix reference: " t)
      }
      if (canon_line) { prevcanon = 1; ro[job]++; next }
      if (prevcanon && raw ~ /^     /) fail("continuation line after canonical runs-on: " t)
      prevcanon = 0
      if (job == "") {
        if (raw ~ /^permissions:/) { tperm = 1; if (t != "permissions:") topbad = topbad "|" t; next }
        if (tperm && raw ~ /^  [^ ]/) { if (t != "contents: read") topbad = topbad "|" t; next }
        tperm = 0
        if (low ~ /secrets|id-token|write/) topbad = topbad "|" t
        next
      }
      if (low ~ /secrets/) secr[job] = 1
      if (low ~ /id-token/) sign[job] = 1
      if (raw ~ /^    runs-on[ \t]*:/) ro[job]++
      if (raw ~ /^    permissions:/) { pc[job]++; inperm = 1; if (t != "permissions:") pother[job] = 1; next }
      if (inperm) {
        if (raw ~ /^      [^ ]/) { if (t == "contents: read") cr[job]++; else pother[job] = 1; next }
        inperm = 0
      }
    }
    END {
      nspot = 0
      for (j in canon) {
        nspot++
        if (canon[j] != 1) fail("job " j " has " canon[j] " opt-in runs-on lines, not exactly 1")
        if (ro[j] != 1) fail("job " j " has " ro[j] " runs-on lines, not exactly 1")
        if (pc[j] != 1 || pother[j] || cr[j] != 1) fail("job " j " can resolve to a spot label without exactly permissions: contents: read")
        if (sign[j]) fail("job " j " names id-token and can resolve to a spot label")
        if (secr[j]) fail("job " j " names secrets and can resolve to a spot label")
      }
      if (nspot > 0 && topbad != "") fail("workflow level names secrets, id-token or write scope in a file with a spot job: " topbad)
      if ((nspot > 0 || mentions > 0) && noncanon != "") fail("cannot judge, not canonical: " noncanon)
      exit bad
    }
  ' "$f") || { echo "FAIL $out"; bad=1; continue; }
done
if [ "$n" -eq 0 ]; then echo "FAIL no workflow files in $dir, nothing was measured"; exit 1; fi
[ "$bad" -eq 0 ] && echo "ok $n workflow file(s): opt-in runs-on only on the allowlisted jobs with their exact line, contents: read, no secrets or id-token"
exit "$bad"
