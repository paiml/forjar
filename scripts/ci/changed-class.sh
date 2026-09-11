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
# THE ONES THAT LOOK HARMLESS AND ARE NOT. Every path in the first group of the
# case below is read BY NAME by something the PR lane runs, and every one of
# them lives under a prefix whose siblings ARE harmless — which is exactly how a
# classifier of this kind goes wrong. The list was not guessed: it is every
# `CARGO_MANIFEST_DIR`-joined path under `docs/` or `.quorum/` found in `tests/`
# and `src/`, and a case in the falsification suite re-derives it, so a test that
# starts reading a new record file turns the suite red rather than blinding
# itself.
#
# MEASURED, AND SMALLER THAN IT LOOKS. Of the ten most recently merged PRs,
# eight touched no `src/` — but only THREE are confined to the record and would
# skip the heavy jobs; the rest also changed scripts, tests or workflows, which
# are code. Three in ten is the honest figure, and it is the one the receipt
# carries.
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
    # Read BY NAME by something the PR lane runs. Listed before the harmless
    # prefixes they live under, because that is exactly how a classifier of
    # this kind goes wrong: the siblings are harmless and these are not.
    README.md) code=true ;;                        # gate D runs its fenced forjar blocks
    CHANGELOG.md) code=true ;;                     # the crux-gate tests read it
    contracts/*) code=true ;;                      # gate G validates the corpus
    docs/audits/surface_audit.csv) code=true ;;    # gate C diffs the live surface against it
    docs/audits/crux-*) code=true ;;               # falsification_crux_audit_shape reads them
    docs/specifications/*) code=true ;;            # read by name from a test
    docs/book/*) code=true ;;                      # the book is asserted against the live surface
    docs/mcp-schema.json) code=true ;;             # the checked-in-copy test names it
    .quorum/enforce.json) code=true ;;             # the quorum gate reads it
    # The record: roadmap rows, the release ledger, receipts, logs, evidence
    # and per-branch quorum artifacts. Nothing the PR lane runs reads any of
    # them, and a case in the falsification suite re-derives this exclusion
    # list from the tree so that it stays true as the suite grows.
    docs/*|.quorum/*) ;;
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
