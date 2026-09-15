# Implementation receipt — PMAT-560 — a service is converged only while the loaded unit executes the declared program

verdict: PASS — a `service` resource may declare `exec_start` and `exec_sha256`; the check then asks systemd which program the LOADED unit starts (`systemctl show -p ExecStart --value`) and hashes the file at that LIVE path, never the declared one; apply fails the resource on a mismatch; the state query carries both so drift sees a swap; a service that declares neither emits exactly what it did before. Fourteen cases execute the emitted shell against a fake host whose `systemctl` prints the line systemd 249 prints; RED 8/10 on 1.30.0; six mutations run, each killing what it names. Two review rounds; every finding acted on.

## Identity

- ticket PMAT-560 (forjar#560, milestone 1.31.0); kind: code
- branch `PMAT-560-unit-exec-parity`, from `main` @ 8b2ccedd
- discover.json: sha256 6f86fa8d3580dad9 — `gate_cmd_fallback=true` (`cargo test --workspace` is the discovered fallback)
- status-line join: `goal.sh set --ticket PMAT-560 --K 80 --khat 40 --basis first-run[U] --phases 5`; `k_measured` not recorded before the session was interrupted — named, not estimated

## What was measured

Live, yoga, 2026-09-15 (paiml/infra `docs/audits/autonomous-run-2026-09-15.md`):
`github-runner-ephemeral.service` declared in
`machines/yoga/forjar-ephemeral.yaml` with a unit file whose `ExecStart`
names `run-ephemeral-docker.sh`; the loaded unit ran
`/opt/github-runner-ephemeral/run-ephemeral-docker-v3.sh` (root, mtime
2026-09-10 17:28); the check reported converged. `service::check_script`
asked `is-active` and `is-enabled` and nothing else.

## Plan and routing

| phase | route (route.sh, verbatim) | executor |
|---|---|---|
| 1 RED test + fields | `route=agy-goal w=1.00 basis=absent note=fable-binding effort=1[U] bucket_collision=true` | direct (the field plumbing touched five reflection gates in one context) |
| 2 fragments, validation, contract, docs | same | direct |
| 3 review round 1 | `route=agy-quorum w=1.00 basis=absent effort=1[U]` | delegate, quorum ×3 |
| 3b review round 2 | same | delegate, quorum ×3 |
| 4 receipt, push, PR | `route=self w=100.00 basis=absent` | self |

`phase-boundary: ticket=PMAT-560 phase=3 orch_model=opus change=none recorded=0`

## Dispatch ledger

| dispatch | mode | agent | turns | maxTurns | resumed | lanes / conversations |
|---|---|---|---|---|---|---|
| PMAT-560/ph4.delegate | paiml-agy-delegate (opus) | a110d77c | 30 | hit | once (reduce only, no relaunch) | quorum ×3: conv-0606397e FAIL, conv-65aaf4b8 voided (503), conv-289c32c7 PASS; child_conversations=3 |
| PMAT-560/ph4.delegate2 | paiml-agy-delegate (opus) | ac755784 | 14 | no | no | quorum ×3: conv-3a491548 PASS, conv-f18cafc0 voided (503), conv-11457aec PASS; child_conversations=3 |

slots used: 1 of 3 at peak · denials: 0 · I-3: attempted=3 (two dispatches, one resume) denied=0 running_peak=1 slots=3.

## Verification (claimed vs re-run)

| check | claimed | re-run here |
|---|---|---|
| acceptance suites | round 1 lane 3: 10 + 75 green | green; suite now 14 cases |
| the awk reads the whole path | round 1 lane 3: yes | NO — lane 1 right; `run.sh evil` read as `run.sh`, exit 0; fixed |
| undeclared services unchanged | all counted lanes: yes | yes; MC kills the control |
| contract falsifiers all tested | round 1 lane 3: yes | NO — 005 misworded, 006 half-tested; fixed |
| `cargo test --workspace --no-fail-fast` | — | 8337cab5: 19,801 pass, 0 fail (after the golden-hash repin); a9fd3024 (final code head): 339 targets, 19,805 pass, 0 fail |
| clippy / fmt / bashrs on the emitted apply | — | clean / clean / 0 errors |
| gate G | — | PASS |

## Falsification

`tests/falsification_unit_exec_parity.rs` — fourteen cases, among them: a
unit running a different path; the declared path with different bytes; the
digest taken over the live program; the converged control; a digest alone;
an unloaded unit; the undeclared control; apply fails and succeeds; the
state query moves; a path with a space read whole; `run.sh evil` never read as
`run.sh`; the marker on stderr; an uppercase digest permanently divergent; the
fake host matches a real systemd 249 line.

| mutation | kills |
|---|---|
| MA check stops extending with the exec assertions | 9 of 14 |
| MB re-hash the declared path | the live-digest case, the stderr case |
| MC probe an undeclared service | the undeclared control |
| MD drop the apply tail | the apply case, the stderr case |
| ME split on a bare space | the space-in-path case, the exit-0 case |
| MF marker to stdout only | the stderr case |

## Jidoka

- RED: `planner::tests_hash::test_golden_hash_pinned_value`. Two new
  `Resource` fields move every recorded desired-state hash — the #403/#406
  fleet migration. Repinned with the upgrade note the test's own comment
  requires; not deleted, not loosened.
- RED: `resource_fields_has_no_stale_entries` — `skip_serializing_if` on the
  flattened fields hid them from the reflection gates. Removed.
- RED (review round 1): the space split. Five-whys: exit 0 over the wrong
  program ← the live path read as the declared one ← `split(..., /[ ;]/)` ←
  the separator was chosen from one sample line with no space in it ← the
  fixture's paths never contained a space. Fixed at the separator; two
  cases added with space-bearing paths.
- RED (lint): bashrs SC2031 and SEC016 on the first emitted apply script —
  parenthesised fallbacks read as subshells, and `exec` in a variable name
  read as the builtin. Brace groups, and the variables renamed.

## Estimates

K̂=40 [U], K=80; actual not counted from the transcript — the session was
interrupted by the operator mid-ticket and resumed; `basis=first-run[U]`.

## Gaps, named

- **The fleet completeness obligation is not discharged here.** The fixture
  proves the shell; `machines/{yoga,gx10}/forjar*.yaml` must declare
  `exec_start`/`exec_sha256` on the runner services and be applied — owed on
  paiml/infra's side once 1.31.0 ships, and logged in its audit file.
- **The program is argv[0].** `ExecStart=/bin/bash /opt/x/run.sh` hashes bash.
  Stated in the book; units should run the script as the program.
- **Several `ExecStart=` lines** (`Type=oneshot`): the first is checked.
- **A root-only script needs `sudo: true`** on the service, or the digest is
  `missing` — a divergence, never a pass.
- **A path containing ` ; `** is misread and reports divergent. Stated.
- **Gate B is main's red** (CB-2112, CB-2115); this branch does not add to it.
- **Harness defect, not filed from here:** `agy-lane.sh` dies of SIGPIPE at its
  KEPT branch when a writes=false lane leaves a large porcelain (round 1) —
  the paiml-implement bundle's, recorded in the evidence.

IMPL-PMAT-560-RECEIPT-END
