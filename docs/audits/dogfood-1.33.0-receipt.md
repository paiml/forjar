# Dogfood receipt — forjar 1.33.0

verdict: NO-GO as measured — MEASURED LATE, on 2026-11-02, against tag v1.33.0 (49b33fbd), five weeks after the release was cut (2026-09-28) and published; gates B, G and H fail, A fails on its one-PR window, F is not_measured, and gate T cannot pass at the tag. This receipt records what the scripts printed; it does not claim the release was measured before it was cut, because it was not.

## How this was measured

- Date: 2026-11-02, by forjar-w3, in a detached worktree at tag `v1.33.0` (49b33fbd), builds under a scratch directory on `/`, with the tag's own `scripts/dogfood/*.sh`: the same scripts the 1.30–1.32 receipts used, run in `make dogfood-release` order (B C D G, then A E T F H).
- The cookbook for gate D is `paiml/forjar-cookbook` at `fa292885a124b26ec89dce21725cb95081f7838f`, the commit the v1.33.0 ledger row names.
- Host `pv` is 0.72.2, newer than the one in use on 2026-09-28. That matters to gate G below.
- Nothing here was run on a PR head; the other receipts' "why on a branch head" does not apply.

## The gates, as the scripts printed them

| gate | result | line |
|---|---|---|
| A | FAIL | `GATE A FAIL PR #654 (PMAT-652) merged with no docs/audits/impl-PMAT-652-receipt.md at HEAD: nothing records that this work went through the implementation harness, and an absent receipt is not a passed one` |
| B | FAIL | `GATE B FAIL the CB-2110..CB-2115 ratchet (Arm 7): REGRESSION: CB-2112: 46 finding(s), ceiling 35 (recorded 2026-09-11); CB-2114: 42 finding(s), ceiling 34 (recorded 2026-09-11); CB-2115: 159 finding(s), ceiling 43 (recorded 2026-09-11)` |
| C | PASS | `GATE C PASS 211 CLI name(s), 12 MCP tool(s), 12 HTTP verb(s); every declared name is live and every live name declared; matches docs/audits/surface_audit.csv; CLI/verb/MCP agree on the fixture` |
| D | PASS | `GATE D PASS 18 documented invocation(s) from README.md run or parse against the fixtures (1 pinned as known-broken above); version claims reconcile with Cargo.toml; 98 cookbook config(s) validate` |
| E | PASS | `GATE E PASS 1 of 1 merged PR(s) since v1.33.0-rc.1 carry a quorum receipt` |
| F | not_measured | `GATE F FAIL line coverage UNMEASURED is below the 95% floor (cargo llvm-cov exit 101)`: the run aborted on a failing test, see below. No percentage was produced. |
| G | FAIL | `GATE G FAIL pv validate contracts/blake3-state-v1.yaml exited 1` — `[ERROR] VS-COUNT-001: verification_summary.total_obligations is 4 but proof_obligations has 3 entries`. Measured with pv 0.72.2; whether pv at cut time reported it is not_measured. |
| H | FAIL | `GATE H FAIL no docs/audits/crux-1.33.0.md: 1 behaviour(s) are being released against no recorded comparison with any other system` — the document is absent AT THE TAG; it was added to main later (the ledger row names it). |
| T | FAIL at the tag | `GATE T FAIL v1.33.0 is reachable from HEAD ... and has no row in docs/roadmaps/releases.yaml`: the ledger row for v1.33.0 was written after the tag. Gate T for v1.33.0 is re-run on the booking branch (release/v1.34.0), where it reconciles the row; see the cover note in that branch's commit. |

## Findings

**Gate A and E measured a one-PR window.** At HEAD = v1.33.0 the scripts' "since the newest tag" lower bound is `v1.33.0-rc.1` (a prerelease sorts above its final tag in `--sort=-v:refname`, the defect fixed on release/v1.34.0 in 3161a8ef), so the window was #654 only. The other ten PRs of the ledger's v1.33.0 row (#589 #606 #610 #630 #632 #637 #640 #643 #646 #650) were not measured by A or E here: **not_measured**. With the lower bound corrected to v1.33.0 the window is empty, which proves nothing.

**Gate F did not produce a coverage figure.** `cargo llvm-cov --workspace` ran `cargo test --tests`, which stopped on one failing test: `tests/falsification_release_workflow_shape.rs` `rule2_release_yml_is_the_only_v_star_tag_push_producer` — `found ["bench.yml", "ci.yml", "coverage.yml", "release.yml"]`, expected only `release.yml`. The lib suite before it passed (13566 passed, 0 failed, 4 ignored). Whether this test also failed on 2026-09-28 is not_measured.

**Gate B's ratchet regressed against ceilings recorded 2026-09-11.** CB-2112 46/35, CB-2114 42/34, CB-2115 159/43. No ceiling is changed in this receipt.

**Gate D was first UNMEASURED** (the cookbook checkout was absent) and PASSED once the cookbook at the ledger's sha was supplied.

DOGFOOD-1.33.0-RECEIPT-END
