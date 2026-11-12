# Dogfood receipt — forjar 1.34.1

verdict: NO-GO on the nine-gate bar, as measured on 2026-11-12 at 5aea6c1e (lane branch lane-1341 = PR #993 head b565aa9d plus the booking and bump commits), before any merge or tag — gates A, C, D and E printed PASS; gates B, F, G and T printed FAIL; gate H printed FAIL because no crux-1.34.1.md exists (not written, see below). Not a clean-room result: `release.yml`'s clean-room on the tag sha is a separate, later measurement and the hard gate.

## How this was measured

- Date 2026-11-12, 00:32Z to 00:59Z. Host **kfocus**, a clone of the lane branch with its own `.git` (origin set to github.com/paiml/forjar), through the capped wrapper `fc-pre.sh`. Nothing was built or run on lambda; gate T was also run read-only on lambda (no compile) and printed the same FAIL as on kfocus.
- Each gate separately, the commit's own unmodified scripts: `timeout 2700 bash scripts/dogfood/<script>.sh`. Order B C D G A E T H F. Gate D needed the cookbook beside the repo: first run printed UNMEASURED (not found), then it was cloned (`paiml/forjar-cookbook` master `9ba66ad7875d16e587ebfbe59837d88e333375fd`) and D re-run at 00:54Z.
- Logs on kfocus under `~/tmp/dogfood-1.34.1/` (not committed). sha256 first 12: B `3f0b22c0bd4a`, C `2b96e13bb86f`, D (re-run) `c4c581282718`, G `0af7450d61d6`, A `1d129491b4fc`, E `e59af8b70dc9`, T `16fd79f74665`, H `621ef7ced275`, F `2a98d1d71ecb`.

## The gates, as the scripts printed them

| gate | result | line |
|---|---|---|
| B | FAIL | `the CB-200 ratchet (scripts/cb200-ratchet.sh) exited 1: the debt grew past its recorded baseline` (652 functions below grade A, ceiling 651). The same line was measured on origin/main dbd1ec3e (`~/tmp/dogfood-1.34.1/B-main.log`, sha256 `fd4fe3f50e23`): not caused by this branch. |
| C | PASS | `211 CLI name(s), 12 MCP tool(s), 12 HTTP verb(s)` agree with docs/audits/surface_audit.csv |
| D | PASS | `18 documented invocation(s) from README.md run or parse against the fixtures (1 pinned as known-broken above); version claims reconcile with Cargo.toml; 98 cookbook config(s) validate` |
| G | FAIL | `pv lint contracts reports 5 error(s)` (the same count as the 1.34.0 receipt; cause not re-investigated) |
| A | PASS | `9 of 9 merged PR(s) since v1.34.0 carry a harness receipt` |
| E | PASS | `9 of 9 merged PR(s) since v1.34.0 carry a quorum receipt` |
| T | FAIL | `v1.34.0: the cookbook at 9ba66ad7... LOCKS forjar 1.33.0 and the release is 1.34.0` (arm T8b, PMAT-537). The v1.34.0 row was booked with the cookbook head as the tool wrote it; the cookbook has no commit that locks 1.34.0. |
| H | FAIL | `no docs/audits/crux-1.34.1.md: 1 behaviour(s) are being released against no recorded comparison` |
| F | FAIL | `line coverage UNMEASURED is below the 95% floor (cargo llvm-cov exit 101)`; the run stopped at `tests/falsification_budget_dry_run.rs`, 3 of 7 failed (the same three as the 1.34.0 receipt). No coverage figure exists for 1.34.1. |

Gate A and E count only the 9 PRs merged on main since v1.34.0; PR #993 (PMAT-992) was not merged when this was measured, so neither gate saw it. Re-run both after it merges.

Note added 2026-11-12 after main (c4ce3a2c) was merged into this branch: #978 (PMAT-913) and #930 (PMAT-920) merged after the table above was measured, so the window v1.34.0..main now holds 11 PRs, not 9. Their roadmap rows now carry `release:v1.34.1` (`scripts/release-goal.sh sync`; `sync --check` exits 0). Gates A and E were not re-run on the 11; the table is unchanged and still describes 5aea6c1e.

## Other targets

On kfocus, same tree: `cargo fmt --all -- --check` rc 0; `cargo clippy --all-targets -- -D warnings` rc 0; `cargo test --lib` 13786 passed, 0 failed, 4 ignored; `falsification_version_matches_manifest` 4 passed; `falsification_992_timeout_needs_no_kill_binary` 2 passed; the release-ledger and changelog readers (`falsification_release_changelog_counts_the_window`, `_release_goal_cut_books_the_tag`, `_release_goals_are_measured`, `_crux_gate_reads_the_release_section`, `_crux_audit_shape`, `_607_release_path_runs_no_python`, `_quorum_anchors_release_shaped`) all passed. `falsification_release_workflow_shape` failed 1 of 8 (`rule2_release_yml_is_the_only_v_star_tag_push_producer`: bench.yml, ci.yml and coverage.yml also list `v*` tag pushes), a known LAB red (forjar#858), not touched.

## What this receipt does not claim

- It does not claim v1.34.1 passed `make dogfood-release`: five of nine gates printed FAIL.
- crux-1.34.1.md is not written: the comparison against other systems is not measured here, and a file naming systems without comparing them would pass gate H without meaning it.
- Nothing about clean-room, crates.io or the tag was measured. No gate script, floor, ceiling or budget was edited.

DOGFOOD-1.34.1-RECEIPT-END
