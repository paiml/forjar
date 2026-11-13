# GH-667 (`forjar#944`, with `GH-669` & `GH-936`) — Extracted Claim Manifest (`C1..C4`, `R1..R2`)

Base commit: `dbd1ec3e34bc39d6c4696c3b605569b312bddf96`
Head commit: `533f1865fb9383378c364b130ebec5a639c97dbf`
Branch: `kz-tool/disk-budget-unreachable-669`

1. **C1 (`[code]`)**: `fb_scan_open_paths` (`src/resources/disk_budget/detect.rs:104`, `src/resources/disk_budget/tests/reaper_cost.rs:70`) scans `/proc/*/exe` alongside `cwd`/`fd` in a single `find` pass and streams `cat /proc/[0-9]*/maps` into `$FB_MAPS`, failing closed (`FB_UNMEASURED=1`, `FB_OPEN_OK=0`) whenever `/proc` or `maps` is empty or times out (`GH-667`, `GH-936`).
2. **C2 (`[code]`)**: `reach_block` and `script` (`src/resources/disk_budget/reaper.rs:85`, `src/resources/disk_budget/reaper.rs:248`) pre-sum reclaimable bytes via `fb_sum_reach` (`src/resources/disk_budget/detect.rs:88`) before any deletion; when all scans are measured and `FB_REACH_BYTES < FB_NEED_BYTES`, `FB_UNREACH=1` skips every rule's delete loop, logs the shortfall, and still exits `1` (`GH-669`).
3. **C3 (`[code]`)**: `rule_block` (`src/resources/disk_budget/reaper.rs:131`, `src/resources/disk_budget/tests/execution.rs:48`) checks `fb_in_use "$cand"` inside the candidate loop for every `ReclaimKind`, and `fb_sum_reach` (`src/resources/disk_budget/reaper/tests.rs:219`) deduplicates nested/overlapping paths while printing plain `%.0f` integers above $2^{31}$ bytes.
4. **C4 (`[test]`)**: `tests/falsification_budget_unreachable.rs:42` and `CHANGELOG.md:10` pin all 6 reachability, empty-`/proc`, empty-`maps`, idle-timeout, and overlapping-rule invariants across isolated temp fixtures (`6/6` green alongside `83/83` `disk_budget` unit tests and `11/11` `falsification_disk_budget` tests).
5. **R1 (`[scope]`)**: Issue `#667` named `tests/falsification_disk_budget_in_use.rs` for the running-binary liveness falsifiers, whereas the PR places those tests inside `src/resources/disk_budget/tests/execution.rs:48` (disclosed in `docs/audits/impl-GH-667-receipt.md`).
6. **R2 (`[doc]`)**: `docs/audits/impl-GH-667-receipt.md` notes `"Red-then-green for the PR's own new tests was not re-measured by this lane"` on the merged head `533f1865`.
