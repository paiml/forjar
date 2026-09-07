#!/usr/bin/env bash
# Gate A — every PR merged since the last tag carries a harness receipt.
#
# Exit code is the gate. The only line a caller must read is the final
# `GATE A PASS|FAIL <detail>`.
#
# WHY THIS IS A SCRIPT AND NOT A PARAGRAPH IN THE SKILL
#
# A and E used to be prose in .claude/skills/forjar-dogfood/SKILL.md, carried
# out by an agent that read GitHub and formed a view. An agent that forms a
# view is not a gate: it cannot be shown to go red, it does not run in
# `make dogfood-release`, and what it produces is a paragraph rather than an
# exit code. The PMAT-163 merge review refuted the branch on exactly that, and
# this file is the answer.
#
# WHAT A RECEIPT IS
#
# `paiml-implement` writes one `docs/audits/impl-<ticket>-receipt.md` per
# implementation ticket. Three properties are checked, and each of them is a
# way a receipt has actually failed:
#
#   1. it EXISTS at HEAD, under the ticket id the PR itself names;
#   2. its LAST line is exactly `IMPL-<ticket>-RECEIPT-END`, so a truncated
#      write is detectable rather than silent — a receipt cut off halfway
#      reads, to everything else, like a receipt that simply said less;
#   3. exactly ONE line matches `^verdict:`. Zero is a receipt that reached no
#      verdict; two is a receipt whose reader cannot tell which verdict the run
#      reached, which is worse than none.
#
# The receipt is read AT HEAD (`git show HEAD:<path>`), never from the working
# tree: a gate that reads uncommitted files reports on a tree that no one else
# will ever have.
#
# UNMEASURED IS FAIL. A gh that cannot answer, a page that fills the limit, a
# PR with no merge commit: each of those means this gate does not know the set
# it is checking, and it says so instead of printing the words of a check that
# ran. Nothing here is skipped for being offline.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

fail() {
  echo "GATE A FAIL $1"
  exit 1
}

# The window (and the constants GH, REPO, PR_PAGE_LIMIT) — shared with gate E
# so the two gates cannot drift into checking different sets of PRs. `fail` is
# defined above because window.sh calls it.
# shellcheck source=scripts/dogfood/lib/window.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib/window.sh"

dogfood_load_window

# The ticket id a PR names, looked for in the branch, then the title, then the
# body — cheapest and most deliberate first. A merged PR that names none has no
# address at which a receipt could be looked for, and is a FAIL rather than a
# skip: "we could not tell which ticket this was" must never read as "this one
# is fine".
pr_ticket() {
  local src
  DOGFOOD_TICKET=""
  for src in "$@"; do
    if [[ "$src" =~ (PMAT-[0-9]+) ]]; then
      DOGFOOD_TICKET="${BASH_REMATCH[1]}"
      return 0
    fi
  done
}

i=0
checked=0
while [ "$i" -lt "$DOGFOOD_PR_COUNT" ]; do
  dogfood_pr_field "$i" ".number"
  num="$DOGFOOD_FIELD"
  dogfood_pr_field "$i" ".headRefName"
  href="$DOGFOOD_FIELD"
  dogfood_pr_field "$i" ".title"
  title="$DOGFOOD_FIELD"
  dogfood_pr_field "$i" ".body"
  body="$DOGFOOD_FIELD"

  pr_ticket "$href" "$title" "$body"
  id="$DOGFOOD_TICKET"
  if [ -z "$id" ]; then
    fail "PR #${num} (branch ${href}) names no PMAT-<n> ticket in its branch, title or body, so the harness receipt it must carry has no address: a merged PR whose ticket cannot be identified cannot be shown to have been implemented under the harness"
  fi

  receipt="docs/audits/impl-${id}-receipt.md"
  if ! git cat-file -e "HEAD:${receipt}" 2>/dev/null; then
    fail "PR #${num} (${id}) merged with no ${receipt} at HEAD: nothing records that this work went through the implementation harness, and an absent receipt is not a passed one"
  fi

  grc=0
  text="$(git show "HEAD:${receipt}")" || grc=$?
  if [ "$grc" -ne 0 ]; then
    fail "git show HEAD:${receipt} exited ${grc} for PR #${num}, so the receipt cannot be read — UNMEASURED"
  fi

  last="$(printf '%s\n' "$text" | tail -n 1)"
  if [ "$last" != "IMPL-${id}-RECEIPT-END" ]; then
    fail "PR #${num}: the last line of ${receipt} is \"${last}\", not IMPL-${id}-RECEIPT-END — the marker is what makes a truncated receipt detectable rather than silent, and without it the half that mattered may simply be missing"
  fi

  vrc=0
  verdicts="$(printf '%s\n' "$text" | grep -cE '^[Vv]erdict:')" || vrc=$?
  if [ "$vrc" -gt 1 ]; then
    fail "grep exited ${vrc} counting the verdict lines of ${receipt} — UNMEASURED"
  fi
  if [ "$verdicts" -ne 1 ]; then
    fail "PR #${num}: ${receipt} has ${verdicts} line(s) matching ^verdict: — exactly one is required, because zero is a receipt that reached no verdict and two is a receipt whose reader cannot tell which verdict the run reached"
  fi

  echo "GATE A #${num} ${id} ${receipt} ok"
  checked=$((checked + 1))
  i=$((i + 1))
done

# An empty window is a PASS and says so. Nothing has been merged since the tag
# is a fact about the repository; it is not the same as a window this gate
# could not read, and every way of failing to read one is a FAIL above.
if [ "$DOGFOOD_PR_COUNT" -eq 0 ]; then
  echo "GATE A PASS 0 of 0 merged PR(s) since ${DOGFOOD_PREV_TAG} carry a harness receipt: no PR has been merged into main since that tag (${DOGFOOD_PR_OUTSIDE} merged after this HEAD), so the window is empty — which is not the same as unmeasured"
  exit 0
fi

echo "GATE A PASS ${checked} of ${DOGFOOD_PR_COUNT} merged PR(s) since ${DOGFOOD_PREV_TAG} carry a harness receipt"

# mutation: change the comparison `[ "$last" != "IMPL-${id}-RECEIPT-END" ]` to
# `[ "$last" != "IMPL-${id}-RECEIPT-ENDS" ]` — every real receipt's last line
# then reads as truncated and the gate exits 1, which shows the marker is read
# out of the committed receipt and not assumed.
