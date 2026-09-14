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
# THE TRAILER FLOOR (PMAT-540) -- the commit up to which a PR is not judged on
# what its commits claim.
#
# `b4719737` is PR #532's own merge commit, and #532 is the ONE record in thirty
# that this rule would condemn: its branch says PMAT-520 and every commit says
# PMAT-531. A merged branch cannot be renamed, so the choice was to exempt it by
# name or to rewrite history. The floor is set at that commit and no later, so
# every PR merged after it -- #536, #538, #539, #541, #543 and everything since
# -- IS judged, and all of them pass. A floor at main's tip would have exempted
# five PRs that need no exemption.
#
# The exempted set cannot grow: PMAT-535 refuses a branch naming a ticket no
# commit claims at push time, which is the only way a record like #532 was made.
#
# Measured over the thirty most recently merged PRs: 29 agree, #532 is the one
# mismatch, and none lacks a trailer. #496 LOOKS like a second mismatch --
# branch PMAT-218, commits PMAT-219 -- until the ids are resolved, because
# PMAT-219's row declares the misnomer as its `alias:`; resolving before
# comparing is why it counts among the 29.
TRAILER_FLOOR="${TRAILER_FLOOR:-b4719737}"

# shellcheck source=scripts/dogfood/lib/window.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib/window.sh"

dogfood_load_window

