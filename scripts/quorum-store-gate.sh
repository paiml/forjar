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
#   - QUORUM_REPO is owner/name ([A-Za-z0-9_.-] only, no "..")
#   - the file for exactly this head sha exists, is a regular file (symlinks
#     are refused) and holds EXACTLY ONE JSON document (missing store, dir or
#     file is a FAIL: UNMEASURED is failing, there is no skip path)
#   - body .sha == head sha, .repo == repo, .pr == PR number
#   - author_model is a real model id (not empty / "unknown", case-insens.)
#   - >= 3 distinct NORMALIZED models (lowercase, trimmed, provider prefix such
#     as "anthropic/" stripped) AND >= 3 distinct normalized lane names, each
#     backed by a qualifying lane: lane and model non-empty strings, verdict
#     exactly "PASS", no blockers, model not in the author's family, and no
#     other verdict by the same normalized lane name anywhere in the receipt
#   - author family rule: a lane model is the author's family if, normalized,
#     it equals the author, shares the author's first 3 dash tokens
#     (claude-opus-5-5 ~ claude-opus-5-5-2026...), or either string contains
#     the other (claude-opus-5, opus). Same-family lanes do not count.
#   - no waiver/override in any key or any string VALUE (case-insensitive)
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
[[ "$repo" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || fail "QUORUM_REPO '$repo' is not owner/name"
[[ "$repo" != *..* ]] || fail "QUORUM_REPO '$repo' contains '..'"

[ -d "$store" ] || fail "receipt store $store is missing (UNMEASURED)"
file="$store/quorum/$repo/$pr/$head.json"
[ ! -L "$file" ] || fail "$file is a symlink (refused)"
[ -f "$file" ] || fail "no receipt at $file for head $head"

jq -s -e 'length == 1 and (.[0] | type == "object")' "$file" >/dev/null 2>&1 \
  || fail "$file is not exactly one JSON object document"

verdict="$(jq -s -r --arg repo "$repo" --argjson pr "$pr" --arg head "$head" '
  def trim: gsub("^\\s+|\\s+$"; "");
  def nmodel: ascii_downcase | trim | sub("^[^/]*/"; "") | trim;
  def nlane: ascii_downcase | trim | gsub("\\s+"; " ");
  def fam: split("-")[:3] | join("-");
  def fam_eq($a; $b): ($a | fam) == ($b | fam);
  def same($a; $b): $a == $b or fam_eq($a; $b) or ($a | contains($b)) or ($b | contains($a));
  def nes: type == "string" and (trim != "");
  def bad: if (.sha // "") != $head then "sha \(.sha // "null") != head \($head)"
    elif (.repo // "") != $repo then "repo \(.repo // "null") != \($repo)"
    elif (.pr // null) != $pr then "pr \(.pr // "null") != \($pr)"
    elif ((.author_model // "") | (type != "string" or ((nmodel) | . == "" or . == "unknown"))) then "author_model missing or unknown, independence unprovable"
    elif ([.. | objects | keys[] | select(test("waiv|overrid"; "i"))] | length) > 0 then "waiver/override key present"
    elif ([.. | strings | select(test("waiv|overrid"; "i"))] | length) > 0 then "waiver/override text in a value"
    elif ((.verdicts // null) | type) != "array" then "verdicts is not an array"
    else null end;
  .[0] as $r
  | ($r | bad) as $b
  | if $b != null then "BAD \($b)" else
      ($r.author_model | nmodel) as $au
      | ([ $r.verdicts[] | select(type == "object" and (.lane | nes) and .verdict != "PASS") | .lane | nlane ] | unique) as $dq
      | [ $r.verdicts[]
          | select(type == "object" and (.lane | nes) and (.model | nes) and .verdict == "PASS"
                   and (.blockers == null or .blockers == [])
                   and ((.lane | nlane) as $l | ($dq | index($l)) == null)
                   and (same(($au); (.model | nmodel)) | not))
          | {l: (.lane | nlane), m: (.model | nmodel)} ] as $q
      | ($q | map(.m) | unique) as $ms | ($q | map(.l) | unique) as $ls
      | if ($ms | length) >= 3 and ($ls | length) >= 3 then "OK \($ms | length) independent models over \($ls | length) lanes: \($ms | join(","))"
        else "BAD only \($ms | length) distinct models / \($ls | length) lanes qualify (need >= 3 each)" end
    end' "$file")" || fail "could not evaluate $file"

case "$verdict" in
  OK*) echo "PASS quorum receipt: $verdict ($file)" ;;
  *) fail "${verdict#BAD } ($file)" ;;
esac
# mutation: change `>= 3` to `>= 0`, drop the `same(...) | not` author clause, loosen the `.repo != $repo` check, or drop the author_model `unknown` test and the lane-floor, author-family, repo-mismatch and unknown-author tests in tests/falsification_quorum_gate_reads_the_receipt_store.rs and tests/falsification_quorum_gate_bypasses.rs go red
