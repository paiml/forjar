#!/usr/bin/env bash
# quorum-fleet-receipt.sh <repo> <pr> <head-sha>
#
# The C48 LOW bar for the `quorum receipt` CI check (Noah ruling C58 item 3): green on ONE PASS from a
# model that is not the author's, bound to exactly this PR head. The fleet's review verdict is a receipt
# OUTSIDE the tree, <root>/<repo>/<pr>/<sha>.json (the schema scripts/arm-reviewed reads):
#   {"repo":"paiml/forjar","pr":123,"sha":"<40-hex>","author_model":"<id>",
#    "verdicts":[{"lane":"<name>","model":"<id>","verdict":"PASS|FAIL|BLOCKER","blockers":[...]}]}
# It cannot live in the tree: committing it would change the head it is bound to.
#
# Exit 0 only when: the file for THIS sha exists and parses, its .sha/.repo/.pr match the arguments,
# no verdict is FAIL or BLOCKER, no verdict has non-empty blockers, and at least one PASS comes from a
# model != author_model. Anything else (no receipt, stale head, malformed) exits 1 with the reason:
# UNMEASURED IS A FAILURE, NOT A PASS. The HIGH tier (one Opus pass) is the reviewer's call; this
# script does not decide tiers. The legacy .quorum/<branch>.json gate stays as quorum-gate.sh.
set -euo pipefail

[ "$#" -eq 3 ] || { echo "usage: quorum-fleet-receipt.sh <repo> <pr> <head-sha>" >&2; exit 2; }
repo="$1"; pr="$2"; sha="$3"
root="${QUORUM_FLEET_ROOT:-/mnt/nvme-raid0/cop-inbox/promote-state/quorum}"

refuse() { echo "✗ fleet quorum receipt: $1" >&2; exit 1; }
[[ "$sha" =~ ^[0-9a-f]{40}$ ]] || refuse "head sha '$sha' is not 40-hex"
command -v jq >/dev/null 2>&1 || refuse "jq not installed (unmeasured)"
receipt="$root/$repo/$pr/$sha.json"
[ -f "$receipt" ] || refuse "no receipt at $receipt (a PASS for an older head does not count)"
jq -e . <"$receipt" >/dev/null 2>&1 || refuse "receipt is not valid JSON"

IFS= read -r -d '' prog <<'JQ' || true
  def okstr: type == "string" and length > 0;
  if type != "object" then "receipt not an object"
  elif (.sha | type) != "string" or .sha != $sha then "receipt sha does not match the head"
  elif (.repo | type) != "string" or .repo != $repo then "receipt repo mismatch"
  elif ((.pr | tostring) != $pr) or ((.pr | type) != "number" and (.pr | type) != "string") then "receipt pr mismatch"
  elif (.author_model | okstr | not) then "author_model missing"
  elif (.verdicts | type) != "array" then "verdicts missing"
  elif (.verdicts | all(type == "object") | not) then "malformed verdict entry"
  elif (.verdicts | all((.verdict | IN("PASS","FAIL","BLOCKER")) and (.model | okstr)) | not) then "verdict/model invalid"
  elif (.verdicts | any(.verdict == "BLOCKER" or .verdict == "FAIL")) then "FAIL or BLOCKER verdict present"
  elif (.verdicts | any(((.blockers // []) | type) != "array" or ((.blockers // []) | length) > 0)) then "non-empty blockers"
  else
    .author_model as $a
    | ([.verdicts[] | select(.verdict == "PASS" and .model != $a) | .model] | unique | length) as $n
    | if $n >= 1 then "OK" else "need 1 PASS from a non-author model, have \($n)" end
  end
JQ
decision="$(jq -r --arg repo "$repo" --arg pr "$pr" --arg sha "$sha" "$prog" <"$receipt")" || refuse "jq evaluation failed"
[ "$decision" = OK ] || refuse "$decision"
echo "✓ fleet quorum receipt: >=1 non-author PASS at $sha ($receipt)"
