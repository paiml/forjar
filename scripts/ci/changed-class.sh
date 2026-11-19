#!/usr/bin/env bash
# PMAT-237 — what can this change break?
#
# Reads one path per line on stdin and prints TWO decisions:
#
#   code=true|false   whether the heavy jobs — the workspace suite, the dogfood
#                     gates, the ledger replay, the benchmarks — can possibly be
#                     affected and must run.
#   gate_c=true|false   whether gate C -- the surface measured from the built
#   gate_d=true|false   binary -- and gate D -- the documented invocations run
#                     against it -- can move. Two booleans and not one string,
#                     because a workflow `if:` compares them EXACTLY: GitHub's
#                     `contains()` is a case-insensitive SUBSTRING test, and a
#                     gate whose selection depends on which letters happen to
#                     appear in a summary word is a gate waiting to be wrong.
#   gates=C,D|C|D|none  the same decision as one readable token, for the run
#                     summary and for the workflow gate's own refusal of an
#                     unmeasured selection. `none`, never the empty string.
#
# PMAT-542 added the per-gate lines. `code=` alone is one boolean for every
# heavy job, so a change to `scripts/` or `tests/` — which genuinely needs the
# guard tests — also paid for a release build and two gates that read a surface
# it cannot reach. Measured on the 25 PRs merged as of cddf78cd, run through
# this very script: 3 were code=false and already skipped everything, 15 can
# move gate C or D, and 7 CANNOT and paid 21.3 minutes of a 25.2-minute
# critical path anyway.
#
# WHAT EACH GATE READS was derived by reading the two scripts, and the first
# version of this list was WRONG in the fail-open direction -- a review round
# measured it. Gate C runs scripts/dogfood/lib/binary.sh, diffs the live surface
# against docs/audits/surface_audit.csv, and evaluates
# tests/fixtures/dogfood/local-files.yaml two ways. Gate D runs the same binary
# resolver, every fenced invocation in README.md against the whole
# tests/fixtures/dogfood/ tree and a clone of paiml/forjar-cookbook, and
# reconciles the version it prints against Cargo.toml.
#
# `gates=` NEVER decides whether a gate is CORRECT, only whether the PR lane
# pays for it now. `make dogfood-release` is unchanged and runs A-H and T over
# the whole window before any tag, and a gate the PR lane did not select prints
# NOT-SELECTED rather than PASS -- an unmeasured check must never print what a
# passing check prints.
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
gate_c=false
gate_d=false
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

  # WHICH BINARY-MEASURING GATE, ALSO AN ALLOW-LIST OF THE HARMLESS.
  #
  # Same direction as `code=` and for the same reason: a deny-list of "things
  # that change the binary" fails open the moment someone adds a crate nobody
  # listed. This names what a change may touch WITHOUT the built binary, its
  # surface snapshot or README's invocations moving; everything else selects
  # both gates. Being wrong here costs 21 minutes; being wrong the other way
  # ships a surface nothing measured.
  case "$f" in
    # THE PREFIXES WHOSE SIBLINGS ARE HARMLESS AND WHICH ARE NOT, first. The
    # closure was derived by reading the two gates rather than their names:
    # each runs `bash lib/binary.sh` to resolve the binary under test (and
    # binary.sh sources nothing further), gate C evaluates
    # tests/fixtures/dogfood/local-files.yaml two ways and compares them, and
    # gate D runs every documented invocation against the whole
    # tests/fixtures/dogfood/ tree. All of that lives under `scripts/` and
    # `tests/`, which the harmless arm below names -- and the first version of
    # this case listed only the two gate scripts, so a PR editing the very
    # fixture gate C compares would have SKIPPED gate C. Found by a review
    # round, in the direction that ships an unmeasured surface.
    #
    # `lib/*` and not `lib/binary.sh`: a sibling that either gate starts
    # sourcing later is then already selected, and the directory holds four
    # files. `scripts/dogfood/` itself stays harmless -- harness.sh, quorum.sh,
    # comply.sh, tagged.sh and the rest are read by NEITHER gate, and selecting
    # the whole directory was measured to take the saving from 10 PRs in 25 to
    # 2, which is the other way to be wrong about this.
    scripts/dogfood/lib/*) gate_c=true; gate_d=true ;;
    scripts/dogfood/surface.sh) gate_c=true ;;
    scripts/dogfood/docs.sh) gate_d=true ;;
    tests/fixtures/*) gate_c=true; gate_d=true ;;
    # Read by BOTH: gate C diffs the live surface against this file, and gate D
    # checks every verb the README names against it.
    docs/audits/surface_audit.csv) gate_c=true; gate_d=true ;;
    # Gate D runs the fenced `forjar` invocations in README.md. Gate C cannot
    # see it: it reads the binary, the CSV and its one fixture.
    README.md) gate_d=true ;;
    # Cannot reach either gate. `tests/` outside the fixtures above is not
    # compiled into the release binary and is read by neither script;
    # `scripts/` outside scripts/dogfood/ is read by neither; `docs/` and
    # `.quorum/` outside the CSV named above are the record.
    tests/*|scripts/*|docs/*|.quorum/*|CLAUDE.md|.claude/*) ;;
    # Everything else -- src/, crates/, contracts/, build.rs, Cargo.toml,
    # Cargo.lock, .github/, examples/, and any path nobody has classified --
    # selects both.
    *) gate_c=true; gate_d=true ;;
  esac
done

# No readable file list is not a licence to skip anything.
if [ "$seen" = false ]; then
  code=true
  gate_c=true
  gate_d=true
fi

# `none`, never the empty string. An empty value cannot be told apart from a
# classifier that did not run, and the workflow gate refuses an empty `gates`
# for exactly that reason -- an unmeasured selection is not a selection.
# Explicit `if`, not `[ … ] && …`. This repository has the rule written down in
# ci.yml's own gate: an AND-list whose test fails is exempt from `set -e` only
# because the test is not the final command, which is a property of where the
# `&&` sits rather than of what the line means. An `if` cannot be broken by
# someone appending to the line.
gates=""
if [ "$gate_c" = true ]; then
  gates="C"
fi
if [ "$gate_d" = true ]; then
  gates="${gates:+$gates,}D"
fi
if [ -z "$gates" ]; then
  gates="none"
fi

echo "code=${code}"
echo "gate_c=${gate_c}"
echo "gate_d=${gate_d}"
echo "gates=${gates}"

# mutation: change the `*) code=true ;;` arm to `*) ;;` — an unclassified path
# then reads as harmless, a PR touching src/ skips every heavy job, and
# `an_unclassified_path_is_code` plus every positive case in
# tests/falsification_pr_lane_runs_what_the_change_can_break.rs go RED.
# mutation: change the `*) gate_c=true; gate_d=true ;;` arm to `*) ;;` — a PR
# touching src/ then skips the release build and both surface gates, and
# `an_unclassified_path_selects_both_gates` goes RED.
# mutation: delete the `tests/fixtures/*` arm — a PR editing the very fixture
# gate C compares two evaluations of then skips gate C, and
# `the_files_the_gates_read_select_exactly_their_own_gate` goes RED.
