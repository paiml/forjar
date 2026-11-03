#!/usr/bin/env bash
# pr-light.sh [head] -- `make pr-light`: the PR's fast guards, run BEFORE the push (PMAT-855; operator C2 item 2,
# 2026-11-03 07:05Z; C4 07:15Z: "make pr-light is ADVISORY (run, attach result to PR, never block push/PR)").
#
# Two legs, both must be green:
#   1. ticket-cite-lint: every non-merge commit in origin/main..<head> (default HEAD) carries a `Pmat-Ticket: <ID>` trailer.
#   2. the tree guards: the ci.yml steps marked `# pr-light` (read-only `cargo test --locked --test <guard>` targets of the
#      examples-validate job), run in ONE cargo invocation. There is NO guard list here: the list is DERIVED from ci.yml, so
#      this cannot drift from the gate (bashrs#266). A marker that is not a plain `cargo test --locked --test NAME` is RED.
# Exit 0 only when both ran and passed. A leg that could not run (no origin/main, no markers) is RED, never a pass. There is
# pr-light itself exits non-zero on red so the signal is real; the pre-push hook prints it and never blocks. The tree guards judge the checked-out tree; the trailer leg judges <head>.
set -uo pipefail
head=${1:-HEAD}
cd "$(git rev-parse --show-toplevel)" || { echo "RED pr-light: not inside a git work tree"; exit 1; }
if ! git rev-parse --verify --quiet 'origin/main^{commit}' >/dev/null; then
  echo "RED pr-light: no origin/main to judge against (git fetch origin main)"
  exit 1
fi
rc=0
echo "== pr-light 1/2: ticket-cite-lint origin/main..$head"
scripts/ticket-cite-lint.sh origin/main "$head" || rc=1
echo "== pr-light 2/2: tree guards (ci.yml steps marked '# pr-light')"
wf=.github/workflows/ci.yml
mapfile -t cmds < <(awk '/^[[:space:]]*- name:/ { mark = 0 } /^[[:space:]]*# pr-light([[:space:]]|$)/ { mark = 1 } mark && /^[[:space:]]*run:/ { sub(/^[[:space:]]*run:[[:space:]]*/, ""); print; mark = 0 }' "$wf")
if [ "${#cmds[@]}" -eq 0 ]; then
  echo "RED pr-light: 0 steps marked '# pr-light' in $wf (0 guards is never a pass)"
  rc=1
else
  args=()
  for c in "${cmds[@]}"; do
    name=${c#cargo test --locked --test }
    if [ "$name" = "$c" ] || ! [[ "$name" =~ ^[A-Za-z0-9_]+$ ]]; then
      echo "RED pr-light: marked step is not a plain 'cargo test --locked --test NAME': $c"
      rc=1
      continue
    fi
    args+=(--test "$name")
  done
  echo "pr-light: ${#cmds[@]} marked guard(s), ${#args[@]} runnable"
  if [ "${#args[@]}" -gt 0 ] && [ "${#args[@]}" -eq $((2 * ${#cmds[@]})) ]; then
    cargo test --locked "${args[@]}" || rc=1
  fi
fi
if [ "$rc" -ne 0 ]; then
  echo "RED pr-light: advisory only, nothing is blocked. Fix the red above before review."
  exit 1
fi
echo "GREEN pr-light: ticket-cite and tree guards passed"
