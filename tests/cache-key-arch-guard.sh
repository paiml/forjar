#!/usr/bin/env bash
# forjar#722: lint.yml and msrv.yml run on arch-neutral clean-room runners, so every
# actions/cache key must carry runner.arch or x86 and aarch64 share one target cache.
# Every `key:` and every single-line `restore-keys:` must carry it in its value (a
# trailing comment does not count). A restore-keys written as a block scalar or a
# sequence is refused, because this guard cannot read the lines under it. A missing
# workflow file is a failure, never a pass.
set -euo pipefail
cd "$(dirname "$0")/.." || exit 2
bad=0
for f in .github/workflows/lint.yml .github/workflows/msrv.yml; do
  if [ ! -f "$f" ]; then
    echo "FAIL $f: file is missing, nothing was measured"; bad=1; continue
  fi
  lines=$(grep -E '^[[:space:]]*(key|restore-keys):' "$f" | sed -E 's/[[:space:]]+#.*$//' || true)
  n=$(printf '%s\n' "$lines" | grep -Ec '^[[:space:]]*(key|restore-keys):' || true)
  a=$(printf '%s\n' "$lines" | grep -Ec 'runner\.arch' || true)
  blk=$(printf '%s\n' "$lines" | grep -Ec '(key|restore-keys):[[:space:]]*([|>][-+]?)?$' || true)
  if [ "$n" -eq 0 ] || [ "$n" -ne "$a" ] || [ "$blk" -ne 0 ]; then
    echo "FAIL $f: $n cache key line(s), $a with runner.arch, $blk empty/block/sequence value(s)"; bad=1
  else
    echo "ok $f: $n cache key line(s) all carry runner.arch"
  fi
done
exit "$bad"
