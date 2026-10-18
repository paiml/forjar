#!/usr/bin/env bash
# forjar#722: lint.yml and msrv.yml run on arch-neutral clean-room runners, so every
# actions/cache key must carry runner.arch or x86 and aarch64 share one target cache.
# Every `key:` and every single-line `restore-keys:` must carry it; a block-scalar
# restore-keys is refused because this guard cannot read its lines.
set -euo pipefail
cd "$(dirname "$0")/.."
bad=0
for f in .github/workflows/lint.yml .github/workflows/msrv.yml; do
  n=$(grep -Ec '^ *(key|restore-keys): ' "$f" || true)
  a=$(grep -E '^ *(key|restore-keys): ' "$f" | grep -c 'runner\.arch' || true)
  blk=$(grep -Ec '^ *restore-keys: *[|>]' "$f" || true)
  if [ "$n" -eq 0 ] || [ "$n" -ne "$a" ] || [ "$blk" -ne 0 ]; then
    echo "FAIL $f: $n cache key line(s), $a with runner.arch, $blk block-scalar restore-keys"; bad=1
  else
    echo "ok $f: $n cache key line(s) all carry runner.arch"
  fi
done
exit "$bad"
