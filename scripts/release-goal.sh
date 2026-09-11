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

# Add label $2 to row $1, textually and idempotently; bump the row's
# `updated:`. Prints one line saying what it did.
label_row() {
  python3 - "$1" "$2" "$STAMP" <<'PY'
import io, re, sys
ticket, label, now = sys.argv[1], sys.argv[2], sys.argv[3]
path = "docs/roadmaps/roadmap.yaml"
text = io.open(path, encoding="utf-8").read()
head = re.search(r"^- id: %s$" % re.escape(ticket), text, re.M)
if not head:
    print("release-goal: %s is not a row of %s" % (ticket, path), file=sys.stderr)
    sys.exit(3)
start = head.start()
after = re.search(r"^- id: ", text[head.end():], re.M)
end = head.end() + after.start() if after else len(text)
block = text[start:end]
if re.search(r"^  - %s$" % re.escape(label), block, re.M):
    print("already  %s %s" % (ticket, label))
    sys.exit(0)
if "\n  labels: []\n" in block:
    block = block.replace("\n  labels: []\n", "\n  labels:\n  - %s\n" % label, 1)
else:
    m = re.search(r"^  labels:\n((?:  - .*\n)*)", block, re.M)
    if not m:
        print("release-goal: %s has no labels: key" % ticket, file=sys.stderr)
        sys.exit(3)
    block = block[: m.end()] + "  - %s\n" % label + block[m.end():]
block = re.sub(r"^  updated: .*$", "  updated: %s" % now, block, count=1, flags=re.M)
io.open(path, "w", encoding="utf-8").write(text[:start] + block + text[end:])
print("labelled %s %s" % (ticket, label))
PY
}

# Remove label $2 from row $1, textually. Prints one line.
unlabel_row() {
  python3 - "$1" "$2" "$STAMP" <<'PY'
import io, re, sys
ticket, label, now = sys.argv[1], sys.argv[2], sys.argv[3]
path = "docs/roadmaps/roadmap.yaml"
text = io.open(path, encoding="utf-8").read()
head = re.search(r"^- id: %s$" % re.escape(ticket), text, re.M)
if not head:
    print("release-goal: %s is not a row of %s" % (ticket, path), file=sys.stderr)
    sys.exit(3)
start = head.start()
after = re.search(r"^- id: ", text[head.end():], re.M)
end = head.end() + after.start() if after else len(text)
block = text[start:end]
line = "  - %s\n" % label
if line not in block:
    print("absent   %s %s" % (ticket, label))
    sys.exit(0)
block = block.replace(line, "", 1)
if re.search(r"^  labels:\n(?!  - )", block, re.M):
    block = re.sub(r"^  labels:\n", "  labels: []\n", block, count=1, flags=re.M)
block = re.sub(r"^  updated: .*$", "  updated: %s" % now, block, count=1, flags=re.M)
io.open(path, "w", encoding="utf-8").write(text[:start] + block + text[end:])
print("removed  %s %s" % (ticket, label))
PY
}

# The tag just below $1 among the tags reachable from $1 -> LOWER.
lower_tag_of() {
  local rc=0 t
  t="$(git tag --list 'v*' --sort=-v:refname --merged "$1" | grep -v -x -F -- "$1" | head -1)" || rc=$?
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
  local tag="${1:-}" lower upper cutline receipts=""
  dogfood_load_releases
  dogfood_releases_field '.dogfood_floor'
  if [ -n "$tag" ]; then
    git rev-parse -q --verify "refs/tags/${tag}" >/dev/null || fail "no such tag: ${tag}"
    lower_tag_of "$tag"; lower="$LOWER"; upper="$tag"
    dogfood_tag_date "$tag"; cutline="    cut: ${DOGFOOD_TAG_DATE}"
    if dogfood_semver_ge "$tag" "$DOGFOOD_FIELD"; then
      receipts="    dogfood: docs/audits/dogfood-${tag#v}-receipt.md
    crux: docs/audits/crux-${tag#v}.md"
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
  DOGFOOD_WINDOW_SOFT=1 dogfood_prs_between "$DOGFOOD_PREV_TAG" HEAD
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
  python3 - "$tag" "$next" "$due" "$row" <<'PY'
import io, re, sys
tag, nxt, due, row = sys.argv[1:5]
path = "docs/roadmaps/releases.yaml"
text = io.open(path, encoding="utf-8").read()
m = re.search(r"^next:\n(?:  .*\n?)*", text, re.M)
if not m:
    print("release-goal: no next: block in %s" % path, file=sys.stderr)
    sys.exit(3)
text = text.replace("\nreleases: []\n", "\nreleases:\n", 1)
m = re.search(r"^next:\n(?:  .*\n?)*", text, re.M)
row = row.rstrip("\n") + "\n"
before = text[: m.start()].rstrip("\n") + "\n"
text = before + row + "next:\n  tag: %s\n  due: %s\n" % (nxt, due)
io.open(path, "w", encoding="utf-8").write(text)
print("declared %s (cut %s) and next %s due %s" % (tag, row.split("cut: ")[1].split("\n")[0], nxt, due))
PY
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
