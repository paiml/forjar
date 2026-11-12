# GH-667 (`forjar#944`) — Judges Claims Digest

### CONFIRMED

1. [code] Single-pass /proc exe and streaming maps scan with fail-closed empty guards (`C1`)
   - evidence: `src/resources/disk_budget/detect.rs:104` and `src/resources/disk_budget/tests/reaper_cost.rs:70` include `/proc/*/exe` in the single `find` pass and stream `cat /proc/[0-9]*/maps` into `$FB_MAPS`, setting `FB_UNMEASURED=1` and `FB_OPEN_OK=0` whenever `/proc` or `maps` times out or returns zero bytes (`GH-667`, `GH-936`).

2. [code] Pre-delete reachability check prevents futile cache sweeps (`C2`)
   - evidence: `src/resources/disk_budget/reaper.rs:85` and `src/resources/disk_budget/reaper.rs:248` evaluate eligible candidate bytes before deleting; when scans are complete and `FB_REACH_BYTES < FB_NEED_BYTES`, `FB_UNREACH=1` logs the shortfall, skips every rule's delete loop, and still exits `1` (`GH-669`).

3. [code] Loop-level fb_in_use across all ReclaimKinds and overflow-safe overlap sum (`C3`)
   - evidence: `src/resources/disk_budget/reaper.rs:131` and `src/resources/disk_budget/tests/execution.rs:48` enforce `fb_in_use "$cand"` inside the candidate loop for all four `ReclaimKind` variants, while `src/resources/disk_budget/detect.rs:88` and `src/resources/disk_budget/reaper/tests.rs:219` deduplicate nested paths and format sums via `%.0f` above $2^{31}$.

4. [test] Six-arm unreachable and fail-closed integration suite (`C4`)
   - evidence: `tests/falsification_budget_unreachable.rs:42` and `CHANGELOG.md:10` verify unreachable shortfall preservation, reachable control reclaim, empty `/proc` and empty `maps` fail-closed retention, idle-check timeout disambiguation, and single counting across overlapping rules (`6/6` passed).

### REFUTED

1. [scope] Running-binary liveness tests live in lib execution.rs rather than tests/falsification_disk_budget_in_use.rs (`R1`)
   - evidence: Ticket `#667` asked for `tests/falsification_disk_budget_in_use.rs`, which is absent from the pushed tree; the running-binary and exhaustive-`ReclaimKind` loop checks live in `src/resources/disk_budget/tests/execution.rs:48` instead.
   - corrected: All 3 lanes verified `a_cargo_target_whose_binary_is_running_survives` and `every_reclaim_kind_runs_the_in_use_check_in_the_loop` run under `cargo test --lib disk_budget` (`83/83` passed on `<host>`), and `docs/audits/impl-GH-667-receipt.md` explicitly records the file-path substitution.

2. [doc] Author merge lane did not re-measure red-then-green revert discrimination at 533f1865 (`R2`)
   - evidence: `docs/audits/impl-GH-667-receipt.md` states `"Red-then-green for the PR's own new tests was not re-measured by this lane"`, leaving direct revert discrimination on the merged tree `533f1865` unrecorded in the author receipt.
   - corrected: Independent 3-lane review perturbed `FB_UNREACH=1` to `FB_UNREACH=0` in `src/resources/disk_budget/reaper.rs:248` (`! cmp -s` verified on `<host>`, `<SCRATCH>`) and confirmed `tests/falsification_budget_unreachable.rs:42` fails on 2 of 6 tests (`fj944_unreach_revert_rc=101`).
