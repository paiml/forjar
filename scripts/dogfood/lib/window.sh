#!/usr/bin/env bash
# The release window: the set of PRs merged into main since the last tag that
# is reachable from HEAD — and, since PMAT-225, the window between ANY two
# refs, and the ticket(s) a PR names.
#
# Gate A (scripts/dogfood/harness.sh) and gate E (scripts/dogfood/quorum.sh)
# ask the same question of GitHub and then check a different file per answer.
# Gate T (scripts/dogfood/tagged.sh) asks it once per tagged release. The
# question lives here once, so that the gates cannot drift into checking
# different windows and reporting the same word.
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

# The GitHub client. Named here because it is a REQUIREMENT of the gates, not
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
# may have been truncated and no gate knows the set it is checking.
PR_PAGE_LIMIT=200

# The newest v* tag reachable from HEAD -> DOGFOOD_PREV_TAG.
#
# Reachability, not recency: a tag on a branch this checkout does not contain
# says nothing about what is in this tree. No tag at all is UNMEASURED and not
# "everything", because an unbounded window would make both gates vacuous on
# the day the tag list fails to fetch.
dogfood_prev_tag() {
  local rc=0 tag
  # PMAT-239: no pipe. `git tag … | head -1` has head exit after one line,
  # git take SIGPIPE, and pipefail report 141 — the same defect that made this
  # gate call a readable registry UNMEASURED at random. Capture, then take the
  # first line with parameter expansion: one process, no signal.
  local tags
  tags="$(git tag --list 'v*' --sort=-v:refname --merged HEAD)" || rc=$?
  tag="${tags%%$'\n'*}"
  if [ "$rc" -ne 0 ]; then
    fail "git tag --merged HEAD exited ${rc} — the lower bound of the PR window cannot be read, so the window is UNMEASURED"
  fi
  if [ -z "$tag" ]; then
    fail "no v* tag is reachable from HEAD, so the set of merged PRs has no lower bound — UNMEASURED, and an unbounded window would make this gate vacuous"
  fi
  DOGFOOD_PREV_TAG="$tag"
}

# The commit date of REF in UTC -> DOGFOOD_REF_DATE.
#
# UTC and not local time: the string is handed to GitHub's `merged:>=` search,
# and a local-time bound silently moves the edge of the window by the offset,
# which drops or admits whatever merged inside it.
dogfood_ref_date() {
  local rc=0 when
  when="$(TZ=UTC git log -1 --format=%cd --date=format-local:%Y-%m-%dT%H:%M:%SZ "$1")" || rc=$?
  if [ "$rc" -ne 0 ] || [ -z "$when" ]; then
    fail "cannot read the commit date of ${1} (git exit ${rc}), so the PR window has no lower bound — UNMEASURED"
  fi
  DOGFOOD_REF_DATE="$when"
}

# The commit date of DOGFOOD_PREV_TAG in UTC -> DOGFOOD_PREV_TAG_DATE.
dogfood_prev_tag_date() {
  dogfood_ref_date "$DOGFOOD_PREV_TAG"
  DOGFOOD_PREV_TAG_DATE="$DOGFOOD_REF_DATE"
}

# The moment TAG was CUT, in UTC -> DOGFOOD_TAG_DATE.
#
# `creatordate` is the tagger date of an annotated tag and the committer date
# of a lightweight one. The release cadence (gate T) counts from here, not
# from the commit date used above: a release commit can sit unreleased for a
# day before anyone tags it, and the window bound is the commit while the
# release clock is the tag. (Refuted by the PMAT-225 plan grill: measured
# against the commit date, a Friday merge tagged on Monday would be overdue at
# the moment it was cut.)
dogfood_tag_date() {
  local rc=0 when
  when="$(TZ=UTC git for-each-ref --format='%(creatordate:format-local:%Y-%m-%dT%H:%M:%SZ)' "refs/tags/$1")" || rc=$?
  if [ "$rc" -ne 0 ] || [ -z "$when" ]; then
    fail "cannot read when ${1} was cut (git for-each-ref exit ${rc}, or no such tag): the release clock has no origin — UNMEASURED"
  fi
  DOGFOOD_TAG_DATE="$when"
}

