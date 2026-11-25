#!/usr/bin/env bash
# report.sh -- RAID-REAP rule R6: weekly RAID report (Mon 06:00Z) to handoff/raid-weekly-<date>.md.
# mutation: change the 7-day window (604800) to 0 and the golden report's reclaimed row goes red.
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
. "$here/lib.sh"
raid=${REAP_RAID_ROOT:-/mnt/nvme-raid0}
out=${REAP_REPORT_DIR:-$raid/cop-inbox/handoff}
reap_init report
date_s=$(date -u -d "@$REAP_NOW" +%F)
read -r pct free < <(disk_stat) || reap_fail "df unreadable"
gb() { awk -v b="$1" 'BEGIN{printf "%.3f", b/1e9}'; }
since=$((REAP_NOW - 604800))
rec=0
[ -f "$REAP_LEDGER" ] && rec=$(awk -F'\t' -v s="$since" '$1>=s && $5 ~ /^removed/ {t+=$4} END{print t+0}' "$REAP_LEDGER")
{
  echo "# RAID weekly report $date_s"
  echo
  echo "used: ${pct}%  free: ${free}G  reclaimed_7d_gb: $(gb "$rec")"
  echo
  echo "## per class (GB)"
  for c in ${REAP_CLASSES:-tmp wt agent-wt worktrees cache}; do
    if [ -d "$raid/$c" ]; then echo "$c $(gb "$(bytes_of "$raid/$c")")"; else echo "$c not_measured"; fi
  done
  echo
  echo "## KEEP (dirty/unpushed/open)"
  cat "$REAP_STATE_DIR"/keep-*.tsv 2>/dev/null || true
  echo
  echo "## not_measured"
  cat "$REAP_STATE_DIR"/nm-*.tsv 2>/dev/null || true
} >"$out/raid-weekly-$date_s.md"
echo "REAP report PASS reclaimed=$(gb "$rec") kept=0 not_measured=0"
