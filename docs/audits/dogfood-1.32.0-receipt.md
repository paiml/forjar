# Dogfood receipt — forjar 1.32.0

verdict: GO — all nine gates (A B C D E F G H T) measured green in one `make dogfood-release` run, exit 0, on 2a39ed91, the head of PR #603 — the last PR of the window — before it merged. Its merge commit b12a8392 carries the SAME tree (`69f684ffea284ce152a52a74eeb6ef08f2eecfac`, `git rev-parse <sha>^{tree}` on both), and b12a8392 is what `v1.32.0` points at. The full log is `docs/audits/logs/dogfood-1.32.0-release.log`.

## Why the gate ran on a branch head and not on the tag

Gate T requires every ticket in the window to say `completed`, and the commit-msg hook refuses a `Pmat-Ticket:` that names a completed row — so the last PR of a window cannot complete its own ticket, and the tag cannot be taken until something has. The practice that satisfies both: run `dogfood-release` on the last PR's branch head, merge it, tag the merge commit, and let this booking PR complete the last ticket (PMAT-566). Tree identity carries over only what the tree determines: the files, and the binary built from them. That is all gates C, G and H read, and most of what B, D and F read, and the merge added no content. It carries over nothing read from outside the tree. Gate B's ratchet counts findings `pmat comply` reads from live GitHub (re-measured on the booking branch, `impl-PMAT-604-receipt.md`), gate D validates a clone of the cookbook's master as it stood then, gate F diffs against origin/main, and gates A, E and T read git history and GitHub's PR state. On 2a39ed91 the A/E/T window did not yet contain #603 (next section). A three-lane review quorum refuted an earlier version of this paragraph, which claimed tree identity for all nine gates. So A and E were re-measured at b12a8392 itself, over the release's full window, in a scratch clone with the local `v1.32.0` tag deleted so that v1.31.0 is the newest tag reachable:

    GATE A PASS 7 of 7 merged PR(s) since v1.31.0 carry a harness receipt
    GATE E PASS 7 of 7 merged PR(s) since v1.31.0 carry a quorum receipt

Both exited 0, and both list #603 (`docs/audits/logs/dogfood-1.32.0-AE-full-window.log`). Gate T's reconciliation of the seven-PR row is the booking branch's own measurement (`impl-PMAT-604-receipt.md`).

## The gates, as the scripts printed them

| gate | line |
|---|---|
| A | `GATE A PASS 6 of 6 merged PR(s) since v1.31.0 carry a harness receipt` |
| B | `GATE B PASS comply clean; ruleset 13878864 requires [gate]; 14 gate script(s) and 4 other tracked script(s) at 0 bashrs errors; ratchet CB-200 held; ratchet CB-2110=49/49 CB-2111=49/49 CB-2112=33/35 CB-2114=33/34 CB-2115=41/43 held; required check(s) [gate] reach a dogfood gate; legacy bashrs errors 1 <= 1` |
| C | `GATE C PASS 211 CLI name(s), 12 MCP tool(s), 12 HTTP verb(s); every declared name is live and every live name declared; matches docs/audits/surface_audit.csv; CLI/verb/MCP agree on the fixture` |
| D | `GATE D PASS 18 documented invocation(s) from README.md run or parse against the fixtures (1 pinned as known-broken above); version claims reconcile with Cargo.toml; 98 cookbook config(s) validate` |
| E | `GATE E PASS 6 of 6 merged PR(s) since v1.31.0 carry a quorum receipt` |
| F | `GATE F PASS line coverage 96.44% >= 95%; forjar-contracts 0 failed / 44 ignored (39 aprender-corpus, ceiling 44); mutants: 1 .rs file(s) changed, none under src/ — cargo mutants has no library or binary target to mutate in this diff` |
| G | `GATE G PASS 44 contract(s) validate; pv lint 0 errors; 4 kernel(s) carry equations and kani harnesses; every contract names a falsifier; citations resolve; 12 verb(s) and 12 resource kind(s) reconciled against the corpus (11 unanchored, 2 ungoverned, both at their recorded ceiling)` |
| H | `GATE H PASS 1 of 1 behaviour bullet(s) under [1.32.0] reconciled in docs/audits/crux-1.32.0.md, each naming >= 3 of the 28 surveyed systems` |
| T | `GATE T PASS 9 tagged release(s) since v1.25.0 reconcile with git and GitHub and 59 ticket(s) carry their tag and say they shipped; 6 ticket(s) from 6 PR(s) merged since v1.31.0 carry release:v1.32.0; cut in flight: Cargo.toml is at 1.32.0` |

## Findings for the orchestrator

**Gates A and E count six PRs, and the release holds seven.** Measured on the branch head before #603 merged, the window was #570, #577, #583, #593, #596, #599 — #603 was not yet a merged PR and so was not in it. #603's own receipts (`impl-PMAT-566-receipt.md`, `impl-PMAT-601-receipt.md`, and the `.quorum/` receipt for its branch) are at HEAD, gates A and E at b12a8392 pass 7 of 7 with #603 in the window, and gate T on this booking branch reconciles the seven-PR row against GitHub. The six-PR count is what the instrument saw at the moment it ran. It is not rounded up here; the seven-PR measurement is the separate run quoted above.

**Gate F's mutation arm had nothing to mutate, and says so.** The branch diff changed one `.rs` file, a test under `tests/`, and no file under `src/`, so `cargo mutants` had no library or binary target in the diff. The one behaviour this release ships (PMAT-565, the lock names its writer) was mutation-tested on its own branch, #570, and that receipt carries the evidence.

**Gate B's ratchet held with margin on three checks.** CB-2112 33/35, CB-2114 33/34, CB-2115 41/43. No ceiling is lowered in this receipt: a ceiling may only be lowered from a measurement of the committed tree the gate reads, and `pmat comply` reads live GitHub, so the next measurement is the next commit's.

**Gate D carries one pinned known-broken invocation, unchanged from 1.31.0.** `forjar make clean` on an `import-makefile` phony target never converges; the log names the reproduction.

## After the gate — measured 2026-09-21 at booking

| arm | state |
|---|---|
| tag | `v1.32.0` → b12a8392 on origin (annotated tag 2f466428) |
| crates.io | 1.32.0 live, published 2026-09-21T09:34:52Z by `make publish-from-tag TAG=v1.32.0` |
| docs.rs | `/crate/forjar/1.32.0` → 200 |
| GitHub Release | draft prerelease, no assets: release.yml's binary builds were still queued on the fleet |
| releases/latest | still v1.31.0 |

**Gate R is not claimed green here.** `releases/latest` resolving to this release is one of its arms, and a draft cannot satisfy it; promoting the release once its binaries attach is the remaining step, and gate R is the instrument that says when it is done.

DOGFOOD-1.32.0-RECEIPT-END
