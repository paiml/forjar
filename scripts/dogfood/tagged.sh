#!/usr/bin/env bash
# Dogfood gate T — every tagged release is declared, every ticket names the tag
# that shipped it, and the next cut is on time.
#
# Exit code is the gate. The only line a caller must read is the final
# `GATE T PASS|FAIL <detail>`.
#
# WHAT THIS GATE IS FOR (PMAT-225, forjar#506)
#
# Gates A and E measure the window since the newest tag and stop there. Nothing
# asked whether a ticket could be found from the release it shipped in, and
# nothing asked whether the newest tag was OLD: eight PRs had merged since
# v1.27.0, no row, receipt or issue named the release any of them would ship
# in, and the only ticket-to-tag link was the squash-commit subject — which is
# lossy (the 1.27.0 release commit names `PMAT-212/213/214`, and a scan for
# `PMAT-[0-9]+` sees one ticket of three).
#
# So this gate joins a DECLARED side against a MEASURED one, the way the
# paiml-implement status line joins `goal.sh set` against the transcript:
#
#   declared   docs/roadmaps/releases.yaml — cadence_days, the floors, one row
#              per tag (cut, prs, tickets, dogfood, crux) and the open goal
#              (next.tag, next.due); and the `release:<tag>` labels on the
#              rows of docs/roadmaps/roadmap.yaml
#   measured   the tags git holds and when they were cut; the PRs GitHub
#              reports merged, placed in a window by ancestry (never by a time
#              upper bound — see lib/window.sh); the ticket(s) each PR names,
#              by the one rule gate A reads; the clock
#
# and it is RED wherever the two disagree:
#
#   T1  the ledger parses and the floors are tags origin carries
#   T2  every reachable tag at or above the floor has a row whose cut, prs and
#       tickets are exactly what git and GitHub say; no row names a tag origin
#       does not carry
#   T3  from harness_floor on, every PR in a window names a roadmap ticket and
#       no PR names an id that is neither a row nor a declared alias
#   T4  every ticket a row names carries `release:<tag>`, no row of the roadmap
#       carries `release:<tag>` for a release it was not in, and every ticket
#       merged since the newest tag carries `release:<next.tag>` — the
#       continuous half: a PR is tagged when it merges, not when the cut is made
#   T5  from dogfood_floor on, the dogfood receipt and the crux document a row
#       names exist at HEAD, the receipt ends in its END marker and reaches
#       exactly one verdict
#   T6  next.tag is above the newest tag, next.due is exactly the newest cut
#       plus cadence_days, and the cut is not OVERDUE: past next.due with PRs
#       merged since the tag and Cargo.toml still at the tagged version, this
#       gate is red and says by how much. A bumped Cargo.toml is a cut in
#       flight, and passes.
#
# The ledger, the registry, the receipts and Cargo.toml are all read AT HEAD.
# A gate that reads uncommitted files reports on a tree no one else will have.
#
# UNMEASURED IS FAIL. A gh that cannot answer, a tag whose creation instant
# cannot be read, a ledger that does not parse: each means this gate does not
# know the set it is checking, and it says so instead of printing the words of
# a check that ran. Nothing here is skipped for being offline.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

fail() {
  echo "GATE T FAIL $1"
  exit 1
}

# shellcheck source=scripts/dogfood/lib/window.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib/window.sh"
# shellcheck source=scripts/dogfood/lib/releases.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib/releases.sh"

# The clock. Overridable ONLY so the falsification tests can stand at a chosen
# instant; the cadence arm measures the wall clock by design.
NOW="${DOGFOOD_NOW:-$(date -u +%s)}" # bashrs disable-line=DET002 — the cadence measures the clock

# Is tag $1 on origin? 0 yes, 1 no, anything else UNMEASURED and fatal.
tag_on_origin() {
  local rc=0
  git ls-remote --exit-code --tags origin "refs/tags/$1" >/dev/null 2>&1 || rc=$?
  case "$rc" in
    0) return 0 ;;
    2) return 1 ;;
    *) fail "git ls-remote origin refs/tags/$1 exited ${rc}: whether origin carries the tag cannot be read — UNMEASURED" ;;
  esac
}