# The PRs GitHub reports as merged into main since LOWER's commit date,
# narrowed to those inside UPPER and not inside LOWER ->
# DOGFOOD_PR_JSON (a JSON array), DOGFOOD_PR_COUNT, DOGFOOD_PR_OUTSIDE,
# DOGFOOD_PR_PREVIOUS.
#
# THE PR SET COMES FROM GITHUB, NOT FROM `git log --merges`. This repository
# squash-merges, and a squash merge leaves one ordinary commit with no second
# parent — so `git log --merges` enumerates nothing and every loop under it
# runs zero times while printing the words of a check that ran. That defect is
# the reason release-check.sh Arm 5 was rewritten (PMAT-178) and the reason
# these gates never re-derive the set locally.
#
# MEMBERSHIP IS ANCESTRY, NEVER A TIME UPPER BOUND. GitHub's `mergedAt` for a
# release's own PR is one second AFTER the commit date of the tag cut on it
# (measured on v1.27.0: 21:46:05Z against 21:46:04Z), so a `merged:<=<tag
# date>` bound would lose exactly the PR that made the release. The search is
# bounded below only; each PR's merge commit is then placed by
# `git merge-base --is-ancestor` — inside LOWER (the previous window, not
# counted), inside UPPER (this window), or neither (merged after UPPER,
# reported and not counted).
dogfood_prs_between() {
  DOGFOOD_WINDOW_UNMEASURED=""
  # PMAT-229: the soft switch is the THIRD ARGUMENT, never an environment
  # variable. An earlier version read `${DOGFOOD_WINDOW_SOFT:-0}`, which any
  # operator could export into their shell and every gate would inherit —
  # a release gate softened from outside is exactly the hole this must not
  # open. An argument cannot be inherited.
  local lower="$1" upper="$2" soft="${3:-}"
  local rc=0 raw n jrc=0 i=0 num oid arc prev_rc keep='[]'
  dogfood_ref_date "$lower"
  raw="$("$GH" pr list --repo "$REPO" --state merged --base main --search "merged:>=${DOGFOOD_REF_DATE}" --limit "$PR_PAGE_LIMIT" --json number,mergedAt,mergeCommit,headRefName,title,body 2>&1)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "${GH} pr list exited ${rc} (${raw}) — the PRs merged since ${lower} cannot be enumerated without GitHub, and an unenumerable window is UNMEASURED, which is not the same as a window with no PRs"
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
      fail "GitHub reports PR #${num} merged since ${lower} with no merge commit, so whether it is inside this window cannot be decided — UNMEASURED"
    fi
    # The previous release's own PR is merged at the very second its tag is
    # cut, so `merged:>=<tag date>` returns it; its merge commit is an ancestor
    # of (or is) the tag, which puts it in the PREVIOUS window, not this one.
    prev_rc=0
    git merge-base --is-ancestor "$oid" "$lower" >/dev/null 2>&1 || prev_rc=$?
    if [ "$prev_rc" -eq 0 ]; then
      # PMAT-228: STDERR. This is a diagnostic about a PR that is NOT in the
      # window, and `release-goal.sh cut` captures this function's stdout to
      # build the row it writes into docs/roadmaps/releases.yaml — where the
      # v1.28.0 booking carried this very line, made a YAML comment by the
      # accident of its leading `#`, which is why nothing complained.
      echo "  #${num} ${oid} is inside ${lower} (the previous release) — not counted" >&2
      DOGFOOD_PR_PREVIOUS=$((DOGFOOD_PR_PREVIOUS + 1))
      i=$((i + 1))
      continue
    fi
    if [ "$prev_rc" -ne 1 ]; then
      fail "git merge-base --is-ancestor ${oid} ${lower} exited ${prev_rc} for PR #${num}: the commit GitHub names is not in this checkout (run: git fetch origin) — UNMEASURED"
    fi
    arc=0
    git merge-base --is-ancestor "$oid" "$upper" >/dev/null 2>&1 || arc=$?
    case "$arc" in
      0) keep="$(printf '%s' "$keep" | jq -c --arg o "$oid" '. + [$o]')" ;;
      1)
        # PMAT-228: stderr, for the same reason as the note above.
        echo "  #${num} ${oid} is outside ${upper} (merged after it) — not counted" >&2
        DOGFOOD_PR_OUTSIDE=$((DOGFOOD_PR_OUTSIDE + 1))
        ;;
      *) fail "git merge-base --is-ancestor ${oid} ${upper} exited ${arc} for PR #${num}: the commit GitHub names is not in this checkout (run: git fetch origin), so membership of this window is UNMEASURED" ;;
    esac
    i=$((i + 1))
  done

  DOGFOOD_PR_JSON="$(printf '%s' "$raw" | jq -c --argjson keep "$keep" '[ .[] | select((.mergeCommit.oid // "-") as $o | $keep | index($o) != null) ]')"
  DOGFOOD_PR_COUNT="$(printf '%s' "$DOGFOOD_PR_JSON" | jq -r 'length')"
  # An empty window is a fact only when nothing reached UPPER since LOWER.
  # Commits with no merged PR containing them are work that bypassed review
  # (or a window GitHub could not describe), and a gate that said "PASS 0 of
  # 0" over them would be the vacuous pass release-check.sh Arm 5 refuses
  # (PMAT-178); every gate refuses it here for the same reason.
  local crc=0 commits_since
  commits_since="$(git rev-list --count "${lower}..${upper}")" || crc=$?
  if [ "$crc" -ne 0 ] || [ -z "$commits_since" ]; then
    fail "git rev-list --count ${lower}..${upper} exited ${crc}: whether anything landed since ${lower} cannot be read — UNMEASURED"
  fi
  if [ "$DOGFOOD_PR_COUNT" -eq 0 ] && [ "$commits_since" -gt 0 ]; then
    # PMAT-229: THE STATUS LINE MAY RENDER THIS; NO GATE MAY PASS OVER IT.
    #
    # A third argument of `soft` records the refusal instead of taking it,
    # and only `release-goal.sh show` passes it. On a feature branch — which is where
    # an operator reads the cadence — the branch's own commits are in no merged
    # PR, so this fired and `make release-goal` printed no goal at all: not the
    # tag, not the due instant, not the bar. None of those depend on the
    # unmeasured commits; only the merged count does, and it is printed as
    # UNMEASURED. The caller still exits non-zero.
    #
    # scripts/dogfood/tagged.sh never passes it and is unchanged: a release gate
    # that rendered a degraded line would be a gate that passed on an
    # unmeasured window.
    if [ "$soft" = "soft" ]; then
      DOGFOOD_WINDOW_UNMEASURED="${commits_since} commit(s) reached ${upper} since ${lower} and GitHub reports no merged PR containing any of them: work bypassed review, or the window is UNMEASURED"
      return 0
    fi
    fail "${commits_since} commit(s) reached ${upper} since ${lower} and GitHub reports no merged PR containing any of them: work bypassed review, or the window is UNMEASURED — either way this gate cannot pass over it"
  fi
}

