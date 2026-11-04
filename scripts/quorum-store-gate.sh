#!/usr/bin/env bash
# Quorum receipt gate (CI): reads the receipt STORE, not a committed file.
#
# WHY: a receipt binds to the PR head sha. Committing `.quorum/<branch>.json`
# moves the head, so the committed receipt can never match the head it was
# written for (a loop). The receipt therefore lives outside the tree:
#   ${QUORUM_STORE:-/mnt/nvme-raid0/cop-inbox/promote-state}/quorum/<repo>/<pr>/<headsha>.json
#   schema: {repo, pr, sha, author_model, verdicts:[{lane, model, verdict, blockers}]}
#
# DESIGN (PMAT-554 round 3): the receipt is UNTRUSTED. No string normalization
# anywhere (that was an arms race); every identity is an EXACT byte match.
#   - models: a lane model counts only if it is a key of scripts/quorum-models.json
#     (exact, ASCII). Anything else disqualifies that lane.
#   - author: NOT taken from the receipt. It is derived from the PR commits'
#     Co-Authored-By trailers (env PR_AUTHOR_MODELS, newline-separated names, or
#     computed from `git log $PR_BASE_SHA..$PR_HEAD_SHA`), mapped to ids by the
#     same allowlist file. No trailer / an unmapped name = FAIL. The receipt's
#     author_model must equal one of those ids. A lane whose model id is any
#     commit author id does not count (same id never).
#   - receipt text: ASCII only (any byte >= 0x80 or any \u escape = FAIL);
#     waiver scan (waiv|overrid|bypass|skip_quorum, case-insens.) over the raw
#     text, i.e. keys and values; exactly one JSON document; no duplicate keys.
#   - schema: exact keys per verdict entry (lane, model, verdict, blockers);
#     blockers an array; lane names [a-z0-9_-]+; sha 40 lowercase hex; pr a JSON
#     integer literal. A lane name seen twice (any verdict) is disqualified; a
#     model on two entries is disqualified (lane<->model 1:1). PASS needs >= 3
#     qualifying lanes over >= 3 distinct model ids.
#
# Inputs (env): PR_NUMBER, PR_HEAD_SHA (required); PR_AUTHOR_MODELS or
# PR_BASE_SHA (one required); QUORUM_REPO (default paiml/forjar); QUORUM_STORE.
#
# NOT TOUCHED: scripts/quorum-gate.sh (local pre-push hook) and
# scripts/dogfood/{quorum,release-check}.sh (gates E / R).
set -euo pipefail

fail() { echo "FAIL quorum receipt: $*" >&2; exit 1; }

repo="${QUORUM_REPO:-paiml/forjar}"
store="${QUORUM_STORE:-/mnt/nvme-raid0/cop-inbox/promote-state}"
pr="${PR_NUMBER:-}"
head="${PR_HEAD_SHA:-}"
allow="$(dirname "${BASH_SOURCE[0]}")/quorum-models.json"

