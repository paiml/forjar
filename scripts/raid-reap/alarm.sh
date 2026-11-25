#!/usr/bin/env bash
# alarm.sh -- RAID-REAP rule R5: used >= 70% writes ONE inbox line per state change and exits 1.
# The "/" floor alarm (100 G, #5264) is separate. mutation: change REAP_ALARM_PCT default 70 to 71 and the 70% fixture stays green.
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
. "$here/lib.sh"
thr=${REAP_ALARM_PCT:-70}
inbox=${REAP_INBOX:-/mnt/nvme-raid0/cop-inbox/inbox.md}
reap_init alarm
read -r pct free < <(disk_stat) || reap_fail "df unreadable"
case $pct in '' | *[!0-9]*) reap_fail "bad pct '$pct'" ;; esac
state="$REAP_STATE_DIR/alarm.state"
if [ "$pct" -ge "$thr" ]; then
  if [ "$(cat "$state" 2>/dev/null || true)" != alarm ]; then
    printf '%s | reaper | RAID %s%% used %sG free\n' "$(date -u -d "@$REAP_NOW" +%H:%MZ)" "$pct" "$free" >>"$inbox"
    echo alarm >"$state"
  fi
  echo "REAP alarm FAIL reclaimed=0.000 kept=0 not_measured=0 used=${pct}%"
  exit 1
fi
rm -f "${state:?}"
echo "REAP alarm PASS reclaimed=0.000 kept=0 not_measured=0 used=${pct}%"
