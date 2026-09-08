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
# under ~/.cache/paiml-mcp-agent-toolkit/comply/index/<tree>-<hash>/. Measured
# 2026-09-08 (PMAT-206): after `observe::classify` became a table, comply went
# on reporting `classify [F] (complexity: 34)` at its old line and the count sat
# three above the ceiling through three real reductions. Clearing that cache and
# re-running gave the true number, 651, immediately.
#
# A gate quoting a tree that no longer exists is worse than no gate. So the
# cache is judged by AGE, before comply runs: if the newest tracked source file
# is newer than the cache, the cache cannot describe this tree and is removed.
# The staleness is announced rather than swallowed — the point is a true
# measurement AND a visible tool defect, not a quiet one.
#
# The directory is found by NAME under the tool's own cache root, never by
# pattern-matching comply's JSON: that output carries source snippets and file
# paths from violations, so a regex over it can hand `rm -rf` a path out of the
# repository (found by the PMAT-206 quorum, four lanes).
cache_root="${PMAT_COMPLY_CACHE:-$HOME/.cache/paiml-mcp-agent-toolkit/comply/index}"
# `sed -n 1p`, not `head -1`: head closes the pipe after one line, `ls` takes
# SIGPIPE, xargs reports 125 for a signalled child, and under `pipefail` the
# whole script died here with no output at all (measured: exit 125, 0 lines).
newest_src=$(git ls-files -z -- '*.rs' '*.py' | xargs -0 -r stat -c '%Y %n' 2>/dev/null | sort -rn | sed -n '1p' | cut -d' ' -f2-)
if [ -n "$newest_src" ] && [ -d "$cache_root" ]; then
  for dir in "$cache_root/$(basename "$PWD")"-*; do
    [ -d "$dir" ] || continue
    case "$dir" in
      "$cache_root"/*) ;;
      *) continue ;;
    esac
    if [ "$newest_src" -nt "$dir" ]; then
      echo "  NOTE: ${dir} is older than ${newest_src} — pmat comply would grade a tree that no longer exists."
      echo "        Removing it so this run measures THIS tree. (pmat query --rebuild-index does not refresh it.)"
      # `:?` is not decoration: an unset or empty variable here would make
      # this `rm -rf` a command about /. The case above already pinned it
      # under the cache root; this pins it non-empty.
      [ -n "$dir" ] && [ "$dir" != "/" ] && rm -rf "${dir:?}"
    fi
  done
fi

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
