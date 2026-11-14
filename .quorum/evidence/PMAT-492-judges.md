# Judges — Shared-runner `DPkg::Lock::Timeout=600` & per-invocation `apt-get` guard (PMAT-492 / forjar#951)

Round count and heads: Round 1 `77742fbe` (`0/3 PASS`), Round 2 `d684c8dd` (`2/3 PASS`), Round 3 `7af27752` (`3/3 PASS`); see `PMAT-492-lanes.md`. Every claim `C1..C4` and `R1..R2` met 3 independent refuters.

## CONFIRMED

1. [code] C1 — `.github/workflows/release.yml:291`, `.github/workflows/binary-release.yml:194`, and `.github/workflows/nightly.yml:141` pass `-o DPkg::Lock::Timeout=600` on all 6 `apt-get` `update` and `install` calls.
   - evidence: `scripts/apt-lock-timeout-guard.sh:39` verified `6 apt-get call(s) over 20 workflow file(s)` (`rc=0`), and reverting `.github/workflows/release.yml:291` drove `falsification_492_apt_lock_guard` RED (`FJ_951_REVERT_RC=101`).
2. [code] C2 — `scripts/apt-lock-timeout-guard.sh:39` parses each shell command separately after joining continuations and stripping `#` comments, requiring `(-o)?DPkg::Lock::Timeout=[1-9][0-9]*` on every `apt-get` `update` or `install` invocation.
   - evidence: `scripts/apt-lock-timeout-guard-test.sh:49` (`--self-test`) passed all `28/28` positive and hostile-bypass fixtures on `<host>` (whereas RED commit `da03460f` failed `15/28` cases on the Round-1 guard).
3. [test] C3 — `tests/falsification_492_apt_lock_guard.rs:25` enforces real-workflow guard execution, planted-defect discrimination, and `--self-test` parity inside `cargo test`, while both shell scripts pass Gate B shell linters.
   - evidence: `cargo test --test falsification_492_apt_lock_guard` (`3/3`), `shellcheck -S error` (`rc=0`), and `bashrs lint --level error` (`0` errors) all passed on `<host>`.
4. [doc] C4 — `CHANGELOG.md:8`, `docs/roadmaps/roadmap.yaml:7110`, and `docs/audits/impl-PMAT-492-receipt.md:15` record `PMAT-492` with reproducible verification commands.
   - evidence: `cargo fmt --all -- --check` (`rc=0`) and `cargo clippy --test falsification_492_apt_lock_guard -- -D warnings` (`rc=0`) passed cleanly on `<host>`.

## REFUTED

1. [code] R1 — Round-1 `scripts/apt-lock-timeout-guard.sh:39` (`@77742fbe`) filtered whole lines with `grep -v 'DPkg::Lock::Timeout'`, passing bare `apt-get` calls disguised by trailing `# DPkg::Lock::Timeout` comments, `Timeout=0`, or `&&` chains.
   - corrected: Commit `d684c8dd` replaced line-grep with an `awk` per-invocation parser (`scripts/apt-lock-timeout-guard.sh:39`) and `28` regression cases in `scripts/apt-lock-timeout-guard-test.sh:49`.
2. [test] R2 — Round-2 `scripts/apt-lock-timeout-guard-test.sh:49` (`@d684c8dd`) used `printf "$2"` on YAML fixture strings, producing 4 `SC2059` `[error]` diagnostics under `bashrs lint` (breaking `scripts/dogfood/comply.sh` Arm 4b).
   - corrected: Commit `7af27752` replaced all 5 variable-format calls in `scripts/apt-lock-timeout-guard-test.sh:49` with `printf '%b' "$2"`, verified at `0` `bashrs lint` errors on `<host>`.
