#!/bin/sh
# no-pull-request-target-guard.sh -- fail if a workflow uses the pull_request_target trigger.
#
# pull_request_target runs with a write token and secrets in the BASE repo context, on
# PRs from forks: the "pwn request" hole. paiml accepts no fork contributions (NO-FORK-1),
# so every PR-triggered workflow uses `pull_request` and guards on a same-repo head.
#
# Usage: no-pull-request-target-guard.sh [WORKFLOW_DIR] [ALLOWLIST_FILE]
#   Allowlist lines: `<workflow file basename>  # reason`, matched as fixed strings. An
#   allowlisted file must contain no checkout step and no `run:` step (the fork-pr-closer
#   case: metadata only, never PR code). The PR gate passes the BASE branch's allowlist,
#   never the PR's own copy.
# Exit: 0 clean, 1 violation, 2 usage, unreadable input or no workflows scanned.
#
# Fails closed: nothing is stripped before matching. The name anywhere in a file (a
# comment, a quoted scalar, any case) is a violation unless it is the tail of a longer
# identifier (`falsification_no_pull_request_target`); so is a YAML hex/unicode
# escape (`\x..`, `\u....`, `\U........`), which could spell the name without its
# letters. A read error is exit 2, never a pass: unmeasured is not clean.
set -eu

dir="${1:-.github/workflows}"
allow="${2:-.github/pull-request-target-allowlist.txt}"

io() { echo "guard: $1" >&2; exit 2; }

[ -d "$dir" ] || io "no such directory: $dir"

# count PATTERN FILE [GREP_FLAG]: matching lines; grep rc 0/1 are answers, >=2 is a read error.
count() {
  rc=0
  n=$(grep -c ${3:+"$3"} -e "$1" -- "$2") || rc=$?
  [ "$rc" -le 1 ] || io "cannot read $2 (grep rc=$rc)"
  echo "$n"
}

# allowlisted BASENAME: 0 if the allowlist names it exactly, 1 if not.
allowlisted() {
  [ -e "$allow" ] || return 1
  [ -f "$allow" ] && [ -r "$allow" ] || io "unreadable allowlist: $allow"
  entries=$(sed -e 's/[[:space:]]*#.*$//' -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' -- "$allow") ||
    io "cannot read allowlist: $allow"
  rc=0
  printf '%s\n' "$entries" | grep -Fqx -- "$1" || rc=$?
  [ "$rc" -le 1 ] || io "allowlist match failed (grep rc=$rc)"
  return "$rc"
}

scanned=0
bad=0
for f in "$dir"/*.yml "$dir"/*.yaml; do
  [ -e "$f" ] || [ -L "$f" ] || continue # unmatched glob
  [ -d "$f" ] && continue                # GitHub reads files only
  [ -f "$f" ] && [ -r "$f" ] || io "unreadable workflow file: $f"
  scanned=$((scanned + 1))
  hits=$(count '\(^\|[^A-Za-z0-9_]\)pull_request_target' "$f" -i) || exit 2
  esc=$(count '\\[xuU][0-9A-Fa-f]' "$f") || exit 2
  if [ "$esc" -gt 0 ]; then
    echo "FAIL $f: YAML hex/unicode escape ($esc line(s)) could spell pull_request_target" >&2
    bad=$((bad + 1))
  fi
  [ "$hits" -gt 0 ] || continue
  base=$(basename "$f")
  if allowlisted "$base"; then
    co=$(count 'actions/checkout' "$f" -i) || exit 2
    run=$(count '^[[:space:]-]*run:' "$f") || exit 2
    if [ "$co" -gt 0 ] || [ "$run" -gt 0 ]; then
      echo "FAIL $f: allowlisted pull_request_target workflow checks out or runs code" >&2
      bad=$((bad + 1))
    fi
    continue
  fi
  echo "FAIL $f: uses pull_request_target ($hits line(s)); use pull_request with a same-repo head guard" >&2
  bad=$((bad + 1))
done

echo "no-pull-request-target-guard: scanned $scanned workflow file(s), $bad violation(s)"
[ "$scanned" -gt 0 ] || exit 2
[ "$bad" -eq 0 ]
