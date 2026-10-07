#!/usr/bin/env bash
# release-goal.sh — the release goal in the paiml-implement goal shape
# (AUTO-IMPL-SKILL-002, PMAT-225, forjar#506): a DECLARED side a human wrote
# (docs/roadmaps/releases.yaml, and the `release:<tag>` labels on the rows of
# docs/roadmaps/roadmap.yaml) joined against MEASURED facts (the tags git
# holds, the PRs GitHub reports merged, the clock) into one status line with
# `basis=` on every number. scripts/dogfood/tagged.sh (gate T) is the judge;
# this is the instrument you read and the pen you edit with.
#
#   release-goal.sh show
#       <next.tag> <bar> <elapsed>h/<cadence>h left=<h> · <merged> merged, <tagged> tagged
#       · due <iso> basis=docs/roadmaps/releases.yaml:L<n> window=<tag>..HEAD(<sha>) ledger=worktree[(dirty)]
#   release-goal.sh window [TAG]
#       the ledger row MEASURED for TAG (or, with no TAG, for the open window
#       since the newest reachable tag) — what to paste into releases.yaml
#   release-goal.sh tag TICKET TAG        add `release:TAG` to TICKET's row
#   release-goal.sh alias STRAY OWNER     add `alias:STRAY` to OWNER's row
#   release-goal.sh sync [--check]        label every ticket merged since the newest
#                                         tag with `release:<next.tag>`; --check edits
#                                         nothing and exits 1 when one is missing
#   release-goal.sh cut TAG --next NEXT   after `git tag TAG`: append TAG's measured
#                                         row, declare NEXT with due = TAG's cut +
#                                         cadence, and move any `release:TAG` label
#                                         on a ticket TAG did not ship to `release:NEXT`
#
# READS THE WORKING TREE and says so. The gate reads HEAD. A status that
# ignores the label you just added is not a status (the PMAT-225 plan grill).
#
# NEVER A YAML ROUND-TRIP. Every edit is a textual insertion inside one row,
# so `git diff` shows exactly the lines meant. PyYAML's dump re-quotes every
# string and rewrites every timestamp in the file; a 600-line diff of that
# shape was refuted by a merge review on the 1.26.0 branch.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

fail() {
  echo "release-goal: $1" >&2
  exit 2
}

# shellcheck source=scripts/dogfood/lib/window.sh
. scripts/dogfood/lib/window.sh
# shellcheck source=scripts/dogfood/lib/releases.sh
. scripts/dogfood/lib/releases.sh
DOGFOOD_ROADMAP_REF=worktree
DOGFOOD_RELEASES_REF=worktree
NOW="${DOGFOOD_NOW:-$(date -u +%s)}" # bashrs disable-line=DET002
# NOW: the cadence measures the clock; DOGFOOD_NOW pins it for fixtures.
# STAMP: a row's `updated:` is the moment of its edit by definition, as
# `pmat work edit` writes it.
STAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)" # bashrs disable-line=DET002

# One row of docs/roadmaps/roadmap.yaml, edited as text (PMAT-607: this was a
# python3 heredoc). The row is `- id: TICKET` up to the next `- id: `. The
# label lines are `  - LABEL` directly under `  labels:`, and `labels: []` is
# the empty list. The row's `updated:` is bumped, as `pmat work edit` bumps it.
# Exit 0 means the new text is on stdout. 10 means nothing to do. 3 means
# TICKET is not a row, and 4 means the row has no labels: key.
# RG_OP is add or remove.
ROW_EDIT_AWK='
{ line[++n] = $0 }
END {
  t = ENVIRON["RG_T"]; l = "  - " ENVIRON["RG_L"]; op = ENVIRON["RG_OP"]
  for (i = 1; i <= n; i++) if (line[i] == "- id: " t) { s = i; break }
  if (!s) exit 3
  e = n + 1
  for (i = s + 1; i <= n; i++) if (substr(line[i], 1, 6) == "- id: ") { e = i; break }
  at = 0; drop = 0
  for (i = s; i < e; i++) if (line[i] == l) { at = i; break }
  if (op == "add") {
    if (at) exit 10
    for (i = s + 1; i < e; i++) if (line[i] == "  labels: []") { line[i] = "  labels:"; at = i; break }
    if (!at) {
      for (i = s + 1; i < e; i++) if (line[i] == "  labels:") { at = i; break }
      if (!at) exit 4
      while (at + 1 < e && substr(line[at + 1], 1, 4) == "  - ") at++
    }
  } else {
    if (!at) exit 10
    drop = at
    for (i = s + 1; i < e; i++) {
      if (i == drop) continue
      if (line[i] == "  labels:") {
        j = i + 1; if (j == drop) j++
        if (j >= e || substr(line[j], 1, 4) != "  - ") line[i] = "  labels: []"
        break
      }
    }
  }
  for (i = s; i < e; i++) if (substr(line[i], 1, 11) == "  updated: ") { line[i] = "  updated: " ENVIRON["RG_S"]; break }
  for (i = 1; i <= n; i++) {
    if (i == drop) continue
    print line[i]
    if (op == "add" && i == at) print l
  }
}'

