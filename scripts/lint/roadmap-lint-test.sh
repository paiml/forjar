#!/usr/bin/env bash
# Proves roadmap-lint.sh discriminates: one planted defect per case. Each bad case must exit
# 1 with exactly <count> findings, every one containing <pattern>. AWK=mawk|gawk picks the awk.
set -u
here=$(cd "$(dirname "$0")" && pwd) || exit 2
lint="$here/roadmap-lint.sh"
T=$(mktemp -d) || exit 2
trap 'rm -rf -- "${T:?}"' EXIT
fail=0 cases=0
TAB=$(printf '\t') CR=$(printf '\r') NL='
'
# A roadmap with every named row clean; cases add or edit text.
base() {
  local id
  printf 'roadmap:\n'
  for id in PMAT-1048 PMAT-1049 PMAT-1114 PMAT-1115 PMAT-1155 PMAT-1200; do
    printf -- "- id: %s\n  status: completed\n  acceptance_criteria:\n  - 'Issue #%s done'\n  phases: []\n" "$id" "${id#PMAT-}"
  done
}
verdict() { # name want-rc pattern count
  local name=$1 want=$2 pat=$3 count=$4 got=$RC hits n
  [ "$got" -eq "$want" ] || { echo "FAIL $name: want rc=$want got rc=$got"; cat "$T/out"; fail=$((fail + 1)); return; }
  [ "$want" -eq 1 ] || return 0
  hits=$(grep -c -F -- "$pat" "$T/out"); n=$(wc -l <"$T/out")
  if [ "$hits" -ne "$count" ] || [ "$n" -ne "$count" ]; then echo "FAIL $name: want $count findings of '$pat', got $hits of $n"; cat "$T/out"; fail=$((fail + 1)); fi
}
# check <name> <want-rc> <pattern> <count> <sed-expr-or-empty> [appended text]
check() {
  cases=$((cases + 1))
  if [ -n "$5" ]; then base | sed "$5" >"$T/r.yaml"; else base >"$T/r.yaml"; fi
  printf '%s' "${6:-}" >>"$T/r.yaml"
  "$lint" "$T/r.yaml" >"$T/out" 2>&1; RC=$?
  verdict "$1" "$2" "$3" "$4"
}
M='comment marker'
check clean 0 '' 0 ''
check comment-after-space 1 "$M" 1 "s|'Issue #1048 done'|Issue #1048 done|"
check tab-before-hash 1 "$M" 1 "s|'Issue #1048 done'|Issue${TAB}#1048 done|"
check key-hash-value 1 "$M" 1 '' "  title: #1200 broken
"
check leading-hash-item 1 "$M" 1 '' "  - #1 closed
"
check other-indent 1 "$M" 1 '' "      - Issue #1 closed
"
check full-line-comment-second-hash 0 '' 0 '' "# Fix issue #1200 and #2
  # Section #2
"
check quoted-hash-is-fine 0 '' 0 '' "  - 'a #1'
  title: \"b #2\"
"
check digit-boundary 1 'lost their' 1 "s|Issue #1048 done|Issue #10480 done|"
check duplicate-id 1 'duplicate row' 1 '' "- id: PMAT-1048
  status: completed
  acceptance_criteria:
  - 'Issue #1048 again'
"
check bad-status-only 1 'not one of' 1 '1,6s/^  status: completed/  status: planned/'
check whitespace-only-criteria 1 'empty or blank' 1 "s|'Issue #1049 done'|'  '|"
check quoted-empty-two-spaces 1 'empty or blank' 1 "s|  - 'Issue #1049 done'|  -  ''|"
check quoted-blanks-trailing-space 1 'empty or blank' 1 "s|  - 'Issue #1049 done'|  - '   ' |"
check null-word 1 'empty or blank' 1 "s|'Issue #1049 done'|Null|"
check upper-null-word 1 'empty or blank' 1 "s|'Issue #1049 done'|NULL|"
check tilde 1 'empty or blank' 1 "s|'Issue #1049 done'|~|"
check bare-dash 1 'empty or blank' 1 "s|  - 'Issue #1049 done'|  -|"
check empty-list 1 'empty or blank' 1 "/PMAT-1115/,/phases/{s|^  acceptance_criteria:|  acceptance_criteria: []|;/^  - /d;}"
check inprogress-ok-for-1155 0 '' 0 '/PMAT-1155/,/phases/s/^  status: completed/  status: inprogress/'
check inprogress-ok-for-1200 0 '' 0 '/PMAT-1200/,/phases/s/^  status: completed/  status: inprogress/'
check blank-item-beside-good-item 1 'empty or blank' 1 "s|  - 'Issue #1049 done'|&\\${NL}  - '  '|"
check missing-row 1 'row missing' 1 "s/PMAT-1155/PMAT-9999/"
check crlf 1 'unsupported layout: CR' 1 "s|\$|${CR}|"
check four-space-criteria 1 'unsupported layout: criteria' 1 "s|^  - 'Issue #1048 done'|  &|"
check blank-line-before-list 1 'unsupported layout: blank' 1 "s|^  acceptance_criteria:\$|&\\${NL}|"
check reordered-keys 1 'unsupported layout: a row' 1 "s|^- id: PMAT-1048|- status: completed\\${NL}  id: PMAT-1048|"
check indented-row 1 'unsupported layout: a row' 1 "s|^- id: PMAT-1048|  - id: PMAT-1048|"
: >"$T/r.yaml"; "$lint" "$T/r.yaml" >"$T/out" 2>&1; RC=$?; cases=$((cases + 1)); verdict empty-file 1 'row missing' 6
mkdir "$T/d"; "$lint" "$T/d" >"$T/out" 2>&1; RC=$?; cases=$((cases + 1)); verdict directory 2 '' 0
"$lint" "$T/absent.yaml" >"$T/out" 2>&1; RC=$?; cases=$((cases + 1)); verdict unreadable 2 '' 0
base >"$T/a=b.yaml"; (cd "$T" && "$lint" a=b.yaml </dev/null >"$T/out" 2>&1); RC=$?; cases=$((cases + 1)); verdict name-with-equals 0 '' 0
if [ "$fail" -eq 0 ]; then echo "roadmap-lint-test: all $cases cases ok"; else echo "roadmap-lint-test: $fail of $cases failed"; exit 1; fi
