#!/usr/bin/env bash
# Dogfood gate F — the coverage floor, and the mutants the floor cannot see.
#
# Exit code is the gate. The only line a caller must read is the final
# `GATE F PASS|FAIL <detail>`.
#
# WHY BOTH HALVES
#
# Line coverage answers "was this line executed", never "would anything have
# noticed if it were wrong". A suite that calls every function and asserts
# nothing reports 100%. So the floor is necessary and is not sufficient, and
# this gate runs the sufficient half too: `cargo mutants` over EXACTLY the lines
# this branch changed. A survivor is a mutation of new code that the whole suite
# accepted — new code no test would have caught being wrong.
#
# `--in-diff` and not a full sweep: a whole-repo mutation run is hours, so a gate
# built on it would be disabled within a week. Scoped to the diff it is minutes,
# and it asserts the property that actually matters on a release branch — that
# what CHANGED is guarded by something with teeth.
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

# --------------------------- Arm 3: mutants over exactly what this branch changed
if ! git rev-parse --verify --quiet origin/main >/dev/null; then
  fail "no origin/main to diff against: the in-diff scope cannot be computed, so this arm would mutate nothing and say so in green"
fi
work="$(mktemp -d)"
diff_file="${work}/branch.diff"
git diff "origin/main...HEAD" -- '*.rs' >"$diff_file"
grc=0
changed_rs="$(git diff --name-only "origin/main...HEAD" -- '*.rs' | grep -c .)" || grc=$?
if [ "$grc" -gt 1 ]; then
  fail "grep exited ${grc} counting the changed .rs files — the mutation scope is UNMEASURED"
fi
# MUTABLE .rs files, not all of them. `cargo mutants` mutates library and binary
# targets; it does not mutate `tests/`, `benches/` or `examples/`. A branch whose
# only Rust change is a new integration test therefore produces zero mutants and
# no outcomes.json — which the UNMEASURED arm below read as "the tool did not
# run". Measured 2026-09-08 on the 1.26.0 release cut, whose Rust change is one
# falsification test: `INFO No mutants to filter`, exit 0, no outcomes.json, and
# the release gate went red on a branch with nothing to mutate.
#
# Counting the mutable set instead keeps both halves honest: a diff that touches
# a src/ file still MUST produce mutants and kill them, and a tests-only diff
# passes this arm while every other arm still measures it.
mrc=0
# `-z`: `git diff --name-only` QUOTES a path containing a space or a non-ASCII
# byte ("src/a b.rs"), and a leading quote defeats an anchored match, so a diff
# that does touch src/ could read as zero mutable files (found by the PMAT-165
# quorum). NUL-delimited output is never quoted.
mutable_rs="$(git diff --name-only -z "origin/main...HEAD" -- '*.rs' \
  | tr '\0' '\n' | grep -E '(^|/)src/' | grep -c .)" || mrc=$?
if [ "$mrc" -gt 1 ]; then
  fail "grep exited ${mrc} counting the mutable changed .rs files — the mutation scope is UNMEASURED"
fi
if [ "$changed_rs" -gt 0 ] && [ "$mutable_rs" -eq 0 ]; then
  rm -rf "${work:?}"
  echo "GATE F PASS line coverage ${measured:-?} >= ${FLOOR}%; forjar-contracts ${failed} failed / ${ignored} ignored (${annotated} aprender-corpus, ceiling ${IGNORED_EXPECTED}); mutants: ${changed_rs} .rs file(s) changed, none under src/ — cargo mutants has no library or binary target to mutate in this diff"
  exit 0
fi
if [ "$changed_rs" -eq 0 ]; then
  rm -rf "${work:?}"
  echo "GATE F PASS line coverage ${measured:-?} >= ${FLOOR}%; forjar-contracts ${failed} failed / ${ignored} ignored (${annotated} aprender-corpus annotations; both exactly as recorded); no .rs differs from origin/main, so there is nothing to mutate"
  exit 0
fi

rc=0
cargo mutants --in-diff "$diff_file" --timeout 300 --output "$work" >"${work}/mutants.log" 2>&1 || rc=$?
outcomes="${work}/mutants.out/outcomes.json"
if [ ! -f "$outcomes" ]; then
  cat "${work}/mutants.log"
  fail "cargo mutants wrote no outcomes.json (exit ${rc}) — the mutation arm is UNMEASURED, and unmeasured is not clean; artifacts left in ${work}"
fi
total="$(jq -r '.total_mutants // (.outcomes | length)' "$outcomes")"
missed="$(jq -r '[.outcomes[] | select(.summary == "MissedMutant")] | length' "$outcomes")"
timeouts="$(jq -r '[.outcomes[] | select(.summary == "Timeout")] | length' "$outcomes")"
if [ "$total" -eq 0 ]; then
  fail "cargo mutants generated 0 mutants from a diff touching ${changed_rs} .rs file(s) — the in-diff scope resolved to nothing, so this arm proved nothing; artifacts in ${work}"
fi
if [ "$missed" -gt 0 ]; then
  jq -r '.outcomes[] | select(.summary == "MissedMutant") | .scenario.Mutant | "  SURVIVED \(.file):\(.span.start.line) \(.genre) -> \(.replacement)"' "$outcomes"
  fail "${missed} of ${total} mutant(s) of this branch's OWN changes survived the whole suite: that code can be wrong and nothing notices; artifacts in ${work}"
fi
if [ "$timeouts" -gt 0 ]; then
  fail "${timeouts} of ${total} mutant(s) timed out at 300s — a timeout is not a catch, it is a mutant nobody judged; artifacts in ${work}"
fi
rm -rf "${work:?}"

echo "GATE F PASS line coverage ${measured:-?} >= ${FLOOR}%; forjar-contracts ${failed} failed / ${ignored} ignored (${annotated} aprender-corpus annotations; both exactly as recorded); ${total} in-diff mutant(s) over ${changed_rs} changed .rs file(s), 0 survived, 0 timed out"

# mutation: change FLOOR to 90 — Arm 0 then reports the floor as DRIFTED from
# the Makefile's two `--fail-under-lines 95` and the gate exits 1 in a second,
# which is the point: the floor cannot be lowered here alone.