# Edit row $2 (op $1) for label $3 -> prints one line saying what it did.
edit_row() {
  local op="$1" ticket="$2" label="$3" path=docs/roadmaps/roadmap.yaml rc=0
  local tmp="${path}.release-goal.tmp"
  RG_OP="$op" RG_T="$ticket" RG_L="$label" RG_S="$STAMP" awk "$ROW_EDIT_AWK" "$path" >"$tmp" || rc=$?
  if [ "$rc" -ne 0 ]; then
    rm -f "${tmp:?}"
  fi
  case "$op:$rc" in
    add:0) mv "$tmp" "$path"; echo "labelled ${ticket} ${label}" ;;
    remove:0) mv "$tmp" "$path"; echo "removed  ${ticket} ${label}" ;;
    add:10) echo "already  ${ticket} ${label}" ;;
    remove:10) echo "absent   ${ticket} ${label}" ;;
    *:3) echo "release-goal: ${ticket} is not a row of ${path}" >&2; exit 3 ;;
    *:4) echo "release-goal: ${ticket} has no labels: key" >&2; exit 3 ;;
    *) echo "release-goal: awk exited ${rc} editing ${ticket} in ${path}" >&2; exit 3 ;;
  esac
}

# Add label $2 to row $1, textually and idempotently. Prints one line.
label_row() {
  edit_row add "$1" "$2"
}

# Remove label $2 from row $1, textually. Prints one line.
unlabel_row() {
  edit_row remove "$1" "$2"
}

# The release tag just below $1 among those reachable from $1 -> LOWER.
lower_tag_of() {
  local rc=0 t
  # PMAT-239: one capture and one awk, never a three-stage pipe whose last
  # two stages exit early and leave git holding a closed pipe.
  local all
  all="$(git tag --list 'v*' --sort=-v:refname --merged "$1")" || rc=$?
  dogfood_release_tags "$all"
  t="$(awk -v skip="$1" '$0 != skip { print; exit }' <<< "$DOGFOOD_RELEASE_TAGS")"
  if [ "$rc" -gt 1 ]; then
    fail "git tag --merged ${1} failed (exit ${rc})"
  fi
  [ -n "$t" ] || fail "no v* tag is reachable from ${1} below it, so its window has no lower bound"
  LOWER="$t"
}

# The ticket census of DOGFOOD_PR_JSON, by the one rule in lib/window.sh.
window_tickets() {
  dogfood_window_tickets
  TICKETS="$DOGFOOD_WINDOW_TICKETS"
  STRAYS="$DOGFOOD_WINDOW_STRAYS"
  UNTICKETED="$DOGFOOD_WINDOW_UNTICKETED"
}

# PR numbers of DOGFOOD_PR_JSON, ascending -> PRS (comma-separated).
window_prs() {
  PRS="$(printf '%s' "$DOGFOOD_PR_JSON" | jq -r '[.[].number] | sort | map(tostring) | join(", ")')"
}

