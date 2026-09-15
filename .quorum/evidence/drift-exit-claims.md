# PMAT-562 — the claims put to the round

The branch makes `forjar drift` return its verdict as its exit code on every
run: any DRIFTED line exits 1 with no flag, `--tripwire` is accepted and inert,
the unmeasured-only exit (4, forjar#549) is unconditional too. Measured on the
fleet under 1.30.0: `Drift detected: 2 resource(s)` then `rc=0`, on yoga and
gx10 (paiml/infra#605, second signature).

Three lanes, read-only, sandboxed, against a self-contained clone of the PR
worktree at `d1347afe`, dispatched in one message with `--not-before` pinned to
the dispatch instant and `out_dir` keyed by ticket AND session id.

The questions, identical to every lane:

1. Is there ANY path through `cmd_drift` that prints a DRIFTED finding and
   returns Ok — `--dry-run`, `--auto-remediate`, `--alert-cmd`, `--json`, the
   lockless scan, `--all-stacks`?
2. `apply --abort-on-drift` calls `cmd_drift` with the flag set to true. Does
   the change alter apply in any way; does any other caller depend on
   Ok-on-drift?
3. The book now says exit 1. Grep the whole tree for any remaining sentence
   that says drift exits 2, exits 10, or needs `--tripwire` to exit non-zero.
4. The contract: quote any false sentence; does each falsifier's named
   mutation turn its cited test red?
5. The CHANGELOG entry: quote any sentence a reader could check and find
   false.
6. Is an ignored bool parameter the right shape for the kept flag, given the
   fleet's cron lines and Makefiles pass `--tripwire` today?

The acceptance command every lane was told to run:

    cargo test --test falsification_drift_verdict_reaches_the_exit_code \
      && cargo test --lib -- cli::tests_drift cli::tests_check \
      && cargo test --test falsification_drift_unmeasured_is_not_drift \
      && cargo test --locked --test falsification_contract_citations_resolve

And the standing instruction: a finding not re-executed is a claim, say so;
no edits, no push, no scratch cargo projects.