# The reachable v* tags at or above $FLOOR, ascending -> TAGS (space-separated).
reachable_tags_from_floor() {
  local rc=0 all t keep=""
  all="$(git tag --list 'v*' --merged HEAD | sort -V)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "git tag --merged HEAD exited ${rc}: the set of tagged releases cannot be read — UNMEASURED"
  fi
  for t in $all; do
    if dogfood_semver_ge "$t" "$FLOOR"; then
      keep="${keep:+$keep }$t"
    fi
  done
  TAGS="$keep"
}

# The tag just below $1 among the tags reachable from it -> LOWER.
lower_tag_of() {
  local rc=0 t
  t="$(git tag --list 'v*' --sort=-v:refname --merged "$1" | grep -v -x -F -- "$1" | head -1)" || rc=$?
  if [ "$rc" -gt 1 ]; then
    fail "git tag --merged ${1} exited ${rc}: the lower bound of ${1}'s window cannot be read — UNMEASURED"
  fi
  if [ -z "$t" ]; then
    fail "no v* tag is reachable from ${1} below it, so its window has no lower bound — UNMEASURED"
  fi
  LOWER="$t"
}

# A declared list (jq path $2 of row $1, sorted by version) -> DECLARED.
declared_list() {
  local rc=0 v
  v="$(printf '%s' "$1" | jq -r "$2 | map(tostring) | .[]" | sort -V | tr '\n' ' ' | sed 's/ *$//')" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "cannot read ${2} out of the declared row (jq exit ${rc}) — UNMEASURED"
  fi
  DECLARED="$v"
}

# The measured PR numbers of DOGFOOD_PR_JSON, ascending -> MEASURED_PRS.
measured_prs() {
  local rc=0 v
  v="$(printf '%s' "$DOGFOOD_PR_JSON" | jq -r '.[].number' | sort -V | tr '\n' ' ' | sed 's/ *$//')" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "cannot read the PR numbers of the window (jq exit ${rc}) — UNMEASURED"
  fi
  MEASURED_PRS="$v"
}

# T3 for one window ($1 names it): strays and unticketed PRs are red from the
# harness floor on.
harness_regime() {
  if [ -n "$DOGFOOD_WINDOW_STRAYS" ]; then
    fail "${1}: PR(s) ${DOGFOOD_WINDOW_STRAYS} name a ticket id that is neither a row of docs/roadmaps/roadmap.yaml at HEAD nor declared by any row as its alias:<id> — a PR whose ticket is not on the roadmap is linked to nothing (declare the misnomer with: scripts/release-goal.sh alias STRAY OWNER)"
  fi
  if [ -n "$DOGFOOD_WINDOW_UNTICKETED" ]; then
    fail "${1}: PR(s) ${DOGFOOD_WINDOW_UNTICKETED} name no roadmap ticket in branch, title or body, so the release they shipped in can be linked to no ticket"
  fi
}

# T4 for one tag: every declared ticket carries release:<tag>, and every row
# carrying release:<tag> is a declared ticket.
labels_of_release() {
  local tag="$1" tickets="$2" t
  for t in $tickets; do
    dogfood_row_labels "$t"
    case " $DOGFOOD_LABELS " in
      *" release:${tag} "*) ;;
      *) fail "${t} shipped in ${tag} (PR(s) $(printf '%s' "$DOGFOOD_PR_JSON" | jq -r '[.[].number] | map("#" + tostring) | join(" ")')) and its roadmap row does not carry the label release:${tag}: the ticket cannot be found from its release (add it with: scripts/release-goal.sh tag ${t} ${tag})" ;;
    esac
  done
  dogfood_rows_with_label "release:${tag}"
  for t in $DOGFOOD_ROWS; do
    case " $tickets " in
      *" $t "*) ;;
      *) fail "${t} carries the label release:${tag} and no PR in ${tag}'s window names it: a ticket claiming a release it was not in is a fabricated link (a goal that missed the cut moves to the next release: scripts/release-goal.sh cut moves it)" ;;
    esac
  done
}

