#!/usr/bin/env bash
# Dogfood gate F — the coverage floor (mutation runs nightly only).
#
# Exit code is the gate. The only line a caller must read is the final
# `GATE F PASS|FAIL <detail>`.
#
# WHY NO MUTATION HERE
#
# Mutation testing never runs in a build, a PR, a merge queue, a release gate or
# locally through this script: operator ruling 2026-11-10, "NO build can do
# mutation outside of nightly; the end. FOREVER". It lives in
# .github/workflows/mutation.yml, `schedule` trigger only (infra#3828).
#
# HOW THE FLOOR IS ENFORCED
#
# By `--fail-under-lines`, inside llvm-cov, never by comparing a percentage in
# shell. `Makefile:coverage-check` already records why at length: an EMPTY
# percentage — broken instrumentation, a changed summary format — makes
# `[ "$COV" -lt 95 ]` false, and the shell prints a pass on a run that measured
# nothing. Here an unmeasurable run is a non-zero exit from the tool that did the
# measuring, so "unmeasured" and "met" cannot be confused.
#
# THE FLOOR MAY NOT BE LOWERED. 95 is the number in CLAUDE.md and in the
# Makefile. Editing it down here is the cheapest way to make this gate green,
# which is why Arm 0 compares the copies instead of trusting this one.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

# The line-coverage floor, in percent. MAY NOT BE LOWERED.
FLOOR=95

# The vendored aprender-contracts suite (#452). `crates/forjar-contracts` is a
# byte-vendored copy of a crate whose tests read APRENDER's contract corpus —
# contracts/aprender/binding.yaml, softmax-kernel-v1.yaml, "100+ contracts".
# forjar vendors the crate and not the corpus, so those tests fail on NotFound:
# a fact about the vendoring, never about the code under test. They carry
# `#[cfg_attr(not(feature = "aprender-corpus"), ignore = "…")]` — compiled on
# every run, each printing its own reason, none deleted.
#
# BOTH NUMBERS ARE EXACT, NOT CEILINGS. A ceiling catches only the direction
# where somebody parks a new test, and silently accepts the other one: an
# ignored test deleted or un-ignored leaves the ceiling slack, and the next
# parked test then fits under it with nothing said. An exact figure is a ratchet
# in both directions — every change to the excluded set is a deliberate edit of
# these two lines, in the diff, where a reviewer sees it. That is the only way
# an exclusion mechanism stays honest.
APRENDER_ANNOTATIONS=39
IGNORED_EXPECTED=44

fail() {
  echo "GATE F FAIL $1"
  exit 1
}

# `grep` exits 1 on "no match", and `set -o pipefail` turns that into a silent
# script death in the middle of an assignment — a gate that prints nothing and
# exits non-zero is indistinguishable from one that never ran. Every place
# below where "no match" is a MEANINGFUL answer captures rc instead, so that
# "none" (1) and "grep could not run" (>=2, UNMEASURED) stay apart.

# ------------------------------------------- Arm 0: the floor has not drifted
# Every `--fail-under-lines` in the Makefile must carry the same number as
# $FLOOR. The comparison runs before anything expensive, so a lowered floor is
# reported in a second rather than after a coverage build.
mk_floors="$(sed -n 's/.*--fail-under-lines \([0-9]*\).*/\1/p' Makefile | sort -u)"
if [ -z "$mk_floors" ]; then
  fail "no --fail-under-lines in the Makefile: the repo's coverage floor would then be enforced nowhere but here"
fi
if [ "$mk_floors" != "$FLOOR" ]; then
  fail "coverage floor DRIFTED: this gate says ${FLOOR}, the Makefile says [$(printf '%s' "$mk_floors" | tr '\n' ' ')] — the floor is not the thing to change"
fi

# ----------------------------- Arm 1: the vendored contracts suite is not red
rc=0
contracts_out="$(cargo test -p forjar-contracts --lib 2>&1)" || rc=$?
grc=0
result_line="$(printf '%s\n' "$contracts_out" | grep '^test result:' | tail -1)" || grc=$?
if [ "$grc" -gt 1 ]; then
  fail "grep exited ${grc} reading the libtest summary — UNMEASURED"
fi
if [ -z "$result_line" ]; then
  printf '%s\n' "$contracts_out"
  fail "cargo test -p forjar-contracts --lib printed no 'test result:' line (exit ${rc}) — UNMEASURED, which is not the same as passing"
