# Implementation receipt — PMAT-220 — three places still decided whether they could read THIS host's filesystem on a machine's behalf using a predicate that forgets a namespace

verdict: DONE — the build-I/O probe, the pre-plan probe and output verification now ask `controller_answers_for`, the definition forjar#485 introduced and that drift and the lock-baseline writer already share; the fourth site is deliberately unchanged and its exemption is pinned to the ordering that makes it safe. Closes forjar#495.

## Identity

| field | value |
|---|---|
| ticket | PMAT-220 (kind: code) |
| issue | forjar#495, filed from PMAT-219 rather than fixed there |
| branch | PMAT-220-one-definition-of-local |
| base | d14ffaaf |
| model gate | `model=opus class=opus decision=admit basis=file` |

## Why this was filed rather than fixed inside PMAT-219

Widening `machine_is_local` under four callers that ticket had not measured is exactly the move that turns one fix into a regression. Each site is judged on its own here, and they did not all get the same answer.

| site | decides | outcome |
|---|---|---|
| `core::task::probe` | whether to hash declared build I/O on the controller | strict predicate |
| `core::executor` pre-plan probe | the same, before planning | strict predicate |
| `core::executor::output_verify` | whether to stat declared artifacts on the controller | strict predicate |
| `transport::exec_script_tracked` | local versus SSH | **unchanged** |

The fourth is unreachable for a namespace: the dispatcher returns for pepita and for container before it reaches `machine_is_local`, so the missing exclusion decides nothing there. Tightening it would be a change with no behaviour behind it, made to satisfy a rule rather than a defect. All three lanes checked that reasoning and confirmed it, one of them by measurement.

## The measurement that decides the whole ticket

A pepita namespace is not a synonym for this host. `PepitaConfig` carries its own `rootfs` (`debootstrap:jammy`, or a path like `/opt/rootfs`) and the transport unshares the mount namespace. So `/home/<user>/project` inside the namespace is not the controller's path of the same name, and hashing the controller answers about the wrong filesystem — the same defect as forjar#485.

Measured before the fix: `missing_outputs` reported a namespaced machine's declared artifacts as missing, because it looked for them here.

## What review changed

Three lanes, 3 of 3 FAIL, and every finding re-run.

- **The rule named three files**, so a fourth controller-side read added elsewhere would have stayed green — vacuous against exactly the thing it exists to prevent. It sweeps all of `src/` now, with one file exempt by name and a case that fails if the exemption stops being needed. Measured: the same call added to `src/core/state/reconstruct.rs` is caught by name.
- **`code_only` stripped whole-line `//` and nothing else**, so a trailing comment or a `/* */` block was a door. All three forms are stripped, and a trailing-comment fake fix is proven not to pass.
- **The ordering check compared string positions**, which a binding above the early returns would satisfy while changing what the code does. It now requires each predicate to sit in a guard whose body returns: a mention is not a dispatch.

That first finding is the third time this repository has shipped a rule that reads its own explanation, after RULE 8 of the release-workflow gate and the cargo PATH prelude. The comment-stripping half was caught within one run because of those two; the other two halves were caught by review.

## The disagreement that located a real defect

Lanes 1 and 2 called the probe change a regression: with the namespace excluded, `probes.get` returns `None`, the planner skips the staleness check and a namespaced task stops rebuilding when its sources change. Lane 3 answered that the previous behaviour did not detect those changes either, since it hashed the controller.

Lane 3 is right, and settling it found the actual defect. `src/core/planner/mod.rs` cannot tell "probed, nothing stale" from "never probed" — a missing probe falls through to comparing config hashes. That is true for **every** non-local machine and has been since the probe existed, so every SSH target already behaves this way; this ticket adds namespaces to an existing set rather than creating the behaviour.

It is the "unmeasured reads as clean" shape this repository fights everywhere else — `DriftCensus` names why each resource was skipped and never counts an unevaluated one as clean; the planner has no such census. Filed as **forjar#497** with that framing.

## Verification, all my own runs

| command | result |
|---|---|
| `cargo test --workspace` | **exit 0, 313 binaries** |
| `cargo test --lib output_verify` | 9 passed |
| `cargo test --test falsification_a_namespace_is_not_the_controller` | 2 passed |
| `cargo fmt` / `clippy -D warnings` | clean / exit 0 |

Mutations, each killing exactly its own case: the loose predicate restored at any of the three sites; a fourth site added in an unrelated file; a trailing-comment fake fix. No lane ran cargo — the delegate forbade it on disk grounds — so every number is an orchestrator rerun.

## Gaps

- forjar#497 is open and this ticket makes its silence louder for one more machine type while removing a wrong answer. That trade is deliberate and named rather than absorbed.
- Output artifacts on a namespaced machine are now verified nowhere. Verifying them properly means running `test -e` inside the namespace through `exec_script`, which is a larger change than this ticket; the current outcome matches what the function's own doc already promises for a target this host cannot answer for.
- `machine_is_local` still exists with its pepita hole. It is now unreachable for a namespace at its only remaining caller, and the rule fails if a new caller appears.
- Gate F's mutation arm cannot run on this host (PMAT-216); no `cargo mutants` figure is claimed.

IMPL-PMAT-220-RECEIPT-END
