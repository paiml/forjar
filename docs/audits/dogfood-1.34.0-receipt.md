# Dogfood receipt — forjar 1.34.0

verdict: NO-GO on the nine-gate bar, as measured on 2026-11-11 at 4a0bca07, the head of PR #886 (chore/bump-1.34.0), before it merged — gates A and E are NO-GO (60 of 62 window PRs have no impl receipt, across 29 ticket ids; 43 quorum receipts absent and 2 invalid; 13 trailer mismatches), gates B, G and F also printed FAIL (F produced no coverage figure; after the F source fix it still prints FAIL on a second, unrelated failing test, see "Gate F after the fix"), and gates C, D, H and T printed PASS. v1.34.0 is tagged on the ordered gates by Noah's ruling of 2026-11-11 10:25Z, not on this bar; `release.yml`'s clean-room on the tag sha is the hard gate. The A/E debt is paiml/forjar#990 (sub-issue of epic #696). No gate script was changed and no gate was waived.

## How this was measured

- Date: 2026-11-11, 10:27:37Z to 10:53:08Z. Commit: `4a0bca070ebfb1383dbb3ae65b00f960ad8b44c4`, detached, clean tree, in a scratch clone on host **kfocus** (`/home/noah/tmp/ic1111-forjar-886`). Nothing was built or run on lambda or intel.
- Toolchain on kfocus: rustc 1.93.0 / cargo 1.93.0 (the repo's `rust-toolchain`), cargo-llvm-cov 0.9.1, pmat 3.42.0, pv 0.72.3 (9c34b9a91), bashrs 7.4.2, gh 2.45.0, jq 1.8.2, python3 3.12.3, git 2.43.0. `CARGO_BUILD_JOBS=8`, `nice -n 10`. No other environment variable was set: `TRAILER_FLOOR`, `COOKBOOK`, `COMPLY_BASE_REF`, `DOGFOOD_NOW` and `FORJAR_DOGFOOD_BIN` were all at the scripts' defaults.
- Each gate was run **separately**, one at a time, as `nice -n 10 timeout <s> bash scripts/dogfood/<script>.sh`, with the commit's own unmodified scripts. The aggregate `make dogfood-release` was not used, because it stops at the first red (gate B here) and would have measured one gate of nine. The order was B C D G, A E T, H, F: `make dogfood-release` order, except that F (the long one) ran last, after H. No timeout fired.
- The cookbook for gate D is `paiml/forjar-cookbook` master at `9ba66ad7875d16e587ebfbe59837d88e333375fd`, cloned beside the scratch clone with kfocus's own auth. Its `Cargo.lock` pins forjar **1.33.0**, not 1.34.0 (see findings).
- Logs are on kfocus under `/home/noah/tmp/dogfood-1.34.0/` (`<gate>.log`, `summary.log`, `enum-ae.log`). They are not committed. sha256, first 12: A `e51d2cdda6e3`, B `3eaf9335d463`, C `270e6dbcb2b1`, D `f9dfce996596`, E `3f52e3bd6b2d`, F `74a518f54ef8`, G `5f6eabb35650`, H `86014d56d181`, T `aa9e6c5e10e0`, enum-ae `abe9ec940934`.
- GitHub budget before the GitHub-reading gates: REST 4688 remaining at 10:27Z, 4900 at 10:43Z.

## The gates, as the scripts printed them

| gate | script | result | rc | start–end (UTC) | line |
|---|---|---|---|---|---|
| B | `comply.sh` | FAIL | 1 | 10:27:37–10:28:01 | `CB-2113: Commit Traceability` then `GATE B FAIL pmat comply check reports failing check(s) other than CB-200 and the five owned by the ratchet in Arm 7 (CB-2110 CB-2111 CB-2112 CB-2114 CB-2115)` |
| C | `surface.sh` | PASS | 0 | 10:28:01–10:33:29 | `GATE C PASS 211 CLI name(s), 12 MCP tool(s), 12 HTTP verb(s); every declared name is live and every live name declared; matches docs/audits/surface_audit.csv; CLI/verb/MCP agree on the fixture` |
| D | `docs.sh` | PASS | 0 | 10:33:29–10:37:46 | `GATE D PASS 18 documented invocation(s) from README.md run or parse against the fixtures (1 pinned as known-broken above); version claims reconcile with Cargo.toml; 98 cookbook config(s) validate` |
| G | `contracts.sh` | FAIL | 1 | 10:37:46–10:42:00 | `GATE G FAIL pv lint contracts reports 5 error(s)` |
| A | `harness.sh` | FAIL (NO-GO) | 1 | 10:42:01–10:42:02 | `GATE A FAIL PR #982 (PMAT-869) merged with no docs/audits/impl-PMAT-869-receipt.md at HEAD: nothing records that this work went through the implementation harness, and an absent receipt is not a passed one` |
| E | `quorum.sh` | FAIL (NO-GO) | 1 | 10:42:02–10:42:04 | `GATE E FAIL PR #982 (branch fix/dogfood-guards-deleted-test) merged with no .quorum/fix-dogfood-guards-deleted-test.json at HEAD: its claims were never refuted by anything this gate can read, and an absent receipt is not a passed quorum` |
| T | `tagged.sh` | PASS | 0 | 10:42:04–10:43:09 | `GATE T PASS 11 tagged release(s) since v1.25.0 reconcile with git and GitHub and 78 ticket(s) carry their tag and say they shipped; 32 ticket(s) from 62 PR(s) merged since v1.33.0 carry release:v1.34.0; cut in flight: Cargo.toml is at 1.34.0` |
| H | `crux-reconcile.sh` | PASS | 0 | 10:43:09–10:43:09 | `GATE H PASS 7 of 7 behaviour bullet(s) under [1.34.0] reconciled in docs/audits/crux-1.34.0.md, each naming >= 3 of the 28 surveyed systems` |
| F | `coverage.sh` | FAIL, coverage not_measured | 1 | 10:43:09–10:53:08 | `GATE F FAIL line coverage UNMEASURED is below the 95% floor (cargo llvm-cov exit 101) — write the tests, do not move the floor` |

Four PASS (C D H T), five FAIL (A B E F G). No gate is NOT RUN.

## Findings

**Gates A and E are NO-GO, and each gate printed only its first red.** Both stop at the first failing PR (#982), so their own output carries no counts. The counts below come from a separate read-only enumeration (`enum-ae.log`, 10:43:52Z, rc 0), run from outside the repository: it sources the commit's unmodified `scripts/dogfood/lib/window.sh` and `lib/receipt.sh` and applies gate A's and gate E's predicates to every PR of the window instead of stopping. It is not a gate and its output is not a gate verdict. Window: 62 PRs merged since v1.33.0.

| measure | this receipt | the ruling of 10:25Z |
|---|---|---|
| gate A: ticket ids with no `impl-<id>-receipt.md` at HEAD | 29 ids, covering 60 of 62 PRs | 30 |
| gate A: PRs whose receipt is present and valid | 2 (#724 PMAT-724, #767 PMAT-663) | — |
| gate A: receipts present but invalid | 0 | — |
| gate A: merge-commit `Pmat-Ticket` trailer names a different ticket | 13 | 13 |
| gate E: `.quorum/<slug>.json` absent at HEAD | 43 | 43 |
| gate E: present but invalid | 2 | 2 |
| gate E: present and valid | 17 | — |

The one difference is 29 against 30 missing impl receipts: PMAT-724 got its receipt in 4a0bca07, the commit measured here. The 29 ids: PMAT-145, 150, 554, 613, 615, 629, 633, 671, 674, 683, 698, 717, 721, 725, 727, 735, 756, 760, 773, 775, 789, 826, 828, 829, 838, 840, 869, 986, 5880. PMAT-986, the after-the-fact ticket, accounts for 25 of the 60 PRs. The 13 trailer mismatches are PRs #977, #962, #960, #953, #816, #792, #778, #766, #718, #715, #711, #681, #665. The two invalid quorum receipts: #774 `.quorum/fix-clipboard-win-yanked.json` ("quorum.lanes = -1, below the floor of 3") and #681 `.quorum/test-apply-scoped-secret.json` ("carries a waiver/override key"). This debt is ticketed as **paiml/forjar#990** under epic #696. Nothing here backfills a receipt.

**Gate B failed on its first arm; arms 2 to 7 are not_measured.** `pmat comply check` reports CB-2113 as Fail: "11 of 11 non-merge commit(s) in 5b8f812..HEAD (base origin/main) break traceability". Of the eight it lists by name, four carry `Pmat-Ticket: PMAT-986` and one `PMAT-554`, both completed rows ("work belongs to an open item"), and three carry no trailer; three more are not itemised in the message. These are the commits of PR #886's own branch. Whether CB-2113 still fails once the branch is squash-merged is not_measured. Because the gate exits at arm 1, the ruleset, bashrs, CB-200 and CB-2110..2115 ratchet arms did not run in this measurement.

**Gate G failed on `pv lint`, with pv 0.72.3.** All 44 contracts validated. `pv lint contracts` then printed `Summary: 5 errors, 123 warnings, 0 suppressed`, and on its own last lines `armed meet: Pass` and `Result: PASS`; the gate reads the Summary line. The five errors are all PV-ONT-028 (a binding marked `implemented` that does not resolve to an item), reported by pv's `bindings` check, which pv lists as not armed:

- `apply-receipt-v1`: `forjar::core::executor::resource_ops::displaced_hash`
- `dag-ordering-v1`: `forjar::core::resolver::kahn_sort`
- `overlay-interface-v1`: `forjar::core::parser::resource_types::validate_overlay_ip_injective`
- `plan-apply-equivalence-v1`: `forjar::core::executor::apply_single_resource`
- `recipe-determinism-v1`: `forjar::core::recipe::validate_input_type`

The later arms of gate G (kernels, falsifiers, citations, verb and kind reconciliation) were not reached: not_measured. Whether another pv version reports these five is not_measured.

**Gate F did not produce a coverage figure.** `cargo llvm-cov --workspace --locked --fail-under-lines 95` ran the test suite and stopped on one failing test, exit 101. The lib suite before it passed: 13763 passed, 0 failed, 4 ignored. The failure is `tests/falsification_390_one_stream_reporter.rs`, `only_one_file_in_the_executor_turns_a_stream_into_text`: `run_capture.rs: 6 stream mentions, budget 4`. The test counts `.stdout`/`.stderr` in `src/core/executor/*.rs`, so it is a property of the tree at 4a0bca07 and not of the host. No percentage was produced, and the test targets after this one did not run. Whether PR CI runs this test is not_measured.

**Gate D passed against a cookbook that locks forjar 1.33.0.** The gate validated 98 cookbook configs at `9ba66ad7` with the 1.34.0 binary built from this tree; one README invocation stays pinned known-broken (`forjar make clean` on an `import-makefile` phony target), unchanged since 1.31.0. Gate T's cookbook arm requires the v1.34.0 ledger row to name a cookbook commit whose `Cargo.lock` pins 1.34.0. No such commit was measured here.

**Gate T passed at this head and did not read this receipt.** The newest reachable tag is v1.33.0, so T reconciled 11 tagged releases and the open window (62 PRs, 32 tickets labelled `release:v1.34.0`, cut in flight). Its dogfood-receipt arm applies to v1.34.0 only once the tag exists and the ledger has its row.

**Measured before the merge and before the tag.** Everything above is the head of PR #886. Gates A, E and T read GitHub's merged-PR state and gate B reads the branch's commits against origin/main (5b8f8123), so the same scripts run on the merge commit or the tag may print different lines; none of that was measured here.

## Gate F after the fix (and gates B, G)

**Five whys, two lines.** `run_capture.rs` held 4 stream mentions (2 persisting the run log, 2 in `capture_exec_output` redacting each stream before it is written; budget 4, set by 17d89a83, Refs #406). e7c6a6e0 (PMAT-5880, #782) added `Transcript::redact_output` with the same two redactions for the failure excerpt and left the inline copy in `capture_exec_output`, so the file went to 6; no CI job runs `falsification_390`, so nothing stopped it.

**The fix** (`src/core/executor/run_capture.rs`, +2 -5, behaviour unchanged): `capture_exec_output` calls `transcript.redact_output(output)` instead of rebuilding the same `ExecOutput`. The two are equal on that path because `suppress` has already returned. The test, its budget and every gate script are untouched.

**Checks on kfocus (nice -n 10, CARGO_BUILD_JOBS=8), all rc 0:** `cargo test --test falsification_390_one_stream_reporter` (2 passed); `--test falsification_390ab_parallel_wave_parity` (5), `--test falsification_e04_run_log_secrets` (6), `--test falsification_store_validate_gc` (24); `cargo test --lib executor` (207 passed); `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D warnings`.

**Gate F after the fix: still FAIL, no percentage.** Host kfocus, `nice -n 10 timeout 2700 bash scripts/dogfood/coverage.sh`, log `/home/noah/tmp/dogfood-1.34.0/F-after-fix.log` (sha256 `1a77384b5587`), 11:58Z to about 12:16Z, rc 1. The lib suite and `falsification_390` now pass. `cargo llvm-cov` then stops (exit 101) at a different failure: `tests/falsification_budget_dry_run.rs`, 3 of 7 failed (`a_reaper_invoked_with_no_environment_deletes_nothing`, `a_preview_reports_no_reclaimed_bytes_and_leaves_the_heartbeat_alone`, `the_scheduled_and_apply_time_passes_still_reclaim`), assertion `a preview that reclaimed nothing is correct, not inert`. It is a reaper-script test unrelated to `run_capture.rs`; its cause on kfocus is not_measured (for origin/main see the gx10 measure below). Last line: `GATE F FAIL line coverage UNMEASURED is below the 95% floor (cargo llvm-cov exit 101) — write the tests, do not move the floor`. Two earlier attempts of this same command are void and not counted: one shared the target dir with a still-running copy of itself (`test_fj062_secret_from_env_via_subprocess` lost its executable), one hit `No space left on device` on kfocus (a shared host, root at 100%). Gate F therefore stays FAIL; the 95% floor was not measured and not moved.

**Gate B** stays FAIL / NO-GO. Gates F and G were measured identical by content on origin/main (5b8f8123): `git diff origin/main HEAD` shows 0 difference in `run_capture.rs`, `tests/falsification_390_one_stream_reporter.rs` and `contracts/`. B (CB-2113) sits on the PR branch's own commits and is not measured after a squash.

**Gate G** stays FAIL / NO-GO (5 PV-ONT-028 errors, unchanged).

**Gate F, measured on main (added 12:50Z).** On gx10 (aarch64, disk free), `nice -n 10 cargo test -j 4 --test falsification_budget_dry_run` failed the same 3 of 7 cases, rc 101, at origin/main 5b8f8123 and at 612efcc4. PR #886 did not cause it and it is not a full-disk artefact. No workflow under `.github/workflows/` names the test. Its root cause is not_measured (the generated reaper exits 2 under the test's scrubbed environment). Gate F is therefore NO-GO / UNMEASURED: no coverage figure exists for v1.34.0.

B and G are carried on debt ticket paiml/forjar#990 under epic #696, by Noah's ruling of 2026-11-11 11:09Z (option 2). Gate F (`falsification_budget_dry_run`) is carried on the same ticket by Noah's ruling of 2026-11-11 12:43Z (option 1).

**What was measured when.** Gates A, B, C, D, E, G, H and T and the F-before line were measured at 4a0bca07. The only source change since is the F fix above. Gate T was re-run read-only on lambda after the fix (`bash scripts/dogfood/tagged.sh`, no compile): rc 0, `GATE T PASS 11 tagged release(s) since v1.25.0 reconcile with git and GitHub and 78 ticket(s) carry their tag and say they shipped; 32 ticket(s) from 62 PR(s) merged since v1.33.0 carry release:v1.34.0; cut in flight: Cargo.toml is at 1.34.0`.

## What this receipt does not claim

- It does not claim v1.34.0 passed `make dogfood-release`. It did not: five of nine gates printed FAIL.
- It does not claim clean-room. `release.yml`'s clean-room on the tag sha is a separate, later measurement, and it is the hard gate of the ruling: red means no publish.
- No gate script, floor, ceiling or budget was edited, and no knob that relaxes a gate was set.

DOGFOOD-1.34.0-RECEIPT-END