# The row for TAG (or the open window), exactly as the ledger spells it: from
# dogfood_floor on, the receipt and crux paths the release must carry (the
# PMAT-225 quorum refuted a `window` that omitted them — its output and the
# ledger's rows were not the same text).
cmd_window() {
  local tag="${1:-}" lower upper cutline receipts="" cookbook_floor cookbook=""
  dogfood_load_releases
  dogfood_releases_field '.cookbook_floor // ""'; cookbook_floor="$DOGFOOD_FIELD"
  dogfood_releases_field '.dogfood_floor'
  if [ -n "$tag" ]; then
    git rev-parse -q --verify "refs/tags/${tag}" >/dev/null || fail "no such tag: ${tag}"
    lower_tag_of "$tag"; lower="$LOWER"; upper="$tag"
    dogfood_tag_date "$tag"; cutline="    cut: ${DOGFOOD_TAG_DATE}"
    if dogfood_semver_ge "$tag" "$DOGFOOD_FIELD"; then
      receipts="    dogfood: docs/audits/dogfood-${tag#v}-receipt.md
    crux: docs/audits/crux-${tag#v}.md"
    fi
    # PMAT-241: the cookbook commit this release was qualified against. Taken
    # from the cookbook's canonical branch at the moment of the cut, because
    # that is what `make dogfood-published VERSION=` will have run gates C and
    # D against. A branch name would move; a sha does not.
    if [ -n "$cookbook_floor" ] && dogfood_semver_ge "$tag" "$cookbook_floor"; then
      local ls rc2=0
      ls="$(git ls-remote https://github.com/paiml/forjar-cookbook refs/heads/master)" || rc2=$?
      if [ "$rc2" -ne 0 ] || [ -z "$ls" ]; then
        fail "git ls-remote on paiml/forjar-cookbook exited ${rc2}: the cookbook commit this release is qualified against cannot be read, and an unread one must not be written down"
      fi
      cookbook="    cookbook: ${ls%%[[:space:]]*}"
    fi
  else
    dogfood_prev_tag; lower="$DOGFOOD_PREV_TAG"; upper="HEAD"
    dogfood_releases_field '.next.tag'; tag="$DOGFOOD_FIELD"
    cutline="    cut: null   # not cut yet — the open window since ${lower}"
  fi
  dogfood_prs_between "$lower" "$upper"
  window_prs
  window_tickets
  echo "  - tag: ${tag}"
  echo "$cutline"
  echo "    prs: [${PRS}]"
  echo "    tickets: [$(printf '%s' "$TICKETS" | sed 's/ /, /g')]"
  [ -z "$receipts" ] || echo "$receipts"
  [ -z "$cookbook" ] || echo "$cookbook"
  [ -z "$STRAYS" ] || echo "    # stray ids (no row, no alias): ${STRAYS}"
  [ -z "$UNTICKETED" ] || echo "    # PRs naming no roadmap ticket: ${UNTICKETED}"
}

cmd_show() {
  local next due cadence_h elapsed_h left_h filled bar i merged tagged=0 t dirty sha color reset=""
  DOGFOOD_WINDOW_UNMEASURED=""
  dogfood_load_releases
  dogfood_releases_field '.next.tag'; next="$DOGFOOD_FIELD"
  dogfood_releases_field '.next.due'; due="$DOGFOOD_FIELD"
  dogfood_releases_field '.cadence_days'; cadence_h=$((DOGFOOD_FIELD * 24))
  dogfood_prev_tag
  dogfood_tag_date "$DOGFOOD_PREV_TAG"
  dogfood_epoch "$DOGFOOD_TAG_DATE"; elapsed_h=$(( (NOW - DOGFOOD_EPOCH) / 3600 ))
  dogfood_epoch "$due"; left_h=$(( (DOGFOOD_EPOCH - NOW) / 3600 ))
  # PMAT-229: the status line renders what it can and says what it cannot.
  # An operator runs `make release-goal` from a feature branch, where the
  # branch's own commits are in no merged PR; refusing to print the due
  # instant and the bar because the MERGED COUNT is unmeasurable made the
  # cadence unreadable exactly where the work happens. Gate T is unchanged.
  dogfood_prs_between "$DOGFOOD_PREV_TAG" HEAD soft
  if [ -n "${DOGFOOD_WINDOW_UNMEASURED:-}" ]; then
    merged=UNMEASURED
    tagged=UNMEASURED
  else
    window_tickets
    merged=0
    for t in $TICKETS; do
      merged=$((merged + 1))
      dogfood_row_labels "$t"
      case " $DOGFOOD_LABELS " in *" release:${next} "*) tagged=$((tagged + 1)) ;; esac
    done
  fi
  filled=$(( elapsed_h * 10 / cadence_h ))
  [ "$filled" -le 10 ] || filled=10
  [ "$filled" -ge 0 ] || filled=0
  bar=""; i=0
  while [ "$i" -lt 10 ]; do
    if [ "$i" -lt "$filled" ]; then bar="${bar}█"; else bar="${bar}░"; fi
    i=$((i + 1))
  done
  color=""
  if [ -t 1 ]; then
    reset=$'\033[0m'
    if [ $((elapsed_h * 10)) -lt $((cadence_h * 8)) ]; then color=$'\033[32m'
    elif [ "$elapsed_h" -lt "$cadence_h" ]; then color=$'\033[33m'
    else color=$'\033[31m'; fi
  fi
  dirty=""
  [ -z "$(git status --porcelain -- docs/roadmaps Cargo.toml)" ] || dirty="(dirty)"
  sha="$(git rev-parse --short HEAD)"
  printf '%s%s %s %sh/%sh left=%sh%s · %s merged, %s tagged · due %s basis=docs/roadmaps/releases.yaml:L%s window=%s..HEAD(%s) ledger=worktree%s\n' \
    "$color" "$next" "$bar" "$elapsed_h" "$cadence_h" "$left_h" "$reset" \
    "$merged" "$tagged" "$due" "$DOGFOOD_RELEASES_NEXT_LINE" "$DOGFOOD_PREV_TAG" "$sha" "$dirty"
  [ -z "$STRAYS" ] || echo "stray ids: ${STRAYS}"
  [ -z "$UNTICKETED" ] || echo "PRs naming no roadmap ticket: ${UNTICKETED}"
  # The line is rendered and the exit code is still the truth: a script that
  # read this as a pass would be reading a degraded line as a measured one.
  if [ -n "${DOGFOOD_WINDOW_UNMEASURED:-}" ]; then
    echo "release-goal: ${DOGFOOD_WINDOW_UNMEASURED}" >&2
    return 2
  fi
}

