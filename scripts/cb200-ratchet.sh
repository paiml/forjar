#!/usr/bin/env bash
# CB-200 ratchet: the recorded ceiling may only shrink.
#
# UNMEASURED is a failure, not a pass. The dogfood protocol records a case where
# CB-200 reported Skip ("no .pmat/context.db") and that green was the index's
# absence rather than the tree's quality — so a missing measurement exits 1 here
# and says which of the two states it is in.
set -euo pipefail

# THE CEILING LIVES WHERE THE REPO OWNS IT, not under `.pmat/` (#401).
#
# It was `.pmat/cb200-baseline.json`: tracked, and matched by `**/.pmat/` in the
# root .gitignore — and, since pmat started writing its own `.pmat/.gitignore`
# containing `*`, unrescuable by any negation, because a deeper .gitignore
# outranks a shallower one. A shared ratchet floor cannot live in a directory
# another tool declares derived and ignores wholesale. Next to its only reader
# instead.
BASE="scripts/ratchets/cb200-baseline.json"
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

# THAT REBUILD IS NOT THE INDEX THIS GATE READS. `pmat query --rebuild-index`
# refreshes the project index; `pmat comply check` grades from its OWN cache
# under ~/.cache/paiml-mcp-agent-toolkit/comply/index/<tree>-<hash>/, and it
# says so in its output ("measured against <path>"). Measured 2026-09-08
# (PMAT-206): after refactoring `observe::classify` from a 34-arm match into a
# table, comply went on reporting `classify [F] (complexity: 34)` at its old
# line, and the count sat 3 above the ceiling through three real reductions.
# Removing that cache entry and re-running gave the true number immediately.
#
# A gate quoting a tree that no longer exists is worse than no gate: it reports
# a stale number with the authority of a fresh one. So the path is read from
# comply's own message and that directory — nothing else — is removed before
# the measuring run.
probe=$(pmat comply check --format json 2>/dev/null || true)
stale_index=$(printf '%s' "$probe" | grep -oE '/[^ "]*/comply/index/[^ "/]+' | head -1)
if [ -n "$stale_index" ] && [ -d "$stale_index" ]; then
  case "$stale_index" in
    */comply/index/*) rm -rf "$stale_index" ;;
    *) echo "✗ CB-200: refusing to remove an index path that is not under comply/index: $stale_index"; exit 1 ;;
  esac
fi

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
  # WHICH offenders. Without this the gate reports a number and a ceiling and
  # nothing an operator can act on: `pmat comply` prints the ten worst and
  # "... and N more", so a release blocked by a margin of one has no way to
  # find the one. Measured on 2026-09-08 (PMAT-206): three real reductions —
  # observe::classify from a 34-arm match to a table, two purifier functions
  # decomposed, an example's main split — moved the count 654 -> 652 while the
  # printed ten barely changed, and the remaining margin could not be
  # attributed at all. The list goes to a file so the next run can diff it.
  offenders="${CB200_OFFENDERS:-target/cb200-offenders.txt}"
  mkdir -p "$(dirname "$offenders")"
  printf '%s' "$raw" | python3 -c '
import json, sys
def walk(o, out):
    if isinstance(o, dict):
        m = str(o.get("message", ""))
        if "below minimum grade" in m:
            out.extend(l.strip() for l in m.splitlines()[1:] if l.strip())
        for v in o.values():
            walk(v, out)
    elif isinstance(o, list):
        for v in o:
            walk(v, out)
out = []
try:
    walk(json.load(sys.stdin), out)
except Exception as e:
    print(f"(the comply JSON could not be read: {e})")
print("\n".join(out))
' > "$offenders"
  echo "  The offenders pmat named are in ${offenders} ($(wc -l < "$offenders" | tr -d " ") line(s));"
  echo "  pmat prints only the worst ten, so a margin smaller than that is not attributable from here."
  exit 1
elif [ "$now" -lt "$ceiling" ]; then
  echo "✓ CB-200 improved: $now < $ceiling — lower the ceiling in $BASE to lock the gain in"
else
  echo "✓ CB-200 at the recorded ceiling ($now), pre-existing and not growing"
fi
