#!/usr/bin/env bash
# Gate E — every PR merged since the last tag carries a quorum receipt.
#
# Exit code is the gate. The only line a caller must read is the final
# `GATE E PASS|FAIL <detail>`.
#
# WHY THIS IS A SCRIPT AND NOT A PARAGRAPH IN THE SKILL
#
# The same reason as gate A (see scripts/dogfood/harness.sh): a requirement
# discharged by an agent reading GitHub is a requirement with no exit code, no
# place in `make dogfood-release` and no way to be shown red. The PMAT-163
# merge review refuted the branch on exactly that.
#
# WHAT A RECEIPT IS
#
# `scripts/quorum-gate.sh` refuses to push a branch whose claims have not
# survived refutation, and the artifact it leaves is the COMMITTED
# `.quorum/<slug>.json`, where `<slug>` is the head branch with every `/`
# replaced by `-`. That file survives a squash merge because it was committed
# on the branch. `docs/audits/quorum-<pr>.md` was a placeholder name nothing
# ever wrote; it is not what this gate reads.
#
# WHAT IS DEMANDED OF IT, AND WHY EACH FLOOR EXISTS
#
#   quorum.lanes >= 3            — three independent readings; two lanes can
#                                  agree by sharing one blind spot.
#   quorum.judges >= 3           — a majority needs three.
#   quorum.refuters_per_claim >= 3 — a claim one refuter failed to kill is a
#                                  claim that met one refuter.
#   quorum.claims_refuted >= 1   — a round that killed nothing did not hunt.
#                                  This is the anti-vacuity floor: a receipt
#                                  recording 60 confirmations and 0 refutations
#                                  is the shape of a quorum that never ran.
#   evidence.files >= 1          — a verdict that cites no file is an opinion.
#   no `waiver`/`waived`/`override` key ANYWHERE — a waiver is an unrefuted
#                                  claim in a receipt's clothing, and nesting
#                                  it one level deeper must not buy it a pass,
#                                  so every key path is searched, not the top
#                                  level only.
#
# The receipt is read AT HEAD, never from the working tree: a gate that reads
# uncommitted files reports on a tree no one else will ever have.
#
# UNMEASURED IS FAIL: a gh that cannot answer, a page that fills the limit or a
# receipt that is not parsable JSON each mean this gate does not know what it
# checked, and it says so rather than printing the words of a check that ran.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

fail() {
  echo "GATE E FAIL $1"
  exit 1
}

# The window (and the constants GH, REPO, PR_PAGE_LIMIT) — shared with gate A
# so the two gates cannot drift into checking different sets of PRs. `fail` is
# defined above because window.sh calls it.
# shellcheck source=scripts/dogfood/lib/window.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib/window.sh"

dogfood_load_window

# The whole judgement, in one jq program, so that a receipt is either read
# completely or reported as unreadable. It prints the FIRST floor the receipt
# fails, in words, or `ok`.
#
# `num` and `len` refuse a value of the wrong type instead of comparing it: in
# jq every string sorts above every number, so a receipt claiming
# `"judges": "3"` — or `"judges": "many"` — would clear a bare `>= 3` while
# recording nothing countable.
# shellcheck source=scripts/dogfood/lib/receipt.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib/receipt.sh"
QUORUM_JQ="$DOGFOOD_RECEIPT_JQ"

i=0
checked=0
while [ "$i" -lt "$DOGFOOD_PR_COUNT" ]; do
  dogfood_pr_field "$i" ".number"
  num="$DOGFOOD_FIELD"
  dogfood_pr_field "$i" ".headRefName"
  href="$DOGFOOD_FIELD"
  if [ -z "$href" ]; then
    fail "GitHub reports PR #${num} merged since ${DOGFOOD_PREV_TAG} with no headRefName, so its receipt slug (.quorum/<branch>.json) cannot be derived — UNMEASURED"
  fi

  dogfood_slug "$href"
  receipt=".quorum/${DOGFOOD_SLUG}.json"
  if ! git cat-file -e "HEAD:${receipt}" 2>/dev/null; then
    fail "PR #${num} (branch ${href}) merged with no ${receipt} at HEAD: its claims were never refuted by anything this gate can read, and an absent receipt is not a passed quorum"
  fi

  grc=0
  text="$(git show "HEAD:${receipt}")" || grc=$?
  if [ "$grc" -ne 0 ]; then
    fail "git show HEAD:${receipt} exited ${grc} for PR #${num}, so the receipt cannot be read — UNMEASURED"
  fi

  jrc=0
  why="$(printf '%s' "$text" | jq -r "$QUORUM_JQ")" || jrc=$?
  if [ "$jrc" -ne 0 ]; then
    fail "PR #${num}: ${receipt} is not readable as the JSON this gate checks (jq exit ${jrc}) — UNMEASURED, and an unreadable receipt is not a passed quorum"
  fi
  if [ "$why" != "ok" ]; then
    fail "PR #${num}: ${receipt} ${why}"
  fi

  echo "GATE E #${num} ${receipt} ok"
  checked=$((checked + 1))
  i=$((i + 1))
done

# An empty window is a PASS and says so. Nothing merged since the tag is a fact
# about the repository; it is not the same as a window this gate could not
# read, and every way of failing to read one is a FAIL above.
if [ "$DOGFOOD_PR_COUNT" -eq 0 ]; then
  echo "GATE E PASS 0 of 0 merged PR(s) since ${DOGFOOD_PREV_TAG} carry a quorum receipt: no PR has been merged into main since that tag (${DOGFOOD_PR_OUTSIDE} merged after this HEAD), so the window is empty — which is not the same as unmeasured"
  exit 0
fi

echo "GATE E PASS ${checked} of ${DOGFOOD_PR_COUNT} merged PR(s) since ${DOGFOOD_PREV_TAG} carry a quorum receipt"

# mutation: raise the lane floor in DOGFOOD_RECEIPT_JQ (scripts/dogfood/lib/receipt.sh, sourced above) from `len(.quorum.lanes) < 3` to
# `len(.quorum.lanes) < 99` — every committed receipt then reads as thin and
# the gate exits 1, which shows the lane count is read out of the receipt at
# HEAD and not assumed.
