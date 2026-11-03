#!/usr/bin/env bash
# ticket-cite-lint.sh <base> [head] -- the ticket is cited in the commit, checked before the push by `make pr-light` (PMAT-2897).
# Exit 0 only when EVERY non-merge commit in <base>..<head> (head defaults to HEAD) carries a `Pmat-Ticket: <PREFIX>-<number>`
# trailer. A range with no commits, an unreadable base or head, or a commit without the trailer is RED: a check that cannot see
# its input must not pass. Copy of paiml-implement skills/paiml-implement/scripts/ticket-cite-lint.sh (MOD-002 W1): edit both.
set -uo pipefail
base=${1:-}; head=${2:-HEAD}
if [[ -z "$base" ]]; then echo "RED ticket-cite-lint: no base given (usage: ticket-cite-lint.sh <base> [head])"; exit 1; fi
if ! git rev-parse --verify --quiet "$base^{commit}" >/dev/null || ! git rev-parse --verify --quiet "$head^{commit}" >/dev/null; then
  echo "RED ticket-cite-lint: cannot resolve $base or $head to a commit"
  exit 1
fi
mapfile -t shas < <(git rev-list --no-merges "$base..$head")
if [[ ${#shas[@]} -eq 0 ]]; then echo "RED ticket-cite-lint: no commits in $base..$head — nothing was judged"; exit 1; fi
bad=0
for sha in "${shas[@]}"; do
  if ! git show -s --format=%B "$sha" | grep -qE '^Pmat-Ticket: [A-Za-z][A-Za-z0-9]*-[0-9]+[[:space:]]*$'; then
    echo "RED ticket-cite-lint: ${sha:0:12} $(git show -s --format=%s "$sha" | head -c 80) carries no 'Pmat-Ticket: <ID>' trailer"
    bad=1
  fi
done
if [[ "$bad" -ne 0 ]]; then exit 1; fi
echo "ok ticket-cite-lint: ${#shas[@]} commit(s) in $base..$head all cite a ticket"
