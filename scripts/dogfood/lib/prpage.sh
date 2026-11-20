#!/usr/bin/env bash
# PMAT-1164: read a release window past one 200-PR page by bisecting full slices; a read that cannot finish is UNMEASURED,
# never a partial result. SOURCED by lib/window.sh; the time bound only limits what is READ, membership stays ancestry.
# mutation: change `merged:${DOGFOOD_PR_A}..${DOGFOOD_PR_B}` to
#   `merged:${DOGFOOD_PR_A}..${DOGFOOD_PR_A}` and the 450-PR window reads red.
PR_CALL_CAP=40

# One commit-time epoch: `git log -1 --format=%ct REF` -> DOGFOOD_PR_EPOCH.
dogfood_pr_epoch() {
  local rc=0 e
  e="$(git log -1 --format=%ct "$1" 2>/dev/null)" || rc=$?
  case "$e" in '' | *[!0-9]*) rc=1 ;; esac
  if [ "$rc" -ne 0 ] || [ "$e" -le 0 ]; then
    fail "cannot read the commit time of ${1} (git exit ${rc}), so the PR window has no bound — UNMEASURED"
  fi
  DOGFOOD_PR_EPOCH="$e"
}

dogfood_pr_slice() {
  local a="$1" b="$2" fields="$3" rc=0 raw n m saved covered
  DOGFOOD_PR_CALLS=$((DOGFOOD_PR_CALLS + 1))
  if [ "$DOGFOOD_PR_CALLS" -gt "$PR_CALL_CAP" ]; then
    fail "the PR window needed more than ${PR_CALL_CAP} ${GH} calls to read — UNMEASURED"
  fi
  DOGFOOD_PR_A="$(dogfood_pr_ts_of "$a")" || fail "cannot format window bounds — UNMEASURED"
  DOGFOOD_PR_B="$(dogfood_pr_ts_of "$b")" || fail "cannot format window bounds — UNMEASURED"
  raw="$("$GH" pr list --repo "$REPO" --state merged --base main --search "merged:${DOGFOOD_PR_A}..${DOGFOOD_PR_B}" --limit "$PR_PAGE_LIMIT" --json "$fields" 2>&1)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "${GH} pr list exited ${rc} (${raw}) — the PRs merged ${DOGFOOD_PR_A}..${DOGFOOD_PR_B} cannot be enumerated without GitHub, and an unenumerable window is UNMEASURED, which is not the same as a window with no PRs"
  fi
  rc=0
  # PMAT-1173: the page must be ONE array of objects with a numeric `number`, else UNMEASURED.
  n="$(printf '%s' "$raw" | jq -rs 'if length == 1 and (.[0] | type == "array") and (.[0] | all(.[]; type == "object" and (.number | type) == "number")) then (.[0] | length) else error("not one array of PR objects") end')" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "cannot read what ${GH} pr list returned as one JSON array of PRs (jq exit ${rc}): ${raw} — UNMEASURED"
  fi
  if [ "$n" -lt "$PR_PAGE_LIMIT" ]; then
    DOGFOOD_PR_ACC="$(printf '%s\n%s' "$DOGFOOD_PR_ACC" "$raw" | jq -cs 'add | unique_by(.number)')"
    return 0
  fi
  if [ $((b - a)) -le 1 ]; then
    fail "${GH} pr list filled its ${PR_PAGE_LIMIT}-PR page for a single second (${DOGFOOD_PR_A}) and cannot be split further — UNMEASURED"
  fi
  m=$(((a + b) / 2))
  saved="$DOGFOOD_PR_ACC"
  DOGFOOD_PR_ACC='[]'
  dogfood_pr_slice "$a" "$m" "$fields"
  dogfood_pr_slice "$m" "$b" "$fields"
  covered="$(printf '%s\n%s' "$raw" "$DOGFOOD_PR_ACC" | jq -s '(.[0] | map(.number)) - (.[1] | map(.number)) | length == 0')" || covered=false
  if [ "$covered" != true ]; then
    fail "a full ${PR_PAGE_LIMIT}-PR page lists a PR its two halves do not (a short half is a partial page) — UNMEASURED"
  fi
  DOGFOOD_PR_ACC="$(printf '%s\n%s' "$saved" "$DOGFOOD_PR_ACC" | jq -cs 'add | unique_by(.number)')"
}

dogfood_pr_ts_of() {
  date -u -d "@$1" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null
}

# LOWER UPPER FIELDS -> DOGFOOD_PR_RAW (JSON array, every PR merged from
# LOWER's commit time to the latest commit time in LOWER..UPPER plus one day).
# That upper bound is >= every merge's mergedAt: a squash merge's mergedAt is
# its committer date (+~1 s), and the max also covers a late-dated ancestor.
# An empty LOWER..UPPER range still bounds on UPPER's own commit time.
dogfood_pr_fetch() {
  local lower="$1" upper="$2" fields="$3" rc=0 cts to from
  dogfood_pr_epoch "$lower"
  from="$DOGFOOD_PR_EPOCH"
  dogfood_pr_epoch "$upper"
  to="$DOGFOOD_PR_EPOCH"
  cts="$(git log --format=%ct "${lower}..${upper}" 2>/dev/null)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "cannot list the commits in ${lower}..${upper} (git exit ${rc}), so the PR window has no upper bound — UNMEASURED"
  fi
  to="$(printf '%s\n%s\n' "$to" "$cts" | awk 'NF && $1+0 > m { m = $1+0 } END { print m + 86400 }')"
  if [ "$to" -lt "$from" ]; then
    to="$from"
  fi
  DOGFOOD_PR_CALLS=0
  DOGFOOD_PR_ACC='[]'
  dogfood_pr_slice "$from" "$to" "$fields"
  DOGFOOD_PR_RAW="$DOGFOOD_PR_ACC"
}
