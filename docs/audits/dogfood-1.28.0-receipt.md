# Dogfood receipt — forjar 1.28.0

verdict: GO — all nine gates (A B C D E F G H T) measured green on 568dc169 in one `make dogfood-release` run, exit 0; the 95% line floor is green at 96.45% inside llvm-cov; gate F's mutation arm found no library or binary target to mutate because this branch changes no `src/` file, which is a measured zero and not a skip (PMAT-216, cargo mutants dying on this host, stays open for branches that do); gate T reports the cut in flight; gate R pre-tag passes with its four post-tag arms PENDING by name.

## Identity

| field | value |
|---|---|
| branch | `PMAT-226-release-1.28.0` |
| base | `191770fa` (main; the v1.28.0 window is v1.27.0..HEAD, nine merged PRs) |
| measured at | `568dc169` — tracked tree clean at the run (`git status --porcelain` showed only eight untracked files that predate this session and are not part of the release: `test_trap*.sh` ×6, `.pmat-work/PMAT-149/`, `.pmat-work/PMAT-157/`) |
| binary | resolved by `scripts/dogfood/lib/binary.sh` from `cargo build --release` into the script target directory, `forjar 1.28.0`, asserted against the manifest by gate C before any claim |
| tools | pmat 3.39.0, bashrs 7.0.3, cargo-llvm-cov 0.8.5, cargo-mutants 27.0.0, pv 0.65.2, agy 1.2.0 |
| host | this workstation, shared: load average 12–22 during the run from other sessions' builds; nothing here depends on timing |
| frame | `goal.sh set --ticket DF-1.28.0` refused (R-5, the session already names a ticket) — recorded, the frame decides nothing else; `git rev-parse HEAD` = 568dc169 |

## The eight standing requirements, and T

Every line below is quoted from the run, not paraphrased (docs/audits/logs/PMAT-226-dogfood-release.log).

| req | gate | measured |
|---|---|---|
| A | `harness.sh` | PASS 9 of 9 merged PR(s) since v1.27.0 carry a harness receipt |
| B | `comply.sh` | PASS comply clean; ruleset 13878864 requires [gate]; 14 gate script(s) and 2 other tracked script(s) at 0 bashrs errors; ratchet CB-200 held; required check(s) [gate] reach a dogfood gate; legacy bashrs errors 1 <= 1 |
| C | `surface.sh` | PASS 211 CLI name(s), 12 MCP tool(s), 12 HTTP verb(s); every declared name is live and every live name declared; matches docs/audits/surface_audit.csv; CLI/verb/MCP agree on the fixture |
| D | `docs.sh` | PASS 18 documented invocation(s) from README.md run or parse against the fixtures (1 pinned as known-broken above); version claims reconcile with Cargo.toml; 98 cookbook config(s) validate |
| E | `quorum.sh` | PASS 9 of 9 merged PR(s) since v1.27.0 carry a quorum receipt |
| F | `coverage.sh` | PASS line coverage 96.45% >= 95%; forjar-contracts 0 failed / 44 ignored (39 aprender-corpus, ceiling 44); mutants: 1 .rs file(s) changed, none under src/ — cargo mutants has no library or binary target to mutate in this diff |
| G | `contracts.sh` | PASS 40 contract(s) validate; pv lint 0 errors; 4 kernel(s) carry equations and kani harnesses; every contract names a falsifier; citations resolve; 12 verb(s) and 12 resource kind(s) reconciled against the corpus (11 unanchored, 2 ungoverned, both at their recorded ceiling) |
| H | `crux-reconcile.sh` | PASS 9 of 9 behaviour bullet(s) under [1.28.0] reconciled in docs/audits/crux-1.28.0.md, each naming >= 3 of the 28 surveyed systems |
| T | `tagged.sh` | PASS 5 tagged release(s) since v1.25.0 reconcile with git and GitHub and 16 ticket(s) carry their tag; 9 of 9 PR(s) merged since v1.27.0 carry release:v1.28.0; cut in flight: Cargo.toml is at 1.28.0 |

## Gate F, stated plainly

The 95% line floor is the blocking number and it is green, measured the way the gate measures it — inside llvm-cov via `--fail-under-lines 95`, exit 0 — at 96.45% on the workspace, `forjar-contracts` 0 failed / 44 ignored at its recorded ceiling.

The mutation arm ran and had nothing to mutate: `git diff origin/main...HEAD -- '*.rs'` names one file, `tests/falsification_quorum_gate_has_a_triage_shape.rs`, and cargo mutants mutates library and binary targets only. `coverage.sh` counts the mutable set (`src/`) rather than the changed set precisely so a tests-only diff passes this arm while every other arm still measures it (its comment records the 1.26.0 cut that went red for this reason). That is a measured zero. It is NOT a measurement of cargo mutants on this host: PMAT-216 (the tool dies "interrupted" in its first Test phase on any `src/` input here) remains open, and the next branch that touches `src/` will meet it.

## Gate T, the new one

First release under the two-day cadence `docs/roadmaps/releases.yaml` declares. On this branch T6 reads Cargo.toml at 1.28.0 — the declared `next.tag` — and reports the cut in flight; the nine PRs merged since v1.27.0 all carry `release:v1.28.0`; the five tagged releases at or above the floor reconcile with git and GitHub. After the tag, T2 and T6 will be red on main by design until PMAT-227 books the row with `scripts/release-goal.sh cut v1.28.0 --next v1.29.0`; the daily workflow would open its issue if that were still true at 05:00 UTC.

## Gate R, pre-tag

`scripts/dogfood/release-check.sh` on this HEAD: `GATE R PASS pre-tag: 10 PR(s) since v1.27.0 … all carry receipt=ok; PENDING until the tag is cut: tag v1.28.0 not cut, no GitHub release, not on crates.io, not on docs.rs`. The same command is the post-tag audit.

## Falsification of the gates themselves

Every gate script carries a `# mutation:` comment naming the one-line change that turns it red, and `tests/falsification_dogfood_scripts_declare_mutations.rs` asserts the address exists for all ten. This branch's own falsifier is the rail test, red against main's gate script and green here (docs/audits/logs/PMAT-226-gate-tests.log).

DOGFOOD-1.28.0-RECEIPT-END
