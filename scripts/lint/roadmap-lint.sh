#!/usr/bin/env bash
# PMAT-1200 roadmap lint (nightly LAB lane; never run in a PR, merge-queue or release job).
# Usage: roadmap-lint.sh [ROADMAP.yaml]   exit 0 clean, 1 findings, 2 cannot read.
#  1. A plain scalar (sequence item or `key: value`, any indent) with a comment marker
#     (space/tab then `#`, or a leading `#` after `- ` / `key: `) is cut short by YAML.
#  2. Every row of a named id is in an allowed state with non-blank acceptance criteria;
#     a named id with no row fails; all but PMAT-1200 keep their `#<issue>` text.
# Known gaps: quoted, block (| >) and flow ([ {) scalars, anchors (&), aliases (*), tags (!)
# and quoted keys are not inspected; a wrapped quoted scalar whose continuation line has
# ` #` and a ` #` in a block-scalar body would be reported although YAML keeps them;
# completed rows outside the named ones may still have empty criteria.
set -u
f=${1:-"$(dirname "$0")/../../docs/roadmaps/roadmap.yaml"}
[ -r "$f" ] || { echo "roadmap-lint: cannot read $f" >&2; exit 2; }
awk '
BEGIN {
  ok["PMAT-1048"] = ok["PMAT-1049"] = ok["PMAT-1114"] = ok["PMAT-1115"] = " completed "
  ok["PMAT-1155"] = ok["PMAT-1200"] = " completed inprogress "
}
function bad(msg) { print FILENAME ":" NR ": " msg; n++ }
function flush(  id, t, i, found) {
  if (rid == "") return
  if (rid in ok) {
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
  if (s ~ /^#/) next
  # rule 1
  r = s; pre = 0
  while (r ~ /^- /) { sub(/^- +/, "", r); pre = 1 }
  if (match(r, /^[A-Za-z0-9_-]+: +/)) { r = substr(r, RLENGTH + 1); pre = 1 }
  if (r != "" && r !~ /^[\047"|>\[{&*!]/ && ((pre && r ~ /^#/) || r ~ /[ \t]#/)) bad("comment marker cuts the scalar: " substr(r, 1, 60))
  # rule 2
  if (line ~ /^- id: /) { flush(); rid = line; sub(/^- id: +/, "", rid); gsub(/[\047" ]/, "", rid); rst = ""; nc = 0; blank = 0; inc = 0; next }
  if (line ~ /^  status: /) { rst = line; sub(/^  status: +/, "", rst); gsub(/[\047" ]/, "", rst) }
  if (line ~ /^  acceptance_criteria:/) { inc = (line !~ /\[\]/); next }
  if (inc && line ~ /^  - ?/) { t = line; sub(/^  - ?/, "", t); gsub(/^[\047"]|[\047"]$/, "", t); c = t; gsub(/[ \t]/, "", c)
    if (c == "" || c == "null" || c == "~") blank = 1; crit[++nc] = t; next }
  if (inc) inc = 0
}
END { flush(); for (id in ok) if (!(id in seen)) { print FILENAME ": " id ": row missing"; n++ }; exit n ? 1 : 0 }
' "$f"