fi
failed="$(printf '%s' "$result_line" | sed -n 's/.*; \([0-9]*\) failed;.*/\1/p')"
ignored="$(printf '%s' "$result_line" | sed -n 's/.*; \([0-9]*\) ignored;.*/\1/p')"
if [ -z "$failed" ] || [ -z "$ignored" ]; then
  fail "cannot parse the libtest summary '${result_line}' — the format changed and this arm now measures nothing"
fi
if [ "$failed" -ne 0 ]; then
  printf '%s\n' "$contracts_out"
  fail "cargo test -p forjar-contracts --lib: ${failed} failed"
fi
if [ "$ignored" -gt "$IGNORED_EXPECTED" ]; then
  fail "forjar-contracts ignores ${ignored} tests, recorded ${IGNORED_EXPECTED} — a test was parked rather than fixed"
fi
if [ "$ignored" -lt "$IGNORED_EXPECTED" ]; then
  fail "forjar-contracts ignores ${ignored} tests, recorded ${IGNORED_EXPECTED} — the ignored set SHRANK, which is good news this gate has not been told about: set IGNORED_EXPECTED=${ignored} in scripts/dogfood/coverage.sh and commit it with the change that un-ignored the test, so that the smaller number is what the next parked test has to fit under"
fi

# An exclusion is honest only while it covers the set it was written for, so the
# annotations are counted in the source and not merely trusted.
grc=0
annotated="$(grep -rc 'not(feature = "aprender-corpus")' crates/forjar-contracts/src --include='*.rs' | awk -F: '{n += $2} END {print n + 0}')" || grc=$?
if [ "$grc" -gt 1 ]; then
  fail "grep exited ${grc} counting the aprender-corpus annotations — the ratchet is UNMEASURED"
fi
if [ "$annotated" -gt "$APRENDER_ANNOTATIONS" ]; then
  fail "${annotated} tests are gated behind aprender-corpus, recorded ${APRENDER_ANNOTATIONS} — a new test was excluded rather than made to pass"
fi
if [ "$annotated" -lt "$APRENDER_ANNOTATIONS" ]; then
  fail "${annotated} tests are gated behind aprender-corpus, recorded ${APRENDER_ANNOTATIONS} — the exclusion shrank; lower APRENDER_ANNOTATIONS here to record it"
fi

# ------------------------------------------------------ Arm 2: the line floor
# --locked because a gate that silently updates Cargo.lock is measuring a
# dependency set nobody reviewed.
rc=0
cov_out="$(cargo llvm-cov --workspace --locked --fail-under-lines "$FLOOR" 2>&1)" || rc=$?
measured="$(printf '%s\n' "$cov_out" | awk '$1 == "TOTAL" {print $(NF - 3)}' | tail -1)"
if [ "$rc" -ne 0 ]; then
  printf '%s\n' "$cov_out"
  echo ""
  echo "--- the gaps, ranked by uncovered lines (pmat query --coverage-gaps --limit 40) ---"
  gap_rc=0
  gaps="$(pmat query --coverage-gaps --limit 40 2>&1)" || gap_rc=$?
  printf '%s\n' "$gaps"
  if [ "$gap_rc" -ne 0 ]; then
    echo "(pmat query --coverage-gaps exited ${gap_rc}; the floor below is still the verdict)"
  fi
  fail "line coverage ${measured:-UNMEASURED} is below the ${FLOOR}% floor (cargo llvm-cov exit ${rc}) — write the tests, do not move the floor"
fi

# Mutation is NOT run here. Operator ruling (2026-11-10 16:50Z, verbatim): "NO
# build can do mutation outside of nightly; the end. FOREVER". It runs only from
# the `schedule` trigger of .github/workflows/mutation.yml (infra#3828).
echo "GATE F PASS line coverage ${measured:-?} >= ${FLOOR}%; forjar-contracts ${failed} failed / ${ignored} ignored (${annotated} aprender-corpus annotations; both exactly as recorded); mutation runs nightly only"

# mutation: change FLOOR to 90 — Arm 0 then reports the floor as DRIFTED from
# the Makefile's two `--fail-under-lines 95` and the gate exits 1 in a second,
# which is the point: the floor cannot be lowered here alone.
