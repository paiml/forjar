#!/usr/bin/env bash
# The head-bound store receipt check (forjar#876). It is ADDED on top of the
# committed `.quorum/<slug>.json` checks of gates E and R, never instead of
# them: a PR passes only if the committed-file check passes AND this passes.
#
#   dogfood_store_require          fails (UNMEASURED) when QUORUM_STORE is unset
#                                  or empty; call once, before any PR is checked.
#   dogfood_store_check PR HEAD MERGE
#                                  sets DOGFOOD_STORE_TOKEN (ok|missing|waived|
#                                  thin) and DOGFOOD_STORE_WHY. Anything but `ok`
#                                  is a refusal. Never fetches anything.
#
# Step 1 runs the UNCHANGED scripts/quorum-store-gate.sh, read from HEAD with
# its allowlist, on `$QUORUM_STORE/quorum/$REPO/<pr>/<headRefOid>.json`; step 2,
# only if step 1 passed, applies the UNCHANGED DOGFOOD_RECEIPT_JQ floors to that
# same file. The test-only overrides the store gate honours are unset here.
# shellcheck source=scripts/dogfood/lib/receipt.sh
. "$(dirname "${BASH_SOURCE[0]}")/receipt.sh"

dogfood_store_require() {
  if [ -z "${QUORUM_STORE:-}" ]; then
    fail "QUORUM_STORE is unset or empty, so the head-bound store receipts cannot be read — UNMEASURED (export QUORUM_STORE; the gate's built-in default is never read)"
  fi
}

dogfood_store_check() {
  local pr="$1" head="$2" merge="$3" base tmp out rc=0 file why
  DOGFOOD_STORE_TOKEN=missing
  DOGFOOD_STORE_WHY=""
  if ! [[ "$head" =~ ^[0-9a-f]{40}$ ]] || ! git cat-file -e "${head}^{commit}" 2>/dev/null; then
    DOGFOOD_STORE_WHY="headRefOid '${head}' of PR #${pr} is missing, not 40 lowercase hex, or not in this checkout (run: git fetch origin '+refs/pull/*/head:refs/remotes/origin/pr/*')"
    return 0
  fi
  base="$(git merge-base "${merge}^1" "$head" 2>/dev/null)" || {
    DOGFOOD_STORE_WHY="no merge base between ${merge}^1 and ${head} for PR #${pr}"
    return 0
  }
  tmp="$(mktemp -d)"
  if ! { git show HEAD:scripts/quorum-store-gate.sh >"${tmp:?}/quorum-store-gate.sh" 2>/dev/null \
    && git show HEAD:scripts/quorum-models.json >"${tmp:?}/quorum-models.json" 2>/dev/null; }; then
    rm -rf "${tmp:?}"
    DOGFOOD_STORE_WHY="scripts/quorum-store-gate.sh or quorum-models.json is not committed at HEAD"
    return 0
  fi
  out="$(env -u QUORUM_TEST_MODE -u PR_AUTHOR_MODELS QUORUM_REPO="$REPO" QUORUM_STORE="$QUORUM_STORE" \
    PR_NUMBER="$pr" PR_HEAD_SHA="$head" PR_BASE_SHA="$base" bash "${tmp:?}/quorum-store-gate.sh" 2>&1)" || rc=$?
  rm -rf "${tmp:?}"
  if [ "$rc" -ne 0 ]; then
    DOGFOOD_STORE_WHY="store gate refused PR #${pr}: ${out}"
    return 0
  fi
  file="$QUORUM_STORE/quorum/$REPO/$pr/$head.json"
  why="$(dogfood_receipt_status "$(cat "$file")")"
  DOGFOOD_STORE_WHY="$why"
  case "$why" in
    ok) DOGFOOD_STORE_TOKEN=ok ;;
    *waiver*) DOGFOOD_STORE_TOKEN=waived ;;
    *) DOGFOOD_STORE_TOKEN=thin ;;
  esac
  return 0
}
