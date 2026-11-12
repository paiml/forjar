# Claims — Shared-runner `DPkg::Lock::Timeout=600` & per-invocation `apt-get` guard (PMAT-492 / forjar#951)

Extracted from PR `#951` (`seat-s3/492`, Round 1 `77742fbe` -> Round 2 `d684c8dd` -> Round 3 `7af27752`) and attacked across 3 lanes:

- **C1 (All workflow `apt-get` `update`/`install` calls carry `-o DPkg::Lock::Timeout=600`)**: `.github/workflows/release.yml:291`, `.github/workflows/binary-release.yml:194`, and `.github/workflows/nightly.yml:141` pass `-o DPkg::Lock::Timeout=600` across all 6 `apt-get` invocations.
- **C2 (Per-invocation `awk` parser in `scripts/apt-lock-timeout-guard.sh` blocks bypasses)**: `scripts/apt-lock-timeout-guard.sh:39` joins backslash continuations, strips `#` comments, restricts to `run:` lines, splits `&&`/`||`/`;`/`|` command lists, and requires `DPkg::Lock::Timeout=[1-9][0-9]*` on each `apt-get` `update`/`install` call (`28/28` cases in `scripts/apt-lock-timeout-guard-test.sh:49`).
- **C3 (Rust falsifier `tests/falsification_492_apt_lock_guard.rs` & shell lint compliance)**: `tests/falsification_492_apt_lock_guard.rs:25` runs real-workflow, planted-defect, and `--self-test` checks (`3/3` green), and both scripts pass `bash -n`, `shellcheck -S error`, and `bashrs lint --level error`.
- **C4 (Gate A traceability & revert discrimination)**: `CHANGELOG.md:8`, `docs/roadmaps/roadmap.yaml:7110`, and `docs/audits/impl-PMAT-492-receipt.md:15` record `PMAT-492`, and reverting `.github/workflows/release.yml` fails all `3/3` tests (`FJ_951_REVERT_RC=101`).
- **R1 (Round-1 `77742fbe` whole-line `grep` allowed comment, `Timeout=0`, and `&&` chain bypasses)**: Dynamically proven RED in `da03460f` (`15/28` cases failed on the Round-1 guard) before `d684c8dd` rewrote the parser.
- **R2 (Round-2 `d684c8dd` `printf "$2"` broke `bashrs lint` `SC2059` Gate B Arm 4b)**: Fixed in `7af27752` by switching all 5 fixture emitters in `scripts/apt-lock-timeout-guard-test.sh:49` to `printf '%b' "$2"`.
