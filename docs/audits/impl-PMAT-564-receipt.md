# Implementation receipt — PMAT-564 — drift declines over zero coverage, and never grades another manifest's resources

verdict: PASS — `forjar drift -f X` whose declared resources were inspected 0 of N exits 2 with `declined: inspected 0 of N declared; no lock holds them` (or the skip reasons); a locked resource X does not declare is skipped as `in the lock, not in the config` by every detector — file, task, state-query and, after a review lane read it, image — and never graded under X's name. N is the config's count, never the lock's. An unmeasured resource keeps exit 4; one inspected resource is enough to be graded. Six cases through the binary, 5 RED on the unfixed source; five mutations run.

## Identity

- ticket PMAT-564 (forjar#564, milestone 1.31.0); kind: code
- branch `PMAT-564-drift-declines-on-empty-scope`, stacked on `PMAT-562-drift-exits-on-drift` (PR #563), to be rebased onto `main` once #563 merges
- discover.json: sha256 6f86fa8d3580dad9 (discovery ran once in the main checkout; the worktree inherits it) — `gate_cmd_fallback=true`
- status-line join: not measured on this ticket (one goal declaration per session; this is the session's third ticket)

## What was measured

    $ forjar drift -f machines/yoga/forjar-ephemeral.yaml        # 1.30.0, yoga
      inspected 2 of 103 resource(s) in scope: file 2
      skipped 101: declared here, absent from the lock 10, in the lock, not in the config 65, …
      DRIFTED: gitconfig on yoga (…)   DRIFTED: zshrc on yoga (…)
    Drift detected: 2 resource(s)
    rc=0

Inspected 0 of the 10 the manifest declares; graded two files from
`forjar.yaml`, which the run was not given — in the very category its own
census called skipped. Reproduced locally: stack A applied, stack B (three
resources, never applied), `drift -f b.yaml` graded A's tampered file and
exited 1 (post-PMAT-562; 0 before it).

Mechanism: `tripwire::drift::file::detect_drift_with_lifecycle` walked the
lock's file entries and never asked whether the config declared them; the
state-query detector always had. And `image::detect_image_drift` had the same
shape — found by a review lane, not by the author.

## Plan and routing

| phase | route (route.sh, verbatim) | executor |
|---|---|---|
| 1 RED test | `route=agy-goal w=1.00 basis=absent note=fable-binding effort=1[U] bucket_collision=true` | direct |
| 2 fix, contract, docs | same | direct |
| 3 review | `route=agy-quorum w=1.00 basis=absent effort=1[U]` | delegate, quorum ×3 |
| 4 receipt, push, PR | `route=self w=100.00 basis=absent` | self |

## Dispatch ledger

| dispatch | mode | agent | turns | maxTurns | resumed | lanes / conversations |
|---|---|---|---|---|---|---|
| PMAT-564/ph3.delegate | paiml-agy-delegate (opus) | a03228c0 | 21 | no | no | quorum ×3: conv-55a57952 FAIL, conv-b5db623c PASS, conv-abd28ef5 PASS; child_conversations=3 |

slots used: 1 of 3 at peak · denials: 0 · I-3: attempted=1 denied=0 running_peak=1 slots=3 (the transcript gate measures 0 from the worktree's project dir — vacuous there, the ledger above is the count).

## Verification (claimed vs re-run)

| check | claimed | re-run here |
|---|---|---|
| the acceptance suites (6) | lane 2: 39/0; lane 3: 17 result lines; lane 1: pass | all green; the workspace at bd1664a8: 19,699 pass, 88 doctest failures from a shared-target race, doctests alone 88/88 |
| image detector honours the scope | lane 1: NO (measured); lanes 2, 3: "every detector" true (neither read image.rs) | lane 1 right: guard absent at image.rs:36; added with a unit test, M5 kills it |
| N pre- or post-expansion | lanes 1, 3: pre; lane 2: post | lane 2 right: `parse_and_validate` expands before `cmd_drift` sees the config |
| gate C / D / G | — | PASS / PASS / PASS |
| gate B | — | FAIL on main's CB-21xx debt (36 / 34 / 49 here; 36 / 34 / 50 on main) |
| clippy / fmt | — | clean / clean |

## Falsification

`tests/falsification_drift_declines_on_empty_scope.rs`, through the binary:
the decline with its count; another manifest's tampered file never graded; the
census never both inspects and skips; a partially locked manifest is graded
(control) and its drift is a reject; the applied manifest is fully inspected;
`--json` declines with the same code. RED 5/6 with `src/` reverted under the
test file. Three older cases that asserted exit 0 over `inspected 0 of N`
(`--no-task-checks` twice, a guard the lock never heard of) now assert the
decline they always described.

| mutation | kills |
|---|---|
| M1 delete the `decline_on_empty_scope` call | the regression, the `--json` case |
| M2 `declared_only = false` in the config path | 5 of 6 — every case but the fully-inspected control |
| M3 drop the unmeasured guard (= the first cut) | 2 FALSIFY-549 cases: exit 2 where 4 is required |
| M4 decline whenever declared > 0 | the partially-locked control, the applied manifest |
| M5 drop the image detector's config guard | `an_undeclared_locked_image_is_skipped_as_not_in_config` |

Each ran against the committed tree and was restored with `git checkout HEAD --`.

## Jidoka

- RED: the first cut declined over an UNREACHABLE host (two FALSIFY-549
  cases). Five-whys: inspected == 0 ← the unmeasured resource is not
  "inspected" ← the census keeps unmeasured apart from both inspected and
  skipped (forjar#549, by design) ← the decline predicate read only
  `inspected` ← it was written before running the neighbouring suites. Fixed:
  `total_unmeasured > 0` returns before the decline; the cases went green
  untouched.
- RED (review): the image detector graded undeclared locked images. Fixed with
  a guard and a unit test; the contract now names every detector and the
  census rule (`inspected` overwrites a skip) that makes each guard load-bearing.

## Estimates

K̂=8 [U], K=16; actual: 12 orchestrator turns from mint to this receipt. `basis=first-run[U]`.

## Gaps, named

- **`--dry-run` still exits 0 with nothing inspected** — it previews and
  returns before the scan; every lane named it, none called it a defect, and
  it is not: a preview has no verdict to give. Recorded so nobody reads the
  decline as covering it.
- **Two of three lanes shared a model id** (gemini-3.8-flash-high 503'd all
  day); the two resamples agreed with each other and were wrong about
  expansion timing. `partial_reasons` records the duplicate; the merge-rail
  round runs its own three lanes.
- **Gate B is main's red** (CB-2112, CB-2115); this branch measures equal or
  better. The 1.31.0 booking PR is where it closes.
- **`drift` counts findings, not resources**, in `Drift detected: N` (one
  tampered file prints two DRIFTED lines). Pre-existing; not this ticket.

IMPL-PMAT-564-RECEIPT-END
