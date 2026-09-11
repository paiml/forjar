#!/usr/bin/env bash
# PMAT-237 — what can this change break?
#
# Reads one path per line on stdin and prints `code=true` or `code=false`.
# `true` means the heavy jobs — the workspace suite, the dogfood gates, the
# ledger replay, coverage, the benchmarks — can possibly be affected and must
# run. `false` means the change is confined to paths none of them reads.
#
# WHY THIS EXISTS. Measured on the ten most recently merged PRs: eight touched
# no `src/` at all, and every one of them still spent about 131 job-minutes on
# jobs that build the binary — ledger-replay 34, dogfood 26, bench 18, the
# workspace tests 14, coverage 12. The release gate is unchanged and still runs
# everything; this decides only what a PR pays for what it actually changed.
#
# IT IS AN ALLOW-LIST OF THE HARMLESS, NOT A DENY-LIST OF THE HEAVY.
#
# A deny-list fails open: a path nobody classified looks harmless and the suite
# is skipped over it. This names the paths a change may touch without any heavy
# job being able to notice; everything else is code. Being wrong in this
# direction costs forty minutes of CI, being wrong in the other ships untested
# code, and those are not the same mistake.
#
# THE THREE THAT LOOK HARMLESS AND ARE NOT:
#   README.md                      gate D runs every fenced `forjar …` block in it
#   docs/audits/surface_audit.csv  gate C diffs the live surface against it
#   contracts/**                   gate G validates the corpus and its citations
# All three live under paths whose siblings ARE harmless, which is exactly how
# a classifier of this kind goes wrong.
set -euo pipefail

code=false
seen=false
# `|| [ -n "$f" ]` — a final line with no trailing newline is still a file.
# `git diff --name-only` always ends with one, so this never bites in CI; a
# caller piping a bare string does not, and the last path would be dropped
# SILENTLY, which is the fail-open direction. Found by the falsification suite
# before this ever ran anywhere.
while IFS= read -r f || [ -n "$f" ]; do
  [ -n "$f" ] || continue
  seen=true
  case "$f" in
    README.md|docs/audits/surface_audit.csv|contracts/*) code=true ;;
    docs/*|.quorum/*|CHANGELOG.md) ;;
    *) code=true ;;
  esac
done

# No readable file list is not a licence to skip anything.
if [ "$seen" = false ]; then
  code=true
fi

echo "code=${code}"

# mutation: change the `*) code=true ;;` arm to `*) ;;` — an unclassified path
# then reads as harmless, a PR touching src/ skips every heavy job, and
# `an_unclassified_path_is_code` plus every positive case in
# tests/falsification_pr_lane_runs_what_the_change_can_break.rs go RED.
