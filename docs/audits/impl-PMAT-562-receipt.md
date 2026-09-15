# Implementation receipt — PMAT-562 — drift exits 1 on any DRIFTED line, with no flag

verdict: PASS — `forjar drift` returns its verdict as its exit code on every run: any DRIFTED line exits 1, an unmeasured-only run exits 4, a converged stack exits 0, and `--tripwire` is accepted and changes nothing. Measured on the fleet under 1.30.0 as `Drift detected: 2 resource(s)` then `rc=0` (paiml/infra#605, second signature); measured here as rc=1 through the binary. Five cases, four RED on 1.30.0, three mutations run, one of which survived and was closed.

## Identity

- ticket PMAT-562 (forjar#562, milestone 1.31.0); kind: code
- branch `PMAT-562-drift-exits-on-drift`, from `origin/main` @ 8b2ccedd
- discover.json: sha256 6f86fa8d3580dad9 (discovery ran once in the main checkout; the worktree inherits it) — `gate_cmd_fallback=true` (`cargo test --workspace` is the discovered fallback, not a repo-declared gate)
- status-line join: not measured on this ticket (a worktree per PR; the goal
  declaration was made once for the session)

## What was measured

    $ forjar drift -f machines/yoga/forjar-ephemeral.yaml      # 1.30.0, yoga
      DRIFTED: gitconfig on yoga (/home/noah/.gitconfig content changed)
      DRIFTED: zshrc on yoga (/home/noah/.zshrc content changed)
    Drift detected: 2 resource(s)
    rc=0

Reproduced locally at 8b2ccedd with one `file` resource on `127.0.0.1`,
applied, then tampered: rc=0 without a flag, rc=1 with `--tripwire`. So the
NUMBER was already 1 (FALSIFY-549-003 pins it); only the default moved. The
book said 2 in one table and 10 in another for a verdict that has exited 1
since #549 — three documents, three answers, none measured.

## Plan and routing

| phase | route (route.sh, verbatim) | trigger | executor |
|---|---|---|---|
| 1 RED test | `route=agy-goal w=1.00 basis=absent note=fable-binding effort=1[U] bucket_collision=true` | — | direct (one file, held context) |
| 2 fix + docs + contract | same line | — | direct |
| 3 review | `route=agy-quorum w=1.00 basis=absent effort=1[U]` | Phase 4 pre-PR | delegate, quorum ×3 |
| 4 receipt, push, PR | `route=self w=100.00 basis=absent` | — | self |

## Dispatch ledger

| dispatch | mode | agent | turns | maxTurns | resumed | lanes / conversations |
|---|---|---|---|---|---|---|
| PMAT-562/ph3.delegate | paiml-agy-delegate (opus) | ad9656ef | 30 | hit | no | quorum ×3: conv-0d819949 FAIL, conv-3bfbedf5 NO-VERDICT (503), conv-35640912 PASS; child_conversations=3 |

slots used: 1 of 3 at peak · denials: 0 · I-3: attempted=1 denied=0 running_peak=1 slots=3.
The delegate stopped at its cap without a receipt (paiml-implement#141, third time this session); lane files and `reduce.json` were read directly.

## Verification (claimed vs re-run)

| check | claimed | re-run here |
|---|---|---|
| `cargo test --test falsification_drift_verdict_reaches_the_exit_code` | lane 3: pass; lane 1: acceptance failed on two `tests_check_2` cases | 5/5; the two cases pass here (sandbox artefact) |
| `cargo test --workspace --no-fail-fast` | — | 339 targets, 19,779 pass, 2 fail at 8e3af871 (FJ-017 pinned the defect) → inverted, green |
| gate C / D / G | — | PASS / PASS / PASS |
| gate B | — | FAIL, CB-2112 36>35 and CB-2115 49>43 — main measures 36 and 50 (its debt, see below); CB-2114 34 = ceiling after the row got `release:` |
| clippy / fmt | — | clean / clean |

## Falsification

`tests/falsification_drift_verdict_reaches_the_exit_code.rs`, through the
binary: the regression (rc=1, no flag), the converged control (rc=0), the flag
is inert (exit, stdout AND stderr), `--json` exits 1, the help no longer sells
the flag. RED 4/5 at 3c879afa, on 1.30.0's code.

| mutation | kills |
|---|---|
| M1 restore `_tripwire_compat && ` before the return | 3 of 5 (regression, json, no-op); control survives |
| M2 let the flag decorate the error message | SURVIVED the stdout-only comparison → the test now compares stderr; re-run kills the no-op case |
| M3 reject unconditionally | the control, exactly |

Every mutation ran against the committed tree and was restored with
`git checkout HEAD --`; `git status --porcelain` was empty after each.

## Jidoka

- RED: gate B. Five-whys: CB-2114 +1 ← the new row had no `release:` ← the
  issue was on no milestone ← the mint step did not include the milestone ←
  the ceiling file's convention (issue first, milestone, textual `release:`)
  is prose, not a mint script. Fixed for this row; the convention is the
  owning mechanism (PMAT-243's ratchet file). Not filed: the remaining red
  is main's (below).
- RED: FJ-017 lib cases asserting `Ok(())` over drift — the test pinned the
  defect; inverted, with the reason in the test.

## Estimates

K̂=8 turns [U] (first-run for this ticket shape), K=16; actual: 14 of the
orchestrator's turns from mint to this receipt. `basis=first-run[U]`.

## Gaps, named

- **Gate B is red on main and stays red here**: CB-2112 36 (ceiling 35,
  ISSUE-CLOSED on the PMAT-547 and PMAT-555 rows, both merged today and not
  yet booked) and CB-2115 50 on main (five open issues, #557–#561, with no
  roadmap row). This branch measures 36 / 34 / 49 — equal or better on every
  check. The ledger booking PR of the 1.31.0 cut is where those close;
  `make dogfood-release` cannot pass before it.
- **Exit 1 is shared with a general error.** A script that needs "drift"
  distinct from "could not run" reads `--json`'s `drift_count`; the book
  says so. The reserved 10 was never emitted.
- **One tampered file resource prints two DRIFTED lines** (content changed;
  file state changed) and `Drift detected: 2 resource(s)` for one resource.
  Pre-existing; the count names findings, not resources. Not in this ticket.
- **Two dated audit records** (`docs/cli-defects.json`,
  `docs/dogfood-1.12.3-cli-defects.json`) quote the 1.12.3 help text; left as
  records of what that version printed.

IMPL-PMAT-562-RECEIPT-END
