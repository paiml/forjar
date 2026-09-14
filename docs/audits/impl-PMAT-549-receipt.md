# Implementation receipt — PMAT-549 — a query the target never answered is UNMEASURED, not drift

verdict: PASS — `forjar drift` no longer reports a host it could not reach as drifted. An unanswered query is its own state: counted in the census, listed apart from `findings`, exit 4 (connection) under `--tripwire` when nothing else is wrong, and named in the MCP verb's `unmeasured` and `unchecked`. Fourteen mutations, fourteen kills, and reverting `src/` to origin/main turns all four integration cases red.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: file

routes:
  ph1  class=impl            route=self             the reader, the detectors, the census, the CLI and MCP surfaces
  ph2  class=impl            route=self             the listing that failed or never came back
  ph3  class=review          route=agy-quorum w=3   two rounds of three lanes, three judges each
  ph4  class=impl            route=self             the four properties round two named
  ph5  class=orchestration   route=self             evidence, receipt, push, PR

verification:
  cmd="cargo test --locked --lib tripwire::drift"                             rerun_exit=0 (495 passed)
  cmd="cargo test --locked --test falsification_drift_unmeasured_is_not_drift" rerun_exit=0 (4 passed)
  cmd="cargo test --locked --test falsification_e05_verb_drift_contacts_the_host" rerun_exit=0 (5 passed)
  cmd="cargo test --locked --lib mcp::"                                        rerun_exit=0 (70 passed)
  cmd="cargo clippy --locked --all-targets -- -D warnings"                     rerun_exit=0
  cmd="cargo fmt --all -- --check"                                             rerun_exit=0
  cmd="pmat analyze vacuous-tests -f json"                                     0 of 432 in a touched path
  cmd="fourteen mutations, each restored by name"                              rerun_exit=101 each; control 0

## What was measured

`forjar drift` asked a host that answers nothing and reported the answer it never got:

```
forjar 1.28.0, host 203.0.113.9 (TEST-NET-3):
  exit 1   "drift_count": 1   "actual_hash": "MISSING"   error: 1 drift finding(s)
```

A transport error and ssh's own exit 255 both became a `MISSING` finding — counted in `drift_count`,
alerted on, and red under `--tripwire` as drift. The fleet then had two states where it needed three:
clean, drifted, and *not known*.

## The state that was missing

`unmeasured::read` classifies every detector query: `Err`, or exit 255 from an SSH transport, is
`Reading::Unmeasured`; any other exit status is the target's answer, including a local script that exits
255 on its own account. An unmeasured resource is a `DriftFinding::unmeasured` — `actual_hash` is the
`UNMEASURED` sentinel, and `is_unmeasured()` is what every surface branches on.

| surface | before | after |
|---|---|---|
| census | counted as inspected | its own `unmeasured` state; inspected never counts an unanswered query |
| `--json` | in `findings`, in `drift_count` | in `unmeasured`, with `unmeasured_count`; `drift_count == findings.len()` still holds |
| text | `No drift detected.` or a drift line | never `No drift detected.` while anything is unmeasured |
| `--tripwire` | exit 1, drift | exit 4 (connection) when only unmeasured remain; drift still wins when both are present |
| MCP | a finding, `drifted: true` | in `unmeasured` and `unchecked`; `drifted` false when only unmeasured remain |
| `apply` | reconciles the MISSING finding | reconciles it exactly as before; only the printed detail changed |

## Falsification

Fourteen mutations on committed trees, each restored by name, each killing the test that owns it and
nothing else; every control run green. The matrix is in `.quorum/evidence/unmeasured-pmat.md`.

Reverting `src/` to origin/main under the branch's own tests is the other half: all four cases of
`tests/falsification_drift_unmeasured_is_not_drift.rs` fail, and `drift_over_an_unreachable_machine_must_not_answer_clean` fails in the E05 suite while its four neighbours stay green.

## The rounds, and what they changed

Round one killed four of eight claims and every kill produced a code change: the lockless path built its
report as a struct literal and never marked the census; `apply`'s printed line changed and the CHANGELOG
said it had not; no test covered the image or task detectors' unmeasured arm; and paiml/infra's
drift-tripwire.sh reads only `findings`, so it would pass an unreachable machine in silence. Round two
confirmed all eight and named four more untested properties, which this head covers.

## Gaps, named

- **`remote_path_digest` does not go through the reader.** It is the executor's baseline writer, not a
  drift detector, and an unanswered query there still becomes `None`. Disclosed rather than covered; one
  round-one judge dissented on exactly this and the wording was corrected instead of the code.
- **A tripwire run with drift exits 1, not the drift class 10.** The error string is `N drift finding(s)`
  and the legacy classifier looks for `drift detected`. That is on the base commit; this branch keeps it
  and files it rather than folding an exit-code change for drift into a fix about unanswered queries.
- **Without `--tripwire`, `forjar drift` exits 0 whatever it found** — true of drift before this change
  too, and unchanged here.

IMPL-PMAT-549-RECEIPT-END