cmd_sync() {
  local check="${1:-}" next t missing=0
  dogfood_load_releases
  dogfood_releases_field '.next.tag'; next="$DOGFOOD_FIELD"
  dogfood_prev_tag
  dogfood_prs_between "$DOGFOOD_PREV_TAG" HEAD
  window_tickets
  for t in $TICKETS; do
    dogfood_row_labels "$t"
    case " $DOGFOOD_LABELS " in
      *" release:${next} "*) echo "already  ${t} release:${next}" ;;
      *)
        if [ "$check" = "--check" ]; then
          echo "MISSING  ${t} release:${next}"
          missing=$((missing + 1))
        else
          label_row "$t" "release:${next}"
        fi
        ;;
    esac
  done
  [ -z "$STRAYS" ] || echo "stray ids (no row, no alias — declare one with: release-goal.sh alias STRAY OWNER): ${STRAYS}"
  [ -z "$UNTICKETED" ] || echo "PRs naming no roadmap ticket: ${UNTICKETED}"
  [ "$missing" -eq 0 ] || exit 1
}

# Write row $4 (cmd_window's text for tag $1) into the ledger, and make the
# next goal $2, due $3. The row goes after the last row, and `next:` is
# rewritten under it. `releases: []` opens as `releases:`. Text below the
# `next:` block is not kept. PMAT-607: this was a python3 heredoc.
book_row() {
  local path=docs/roadmaps/releases.yaml rc=0 cut
  local tmp="${path}.release-goal.tmp"
  RG_ROW="$4" RG_NEXT="$2" RG_DUE="$3" awk '
    $0 == "next:" { found = 1 }
    found { next }
    NR > 1 && !opened && $0 == "releases: []" { $0 = "releases:"; opened = 1 }
    { line[++n] = $0 }
    END {
      if (!found) exit 3
      while (n > 0 && line[n] == "") n--
      for (i = 1; i <= n; i++) print line[i]
      row = ENVIRON["RG_ROW"]
      sub(/\n+$/, "", row)
      print row
      print "next:"
      print "  tag: " ENVIRON["RG_NEXT"]
      print "  due: " ENVIRON["RG_DUE"]
    }' "$path" >"$tmp" || rc=$?
  if [ "$rc" -ne 0 ]; then
    rm -f "${tmp:?}"
    if [ "$rc" -eq 3 ]; then
      echo "release-goal: no next: block in ${path}" >&2
    else
      echo "release-goal: awk exited ${rc} writing ${path}" >&2
    fi
    exit 3
  fi
  mv "$tmp" "$path"
  cut="${4#*cut: }"
  cut="${cut%%$'\n'*}"
  echo "declared ${1} (cut ${cut}) and next ${2} due ${3}"
}

