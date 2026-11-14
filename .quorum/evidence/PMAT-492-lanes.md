# Lanes — Shared-runner `DPkg::Lock::Timeout=600` & per-invocation `apt-get` guard (PMAT-492 / forjar#951)

Three-round `LANES=3` review by `agy` (`Gemini 4 Carbon (thinking=high)`), distinct from the Claude author model:

- **Round 1 (`77742fbe`)**: `0/3 PASS` — reproduced 3 direct guard bypasses (inline `# DPkg::Lock::Timeout` comment, `DPkg::Lock::Timeout=0`, and `update && install` chains) plus unquoted `$files` and missing Rust falsifier / `--self-test`.
- **Round 2 (`d684c8dd`)**: `2/3 PASS, 1/3 FAIL` — Lane 1 & Lane 3 confirmed the `awk` per-invocation parser (`28/28` cases) and `falsification_492_apt_lock_guard` (`3/3`), while Lane 2 caught 4 `bashrs lint` `[error] SC2059` variable-format `printf` calls in `scripts/apt-lock-timeout-guard-test.sh:49`.
- **Round 3 (`7af27752`)**: `3/3 PASS` — all 3 lanes verified `printf '%b' "$2"` clears `bashrs lint --level error` (`0` errors), `shellcheck -S error` (`0` errors), `--self-test` (`28/28`), and `FJ_951_REVERT_RC=101` on `<host>` (`<SCRATCH>`).
