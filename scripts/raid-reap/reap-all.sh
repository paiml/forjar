#!/usr/bin/env bash
# reap-all.sh -- one reaper pass: tmp, worktrees, alarm. Anti-inertness: used >= 70% and reclaimed == 0 => rc 1.
# mutation: change `-ge "$thr"` to `-gt 100` and the idle-at-75% fixture stops failing.
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
. "$here/lib.sh"
thr=${REAP_ALARM_PCT:-70}
rc=0 total=0
for s in reap-tmp reap-worktrees alarm; do
  r=0; line=$("$here/$s.sh") || r=$?
  echo "$line"
  [ "$r" -eq 0 ] || rc=1
  g=$(printf '%s' "$line" | tail -n 1 | sed -n 's/.*reclaimed=\([0-9.]*\).*/\1/p')
  total=$(awk -v a="$total" -v b="${g:-0}" 'BEGIN{print a+b}')
done
read -r pct _ < <(disk_stat)
if [ "$pct" -ge "$thr" ] && [ "$(awk -v t="$total" 'BEGIN{print (t>0)}')" = 0 ]; then
  echo "REAP all FAIL used=${pct}% reclaimed=0 (anti-inertness)"; exit 1
fi
exit "$rc"