# T5: the dogfood receipt and crux document of row $1 (tag $2, version $3).
receipts_of_release() {
  local row="$1" tag="$2" ver="$3" rc=0 path text last n
  path="$(printf '%s' "$row" | jq -r '.dogfood // ""')"
  if [ -z "$path" ]; then
    fail "${tag} is at or above dogfood_floor and its row declares no dogfood: receipt — a release with no dogfood receipt was not measured before it was cut"
  fi
  if ! git cat-file -e "HEAD:${path}" 2>/dev/null; then
    fail "${tag}: ${path} is not at HEAD, so nothing records that the release was measured before it was cut"
  fi
  text="$(git show "HEAD:${path}")" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "git show HEAD:${path} exited ${rc} — the dogfood receipt of ${tag} cannot be read, UNMEASURED"
  fi
  last="$(printf '%s\n' "$text" | tail -n 1)"
  if [ "$last" != "DOGFOOD-${ver}-RECEIPT-END" ]; then
    fail "${tag}: the last line of ${path} is \"${last}\", not DOGFOOD-${ver}-RECEIPT-END — without the marker a truncated receipt reads as one that said less"
  fi
  rc=0
  n="$(printf '%s\n' "$text" | grep -cE '^[Vv]erdict:')" || rc=$?
  if [ "$rc" -gt 1 ]; then
    fail "grep exited ${rc} counting the verdict lines of ${path} — UNMEASURED"
  fi
  if [ "$n" -ne 1 ]; then
    fail "${tag}: ${path} has ${n} line(s) matching ^verdict: — exactly one is required"
  fi
  path="$(printf '%s' "$row" | jq -r '.crux // ""')"
  if [ -z "$path" ]; then
    fail "${tag} is at or above dogfood_floor and its row declares no crux: document — a release whose behaviours were reconciled against no other system (gate H) is undeclared here"
  fi
  if ! git cat-file -e "HEAD:${path}" 2>/dev/null; then
    fail "${tag}: ${path} is not at HEAD"
  fi
}

# ---------------------------------------------------------------- T1 the ledger
dogfood_load_releases
dogfood_releases_field '.cadence_days'; CADENCE_DAYS="$DOGFOOD_FIELD"
dogfood_releases_field '.floor'; FLOOR="$DOGFOOD_FIELD"
dogfood_releases_field '.harness_floor'; HARNESS_FLOOR="$DOGFOOD_FIELD"
dogfood_releases_field '.dogfood_floor'; DOGFOOD_FLOOR="$DOGFOOD_FIELD"
dogfood_releases_field '.next.tag'; NEXT_TAG="$DOGFOOD_FIELD"
dogfood_releases_field '.next.due'; NEXT_DUE="$DOGFOOD_FIELD"
for t in "$FLOOR" "$HARNESS_FLOOR" "$DOGFOOD_FLOOR"; do
  if ! tag_on_origin "$t"; then
    fail "the ledger's floor ${t} is not a tag origin carries: a floor that names no release bounds nothing"
  fi
done

# ---------------------------------------------- T2 T3 T4 T5 every tagged release
reachable_tags_from_floor
if [ -z "$TAGS" ]; then
  fail "no v* tag at or above ${FLOOR} is reachable from HEAD, so there is no tagged release to reconcile — UNMEASURED, and a gate over no releases would be vacuous"
fi
dogfood_releases_field '.releases | map(.tag) | join(" ")'; DECLARED_TAGS="$DOGFOOD_FIELD"
for t in $DECLARED_TAGS; do
  if ! tag_on_origin "$t"; then
    fail "docs/roadmaps/releases.yaml declares ${t} and origin carries no such tag: a declared release that does not exist"
  fi
done

