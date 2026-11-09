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

# ------------------------------------------------------------ RECURSION GUARD
#
# THIS SCRIPT MUST NEVER RUN INSIDE ITSELF (forjar#522).
#
# `pmat comply check` evaluates `.pmat-ratchet.toml` (CB-2102) and RUNS every
# measurement command it declares. A ratchet entry whose command is this script
# is therefore a cycle by construction, and neither side had a guard. Measured
# on Lambda-Vector, 2026-09-11, pmat 3.40.0, 48 cores, from an idle box:
#
#   total processes        9,740
#   comply-count.sh        5,781
#   pmat comply check      1,462
#   load average (1 min)   379, peaking at 3,026
#   CPU pressure some/10s  83%
#
# The `.pmat-ratchet.toml` that caused it was never committed and is not on
# disk, so the cycle cannot occur in this repository today. That is not a
# safeguard — it is the absence of one input — and the next person to declare a
# ratchet metric would rediscover it on a shared machine. Two things stop it
# here: a sentinel that refuses the re-entry outright, and a process cap so a
# cycle nobody anticipated dies in the hundreds rather than the thousands.
#
# The refusal exits NON-ZERO and prints nothing on stdout. A 0 would read as
# "this check reports no findings", which is the largest improvement in the
# project's history spelled identically to a script eating the machine.
if [ -n "${COMPLY_COUNT_ACTIVE:-}" ]; then
  echo "comply-count: refusing to run inside itself — pmat comply check re-entered the ratchet (forjar#522)" >&2
  exit 3
fi
export COMPLY_COUNT_ACTIVE=1

# A cycle this guard does not anticipate still dies small.
#
# `ulimit -u` is RLIMIT_NPROC, which on Linux is PER USER and counts THREADS,
# not processes. Both halves of that were learned the hard way here:
#
#   a fixed `ulimit -u 256`        killed this script's own fork at once —
#                                  the account was already running 226
#                                  PROCESSES, and 2,352 THREADS
#   `processes + 64`               did too, for the same reason: the limit is
#                                  compared against the thread count
#   measured, this account          2,352 threads at rest, 2,486 at the peak of
#                                  one `pmat comply check` — so a comply run
#                                  costs about 134 threads
#
# A cap that fails on a busy machine is the gate going red for the wrong
# reason, which is the defect this whole file exists to avoid. So the cap is
# RELATIVE and measured in the unit the kernel actually compares: threads now,
# plus room for several comply runs and no more. A cycle then dies within its
# first level or two instead of at 9,740 processes.
#
# AND IT FAILS CLOSED. An unreadable thread count or a refused `ulimit` used to
# print a warning and run the measurement anyway. Three review lanes refuted
# that, and they are right: a warning on stderr, in a gate whose caller captures
# stdout, is indistinguishable from no cap at all — and this repository has now
# measured what no cap costs. An unbounded run is an UNGUARDED run, and a red
# gate is the cheaper of the two failures by a factor of 9,740.
threads="$(ps -u "$(id -un)" -L --no-headers 2>/dev/null | wc -l)" || threads=""
case "$threads" in
  ''|0|*[!0-9]*)
    echo "comply-count: cannot read this account's thread count (ps), so a runaway cycle could not be bounded — refusing to measure rather than run unguarded (forjar#522)" >&2
    exit 4
    ;;
esac
if ! ulimit -u $((threads + 512)) 2>/dev/null; then
  echo "comply-count: ulimit -u refused at $((threads + 512)), so a runaway cycle could not be bounded — refusing to measure rather than run unguarded (forjar#522)" >&2
  exit 4
fi

want="${1:?usage: comply-count.sh CB-NNNN}"

rc=0
json="$(pmat comply check --format json 2>/dev/null)" || rc=$?
if [ -z "$json" ]; then
  echo "comply-count: pmat comply check produced no output (exit ${rc})" >&2
  exit 2
fi

case "$json" in
  *"{"*) body="{${json#*\{}" ;;
  *) echo "comply-count: no JSON object in comply output" >&2; exit 1 ;;
esac

# First check whose name starts "<want>:" decides; the output is a count or an
# "E:" line (error text for stderr). A rotted id must not read as zero findings.
res=$(printf '%s' "$body" | jq -r --arg want "$want" '
  ([(.checks // [])[] | select((.name // "" | tostring) | startswith($want + ":"))] | .[0]) as $c
  | if $c == null then
      "E:comply-count: no check named \($want) in the comply roster — the id has rotted, and a rotted id must not read as zero findings"
    elif (($c.status // "") | tostring | ascii_downcase) == "pass" then "0"
    else
      (($c.message // "") | tostring) as $m
      # "44 finding(s) — ORPHAN-ROADMAP 24, …". The leading integer is the count
      # the check itself reports; this script never recounts it from the sample,
      # which is truncated to a handful of lines.
      | ([$m | capture("^\\s*(?<n>[0-9]+)\\s+finding")] | .[0]) as $r
      | if $r then ($r.n | tonumber | tostring)
        else "E:comply-count: \($want) reports \(($c.status // "") | tostring | ascii_downcase) but its message carries no finding count: \($m[0:120])"
        end
    end' 2>&1) || { echo "comply-count: comply output does not parse: $res" >&2; exit 1; }
case "$res" in
  E:*) echo "${res#E:}" >&2; exit 1 ;;
  *) printf '%s\n' "$res" ;;
esac
