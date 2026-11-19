#!/usr/bin/env bash
# PMAT-1200 roadmap lint: a nightly-only check, never run in a PR, merge-queue or release job.
# Usage: roadmap-lint.sh [ROADMAP.yaml]   exit 0 clean, 1 findings, 2 cannot read.
#  1. A plain scalar (sequence item or `key: value`, any indent) with a comment marker
#     (space/tab then `#`, or a leading `#` after `- ` / `key: `) is cut short by YAML.
#  2. Every row of a named id is unique, in an allowed state, with non-blank acceptance
#     criteria; a named id with no row fails; all but PMAT-1200 keep their `#<issue>` text.
#  3. Layouts the line scanner cannot read stop the run with "unsupported layout": CR line
#     ends, a row not opened by `- id:` at column 0, criteria indented deeper than 2 spaces,
#     or a blank line between `acceptance_criteria:` and its first item.
# Known gaps: quoted, block (| >) and flow ([ {) scalars, anchors (&), aliases (*), tags (!)
# and quoted keys are not inspected, so a wrapped plain scalar whose continuation line starts
# with a quote or one of those characters is not checked; a wrapped quoted scalar whose
# continuation line has ` #`, and a ` #` in a block-scalar body, are reported although YAML
# keeps them; a trailing comment after a quoted `status:` is reported as a wrong state;
# completed rows outside the named ones may still have empty criteria.
set -u
export LC_ALL=C
f=${1:-"$(dirname "$0")/../../docs/roadmaps/roadmap.yaml"}
{ [ -f "$f" ] && [ -r "$f" ]; } || { echo "roadmap-lint: cannot read $f" >&2; exit 2; }
case $f in /*) ;; *) f=./$f ;; esac   # a name containing `=` is not an awk assignment
awk() { command "${AWK:-awk}" "$@"; }
awk '
BEGIN {
  ok["PMAT-1048"] = ok["PMAT-1049"] = ok["PMAT-1114"] = ok["PMAT-1115"] = " completed "
  ok["PMAT-1155"] = ok["PMAT-1200"] = " completed inprogress "
}
function bad(msg) { print FILENAME ":" NR ": " msg; n++ }
function die(msg) { bad("unsupported layout: " msg); stop = 1; exit 1 }
function flush(  t, i, found) {
  if (rid == "") return
  if (rid in ok) {
    if (rid in seen) bad(rid ": duplicate row")
    seen[rid] = 1
    if (index(ok[rid], " " rst " ") == 0) bad(rid ": status `" rst "` not one of" ok[rid])
    if (nc == 0 || blank) bad(rid ": acceptance criteria empty or blank")
    else if (rid != "PMAT-1200") {
      found = 0
      for (i = 1; i <= nc; i++) { t = " " crit[i] " "; if (match(t, "#" substr(rid, 6) "[^0-9]")) found = 1 }
      if (!found) bad(rid ": criteria lost their #" substr(rid, 6) " text")
    }
  }
  rid = ""
}
{
  line = $0; s = line; sub(/^[ \t]+/, "", s)
  if (line ~ /\r/) die("CR line ends")
  if (s ~ /^#/) next
  # rule 3
  if (line ~ /^ +- id: / || (line ~ /^- / && line !~ /^- id: /)) die("a row must open with `- id:` at column 0")
  if (inc && nc == 0 && s == "") die("blank line before the first criterion")
  if (inc && line ~ /^   +- /) die("criteria must be indented 2 spaces")
  # rule 1
  r = s; pre = 0
  while (r ~ /^- /) { sub(/^- +/, "", r); pre = 1 }
  if (match(r, /^[A-Za-z0-9_-]+: +/)) { r = substr(r, RLENGTH + 1); pre = 1 }
  if (r != "" && r !~ /^[\047"|>\[{&*!]/ && ((pre && r ~ /^#/) || r ~ /[ \t]#/)) bad("comment marker cuts the scalar: " substr(r, 1, 60))
  # rule 2
  if (line ~ /^- id: /) { flush(); rid = line; sub(/^- id: +/, "", rid); gsub(/[\047" ]/, "", rid); rst = ""; nc = 0; blank = 0; inc = 0; next }
  if (line ~ /^  status: /) { rst = line; sub(/^  status: +/, "", rst); gsub(/[\047" ]/, "", rst) }
  if (line ~ /^  acceptance_criteria:/) { inc = (line !~ /\[\]/); next }
  if (inc && line ~ /^  -([ \t]|$)/) {
    t = line; sub(/^  -[ \t]*/, "", t); sub(/[ \t]+$/, "", t)
    if (t ~ /^\047.*\047$/ || t ~ /^".*"$/) t = substr(t, 2, length(t) - 2)
    c = tolower(t); gsub(/[ \t]/, "", c)
    if (c == "" || c == "null" || c == "~") blank = 1
    crit[++nc] = t; next
  }
  if (inc) inc = 0
}
END { if (stop) exit 1; flush(); for (id in ok) if (!(id in seen)) { print FILENAME ": " id ": row missing"; n++ }; exit n ? 1 : 0 }
' "$f"
