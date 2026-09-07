#!/usr/bin/env bash
# The ONE predicate that says whether a committed quorum receipt is a quorum.
# Sourced by scripts/dogfood/quorum.sh (gate E, before the tag) and by
# scripts/dogfood/release-check.sh Arm 5 (after the tag), so the two cannot
# drift apart: the fourth PMAT-163 merge review found Arm 5 checking a
# top-level `waived` key and the lane and judge floors only, while gate E
# also refused nested waiver/override keys, thin refuter counts, a round that
# killed no claim, and a receipt citing no evidence. A release's last check
# must be at least as strict as its first.
#
# `dogfood_receipt_status BLOB` prints "ok" or one sentence naming the first
# floor the receipt is under. Not valid JSON is a reason too, never a pass.

DOGFOOD_RECEIPT_JQ='
  def num($v): if ($v | type) == "number" then $v else -1 end;
  def len($v): if ($v | type) == "array" then ($v | length) else -1 end;
  if ([paths | .[-1]? | tostring] | any(test("waiv|override")))
    then "carries a waiver/override key: a waived quorum is an unrefuted claim, not a passed one"
  elif len(.quorum.lanes) < 3
    then "quorum.lanes = \(len(.quorum.lanes)), below the floor of 3 that scripts/quorum-gate.sh enforces at push time"
  elif num(.quorum.judges) < 3
    then "quorum.judges = \(num(.quorum.judges)), below the floor of 3: a majority needs three"
  elif num(.quorum.refuters_per_claim) < 3
    then "quorum.refuters_per_claim = \(num(.quorum.refuters_per_claim)), below the floor of 3: a claim one refuter failed to kill only met one refuter"
  elif num(.quorum.claims_refuted) < 1
    then "quorum.claims_refuted = \(num(.quorum.claims_refuted)): a round that killed no claim did not hunt, which is the vacuous receipt this floor exists to refuse"
  elif len(.evidence.files) < 1
    then "evidence.files is empty: a verdict that cites no file is an opinion"
  else "ok"
  end
'

dogfood_receipt_status() {
  local rc=0 why
  why="$(printf '%s' "$1" | jq -r "$DOGFOOD_RECEIPT_JQ" 2>/dev/null)" || rc=$?
  if [ "$rc" -ne 0 ] || [ -z "$why" ]; then
    printf '%s\n' "is not a JSON object this predicate can read (jq exit ${rc}): an unreadable receipt is no receipt"
    return 0
  fi
  printf '%s\n' "$why"
}
