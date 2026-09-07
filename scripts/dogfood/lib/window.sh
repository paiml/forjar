#!/usr/bin/env bash
# The release window: the set of PRs merged into main since the last tag that
# is reachable from HEAD.
#
# Gate A (scripts/dogfood/harness.sh) and gate E (scripts/dogfood/quorum.sh)
# ask the same question of GitHub and then check a different file per answer.
# The question lives here once, so that the two gates cannot drift into
# checking different windows and reporting the same word.
#
# SOURCED, NOT RUN. Sourcing this file defines functions and three constants
# and does nothing else: no output, no git, no gh. A helper with a side effect
# at source time would run before the caller's `fail()` exists, and would then
# die with no verdict line.
#
# EVERYTHING COMES BACK IN A GLOBAL, NEVER ON STDOUT.
#
# The obvious shape — `tag="$(dogfood_prev_tag)"` — is a trap here. `fail()`
# writes the verdict to stdout, and inside a command substitution that verdict
# is CAPTURED into the variable instead of printed; the subshell's `exit 1`
# then trips `set -e` in the caller, which dies silently with exit 1 and no
# output. That is indistinguishable from a gate that never ran, which is the
# one outcome this whole directory exists to refuse. So each function assigns
# to a DOGFOOD_* global and a failure inside it kills the real script, loudly.
#
# The caller must define `fail()` (printing its own `GATE <letter> FAIL …`)
# BEFORE sourcing this file.

# The GitHub client. Named here because it is a REQUIREMENT of both gates, not
# a convenience: the set of PRs merged since a tag is a fact only GitHub holds,
# so a gh that cannot answer leaves the window unmeasured, and an unmeasured
# window is a FAIL naming the tool — never a PASS over "no PRs found".
# Overridable only so the falsification tests in
# tests/falsification_dogfood_harness_and_quorum.rs can hand the gates a stub
# instead of the network.
GH="${GH:-gh}"

# The repository the PRs belong to, read the way scripts/dogfood/release-check.sh
# reads it: a constant, so that a gate pointed at a fork cannot report on this
# one.
REPO="paiml/forjar"

# The page the PR enumeration asks for. If GitHub fills it exactly, the window
# may have been truncated and neither gate knows the set it is checking.
PR_PAGE_LIMIT=200

# The newest v* tag reachable from HEAD -> DOGFOOD_PREV_TAG.
#
# Reachability, not recency: a tag on a branch this checkout does not contain
# says nothing about what is in this tree. No tag at all is UNMEASURED and not
# "everything", because an unbounded window would make both gates vacuous on
# the day the tag list fails to fetch.
dogfood_prev_tag() {
  local rc=0 tag
  tag="$(git tag --list 'v*' --sort=-v:refname --merged HEAD | head -1)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "git tag --merged HEAD exited ${rc} — the lower bound of the PR window cannot be read, so the window is UNMEASURED"
  fi
  if [ -z "$tag" ]; then
    fail "no v* tag is reachable from HEAD, so the set of merged PRs has no lower bound — UNMEASURED, and an unbounded window would make this gate vacuous"
  fi
  DOGFOOD_PREV_TAG="$tag"
}

# The commit date of DOGFOOD_PREV_TAG in UTC -> DOGFOOD_PREV_TAG_DATE.
#
# UTC and not local time: the string is handed to GitHub's `merged:>=` search,
# and a local-time bound silently moves the edge of the window by the offset,
# which drops or admits whatever merged inside it.
dogfood_prev_tag_date() {
  local rc=0 when
  when="$(TZ=UTC git log -1 --format=%cd --date=format-local:%Y-%m-%dT%H:%M:%SZ "$DOGFOOD_PREV_TAG")" || rc=$?
  if [ "$rc" -ne 0 ] || [ -z "$when" ]; then
    fail "cannot read the commit date of ${DOGFOOD_PREV_TAG} (git exit ${rc}), so the PR window has no lower bound — UNMEASURED"
  fi
  DOGFOOD_PREV_TAG_DATE="$when"
}

