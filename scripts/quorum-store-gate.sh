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
#   - author: NOT taken from the receipt. Derived from the PR commits: EVERY non-merge
#     commit in PR_BASE_SHA..HEAD needs >= 1 Co-Authored-By trailer, each line anchored
#     ^Co-Authored-By: (.+) <[^<>]+>$ with exactly one '<', mapped to an id by the exact
#     allowlist (.authors). No trailer / malformed / unmapped = FAIL. The receipt's
#     author_model must equal one of those ids; a lane whose model id is any commit
#     author id does not count. PR_AUTHOR_MODELS exists ONLY for tests: it is honored
#     only with QUORUM_TEST_MODE=1 and never when GITHUB_ACTIONS is set.
#   - receipt text: ASCII only (any byte >= 0x80 or any \u escape = FAIL);
#     waiver scan (waiv|overrid|bypass|skip_quorum, case-insens.) over the raw
#     text; exactly one JSON document; no duplicate keys at any depth (leaf repeat,
#     leaf-vs-nested, container reopened); exactly one "pr": token and it is the
#     exact integer literal; schema: exact keys per verdict entry (lane, model,
#     verdict, blockers); blockers an array; lane names [a-z0-9_-]+; sha 40
#     lowercase hex. A lane name seen twice is disqualified; a model on two entries
#     is disqualified. PASS needs >= 3 qualifying lanes over >= 3 distinct model ids.
#
# JUDGED FROM BASE: .github/workflows/quorum.yml runs THIS script and
# quorum-models.json as read from the PR BASE commit (git show $PR_BASE_SHA:...),
# never the PR's copy, so a PR cannot weaken its own judge. BOOTSTRAP: the very
# first merge of this gate has no base copy (the job fails closed), so the gate on
# the PR that introduces it cannot be judged from base; that merge is a one-off
# that needs the repo owner's decision. The workflow file itself on a
# pull_request event is also the PR's copy; pinning it needs a ruleset
# required-workflow, outside this script.
#
# IRREDUCIBLE RESIDUAL (NOT closed by this gate): Co-Authored-By trailers and
# receipt contents are written by the author/seat, so author-identity spoofing
# (omit or replace one's own trailer) needs a harness-stamped attestation.
#
# Inputs (env): PR_NUMBER, PR_HEAD_SHA, PR_BASE_SHA (required); QUORUM_REPO
# (default paiml/forjar); QUORUM_STORE.
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
# Every non-merge commit in PR_BASE_SHA..head must carry >= 1 Co-Authored-By
# trailer, each line anchored `^Co-Authored-By: (.+) <[^<>]+>$` with exactly one
# '<', whose name maps EXACTLY through the allowlist. Anything else = FAIL.
trailer_re='^Co-Authored-By: (.+) <[^<>]+>$'
ids=""
check_trailers() { # $1 = trailer lines, $2 = label
  local line name id n=0
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    [[ "$line" =~ $trailer_re ]] || fail "$2: malformed Co-Authored-By trailer '$line'"
    [ "$(printf '%s' "$line" | tr -cd '<' | wc -c)" -eq 1 ] || fail "$2: more than one '<' in trailer '$line'"
    name="${BASH_REMATCH[1]}"
    id="$(jq -er --arg n "$name" '.authors[$n]' "$allow" 2>/dev/null)" || fail "$2: unmapped author '$name'"
    ids+="$id"$'\n'
    n=$((n + 1))
  done <<<"$1"
  [ "$n" -ge 1 ] || fail "$2: no mapped Co-Authored-By trailer"
}
if [ "${QUORUM_TEST_MODE:-}" = "1" ] && [ -z "${GITHUB_ACTIONS:-}" ] && [ -n "${PR_AUTHOR_MODELS:-}" ]; then
  # tests only: PR_AUTHOR_MODELS is ignored whenever GITHUB_ACTIONS is set
  check_trailers "$(printf '%s\n' "$PR_AUTHOR_MODELS" | sed 's/^./Co-Authored-By: &/')" "PR_AUTHOR_MODELS"
elif [ -n "${PR_BASE_SHA:-}" ]; then
  [[ "$PR_BASE_SHA" =~ ^[0-9a-f]{40}$ ]] || fail "PR_BASE_SHA is not a full 40-hex sha"
  commits="$(git rev-list --no-merges "$PR_BASE_SHA..$head")" \
    || fail "git rev-list $PR_BASE_SHA..$head failed (UNMEASURED)"
  [ -n "$commits" ] || fail "no non-merge commits in $PR_BASE_SHA..$head (UNMEASURED)"
  while IFS= read -r c; do
    t="$(git log -1 --format='%(trailers:key=Co-Authored-By,separator=%x0A)' "$c")" \
      || fail "git log $c failed (UNMEASURED)"
    check_trailers "$t" "commit $c"
  done <<<"$commits"
else
  fail "PR_BASE_SHA is not set: author unprovable (UNMEASURED)"
fi
authors="$(printf '%s' "$ids" | jq -R . | jq -s -c 'unique')" || fail "could not build author list"

[ -d "$store" ] || fail "receipt store $store is missing (UNMEASURED)"
file="$store/quorum/$repo/$pr/$head.json"
[ ! -L "$file" ] || fail "$file is a symlink (refused)"
[ -f "$file" ] || fail "no receipt at $file for head $head"

# ---- raw-text checks ----
if LC_ALL=C grep -qP '[^\x00-\x7F]' "$file"; then fail "non-ASCII byte in $file"; fi
if grep -qF '\u' "$file"; then fail "\\u escape in $file (ASCII-only receipts)"; fi
if grep -qiE 'waiv|overrid|bypass|skip_?quorum' "$file"; then fail "waiver/override text in $file"; fi
# the ONE `"pr":` key token in the raw text must be the exact integer literal (a nested
# decoy `"pr":N` makes the count 2 and fails). -z: a pretty-printed comma on the next line is fine.
[ "$(grep -Pzo '"pr"\s*:' "$file" | tr -cd '\0' | wc -c)" -eq 1 ] || fail "\"pr\" key token must appear exactly once in $file"
grep -Pzq '"pr"\s*:\s*'"$pr"'\s*[,}]' "$file" || fail "pr is not the integer literal $pr in $file"

jq -s -e 'length == 1 and (.[0] | type == "object")' "$file" >/dev/null 2>&1 \
  || fail "$file is not exactly one JSON object document"

# duplicate keys at ANY depth. In the --stream event list: (a) a leaf path seen twice, (b) a leaf
# path that is a prefix of another (scalar/empty-container vs nested), (c) a container reopened
# after its closing event (object/array valued duplicate keys with different shapes).
dups="$(jq -n --stream -r '[inputs] as $e
  | [$e[] | select(length == 2) | .[0]] as $p
  | [ $p[] as $a | $p[] as $b | select(($a | length) < ($b | length) and $b[:($a | length)] == $a) ] as $pref
  | [ range(0; $e | length) as $i | $e[$i] | select(length == 1) | .[0] as $q | ($q | length) as $n
      | select($n > 1 and any($e[$i + 1:][]; (.[0] | length) >= $n and .[0][:$n - 1] == $q[:$n - 1])) ] as $reopen
  | if ($p | length) != ($p | unique | length) or ($pref | length) > 0 or ($reopen | length) > 0 then "dup" else "ok" end' "$file" 2>/dev/null)" \
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
    elif (.pr | type) != "number" or (.pr | . != floor) or .pr != $pr then "pr mismatch"
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
