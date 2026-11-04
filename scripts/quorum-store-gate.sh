#!/usr/bin/env bash
# Quorum receipt gate (CI): reads the receipt STORE, not a committed file.
#
# WHY: a receipt binds to the PR head sha. Committing `.quorum/<branch>.json`
# moves the head, so the committed receipt can never match the head it was
# written for (a loop). The receipt therefore lives outside the tree:
#   ${QUORUM_STORE:-/mnt/nvme-raid0/cop-inbox/promote-state}/quorum/<repo>/<pr>/<headsha>.json
#   schema: {repo, pr, sha, author_model, verdicts:[{lane, model, verdict, blockers}]}
#
# PASS needs ALL of:
#   - the file for exactly this head sha exists and parses (missing store,
#     dir or file is a FAIL: UNMEASURED is failing, there is no skip path)
#   - body .sha == head sha, .repo == repo, .pr == PR number
#   - author_model is a real model id (not empty / "unknown")
#   - >= 3 DISTINCT lanes with verdict exactly "PASS", whose model != author_model
#   - no key named like waived/waiver/override anywhere in the document
#
# Inputs (env): PR_NUMBER, PR_HEAD_SHA (required), QUORUM_REPO (default
# paiml/forjar), QUORUM_STORE.
#
# NOT TOUCHED: scripts/quorum-gate.sh (local pre-push hook) and
# scripts/dogfood/{quorum,release-check}.sh (gates E / R) still read the
# committed `.quorum/<branch>.json`; they judge merged history, not this PR.
set -euo pipefail

fail() { echo "FAIL quorum receipt: $*" >&2; exit 1; }

repo="${QUORUM_REPO:-paiml/forjar}"
store="${QUORUM_STORE:-/mnt/nvme-raid0/cop-inbox/promote-state}"
pr="${PR_NUMBER:-}"
head="${PR_HEAD_SHA:-}"

[ -n "$pr" ] || fail "PR_NUMBER is not set (UNMEASURED)"
[ -n "$head" ] || fail "PR_HEAD_SHA is not set (UNMEASURED)"
[[ "$pr" =~ ^[0-9]+$ ]] || fail "PR_NUMBER '$pr' is not a number"
[[ "$head" =~ ^[0-9a-f]{40}$ ]] || fail "PR_HEAD_SHA '$head' is not a full 40-hex sha"
command -v jq >/dev/null || fail "jq not installed (UNMEASURED)"

[ -d "$store" ] || fail "receipt store $store is missing (UNMEASURED)"
file="$store/quorum/$repo/$pr/$head.json"
[ -f "$file" ] || fail "no receipt at $file for head $head"

jq -e . "$file" >/dev/null 2>&1 || fail "$file does not parse as JSON"

verdict="$(jq -r --arg repo "$repo" --argjson pr "$pr" --arg head "$head" '
  def bad: if (.sha // "") != $head then "sha \(.sha // "null") != head \($head)"
    elif (.repo // "") != $repo then "repo \(.repo // "null") != \($repo)"
    elif (.pr // null) != $pr then "pr \(.pr // "null") != \($pr)"
    elif ((.author_model // "") | (type != "string" or . == "" or . == "unknown")) then "author_model missing or unknown, independence unprovable"
    elif ([.. | objects | keys[] | select(test("waiv|overrid"; "i"))] | length) > 0 then "waiver/override key present"
    elif ((.verdicts // null) | type) != "array" then "verdicts is not an array"
    else null end;
  . as $r
  | (bad) as $b
  | if $b != null then "BAD \($b)" else
      ([ $r.verdicts[] | select(.verdict == "PASS" and (.model // "") != "" and .model != $r.author_model) | .lane ] | unique) as $lanes
      | if ($lanes | length) >= 3 then "OK \($lanes | length) independent PASS lanes: \($lanes | join(","))"
        else "BAD only \($lanes | length) distinct PASS lanes with a non-author model (need >= 3)" end
    end' "$file")" || fail "could not evaluate $file"

case "$verdict" in
  OK*) echo "PASS quorum receipt: $verdict ($file)" ;;
  *) fail "${verdict#BAD } ($file)" ;;
esac
# mutation: change `>= 3` to `>= 0` (or drop the `.model != $r.author_model` clause) and the lane-floor and author-model tests in tests/falsification_quorum_gate_reads_the_receipt_store.rs go red
