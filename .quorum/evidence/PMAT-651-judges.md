# Judges — forjar 1.33.0-rc.1 release PR, #647 + #648 on main (PMAT-651)

Round count and heads: see `PMAT-651-lanes.md`.

## CONFIRMED

1. [check] C1 — A stale mount stacked under the declared one is drift: check_script counts every non-autofs filesystem at the target and is RED above one.
   - evidence: the count is `src/resources/mount.rs:79` and the condition `src/resources/mount.rs:152`; asserted red-then-green at `tests/falsification_648_automount_stack.rs:153`.
2. [apply] C2 — For an automount entry the fstab line is written first, the unit is reloaded before the stack is detached, and systemd mounts on a path touch; forjar never runs mount -t.
   - evidence: `src/resources/mount.rs:120` orders daemon-reload before the detach; `tests/falsification_648_automount_stack.rs:169` asserts one mount with the declared options and no mount -t, and `tests/falsification_648_automount_stack.rs:184` and `tests/falsification_648_automount_stack.rs:194` cover a pre-existing stack and an untriggered trigger.
3. [busy] C3 — A busy stack under an automount fails non-zero and is never lazily detached, on both remount paths.
   - evidence: detach_stack at `src/resources/mount.rs:95` only takes the lazy branch when asked, and the wrong-source call at `src/resources/mount.rs:237` passes `!automount`; asserted at `tests/falsification_648_automount_stack.rs:203` and `tests/falsification_648_automount_stack.rs:214`.
4. [plain] C4 — A plain mount keeps mount -t and never calls systemctl.
   - evidence: `src/resources/mount.rs:88` detects automount by the exact option; asserted at `tests/falsification_648_automount_stack.rs:226`.
5. [bench] C5 — query_latency_under_50ms takes the minimum of 5 samples and still fails a query slower than 50ms at its best.
   - evidence: the loop is `src/core/store/tests_db_bench.rs:50` and the bound `src/core/store/tests_db_bench.rs:57`.
6. [scope] C6 — The diff against f281f5ad touches the bench test, the mount resource and its tests, two roadmap rows and the version bump; no workflow file.
   - evidence: `git diff --stat f281f5ad..d1c5b54a` lists 8 files and `-- .github` is empty after the revert in 2ab9f59d.

## REFUTED

1. [busy] R1 — The first head claimed a busy automount stack always fails loudly. In fact the wrong-source branch passed lazy=true, so a busy stack fell back to `umount -l || true`.
   - corrected: d07790c8 passes `!automount` at `src/resources/mount.rs:237`, and `tests/falsification_648_automount_stack.rs:214` goes RED with the old argument and GREEN with the fix.
2. [scope] R2 — The first head claimed the rc-tag skip in bench.yml and coverage.yml belonged to this release. It was outside #647 and #648.
   - corrected: reverted in 2ab9f59d; the rc tag runs bench and coverage like any v* tag.
3. [scope] R3 — Round 2 judged the diff against PMAT-648, which does not cover the #647 bench hunk.
   - corrected: d1c5b54a adds PMAT-651 (forjar#651), the release ticket naming #647, #648 and the bump; round 3 judged against it.