cmd_cut() {
  local tag="$1" next="" cadence_days due row lower t shipped
  shift
  while [ $# -gt 0 ]; do
    case "$1" in
      --next) next="${2:-}"; shift 2 ;;
      *) fail "unknown argument: $1" ;;
    esac
  done
  [ -n "$next" ] || fail "cut needs --next vX.Y.Z: the next goal is declared at the cut, never implied"
  git rev-parse -q --verify "refs/tags/${tag}" >/dev/null || fail "no such tag: ${tag} — cut declares a tag that exists"
  dogfood_load_releases
  dogfood_release_row "$tag"
  [ -z "$DOGFOOD_RELEASE_ROW" ] || fail "${tag} already has a row in docs/roadmaps/releases.yaml"
  dogfood_releases_field '.cadence_days'; cadence_days="$DOGFOOD_FIELD"
  dogfood_tag_date "$tag"
  dogfood_epoch "$DOGFOOD_TAG_DATE"
  dogfood_iso $((DOGFOOD_EPOCH + cadence_days * 86400)); due="$DOGFOOD_ISO"
  row="$(cmd_window "$tag")"
  book_row "$tag" "$next" "$due" "$row"
  # The labels: every ticket the tag shipped carries release:TAG; a ticket
  # that carried release:TAG as a goal and did not ship moves to release:NEXT
  # (the plan grill: a goal that missed the cut must not block it).
  lower_tag_of "$tag"; lower="$LOWER"
  dogfood_prs_between "$lower" "$tag"
  window_tickets
  shipped=" $TICKETS "
  for t in $TICKETS; do
    label_row "$t" "release:${tag}"
  done
  dogfood_rows_with_label "release:${tag}"
  for t in $DOGFOOD_ROWS; do
    case "$shipped" in
      *" $t "*) ;;
      *)
        unlabel_row "$t" "release:${tag}"
        label_row "$t" "release:${next}"
        ;;
    esac
  done
}

cmd="${1:-}"
if [ $# -gt 0 ]; then shift; fi
case "$cmd" in
  show) cmd_show ;;
  window) cmd_window "${1:-}" ;;
  tag) [ $# -eq 2 ] || fail "usage: release-goal.sh tag TICKET TAG"; label_row "$1" "release:$2" ;;
  alias) [ $# -eq 2 ] || fail "usage: release-goal.sh alias STRAY OWNER"; label_row "$2" "alias:$1" ;;
  sync) cmd_sync "${1:-}" ;;
  cut) [ $# -ge 1 ] || fail "usage: release-goal.sh cut TAG --next NEXT"; cmd_cut "$@" ;;
  *) fail "usage: release-goal.sh show | window [TAG] | tag TICKET TAG | alias STRAY OWNER | sync [--check] | cut TAG --next NEXT" ;;
esac
