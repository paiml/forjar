#!/bin/sh
printf '%s\n' "$*" >> "$FJ_SSH_LOG"

sock=""
master=""
for a in "$@"; do
  case "$a" in
    ControlPath=*) sock="${a#ControlPath=}" ;;
    ControlMaster=yes) master=1 ;;
  esac
done
if [ -n "$master" ] && [ -n "$sock" ]; then
  : > "$sock"
fi

prev=""
last=""
for a in "$@"; do prev="$last"; last="$a"; done
if [ "$last" != "bash" ]; then
  exit 0
fi

script=$(cat)
printf '%s\n' "$script" >> "$FJ_SSH_LOG.scripts"
case "$script" in
  *"df -i"*)
    printf 'Filesystem Type Inodes IUsed IFree IUse%% Mounted\n/dev/x ext4 100 1 99 1%% /\n'
    exit 0
    ;;
esac
if [ -n "$FJ_SSH_OVERLAP_LOG" ]; then
  case "$script" in
    *__DIR__*)
      printf 'S %s\n' "$prev" >> "$FJ_SSH_OVERLAP_LOG"
      sleep 0.6
      printf 'E %s\n' "$prev" >> "$FJ_SSH_OVERLAP_LOG"
      ;;
  esac
fi
exit 0