[ -n "$pr" ] || fail "PR_NUMBER is not set (UNMEASURED)"
[ -n "$head" ] || fail "PR_HEAD_SHA is not set (UNMEASURED)"
[[ "$pr" =~ ^[0-9]+$ ]] || fail "PR_NUMBER '$pr' is not a number"
[[ "$head" =~ ^[0-9a-f]{40}$ ]] || fail "PR_HEAD_SHA '$head' is not a full 40-hex sha"
command -v jq >/dev/null || fail "jq not installed (UNMEASURED)"
[ -f "$allow" ] || fail "allowlist $allow missing (UNMEASURED)"
[[ "$repo" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || fail "QUORUM_REPO '$repo' is not owner/name"
[[ "$repo" != *..* ]] || fail "QUORUM_REPO '$repo' contains '..'"

# ---- authors, from the commits (never from the receipt) ----
if [ -n "${PR_AUTHOR_MODELS:-}" ]; then
  raw_authors="$PR_AUTHOR_MODELS"
elif [ -n "${PR_BASE_SHA:-}" ]; then
  [[ "$PR_BASE_SHA" =~ ^[0-9a-f]{40}$ ]] || fail "PR_BASE_SHA is not a full 40-hex sha"
  raw_authors="$(git log --format='%(trailers:key=Co-Authored-By,valueonly,separator=%x0A)' "$PR_BASE_SHA..$head")" \
    || fail "git log $PR_BASE_SHA..$head failed (UNMEASURED)"
else
  fail "neither PR_AUTHOR_MODELS nor PR_BASE_SHA is set: author unprovable (UNMEASURED)"
fi
authors="$(jq -n -c --slurpfile al "$allow" --arg raw "$raw_authors" '
  [ $raw | split("\n")[] | select(length > 0) | sub(" <.*$"; "") ] as $names
  | if ($names | length) == 0 then error("no Co-Authored-By trailer")
    else [ $names[] | . as $n | ($al[0].authors | if has($n) then .[$n] else error("unmapped author \($n)") end) ] | unique end' 2>&1)" \
  || fail "author models not derivable from commit trailers: $authors"

[ -d "$store" ] || fail "receipt store $store is missing (UNMEASURED)"
file="$store/quorum/$repo/$pr/$head.json"
[ ! -L "$file" ] || fail "$file is a symlink (refused)"
[ -f "$file" ] || fail "no receipt at $file for head $head"

# ---- raw-text checks ----
if LC_ALL=C grep -qP '[^\x00-\x7F]' "$file"; then fail "non-ASCII byte in $file"; fi
if grep -qF '\u' "$file"; then fail "\\u escape in $file (ASCII-only receipts)"; fi
if grep -qiE 'waiv|overrid|bypass|skip_?quorum' "$file"; then fail "waiver/override text in $file"; fi
grep -Pq '"pr"\s*:\s*'"$pr"'\s*[,}]' "$file" || fail "pr is not the integer literal $pr in $file"

jq -s -e 'length == 1 and (.[0] | type == "object")' "$file" >/dev/null 2>&1 \
  || fail "$file is not exactly one JSON object document"

dups="$(jq -n --stream -r '[inputs | select(length == 2) | .[0]] as $p
  | [ $p[] as $a | $p[] as $b | select(($a | length) < ($b | length) and $b[:($a | length)] == $a) ] as $pref
  | if ($p | length) != ($p | unique | length) or ($pref | length) > 0 then "dup" else "ok" end' "$file" 2>/dev/null)" \
  || fail "could not stream-parse $file"
[ "$dups" = "ok" ] || fail "duplicate JSON object keys in $file"

n="$(jq -s '.[0].verdicts | length' "$file" 2>/dev/null)" || fail "verdicts missing in $file"
for k in lane model verdict blockers; do
  c="$(grep -o "\"$k\"" "$file" | wc -l)"
  [ "$c" -eq "$n" ] || fail "token \"$k\" appears $c times for $n verdicts (duplicate key)"
done

verdict="$(jq -s -r --arg repo "$repo" --argjson pr "$pr" --arg head "$head" \
  --slurpfile al "$allow" --argjson authors "$authors" '
  def want: ["blockers","lane","model","verdict"];
  def isin($a): . as $x | any($a[]; . == $x);
  def bad: if (.sha | type) != "string" or (.sha | test("^[0-9a-f]{40}$") | not) or .sha != $head then "sha is not the head sha"
    elif .repo != $repo then "repo mismatch"
    elif (.pr | type) != "number" or .pr != $pr then "pr mismatch"
    elif (.author_model | type) != "string" or (.author_model | isin($authors) | not) then "author_model is not a commit-trailer author"
    elif (.verdicts | type) != "array" then "verdicts is not an array"
    elif ([.verdicts[] | select(type != "object" or (keys != want)
           or (.lane | type) != "string" or (.lane | test("^[a-z0-9_-]+$") | not)
           or (.model | type) != "string" or (.verdict | type) != "string"
           or (.blockers | type) != "array")] | length) > 0 then "malformed verdict entry (exact keys lane/model/verdict/blockers, blockers array, lane [a-z0-9_-]+)"
    else null end;
  .[0] as $r
  | ($r | bad) as $b
  | if $b != null then "BAD \($b)" else
      $r.verdicts as $v
      | ($v | map(.lane) | group_by(.) | map(select(length > 1) | .[0])) as $duplane
      | ($v | map(.model) | group_by(.) | map(select(length > 1) | .[0])) as $dupmodel
      | [ $v[] | select(.verdict == "PASS" and .blockers == []
                        and (.lane | isin($duplane) | not)
                        and (.model | isin($dupmodel) | not)
                        and (.model as $m | $al[0].models | has($m))
                        and (.model | isin($authors) | not)) ] as $q
      | ($q | map(.model)) as $ms
      | if ($ms | length) >= 3 then "OK \($ms | length) independent models/lanes: \($ms | join(","))"
        else "BAD only \($ms | length) qualifying lanes (need >= 3 distinct lanes and models, none the commit author)" end
    end' "$file")" || fail "could not evaluate $file"

case "$verdict" in
  OK*) echo "PASS quorum receipt: $verdict ($file)" ;;
  *) fail "${verdict#BAD } ($file)" ;;
esac
# mutation: change `>= 3` to `>= 0`, drop the `isin($authors)` lane clause, drop the `has(.model)` allowlist clause, or loosen the `.repo != $repo` check and the tests in tests/falsification_quorum_gate_reads_the_receipt_store.rs and tests/falsification_quorum_gate_bypasses.rs go red
