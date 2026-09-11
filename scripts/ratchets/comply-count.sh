#!/usr/bin/env bash
# Count the findings ONE comply check reports, for the `.pmat-ratchet.toml`
# entry that ratchets it (PMAT-243).
#
# Usage: scripts/ratchets/comply-count.sh CB-2110
#
# Prints one integer on stdout and nothing else, because that is what
# `pmat comply ratchet` reads. Every unmeasurable outcome exits non-zero rather
# than printing a zero: the ratchet's own documentation says a measurement of 0
# against a non-zero baseline has two explanations — the largest improvement in
# the project's history, or a predicate that has rotted — and nothing in the
# number can tell them apart. A rotted check id would print 0 and read as
# perfection, so an id the roster does not carry is a FAILED measurement.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

want="${1:?usage: comply-count.sh CB-NNNN}"

rc=0
json="$(pmat comply check --format json 2>/dev/null)" || rc=$?
if [ -z "$json" ]; then
  echo "comply-count: pmat comply check produced no output (exit ${rc})" >&2
  exit 2
fi

printf '%s' "$json" | python3 -c '
import json, re, sys

want = sys.argv[1]
raw = sys.stdin.read()
i = raw.find("{")
if i < 0:
    sys.exit("comply-count: no JSON object in comply output")
try:
    d = json.loads(raw[i:])
except Exception as e:
    sys.exit("comply-count: comply output does not parse: %s" % e)

for c in d.get("checks", []) or []:
    name = str(c.get("name", ""))
    if not name.startswith(want + ":"):
        continue
    status = str(c.get("status", "")).lower()
    msg = str(c.get("message", ""))
    if status == "pass":
        print(0)
        sys.exit(0)
    # "44 finding(s) — ORPHAN-ROADMAP 24, …". The leading integer is the count
    # the check itself reports; this script never recounts it from the sample,
    # which is truncated to a handful of lines.
    m = re.match(r"\s*(\d+)\s+finding", msg)
    if m:
        print(int(m.group(1)))
        sys.exit(0)
    sys.exit("comply-count: %s reports %s but its message carries no finding count: %.120s"
             % (want, status, msg))

sys.exit("comply-count: no check named %s in the comply roster — the id has rotted, "
         "and a rotted id must not read as zero findings" % want)
' "$want"