# The PRs merged since DOGFOOD_PREV_TAG that are inside HEAD — the window
# gates A and E check.
dogfood_merged_prs() {
  dogfood_prs_between "$DOGFOOD_PREV_TAG" HEAD
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

# ---------------------------------------------------------------------------
# The ticket registry and the ticket(s) a PR names (PMAT-225, forjar#506).
#
# The registry is docs/roadmaps/roadmap.yaml, read ONCE per run and AT HEAD
# by default (a gate reads committed state, never the working tree; see the
# receipts above). scripts/release-goal.sh sets DOGFOOD_ROADMAP_REF=worktree,
# because a status line that ignores the label you just added is not a status.

# Every `- id:` / label pair of the registry -> DOGFOOD_ROW_LABELS (lines of
# "<id> <label>"), and every id -> DOGFOOD_ROW_IDS (one per line).
dogfood_roadmap_rows() {
  local ref="${DOGFOOD_ROADMAP_REF:-HEAD}" rc=0 text
  if [ -n "${DOGFOOD_ROW_IDS_LOADED:-}" ]; then
    return 0
  fi
  if [ "$ref" = "worktree" ]; then
    text="$(cat docs/roadmaps/roadmap.yaml)" || rc=$?
  else
    text="$(git show "${ref}:docs/roadmaps/roadmap.yaml")" || rc=$?
  fi
  if [ "$rc" -ne 0 ]; then
    fail "docs/roadmaps/roadmap.yaml cannot be read at ${ref} (exit ${rc}): the ticket registry is unreadable, so no PR's ticket can be identified — UNMEASURED"
  fi
  DOGFOOD_ROW_IDS="$(printf '%s\n' "$text" | sed -n 's/^- id: \(PMAT-[0-9][0-9]*\)$/\1/p')"
  # `labels: []` opens a list that the next key closes; `labels:` followed by
  # `  - x` lines is the populated shape. Both are one awk state.
  DOGFOOD_ROW_LABELS="$(printf '%s\n' "$text" | awk '
    /^- id: /       { id = $3; inlist = 0; next }
    /^  labels:/    { inlist = 1; next }
    inlist && /^  - / { print id " " $2; next }
    { inlist = 0 }')"
  # PMAT-236: the STATUS of every row, from the same single read. Gate T
  # reconciles the ledger and the labels and never looked at this, so the
  # roadmap could say a shipped ticket had not started — and did, for sixteen
  # tickets across five releases, with nothing going red.
  DOGFOOD_ROW_STATUSES="$(printf '%s\n' "$text" | awk '
    /^- id: /        { id = $3; next }
    /^  status: /    { if (id != "") { print id " " $2; id = "" } next }')"
  DOGFOOD_ROW_IDS_LOADED=1
}

# The status of row $1 -> DOGFOOD_ROW_STATUS (empty when the row has none).
dogfood_row_status() {
  local rc=0 hit
  dogfood_roadmap_rows
  # ONE PROCESS. `grep … | head -1` is the same defect one line over: head
  # exits after the first line, grep takes SIGPIPE, and pipefail reports 141 —
  # which is exactly what PMAT-239 is about. awk matches, prints and exits by
  # itself, with nothing to signal.
  hit="$(awk -v id="$1" '$1 == id { print $2; exit }' <<< "$DOGFOOD_ROW_STATUSES")" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "awk exited ${rc} reading the status of $1 — UNMEASURED"
  fi
  DOGFOOD_ROW_STATUS="$hit"
}

# Is $1 a roadmap row? Exit 0 or 1 — usable in `if`, never under `$(...)`.
dogfood_is_row() {
  local rc=0
  dogfood_roadmap_rows
  # PMAT-239: a HERE-STRING, never a pipe. `grep -q` exits at the first match
  # and closes the pipe; `printf` then takes SIGPIPE, and under `set -o
  # pipefail` the pipeline's status becomes 141. Measured on main: gate T
  # reported `grep exited 141 looking PMAT-225 up in the ticket registry —
  # UNMEASURED` on one run and passed on the next two, which is a gate that
  # fails CI at random. A here-string has no second process to kill.
  grep -q -x -F -- "$1" <<< "$DOGFOOD_ROW_IDS" || rc=$?
  if [ "$rc" -gt 1 ]; then
    fail "grep exited ${rc} looking $1 up in the ticket registry — UNMEASURED"
  fi
  return "$rc"
}

# The labels of row $1 -> DOGFOOD_LABELS (space-separated, possibly empty).
dogfood_row_labels() {
  local rc=0 hits
  dogfood_roadmap_rows
  hits="$(printf '%s\n' "$DOGFOOD_ROW_LABELS" | grep -E "^$1 " | cut -d' ' -f2-)" || rc=$?
  if [ "$rc" -gt 1 ]; then
    fail "grep exited ${rc} reading the labels of $1 — UNMEASURED"
  fi
  DOGFOOD_LABELS="$(printf '%s\n' "$hits" | tr '\n' ' ' | sed 's/ *$//')"
}

# Every row carrying label $1 -> DOGFOOD_ROWS (space-separated, possibly empty).
dogfood_rows_with_label() {
  local rc=0 hits
  dogfood_roadmap_rows
  hits="$(printf '%s\n' "$DOGFOOD_ROW_LABELS" | grep -F -- " $1" | grep -E " $(printf '%s' "$1" | sed 's/[][\\.*^$+?(){}|]/\\&/g')\$" | cut -d' ' -f1)" || rc=$?
  if [ "$rc" -gt 1 ]; then
    fail "grep exited ${rc} looking for rows labelled $1 — UNMEASURED"
  fi
  DOGFOOD_ROWS="$(printf '%s\n' "$hits" | tr '\n' ' ' | sed 's/ *$//')"
}

# Every `PMAT-<n>` in $1, one per line, in order of appearance, with the slash
# shorthand expanded (`PMAT-212/213/214` is three ids — the 1.27.0 release
# commit names its tickets that way, and a plain scan sees one of three) ->
# DOGFOOD_IDS. "None" is a legitimate answer (grep exit 1); grep failing to
# run (>= 2) is UNMEASURED.
dogfood_ids_in() {
  local rc=0 hits
  hits="$(printf '%s\n' "$1" | grep -o -E 'PMAT-[0-9]+(/[0-9]+)*')" || rc=$?
  if [ "$rc" -gt 1 ]; then
    fail "grep exited ${rc} scanning a PR for ticket ids — UNMEASURED"
  fi
  DOGFOOD_IDS="$(printf '%s\n' "$hits" | awk -F'[-/]' 'NF > 1 { for (i = 2; i <= NF; i++) print "PMAT-" $i }')"
}

# The row $1 resolves to -> DOGFOOD_ROW: $1 itself when it is a row; the one
# row that declares the label `alias:$1` when $1 is not (PR #496's branch is
# named PMAT-218, a ticket that never existed, and its title names PMAT-219 —
# the row that owns the work declares the misnomer, and the declaration is in
# the registry where a reader looks); empty when neither.
#
# A stray id is NEVER skipped in favour of the next one: the plan grill
# refuted that rule by construction — a branch whose row was forgotten would
# fall through to an older ticket named in the body, whose receipt already
# exists, and gate A would pass over the missing work.
dogfood_resolve_id() {
  DOGFOOD_ROW=""
  if dogfood_is_row "$1"; then
    DOGFOOD_ROW="$1"
    return 0
  fi
  dogfood_rows_with_label "alias:$1"
  case "$DOGFOOD_ROWS" in
    "") ;;
    *" "*) fail "more than one roadmap row declares alias:$1 (${DOGFOOD_ROWS}), so the id resolves to no single ticket — a declaration that names two owners names none" ;;
    *) DOGFOOD_ROW="$DOGFOOD_ROWS" ;;
  esac
}

