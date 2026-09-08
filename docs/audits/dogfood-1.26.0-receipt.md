# Dogfood receipt — forjar 1.26.0

verdict: GO — all eight standing gates measured green on the tagged sha `818a060d`, and the published-artifact arm passed against what crates.io serves.

## Identity

| field | value |
|---|---|
| sha | `818a060d972b2eb9deceb795334afea29dd28f0d` (tag `v1.26.0`) |
| binary | resolved by `scripts/dogfood/lib/binary.sh`; `forjar 1.26.0`, asserted against the manifest before any claim |
| tools | pmat 3.39.0, bashrs 7.0.1 (CLI) and 6.68.0 (linked), cargo-llvm-cov, cargo-mutants, pv, agy 1.1.27 |
| host | this workstation; the clean-room arm ran on the fleet's container |

## The eight standing requirements

| req | gate | measured |
|---|---|---|
| A | harness receipt per merged PR | PASS 10 of 10 since v1.25.2, each `docs/audits/impl-<id>-receipt.md` ending in its END marker with exactly one verdict line |
| B | `pmat comply` | PASS; six arms, including the CB-200 ratchet held at its recorded ceiling of 651 and the required check proven to reach a dogfood gate |
| C | CLI / MCP / HTTP surface from the built artifact | PASS 211 CLI names, 12 MCP tools, 12 HTTP verbs; declared equals live, and the committed ledger matches |
| D | README and cookbook as executable claims | PASS 18 invocations run or parsed against fixtures, 1 pinned known-broken (PMAT-189); 98 cookbook configs validate |
| E | quorum receipt per merged PR | PASS 10 of 10, each unwaived with ≥3 lanes, judges and refuters per claim and ≥1 refuted claim |
| F | 95% line coverage and in-diff mutants | PASS 96.39% ≥ 95%; the mutation arm reports what it found for the diff under test |
| G | `pv` contract depth | PASS 40 contracts validate and lint; 4 kernels carry equations and Kani harnesses; every citation resolves; 12 verbs and 12 resource kinds reconcile |
| H | CRUX reconciliation | PASS 3 of 3 behaviour paragraphs under `[1.26.0]` have a row naming ≥3 of the 28 surveyed systems |

## Published arm

`make dogfood-published VERSION=1.26.0` installed 1.26.0 from crates.io into a scratch root and re-ran gates C and D against that binary: both PASS, `forjar --version` reports 1.26.0.

## Gates repaired during this run, each with a falsifier

- H read only `[Unreleased]`, which the cut empties — it now reads the version section when that one is empty, pinned by four cases in `tests/falsification_crux_gate_reads_the_release_section.rs`.
- F called a diff whose only Rust change is a test UNMEASURED — it now counts the changed files under `src/`, pinned by five cases in `tests/falsification_coverage_gate_mutation_scope.rs`.
- G looked for a falsifier citation in one field where `pv`'s schema allows two — sixteen wired falsifiers read as prose until it read `command:` as well.
- B's CB-200 ratchet was grading a stale `pmat comply` cache and reported a number for a tree that no longer existed — pinned by four cases in `tests/falsification_cb200_ratchet_measures_this_tree.rs`.

No threshold moved in any of them: the coverage floor stays 95, the CB-200 ceiling stays 651, and the unanchored-contract ceiling shrank by one.

DOGFOOD-1.26.0-RECEIPT-END
