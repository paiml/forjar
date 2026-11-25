#!/usr/bin/env bash
# pr-state.sh <repo-dir> <branch> -- print done|open|none for the branch's PRs; nonzero = not_measured.
# One cached gh call per branch with If-None-Match (a 304 costs 0 under fleet rule GH-1); refuses below 1000 left.
# mutation: change the 1000 floor to 0 and the rate-limit refusal stops firing.
set -euo pipefail
repo=$1 br=$2
cache=${REAP_GH_CACHE:-/mnt/nvme-raid0/reaper-state/gh-cache}
mkdir -p "$cache"
left=$(gh api rate_limit --jq .resources.core.remaining) || exit 3
[ "$left" -ge 1000 ] || exit 3
slug=$(git -C "$repo" remote get-url origin | sed -E 's#(\.git)?$##; s#.*[:/]([^/]+/[^/]+)$#\1#')
key=$(printf '%s@%s' "$slug" "$br" | tr '/:' '__')
etag=""; [ -f "$cache/$key.etag" ] && etag=$(cat "$cache/$key.etag")
resp=$(gh api -i -H "If-None-Match: $etag" "repos/$slug/pulls?head=${slug%%/*}:$br&state=all" 2>&1) || {
  case $resp in HTTP*304*) ;; *) exit 3 ;; esac
}
if printf '%s' "$resp" | head -n 1 | grep -q ' 304'; then body=$(cat "$cache/$key.json")
else
  printf '%s' "$resp" | awk 'tolower($1)=="etag:"{print $2}' | tr -d '\r' >"$cache/$key.etag"
  body=$(printf '%s' "$resp" | sed '1,/^\r\?$/d'); printf '%s' "$body" >"$cache/$key.json"
fi
n=$(printf '%s' "$body" | jq 'length') || exit 3
[ "$n" -gt 0 ] || { echo none; exit 0; }
if [ "$(printf '%s' "$body" | jq '[.[] | select(.state=="open")] | length')" -gt 0 ]; then echo open; else echo "done"; fi