# The PRs GitHub reports as merged into main since DOGFOOD_PREV_TAG_DATE,
# narrowed to those actually inside this HEAD ->
# DOGFOOD_PR_JSON (a JSON array), DOGFOOD_PR_COUNT, DOGFOOD_PR_OUTSIDE.
#
# THE PR SET COMES FROM GITHUB, NOT FROM `git log --merges`. This repository
# squash-merges, and a squash merge leaves one ordinary commit with no second
# parent — so `git log --merges` enumerates nothing and every loop under it
# runs zero times while printing the words of a check that ran. That defect is
# the reason release-check.sh Arm 5 was rewritten (PMAT-178) and the reason
# these gates never re-derive the set locally.
#
# GitHub's window is a time range, so it also holds PRs merged after this
# tree's HEAD. Membership is decided by ancestry, on the merge commit GitHub
# names, in this checkout: those are reported as outside and not counted, which
# is a fact about the window and not a failure of it.
dogfood_merged_prs() {
  local rc=0 raw n jrc=0 i=0 num oid arc keep='[]'
  raw="$("$GH" pr list --repo "$REPO" --state merged --base main --search "merged:>=${DOGFOOD_PREV_TAG_DATE}" --limit "$PR_PAGE_LIMIT" --json number,mergedAt,mergeCommit,headRefName,title,body 2>&1)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "${GH} pr list exited ${rc} (${raw}) — the PRs merged since ${DOGFOOD_PREV_TAG} cannot be enumerated without GitHub, and an unenumerable window is UNMEASURED, which is not the same as a window with no PRs"
  fi
  n="$(printf '%s' "$raw" | jq -r 'if type == "array" then length else error("not an array") end')" || jrc=$?
  if [ "$jrc" -ne 0 ]; then
    fail "cannot parse what ${GH} pr list returned as a JSON array (jq exit ${jrc}): ${raw} — UNMEASURED"
  fi
  if [ "$n" -ge "$PR_PAGE_LIMIT" ]; then
    fail "${GH} pr list returned ${n} PR(s), which fills the --limit of ${PR_PAGE_LIMIT}: the window may be truncated and this gate would then check a subset while reporting the whole — UNMEASURED"
  fi

  DOGFOOD_PR_OUTSIDE=0
  DOGFOOD_PR_PREVIOUS=0
  while [ "$i" -lt "$n" ]; do
    num="$(printf '%s' "$raw" | jq -r ".[${i}].number // \"-\"")"
    oid="$(printf '%s' "$raw" | jq -r ".[${i}].mergeCommit.oid // \"-\"")"
    if [ "$oid" = "-" ] || [ -z "$oid" ]; then
      fail "GitHub reports PR #${num} merged since ${DOGFOOD_PREV_TAG} with no merge commit, so whether it is inside this HEAD cannot be decided — UNMEASURED"
    fi
    # The previous release's own PR is merged at the very second its tag is
    # cut, so `merged:>=<tag date>` returns it; its merge commit is an ancestor
    # of (or is) the tag, which puts it in the PREVIOUS window, not this one.
    prev_rc=0
    git merge-base --is-ancestor "$oid" "$DOGFOOD_PREV_TAG" >/dev/null 2>&1 || prev_rc=$?
    if [ "$prev_rc" -eq 0 ]; then
      echo "  #${num} ${oid} is inside ${DOGFOOD_PREV_TAG} (the previous release) — not counted"
      DOGFOOD_PR_PREVIOUS=$((DOGFOOD_PR_PREVIOUS + 1))
      i=$((i + 1))
      continue
    fi
    if [ "$prev_rc" -ne 1 ]; then
      fail "git merge-base --is-ancestor ${oid} ${DOGFOOD_PREV_TAG} exited ${prev_rc} for PR #${num}: the commit GitHub names is not in this checkout (run: git fetch origin) — UNMEASURED"
    fi
    arc=0
    git merge-base --is-ancestor "$oid" HEAD >/dev/null 2>&1 || arc=$?
    case "$arc" in
      0) keep="$(printf '%s' "$keep" | jq -c --arg o "$oid" '. + [$o]')" ;;
      1)
        echo "  #${num} ${oid} is outside this HEAD (merged after it) — not counted"
        DOGFOOD_PR_OUTSIDE=$((DOGFOOD_PR_OUTSIDE + 1))
        ;;
      *) fail "git merge-base --is-ancestor ${oid} HEAD exited ${arc} for PR #${num}: the commit GitHub names is not in this checkout (run: git fetch origin), so membership of this window is UNMEASURED" ;;
    esac
    i=$((i + 1))
  done

  DOGFOOD_PR_JSON="$(printf '%s' "$raw" | jq -c --argjson keep "$keep" '[ .[] | select((.mergeCommit.oid // "-") as $o | $keep | index($o) != null) ]')"
  DOGFOOD_PR_COUNT="$(printf '%s' "$DOGFOOD_PR_JSON" | jq -r 'length')"
}

# The three steps in the only order they work in.
dogfood_load_window() {
  dogfood_prev_tag
  dogfood_prev_tag_date
  dogfood_merged_prs
}

# One field of one PR in the window -> DOGFOOD_FIELD.
#
# $1 is the index, $2 a jq path. Absent or null reads back as the empty string,
# so a caller decides what a missing field means rather than inheriting jq's
# "null".
dogfood_pr_field() {
  local rc=0 v
  v="$(printf '%s' "$DOGFOOD_PR_JSON" | jq -r "(.[${1}]${2}) // \"\"")" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "cannot read ${2} of PR index ${1} out of the window GitHub returned (jq exit ${rc}) — UNMEASURED"
  fi
  DOGFOOD_FIELD="$v"
}

# The receipt slug of a head branch -> DOGFOOD_SLUG.
#
# Every `/` becomes `-`, which is the rewrite scripts/quorum-gate.sh applies
# when it names `.quorum/<slug>.json` at push time. A gate that spelled the
# slug differently would look for a file nothing writes and report every PR as
# missing its receipt.
dogfood_slug() {
  DOGFOOD_SLUG="${1//\//-}"
}
