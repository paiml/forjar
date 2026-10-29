# Judges — PMAT-832

Round count and heads: see `PMAT-832-lanes.md`.

## CONFIRMED

1. [diff] C1 — check resolves the config's data: sources right after parse_and_validate and before the selectors and any template render, so a healthy declared source no longer surfaces as the misleading unknown data source error.
   - evidence: src/cli/check.rs:368 calls resolver::resolve_data_sources(&mut config)? on the already-mutable config, propagating a real failure as the check error.
2. [diff] C2 — drift resolves sources after load_env_params, so env-supplied params are in place, and a failing source only prints a warning and still returns the config, so drift is not aborted by one bad source.
   - evidence: src/cli/drift.rs:192 matches the Err from resolve_data_sources and eprintln!s a warning: data sources line; the Ok(Some(cfg)) return follows unchanged.
3. [test] C3 — the tests run apply, then check or drift, against a fixture with a declared command source and assert unknown data source is absent, plus a broken source (exit 127) that must be named as itself.
   - evidence: tests/falsification_check_drift_resolve_data_sources.rs:46 and :59 are the positive test and the broken-source test; the file is new, so on origin/main the first two print the misleading error.
## REFUTED

1. [test] R1 — the tests assert on output text only and discard exit statuses (the apply result is dropped), and no test runs drift against a broken source to pin the warning path, so the positive tests could pass for the wrong reason.
   - evidence: tests/falsification_check_drift_resolve_data_sources.rs:32 drops the apply result; recorded as a known gap and a follow-up, the shipped code was judged correct by all three lanes.
2. [doc] R2 — the comment in drift.rs says the per-resource template warning names what the failed source affects, which the Opus lane found overstated: the per-resource text may still read unknown data source.
   - evidence: src/cli/drift.rs:192 is the line the comment sits above; recorded as a known gap, the behaviour (warn, not abort) is as the ticket states.