# The ticket(s) a PR names, from its head branch $1, title $2 and body $3 ->
#   DOGFOOD_TICKET   the receipt address gate A reads: the FIRST id in the
#                    branch, then the title, then the body, resolved; empty when
#                    the PR names no id at all, or its first id is stray
#   DOGFOOD_TICKETS  every resolved id in the branch and title, in order of
#                    first appearance (the release a PR ships in is every ticket
#                    it names); when branch and title name none, the body's first
#   DOGFOOD_STRAY_IDS every id in the branch and title that resolves to nothing
#
# Cheapest and most deliberate source first, as gate A has always read it.
dogfood_pr_tickets() {
  local id first=""
  DOGFOOD_TICKET=""
  DOGFOOD_TICKETS=""
  DOGFOOD_STRAY_IDS=""
  dogfood_ids_in "$1
$2"
  for id in $DOGFOOD_IDS; do
    [ -z "$first" ] && first="$id"
    dogfood_take_id "$id"
  done
  if [ -z "$first" ]; then
    dogfood_ids_in "$3"
    for id in $DOGFOOD_IDS; do
      first="$id"
      dogfood_take_id "$id"
      break
    done
  fi
  if [ -n "$first" ]; then
    dogfood_resolve_id "$first"
    DOGFOOD_TICKET="$DOGFOOD_ROW"
  fi
}

