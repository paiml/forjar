# Dogfood receipt — forjar 1.27.0

verdict: NO-GO ON GATE F ONLY — seven of the eight standing gates measured green on this branch and the 95% line floor is green at 96.38%, but gate F's in-diff MUTATION arm could not run at all on this workstation and an unmeasured arm is not a passed one; the release proceeds on an explicit operator P0 directive with that arm named, ticketed as PMAT-216, and not claimed.

## Identity

| field | value |
|---|---|
| branch | release-1.27.0-fleet-p0 |
| base | 19b9c1f1e42ea254b3322e526d451835fb92e480 |
| binary | resolved by `scripts/dogfood/lib/binary.sh`, `forjar 1.27.0`, asserted against the manifest before any claim |
| tools | pmat 3.39.0, bashrs, cargo-llvm-cov, cargo-mutants, pv, agy |
| host | this workstation |

## The eight standing requirements

| req | gate | measured |
|---|---|---|
| A | harness receipt per merged PR | PASS 1 of 1 since v1.26.0 — and it FAILED first, on PR #484 merging with no `docs/audits/impl-PMAT-208-receipt.md`. The receipt was written and committed; the gate caught a real gap, which is the only reason it is green now |
| B | `pmat comply` | PASS six arms; CB-200 ratchet held; required check `[gate]` reaches a dogfood gate; 12 gate scripts at 0 bashrs errors |
| C | CLI / MCP / HTTP surface from the built artifact | PASS 211 CLI names, 12 MCP tools, 12 HTTP verbs; declared equals live; matches the committed ledger |
| D | README and cookbook as executable claims | PASS 18 invocations, 1 pinned known-broken (PMAT-189); version claims reconcile with `Cargo.toml`; 98 cookbook configs validate |
| E | quorum receipt per merged PR | PASS 1 of 1 since v1.26.0, unwaived |
| F | 95% line coverage and in-diff mutants | **SPLIT: line floor PASS at 96.38% (`cargo llvm-cov --workspace --locked --fail-under-lines 95`, exit 0); mutation arm UNMEASURED — see below** |
| G | `pv` contract depth | PASS 40 contracts validate and lint; 4 kernels carry equations and Kani harnesses; every citation resolves; 12 verbs and 12 resource kinds reconcile at their recorded ceilings |
| H | CRUX reconciliation | PASS 3 of 3 behaviour paragraphs under `[1.27.0]` have a row naming at least 3 of the 28 surveyed systems |

## Gate F, stated plainly

The 95% line floor is the blocking number and it is green, measured the way the gate measures it — inside llvm-cov via `--fail-under-lines`, never by comparing a printed percentage in shell. Total lines 96.38%, regions 95.62%, functions 92.71%.

The mutation arm did not run. `cargo mutants` prints `Found N mutants to test` and then, in its first Test phase, `ERROR interrupted / scenario execution internal error err=interrupted phase=Test`, and writes no `outcomes.json`. The gate correctly refuses to call that clean.

It is not this branch. Reproduced four times with different inputs:

| attempt | input | result |
|---|---|---|
| 1 | `--in-diff` the release diff | interrupted (also raced a concurrent edit, so discarded) |
| 2 | `--in-diff`, tree settled | killed, exit 137, under memory pressure from other sessions |
| 3 | `--in-diff`, 107 GB free, fully detached with `setsid` | interrupted, 47 mutants found |
| 4 | `--baseline skip` | interrupted |
| 5 | `--file src/cli/drift_state.rs`, which shares nothing with the release diff | interrupted, 22 mutants found |

Attempt 5 is the one that settles authorship: a file selection with no relation to this release fails identically. Ticketed as PMAT-216 with the first suspect named — `src/core/store/mutation_runner.rs` emits `pkill -f '<resource_id>'`, and a `pkill -f` inside a test can match processes outside it, a self-match hazard this fleet has hit before.

**This is a NO-GO by the letter of the standing requirements, and it is recorded as one.** The release proceeds because the operator declared four fleet-blocking defects P0 with the drift lane unable to be green until they land. That is an operator override of a release gate, and the honest form of it is this paragraph, not a green tick.

## Falsification of the gates themselves

Every gate script carries a `# mutation:` comment naming the one-line change that turns it red, and `tests/falsification_dogfood_scripts_declare_mutations.rs` asserts the address exists. Gate A demonstrated its own value during this run by failing on a real missing receipt rather than on a threshold.

DOGFOOD-1.27.0-RECEIPT-END
