#!/usr/bin/env bash
# forjar#492: shared self-hosted runners contend on the apt lock. Every
# `apt-get update|install` in a workflow must carry DPkg::Lock::Timeout so apt
# waits instead of failing the leg. Exit 0 ok, 1 violation, 2 not measured.
set -u
dir="${1:-.github/workflows}"
[ -d "$dir" ] || { echo "apt-lock-timeout-guard: $dir missing (not measured)"; exit 2; }
files=$(ls "$dir"/*.yml "$dir"/*.yaml 2>/dev/null)
[ -n "$files" ] || { echo "apt-lock-timeout-guard: 0 workflow files (not measured)"; exit 2; }
# shellcheck disable=SC2086
calls=$(grep -nE 'apt-get[[:space:]].*(update|install)' $files | grep -vE '^[^:]+:[0-9]+:[[:space:]]*#' || true)
bad=$(printf '%s\n' "$calls" | grep -v 'DPkg::Lock::Timeout' | grep -v '^$' || true)
n=$(printf '%s\n' "$calls" | grep -c . || true)
echo "apt-lock-timeout-guard: $n apt-get call(s) over $(echo "$files" | wc -l) workflow file(s)"
[ "$n" -gt 0 ] || { echo "0 calls scanned (not measured)"; exit 2; }
if [ -n "$bad" ]; then echo "VIOLATION (no DPkg::Lock::Timeout):"; echo "$bad"; exit 1; fi
exit 0
