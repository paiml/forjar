#!/usr/bin/env bash
# Proves roadmap-lint.sh discriminates: one planted defect per case (exit 0 clean, 1 findings).
set -u
here=$(cd "$(dirname "$0")" && pwd) || exit 2
lint="$here/roadmap-lint.sh"
T=$(mktemp -d) || exit 2
trap 'rm -rf -- "${T:?}"' EXIT
fail=0
# rows <PMAT-1048 override block>: a roadmap with every named row clean, plus extra text.
base() {
  local id n
  printf 'roadmap:\n'
  for id in PMAT-1048 PMAT-1049 PMAT-1114 PMAT-1115 PMAT-1155 PMAT-1200; do
    n=${id#PMAT-}
    printf -- "- id: %s\n  status: completed\n  acceptance_criteria:\n  - 'Issue #%s done'\n  phases: []\n" "$id" "$n"
  done
}
# check <name> <want-rc> <sed-expr-or-empty> [appended text]
check() {
  local name=$1 want=$2 expr=$3 extra=${4:-} got
  if [ -n "$expr" ]; then base | sed "$expr" >"$T/r.yaml"; else base >"$T/r.yaml"; fi
  printf '%b' "$extra" >>"$T/r.yaml"
  "$lint" "$T/r.yaml" >"$T/out" 2>&1; got=$?
  [ "$got" -eq "$want" ] || { echo "FAIL $name: want rc=$want got rc=$got"; cat "$T/out"; fail=$((fail + 1)); }
}
check clean 0 ''
check comment-after-space 1 "s|'Issue #1048 done'|Issue #1048 done|"
check tab-before-hash 1 "s|'Issue #1048 done'|Issue\t#1048 done|"
check key-hash-value 1 '' '  title: #1200 broken\n'
check leading-hash-item 1 '' '  - #1 closed\n'
check other-indent 1 '' '      - Issue #1 closed\n'
check full-line-comment-second-hash 0 '' '# Fix issue #1200 and #2\n  # Section #2\n'
check quoted-hash-is-fine 0 '' "  - 'a #1'\n  title: \"b #2\"\n"
check digit-boundary 1 "s|Issue #1048 done|Issue #10480 done|"
check duplicate-id-bad-status 1 '' '- id: PMAT-1048\n  status: planned\n  acceptance_criteria:\n  - x #1048\n'
check whitespace-only-criteria 1 "s|'Issue #1049 done'|'  '|"
check null-criteria 1 '' '- id: PMAT-1114\n  status: completed\n  acceptance_criteria:\n  -\n'
check empty-list 1 '' '- id: PMAT-1115\n  status: completed\n  acceptance_criteria: []\n'
check wrong-state 1 '1,6s/^  status: completed/  status: planned/'
check inprogress-ok-for-1155 0 '/PMAT-1155/,/phases/s/^  status: completed/  status: inprogress/'
check blank-item-beside-good-item 1 "s|  - 'Issue #1049 done'|&\n  - '  '|"
check missing-row 1 "s/PMAT-1155/PMAT-9999/"
"$lint" "$T/absent.yaml" >/dev/null 2>&1; [ $? -eq 2 ] || { echo "FAIL unreadable: want rc=2"; fail=$((fail + 1)); }
if [ "$fail" -eq 0 ]; then echo "roadmap-lint-test: all cases ok"; else echo "roadmap-lint-test: $fail failed"; exit 1; fi