# The ticket a PR names — the first `PMAT-<n>` in the branch, then the title,
# then the body, cheapest and most deliberate first — is read by
# `dogfood_pr_tickets` in lib/window.sh, the one rule gates A and T share. An
# id that is not a roadmap row resolves only through a row that declares
# `alias:<id>` (PMAT-225); otherwise the PR has NO address at which a receipt
# could be looked for, and that is a FAIL rather than a skip, or a fall-through
# to the next id: "we could not tell which ticket this was" must never read as
# "this one is fine", and a forgotten roadmap row must never be papered over by
# an older ticket the body happens to mention.

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

  dogfood_pr_tickets "$href" "$title" "$body"
  id="$DOGFOOD_TICKET"
  if [ -z "$id" ] && [ -n "$DOGFOOD_STRAY_IDS" ]; then
    fail "PR #${num} (branch ${href}) names ${DOGFOOD_STRAY_IDS}, which is not a row of docs/roadmaps/roadmap.yaml at HEAD and which no row declares as its alias:<id>, so the harness receipt it must carry has no address: a ticket that is not on the roadmap cannot be shown to have been implemented under the harness"
  fi
  if [ -z "$id" ]; then
    fail "PR #${num} (branch ${href}) names no PMAT-<n> ticket in its branch, title or body, so the harness receipt it must carry has no address: a merged PR whose ticket cannot be identified cannot be shown to have been implemented under the harness"
  fi

  # THE MERGE COMMIT MUST CLAIM THE TICKET THE PR IS FILED UNDER (PMAT-540).
  #
  # `dogfood_pr_tickets` resolves a PR's ticket from the BRANCH, then the TITLE,
  # then the BODY, and PMAT-535 closed only the first: `scripts/quorum-gate.sh`
  # refuses, at push time, a branch that names a PMAT-<n> no commit claims. It
  # cannot close the other two, because at push time the pull request does not
  # exist and its title is typed afterwards. Here it does exist, and its merge
  # commit is an ancestor of HEAD, so the two sources can be compared.
  #
  # Measured on PR #532: branch `PMAT-520-book-v1.29.0`, every commit's trailer
  # PMAT-531. Every window arm credited PMAT-520 -- already SHIPPED in 1.29.0 --
  # to the v1.30.0 window, gate T demanded `release:v1.30.0` on it, and the
  # ticket that owned the work was invisible to all three gates. Nobody added
  # that label by hand.
  #
  # READ BY THE LINE, not with `%(trailers:key=…)`. git's parser reads the LAST
  # PARAGRAPH only, and GitHub appends its own to a squash message: after the
  # branch commits' bodies it writes a `---------` separator and a
  # `Co-authored-by:` block, and THAT becomes the last paragraph. Measured on
  # b4719737, the merge commit of #532 itself: `%(trailers)` returns the
  # `Co-authored-by:` line alone and `%(trailers:key=Pmat-Ticket)` returns
  # NOTHING, while the line is plainly in the message. An arm built on git's
  # parser would be blind to the exact commit it exists for. pmat's CB-2113 asks
  # git the same way, which is why it saw nothing either.
  dogfood_pr_field "$i" ".mergeCommit.oid"
  merge="$DOGFOOD_FIELD"
  if [ -z "$merge" ]; then
    fail "PR #${num} (${id}) is in the window with no merge commit oid, so what its commits claim cannot be read — UNMEASURED"
  fi

  # THE FLOOR, AND WHY THERE IS ONE.
  #
  # A rule cannot condemn a record it arrived after. PR #532 is merged, and a
  # merge commit cannot be renamed: the only honest remedies were to exempt it
  # or to rewrite history. The floor exempts every PR merged up to this commit
  # by name, so a reader can see exactly which records the rule does not judge,
  # and PMAT-535 already refuses that shape at push time -- so the exemption
  # covers a set that cannot grow.
  #
  # A FLOOR THIS REPOSITORY DOES NOT CARRY EXEMPTS NOTHING. The floor is an
  # EXEMPTION, so failing to find it must make the gate stricter and not
  # looser: a typo, a shallow clone or a fixture repository all mean no PR is
  # excused, which is the direction that cannot hide a defect.
  frc=1
  if git rev-parse --verify "${TRAILER_FLOOR}^{commit}" >/dev/null 2>&1; then
    frc=0
    git merge-base --is-ancestor "$merge" "$TRAILER_FLOOR" >/dev/null 2>&1 || frc=$?
  fi
  if [ "$frc" -eq 0 ]; then
    echo "GATE A #${num} ${id} predates the trailer floor ${TRAILER_FLOOR} — not judged on what its commits claim"
  elif [ "$frc" -gt 1 ]; then
    fail "git merge-base --is-ancestor exited ${frc} comparing PR #${num}'s merge commit ${merge} with the trailer floor ${TRAILER_FLOOR} — UNMEASURED"
  else
    mrc=0
    mmsg="$(git log -1 --format=%B "$merge")" || mrc=$?
    if [ "$mrc" -ne 0 ]; then
      fail "git log -1 on PR #${num}'s merge commit ${merge} exited ${mrc}, so what its commits claim cannot be read — UNMEASURED"
    fi
    # Normalised exactly as scripts/quorum-gate.sh normalises it, and for the
    # same measured reasons: a CRLF message glues \r to the id, a
    # `Pmat-Ticket: PMAT-1, PMAT-2` line is two claims, and being stricter than
    # the commit-msg hook about indentation or case can only invent refusals.
    # Guarded, because `set -euo pipefail` turns a failure anywhere in this
    # pipeline into a bare exit -- and an exit code with no `GATE A` line is a
    # death rather than a verdict, which this file refuses everywhere else.
    crc=0
    claims="$(printf '%s\n' "$mmsg" \
      | sed -n 's/^[[:space:]]*[Pp][Mm][Aa][Tt]-[Tt][Ii][Cc][Kk][Ee][Tt]:[[:space:]]*//p' \
      | tr -d '\r' | tr ',' ' ' | tr '\n' ' ' | tr -s ' ')" || crc=$?
    if [ "$crc" -ne 0 ]; then
      fail "reading the Pmat-Ticket lines out of PR #${num}'s merge commit ${merge} exited ${crc} — UNMEASURED"
    fi
    # Resolve each claim the way the PR's own id was resolved, so an id that a
    # roadmap row declares as its `alias:` compares equal to the row. PR #496's
    # branch says PMAT-218 and its commits say PMAT-219, and PMAT-219's row
    # declares the misnomer -- that PR is correctly filed and must not be red.
    # Deduplicated: a squash message repeats the trailer once per squashed
    # commit, and "claims PMAT-531 PMAT-531 PMAT-531 PMAT-531 PMAT-531" tells a
    # reader nothing the first one did not.
    resolved=""
    for claim in $claims; do
      dogfood_resolve_id "$claim"
      claim="${DOGFOOD_ROW:-$claim}"
      case " $resolved " in
        *" $claim "*) ;;
        *) resolved="${resolved:+$resolved }$claim" ;;
      esac
    done
    # ONLY when the commits claim SOMETHING. A merge commit carrying no
    # `Pmat-Ticket:` line at all is the commit-msg hook's finding and CB-2113's,
    # not this gate's, and an arm that also reported it would be saying someone
    # else's finding in its own words.
    #
    # This is a real exemption and it is stated as one: a PR whose merge commit
    # claims nothing PASSES here. None of the thirty most recently merged PRs is
    # in that shape, and two other checks already judge it, but "the merge
    # commit claims the ticket" is not what this arm enforces -- it enforces
    # "does not claim a DIFFERENT one".
    if [ -n "$resolved" ]; then
      case " $resolved " in
        *" $id "*) ;;
        *)
          fail "PR #${num} is filed under ${id} -- from its branch '${href}', then its title -- and its merge commit ${merge} claims ${resolved}. Gate A resolves the receipt path from that id, gate E resolves .quorum/<slug>.json from it and gate T resolves the RELEASE WINDOW from it, so a PR filed under one ticket whose work claims another credits the work to the wrong ticket in all three. Rename the branch or fix the title BEFORE merging; after the merge neither can be changed" ;;
      esac
    fi
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

# mutation: drop the `*" $id "*) ;;` arm of the trailer case -- every PR whose
# merge commit claims its own ticket then reads as a mismatch and the gate exits
# 1 on the first one, which shows the comparison is made against what the commit
# actually says and not assumed.
# mutation: change the comparison `[ "$last" != "IMPL-${id}-RECEIPT-END" ]` to
# `[ "$last" != "IMPL-${id}-RECEIPT-ENDS" ]` — every real receipt's last line
# then reads as truncated and the gate exits 1, which shows the marker is read
# out of the committed receipt and not assumed.
