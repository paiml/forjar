# Implementation receipt — PMAT-598 — jobs that need rsync or the provers run where those exist

verdict: PASS — merged as PR #599 (7bfb5720). Five jobs now ask `runs-on: [self-hosted, clean-room, X64, intel]`: proofs/kani and proofs/lean, and coverage, stress and mutation, which run unfiltered lib tests including nas_archive's rsync-requiring safety test. Positive control on the PR, each on a named bare-metal runner: kani on intel-clean-room-10, lean on intel-clean-room-8, coverage on intel-clean-room-4 — all three had been red on yoga containers. Quorum gate passed over three rounds, 5 confirmed and 9 refuted.

## How this was implemented — stated, not inferred

Directly, in a forjar session, not through the paiml-implement harness; no
discover.json, route.sh verdict or dispatch ledger exists and none is invented.
Quorum: `.quorum/PMAT-598-tool-jobs-on-bare-metal.json`.

## What was measured

    intel-clean-room-N   clean-room,intel,build               bare metal   lean kani rsync PRESENT
    yoga-build*          clean-room,yoga,ephemeral,docker,..  containers   lean kani rsync ABSENT

Commit 537252d3: four proofs runs went fail/pass/fail/pass purely by runner. The
inventory had to be taken in the job's venue — an ssh login shell reported lean
absent on intel, a false negative.

## What three quorum rounds changed

Round 1 widened the fix from three jobs to five. Rounds 2 and 3 each found a
false negative in the test's detector, round 3's in round 2's own fix. Round 3
also produced a lane hand-trace that was wrong — it reported a `&&` chain as
classified correctly — refuted the moment it became an asserted case.

## Gaps

- `intel` is a hostname standing in for a capability; it comes out when a
  capability label exists on the fleet or yoga's images converge.
- The final two detector refinements were made after round 3 and no lane has
  reviewed them; asserted cases and RED proofs cover them.
- `ci / test` runs through the reusable sovereign-ci workflow in paiml/.github and
  is not routed by this change.
- On main after merge, `mutation` landed correctly on intel-clean-room-2 and
  failed for a different reason: cargo-mutants 27.1.0 needs rustc 1.91 and the
  runner resolved 1.89.0 (forjar#566).

IMPL-PMAT-598-RECEIPT-END
