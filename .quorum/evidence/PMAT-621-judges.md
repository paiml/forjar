# PMAT-621 (`forjar#933`) — Judges Claims Digest

### CONFIRMED

1. [code] Container ID capture and unconditional post-export removal (`C1`)
   - evidence: `src/core/store/provider.rs:125` binds `cid=$(docker create {}{tag})` and runs `docker rm "$cid" >/dev/null` after the export pipeline whether or not `docker export` or `tar` succeeded, eliminating the per-call `Created` container leak while short-circuiting cleanly if `docker create` itself fails.

2. [code] Subshell pipefail protection against truncated rootfs store commits (`C2`)
   - evidence: `src/core/store/provider.rs:126` wraps `docker export "$cid" | tar -x -C "$STAGING"` inside `(set -o pipefail; ...)`, guaranteeing `rc=$?` is non-zero whenever `docker export` fails mid-stream even when GNU `tar` exits `0` on a 512-byte block boundary, so `execute_import` (`src/core/store/provider.rs:180`) never commits a partial staging directory.

3. [test] Dual structural and behavioural stub-docker falsifiers (`C3`)
   - evidence: `src/core/store/tests_provider.rs:255` pins both generated command syntax and runtime execution against a hermetic stub `docker` whose `export` emits one 512-byte zero block and exits `1`; measured `53/53` green on `t2fw13` with both `FJ933_REV1_KILLED` and `FJ933_REV2_KILLED` failing when reverted.

### REFUTED

1. [diff] Round 1 (@91a0e86) pipeline status masking without pipefail (`R1`)
   - evidence: At `91a0e86`, `src/core/store/provider.rs:120` emitted `docker export "$cid" | tar -x -C "$STAGING"; rc=$?` without `pipefail` under `transport::local::exec_local`, where a failed `docker export` after one tar block yielded `rc=0` from `tar -x` and violated the stated contract (`WITHOUT_PIPEFAIL_RC=0` measured on `t2fw13`).
   - corrected: Head `21a730e8` wraps the pipeline in `(set -o pipefail; docker export "$cid" | tar -x -C "$STAGING")` (`src/core/store/provider.rs:126`) and adds `docker_import_fails_when_export_fails_and_still_removes_container` to fail closed if `pipefail` is ever removed.

2. [doc] Cleanup failure exit-status boundary in docker rm (`R2`)
   - evidence: `src/core/store/provider.rs:126` executes `docker rm "$cid" >/dev/null; exit $rc;`, discarding `docker rm`'s own exit status when `$rc` is `0`, so a daemon hiccup between `export` and `rm` leaves that container behind while returning `0`.
   - corrected: Keeping `exit $rc` matches `docs/audits/impl-PMAT-621-receipt.md` ("exits with the export's status") so a valid staged rootfs is not discarded over a cleanup hiccup while `docker rm` stderr remains unredirected for operator visibility.