checked=0
tickets_checked=0
for tag in $TAGS; do
  dogfood_release_row "$tag"
  if [ -z "$DOGFOOD_RELEASE_ROW" ]; then
    fail "${tag} is reachable from HEAD, at or above the floor ${FLOOR}, and has no row in docs/roadmaps/releases.yaml: a release that was cut and never declared (write it with: scripts/release-goal.sh cut ${tag} --next vX.Y.Z)"
  fi
  row="$DOGFOOD_RELEASE_ROW"
  dogfood_tag_date "$tag"
  declared_cut="$(printf '%s' "$row" | jq -r '.cut')"
  if [ "$declared_cut" != "$DOGFOOD_TAG_DATE" ]; then
    fail "${tag}: the ledger says cut: ${declared_cut} and git says the tag was created at ${DOGFOOD_TAG_DATE} — the declared and the measured instant disagree"
  fi
  lower_tag_of "$tag"
  # Never `>/dev/null` here: fail() prints the verdict on stdout, and a
  # redirected window is a gate that dies with no line (measured: every case
  # of the falsification test reported a death, not a verdict).
  dogfood_prs_between "$LOWER" "$tag"
  measured_prs
  declared_list "$row" '.prs'
  if [ "$DECLARED" != "$MEASURED_PRS" ]; then
    fail "${tag}: the ledger declares prs [${DECLARED}] and GitHub reports [${MEASURED_PRS}] merged between ${LOWER} and ${tag} (by ancestry) — the declared window is not the measured one"
  fi
  dogfood_window_tickets
  if dogfood_semver_ge "$tag" "$HARNESS_FLOOR"; then
    harness_regime "$tag"
  fi
  declared_list "$row" '.tickets'
  if [ "$DECLARED" != "$DOGFOOD_WINDOW_TICKETS" ]; then
    fail "${tag}: the ledger declares tickets [${DECLARED}] and the PRs of its window name [${DOGFOOD_WINDOW_TICKETS}] — the declared and the measured ticket set disagree"
  fi
  labels_of_release "$tag" "$DOGFOOD_WINDOW_TICKETS"
  if dogfood_semver_ge "$tag" "$DOGFOOD_FLOOR"; then
    receipts_of_release "$row" "$tag" "${tag#v}"
  fi
  n=0
  for _ in $DOGFOOD_WINDOW_TICKETS; do n=$((n + 1)); done
  tickets_checked=$((tickets_checked + n))
  DOGFOOD_LEDGER_TICKETS="${DOGFOOD_LEDGER_TICKETS:-} ${DOGFOOD_WINDOW_TICKETS}"
  echo "GATE T ${tag} cut ${DOGFOOD_TAG_DATE}: ${DOGFOOD_PR_COUNT} PR(s), ${n} ticket(s) labelled release:${tag} ok"
  checked=$((checked + 1))
done

# --------------------------------------------------- T4 T6 the open goal
dogfood_load_window
NEWEST="$DOGFOOD_PREV_TAG"
if ! dogfood_semver_ge "$NEXT_TAG" "$NEWEST" || [ "$NEXT_TAG" = "$NEWEST" ]; then
  fail "next.tag ${NEXT_TAG} is not above the newest reachable tag ${NEWEST}: the open goal names a release that already happened"
fi
dogfood_tag_date "$NEWEST"
dogfood_epoch "$DOGFOOD_TAG_DATE"; newest_cut="$DOGFOOD_EPOCH"
dogfood_iso $((newest_cut + CADENCE_DAYS * 86400)); derived_due="$DOGFOOD_ISO"
if [ "$NEXT_DUE" != "$derived_due" ]; then
  fail "next.due is declared ${NEXT_DUE} and ${NEWEST} was cut at ${DOGFOOD_TAG_DATE}, so cadence_days ${CADENCE_DAYS} puts the due instant at ${derived_due} — the declared goal and the derived one disagree"
fi
dogfood_window_tickets
harness_regime "since ${NEWEST}"
open_tagged=0
for t in $DOGFOOD_WINDOW_TICKETS; do
  dogfood_row_labels "$t"
  case " $DOGFOOD_LABELS " in
    *" release:${NEXT_TAG} "*) open_tagged=$((open_tagged + 1)) ;;
    *) fail "${t} merged since ${NEWEST} and its roadmap row does not carry release:${NEXT_TAG}: a PR is tagged when it merges, not when the cut is made (scripts/release-goal.sh sync labels every ticket of the open window)" ;;
  esac
