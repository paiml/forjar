#!/usr/bin/env bash
# forjar#722: lint.yml and msrv.yml run on arch-neutral clean-room runners, so every
# actions/cache key must carry runner.arch or x86 and aarch64 share one target cache.
set -euo pipefail
cd "$(dirname "$0")/.."
bad=0
for f in .github/workflows/lint.yml .github/workflows/msrv.yml; do
  n=$(grep -c '^ *key: ' "$f" || true)
  a=$(grep '^ *\(key\|restore-keys\): ' "$f" | grep -c 'runner\.arch' || true)
  if [ "$n" -eq 0 ] || [ "$n" -ne "$a" ]; then
    echo "FAIL $f: $n cache key(s), $a with runner.arch"; bad=1
  else
    echo "ok $f: $n cache key(s) all carry runner.arch"
  fi
done
exit "$bad"
