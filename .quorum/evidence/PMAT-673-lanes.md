# Lanes — `stress.yml` timeout envelope & `SEC005` token boundary (PMAT-673 / forjar#914)

Three-lane `LANES=3` review by `agy` (`Gemini 4 Carbon (thinking=high)`), distinct from the Claude author model, at head `7c609178d784bb214105e1d8a4eaca081e900494`:

- **Lane 1 (Worst-Case Runtime Budget vs Step Timeout Invariant)**: `PASS` — verified `3 * 4 min = 12 min <= 30 min` (`2.5x` contention margin across 3 legs = 9 nightly stress passes) and zero conflicting job-level timeout in `.github/workflows/stress.yml`.
- **Lane 2 (Matrix Failure Isolation & `SEC005` Boundary Falsifier)**: `PASS` — verified `fail-fast: false` in `.github/workflows/stress.yml` and `falsification_sec005_needs_a_token_boundary` (`2/2` green).
- **Lane 3 (Revert Kill, Non-Blocking Workflow Scope & Traceability)**: `PASS` — verified `stress.yml` is a nightly workflow (`RULES-GATES-1111` clean) and reverting `.github/workflows/stress.yml` to `origin/main` on `<host>` (`<SCRATCH>`) fails both tests (`FJ_914_REVERT_RC=101`).
