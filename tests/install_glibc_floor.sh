#!/usr/bin/env bash
# forjar#1032: install.sh must hand a glibc host the gnu asset only when the host
# glibc is at least the floor the gnu asset needs; otherwise the static musl asset.
# ldd is stubbed on a private PATH, so the host's real glibc is never consulted.
set -euo pipefail
cd "$(dirname "$0")/.." || exit 2
work=$(mktemp -d)
[ -n "$work" ] && [ -d "$work" ] || exit 2
case "$work" in /|"") exit 2 ;; esac
trap 'rm -rf "$work"' EXIT
# A PATH with only the tools install.sh's detection uses (no real ldd).
mkdir "$work/base"
for t in uname grep head tail tr sed awk cat sh; do
  p=$(command -v "$t") || { echo "FAIL: tool $t missing"; exit 1; }
  ln -s "$p" "$work/base/$t"
done
# Pull only the two detection functions out of install.sh (the script itself runs on source).
awk '/^detect_os\(\) \{/,/^}/; /^detect_libc\(\) \{/,/^}/; /^FJ_GLIBC_MIN/; /^_fj_glibc_ok\(\) \{/,/^}/' install.sh > "$work/detect.sh"
[ -s "$work/detect.sh" ] || { echo "FAIL: detection functions not found in install.sh"; exit 1; }
bad=0
# check <name> <expected> <ldd banner or NONE>
check() {
  local name=$1 want=$2 banner=$3 dir="$work/p-$1"
  mkdir "$dir"
  cp -s "$work"/base/* "$dir"/
  if [ "$banner" != NONE ]; then
    printf '#!/bin/sh\necho "%s"\n' "$banner" > "$dir/ldd"
    chmod +x "$dir/ldd"
  fi
  local got
  got=$(PATH="$dir" sh -c '. "$1"; die() { echo "$1" >&2; exit 1; }; detect_libc' _ "$work/detect.sh" 2>&1) || got="ERR:$got"
  if [ "$got" = "$want" ]; then echo "ok $name -> $got"; else echo "FAIL $name: want $want got $got"; bad=1; fi
}
check old-glibc-2.35 musl "ldd (Ubuntu GLIBC 2.35-0ubuntu3.8) 2.35"
check old-glibc-2.34 musl "ldd (GNU libc) 2.34"
check new-glibc-2.39 gnu "ldd (Ubuntu GLIBC 2.39-0ubuntu8.3) 2.39"
check newer-glibc-2.41 gnu "ldd (GNU libc) 2.41"
check glibc-3.0 gnu "ldd (GNU libc) 3.0"
check musl-host musl "musl libc (x86_64)"
check no-ldd musl NONE
check unparseable musl "ldd: something odd"
exit "$bad"
