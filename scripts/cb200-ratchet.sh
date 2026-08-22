#!/usr/bin/env bash
# CB-200 ratchet: the recorded ceiling may only shrink.
#
# UNMEASURED is a failure, not a pass. The dogfood protocol records a case where
# CB-200 reported Skip ("no .pmat/context.db") and that green was the index's
# absence rather than the tree's quality — so a missing measurement exits 1 here
# and says which of the two states it is in.
set -euo pipefail

BASE=".pmat/cb200-baseline.json"
[ -f "$BASE" ] || { echo "✗ no $BASE — the ceiling is undeclared"; exit 1; }

ceiling=$(python3 -c "import json;print(json.load(open('$BASE'))['ceiling'])")

# REBUILD THE INDEX FIRST. CB-200 grades from `.pmat/context.db`, and when that
# is older than the sources it reports PRE-CHANGE numbers while still printing a
# count as though it were authoritative. Its own message says to refresh with
# `pmat query "x"` — that does NOT rebuild; only `--rebuild-index` does. Watched
# live: after refactoring 17 files the gate still reported the old 61, and the
# ratchet then failed as a REGRESSION against a ceiling the tree had already
# met. A stale measurement is not a measurement.
pmat query "resource" --rebuild-index --limit 1 >/dev/null 2>&1 || true

# `|| true` on the producer would be the exact defect this protocol warns about,
# so the JSON is captured and its absence handled explicitly instead.
raw=$(pmat comply check --format json 2>/dev/null || true)
[ -n "$raw" ] || { echo "✗ CB-200 UNMEASURED: pmat comply produced no output"; exit 1; }

now=$(printf '%s' "$raw" | python3 -c '
import json, sys, re
out = []
def walk(o):
    if isinstance(o, dict):
        m = str(o.get("message", ""))
        if "below minimum grade" in m:
            n = re.search(r"(\d+) function", m)
            if n:
                out.append(int(n.group(1)))
        for v in o.values():
            walk(v)
    elif isinstance(o, list):
        for v in o:
            walk(v)
try:
    walk(json.load(sys.stdin))
except Exception:
    pass
print(out[0] if out else -1)
')

# A count carried on a stale index is worse than no count: it reads as authority.
if printf '%s' "$raw" | grep -q 'index is stale'; then
  echo "✗ CB-200 measured against a STALE index — refusing to report a number the tree has already changed."
  echo "  Run: pmat query \"x\" --rebuild-index"
  exit 1
fi

if [ "$now" -lt 0 ]; then
  echo "✗ CB-200 UNMEASURED — no grade line in pmat comply output. Unmeasured is not passing."
  exit 1
elif [ "$now" -gt "$ceiling" ]; then
  echo "✗ CB-200 REGRESSED: $now functions below grade A, recorded ceiling is $ceiling"
  echo "  The ratchet may only shrink. Fix the new offenders rather than raising it."
  exit 1
elif [ "$now" -lt "$ceiling" ]; then
  echo "✓ CB-200 improved: $now < $ceiling — lower the ceiling in $BASE to lock the gain in"
else
  echo "✓ CB-200 at the recorded ceiling ($now), pre-existing and not growing"
fi
