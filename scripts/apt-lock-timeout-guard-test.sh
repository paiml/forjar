#!/usr/bin/env bash
# Proves apt-lock-timeout-guard.sh discriminates, one planted defect per case.
# Exit codes of the guard: 0 ok, 1 violation, 2 not measured. Fails on the first
# case whose exit code differs from the expected one.
set -u
here=$(cd "$(dirname "$0")" && pwd) || exit 2
g="$here/apt-lock-timeout-guard.sh"
T=$(mktemp -d) || exit 2
trap 'rm -rf -- "${T:?}"' EXIT

fail=0
n=0
# check <name> <want-rc> <dir>
check() {
  local name=$1 want=$2 dir=$3 got
  "$g" "$dir" >/dev/null 2>&1
  got=$?
  n=$((n + 1))
  if [ "$got" -ne "$want" ]; then
    echo "FAIL $name: want rc=$want got rc=$got"
    fail=$((fail + 1))
  fi
}
# fx <name> <printf-format-with-\n>: a dir holding one workflow file with that body.
fx() {
  local d="$T/$1"
  mkdir -p "$d" || exit 2
  printf '%b' "$2" >"$d/w.yml"
  printf '%s\n' "$d"
}
HDR='jobs:\n  a:\n    steps:\n'
GOOD='      - run: sudo apt-get -o DPkg::Lock::Timeout=600 install -y musl-tools\n'

# The real tree passes.
check real-tree 0 "$here/../.github/workflows"

# Planted defects the guard must catch (rc=1).
check bare-install 1 "$(fx bare "$HDR      - run: sudo apt-get install -y musl-tools\n")"
check bare-update 1 "$(fx bareu "$HDR      - run: sudo apt-get update -qq\n")"
check trailing-comment-names-the-option 1 "$(fx tc "$HDR      - run: sudo apt-get install -y musl-tools # TODO add DPkg::Lock::Timeout\n")"
check timeout-zero 1 "$(fx t0 "$HDR      - run: sudo apt-get -o DPkg::Lock::Timeout=0 install -y musl-tools\n")"
check timeout-empty 1 "$(fx te "$HDR      - run: sudo apt-get -o DPkg::Lock::Timeout= install -y musl-tools\n")"
check timeout-not-a-number 1 "$(fx tn "$HDR      - run: sudo apt-get -o DPkg::Lock::Timeout=soon install -y musl-tools\n")"
check chained-second-call-bare 1 "$(fx ch "$HDR      - run: sudo apt-get -o DPkg::Lock::Timeout=600 update && sudo apt-get install -y musl-tools\n")"
check semicolon-second-call-bare 1 "$(fx sc "$HDR      - run: sudo apt-get -o DPkg::Lock::Timeout=600 update; sudo apt-get install -y musl-tools\n")"
check after-then 1 "$(fx th "$HDR      - run: |\n          if true; then sudo apt-get install -y x; fi\n")"
check bare-in-block-scalar 1 "$(fx bs "$HDR      - run: |\n          set -e\n          sudo apt-get install -y x\n")"
check continuation-line-bare 1 "$(fx cb "$HDR      - run: |\n          sudo apt-get \\\\\n            install -y x\n")"
check yaml-extension 1 "$(d="$T/yx"; mkdir -p "$d"; printf '%b' "${HDR}      - run: sudo apt-get update\n" >"$d/w.yaml"; echo "$d")"
check one-good-one-bad-file 1 "$(d=$(fx gb "$HDR$GOOD"); printf '%b' "${HDR}      - run: sudo apt-get update\n" >"$d/z.yml"; echo "$d")"
check space-in-dir-name 1 "$(d="$T/has space"; mkdir -p "$d"; printf '%b' "${HDR}      - run: sudo apt-get update\n" >"$d/w.yml"; echo "$d")"

# Compliant shapes the guard must accept (rc=0). Each carries a second, good call so a pass is not "0 calls".
check good-call 0 "$(fx good "$HDR$GOOD")"
check attached-option-form 0 "$(fx at "$HDR      - run: sudo apt-get -oDPkg::Lock::Timeout=30 install -y x\n")"
check both-chained-calls-good 0 "$(fx bg "$HDR      - run: sudo apt-get -o DPkg::Lock::Timeout=600 update && sudo apt-get -o DPkg::Lock::Timeout=600 install -y x\n")"
check continuation-line-good 0 "$(fx cg "$HDR      - run: |\n          sudo apt-get \\\\\n            -o DPkg::Lock::Timeout=600 install -y x\n")"
check comment-line-ignored 0 "$(fx cl "$HDR$GOOD          # sudo apt-get install -y nothing\n")"
check echo-prose-is-not-a-call 0 "$(fx ep "$HDR$GOOD      - run: echo \"apt-get is missing; install musl-tools manually\"\n")"
check echo-warning-prose 0 "$(fx ew "$HDR$GOOD      - run: |\n          echo \"::warning::apt-get is not available. install musl-tools.\"\n")"
check step-name-is-not-a-call 0 "$(fx sn "$HDR$GOOD      - name: apt-get install deps\n")"
check command-v-probe-is-not-a-call 0 "$(fx cv "$HDR$GOOD      - run: command -v apt-get >/dev/null\n")"
check space-in-dir-name-good 0 "$(d="$T/has space ok"; mkdir -p "$d"; printf '%b' "$HDR$GOOD" >"$d/w.yml"; echo "$d")"

# Not measured (rc=2): the guard says so rather than passing.
check missing-dir 2 "$T/does-not-exist"
mkdir "$T/empty"
check empty-dir 2 "$T/empty"
check only-prose-no-calls 2 "$(fx op "${HDR}      - run: echo \"apt-get is missing; install musl-tools manually\"\n")"

if [ "$fail" -ne 0 ]; then
  echo "FAIL apt-lock-timeout-guard: $fail of $n cases wrong"
  exit 1
fi
echo "PASS apt-lock-timeout-guard ($n cases: real tree ok, planted defects caught, compliant shapes accepted, not-measured is 2)"