# Classify one id into DOGFOOD_TICKETS or DOGFOOD_STRAY_IDS, once.
dogfood_take_id() {
  dogfood_resolve_id "$1"
  if [ -n "$DOGFOOD_ROW" ]; then
    case " $DOGFOOD_TICKETS " in
      *" $DOGFOOD_ROW "*) ;;
      *) DOGFOOD_TICKETS="${DOGFOOD_TICKETS:+$DOGFOOD_TICKETS }$DOGFOOD_ROW" ;;
    esac
  else
    case " $DOGFOOD_STRAY_IDS " in
      *" $1 "*) ;;
      *) DOGFOOD_STRAY_IDS="${DOGFOOD_STRAY_IDS:+$DOGFOOD_STRAY_IDS }$1" ;;
    esac
  fi
}

# The ticket census of the window in DOGFOOD_PR_JSON -> DOGFOOD_WINDOW_TICKETS
# (space-separated, version-sorted, unique), DOGFOOD_WINDOW_STRAYS ("#<pr>:<id>,
# <id>" per PR naming an id that resolves to no row), DOGFOOD_WINDOW_UNTICKETED
# ("#<pr>" per PR naming no row at all). Every PR's ids go through
# `dogfood_pr_tickets`, the one rule. An empty census is a legitimate answer:
# `awk NF` selects the non-empty lines and, unlike `grep -v '^$'`, exits 0 when
# there are none — a grep exit 1 under pipefail inside this assignment would
# kill the caller with no verdict line (measured on the v1.25.1 window, which
# names no ticket at all).
dogfood_window_tickets() {
  local i=0 num href title body all="" strays="" none=""
  while [ "$i" -lt "$DOGFOOD_PR_COUNT" ]; do
    dogfood_pr_field "$i" ".number"; num="$DOGFOOD_FIELD"
    dogfood_pr_field "$i" ".headRefName"; href="$DOGFOOD_FIELD"
    dogfood_pr_field "$i" ".title"; title="$DOGFOOD_FIELD"
    dogfood_pr_field "$i" ".body"; body="$DOGFOOD_FIELD"
    dogfood_pr_tickets "$href" "$title" "$body"
    all="$all $DOGFOOD_TICKETS"
    [ -z "$DOGFOOD_STRAY_IDS" ] || strays="$strays #${num}:${DOGFOOD_STRAY_IDS// /,}"
    [ -n "$DOGFOOD_TICKETS" ] || none="$none #${num}"
    i=$((i + 1))
  done
  # shellcheck disable=SC2086 — $all is a list of ids, split on purpose
  DOGFOOD_WINDOW_TICKETS="$(printf '%s\n' $all | awk 'NF' | sort -u -V | tr '\n' ' ' | sed 's/ *$//')"
  DOGFOOD_WINDOW_STRAYS="${strays# }"
  DOGFOOD_WINDOW_UNTICKETED="${none# }"
}