done

# T7 (PMAT-236): A SHIPPED TICKET SAYS IT SHIPPED.
#
# T2 and T4 reconcile the ledger and the `release:<tag>` labels, and neither
# looks at `status`. So the roadmap said none of the 1.28.0 work had started on
# the day 1.28.0 shipped: sixteen tickets across five releases read `planned` or
# `inprogress` while their labels were correct, and no gate went red for it
# (PMAT-235 backfilled them). The label is the link a release needs; the status
# is what a person reads, and a record only half true is the kind that is
# trusted right up until it matters.
# The open window's tickets too: their PRs have merged, so the work has landed
# and `completed` is what PMAT-235 established that means. This is the arm that
# catches the drift as it happens rather than five releases later.
check_status() {
  local t="$1" where="$2"
  dogfood_row_status "$t"
  case "$DOGFOOD_ROW_STATUS" in
    completed|cancelled) ;;
    "") fail "${t} ${where} and has no status on its roadmap row — UNMEASURED" ;;
    *) fail "${t} ${where} and its roadmap row still reads status: ${DOGFOOD_ROW_STATUS}: the work has landed and the roadmap says it has not started (move it with: pmat work edit ${t} -s inprogress && pmat work edit ${t} -s completed)" ;;
  esac
}
for t in $DOGFOOD_LEDGER_TICKETS; do
  check_status "$t" "is named by a tagged release in docs/roadmaps/releases.yaml"
done
for t in $DOGFOOD_WINDOW_TICKETS; do
  check_status "$t" "is named by a PR merged since ${NEWEST}"
done

dogfood_epoch "$NEXT_DUE"; due="$DOGFOOD_EPOCH"
dogfood_cargo_version
case "$DOGFOOD_CARGO_VERSION" in
  "${NEWEST#v}") in_flight="" ;;
  "${NEXT_TAG#v}") in_flight="cut in flight: Cargo.toml is at ${DOGFOOD_CARGO_VERSION}" ;;
  *) fail "Cargo.toml at HEAD says ${DOGFOOD_CARGO_VERSION}, which is neither the newest tag ${NEWEST} nor the declared next ${NEXT_TAG}: the tree is at a version no goal names" ;;
esac
if [ "$NOW" -gt "$due" ] && [ "$DOGFOOD_PR_COUNT" -gt 0 ] && [ -z "$in_flight" ]; then
  fail "the cut of ${NEXT_TAG} is OVERDUE by $(( (NOW - due) / 3600 ))h: due ${NEXT_DUE} (${NEWEST} cut + ${CADENCE_DAYS} day(s)), ${DOGFOOD_PR_COUNT} PR(s) merged since ${NEWEST}, and Cargo.toml is still at ${DOGFOOD_CARGO_VERSION}"
fi
if [ -n "$in_flight" ]; then
  clock="$in_flight"
elif [ "$NOW" -gt "$due" ]; then
  clock="past due ${NEXT_DUE} with nothing merged since ${NEWEST} — nothing to cut"
else
  clock="due ${NEXT_DUE}, $(( (due - NOW) / 3600 ))h left"
fi

echo "GATE T PASS ${checked} tagged release(s) since ${FLOOR} reconcile with git and GitHub and ${tickets_checked} ticket(s) carry their tag and say they shipped; ${open_tagged} ticket(s) from ${DOGFOOD_PR_COUNT} PR(s) merged since ${NEWEST} carry release:${NEXT_TAG}; ${clock}"

# mutation: change `dogfood_iso $((newest_cut + CADENCE_DAYS * 86400))` to
# `dogfood_iso $((newest_cut + CADENCE_DAYS * 86400))` — the derived due
# instant then disagrees with the declared one by a few seconds on the real
# ledger and the gate exits 1, which shows next.due is re-derived from the
# tag's own creation instant and not read back from the file.
