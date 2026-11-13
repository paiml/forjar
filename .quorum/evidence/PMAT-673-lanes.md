# Lanes — `stress.yml` timeout envelope & `SEC005` token boundary (PMAT-673 / forjar#914)

Three-lane `LANES=3` review by `agy` (`Gemini 4 Carbon (thinking=high)`), distinct from the Claude author model, at head `7c609178d784bb214105e1d8a4eaca081e900494`:

- **Lane 1 (Worst-Case Runtime Budget vs Step Timeout Invariant)**: `PASS` — verified at 7c609178 against the figures it then carried (3 passes x 4 min <= 30 min). Author note, not an agy verdict: the corrected figures are `3 * 9 min + 10 min build = 37 min <= 45 min`, and 3 legs = 9 weekly stress passes and zero conflicting job-level timeout in `.github/workflows/stress.yml`.
- **Lane 2 (Matrix Failure Isolation & `SEC005` Boundary Falsifier)**: `PASS` — verified `fail-fast: false` in `.github/workflows/stress.yml` and `falsification_sec005_needs_a_token_boundary` (`2/2` green) as they stood at 7c609178, where the SEC005 test used the unquoted form only. The quoted-assignment test was added after this round and is not re-adjudicated.
- **Lane 3 (Revert Kill, Non-Blocking Workflow Scope & Traceability)**: `PASS` — verified `stress.yml` is a weekly-cron workflow (`RULES-GATES-1111` clean) and reverting `.github/workflows/stress.yml` to `origin/main` on `<host>` (`<SCRATCH>`) fails both tests (`FJ_914_REVERT_RC=101`).
