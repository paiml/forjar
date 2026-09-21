# Claims — quorum.yml's private CARGO_HOME and the lint that keeps it (PMAT-588)

Counts and heads live in `PMAT-588-lanes.md`; numbers here quote it.

The adjudicated claim set, as the round-4 head `3eaa29f3` states it:

- **C1** quorum.yml's `receipt` job sets a job-private `CARGO_HOME` (`${{ github.workspace }}/../cargo-home-${{ github.job }}`, the value its four sibling workflows use); the rust-cache step stays
- **C2** `scripts/lint-rust-cache-guard.sh` decides PER JOB: self-hosted unless runs-on names only a hosted image; exonerated only by a non-shared CARGO_HOME at workflow env, job env, or the cache step's own env; a 12-case self-test, eight cases one per shape
- **C3** wired: `make rust-cache-guard` (self-test, then scan), `lint:` depends on it, lint.yml runs it
- **C4** the Rust falsification test RUNS the lint (self-test, tree scan with anti-vacuity floors, and a negative control on the real quorum.yml) instead of re-implementing the rule
- **C5** the change only adds a check and an env; no permissions, secrets, publish step or required-check name changes

## What the four rounds did

Round 1 found the lint deciding per FILE and the Rust copy of the rule reading a
comment as a declaration — three wrong verdicts from two parsers of one rule. Both
were replaced by one implementation (the lint, per job) with the test driving it.
Round 2 found the test asserting only the patched tree; it now carries its own
negative control. Rounds 3 and 4 ran on the same head; round 3's one FAIL was a
lane error refuted by running the self-test (see lanes).

## Why it matters on this fleet

Every intel runner shares one home, so `~/.cargo` is shared. rust-cache's post
step empties the shared `~/.cargo/bin` (`cleanBin`) and `registry/src`
(`cleanRegistry`). Measured 2026-09-21 06:19:02Z: this repo's quorum.yml post
step on intel-clean-room-15; `~/.cargo/bin` modified 06:19:15Z with 0 regular
files and 15 rustup symlinks left, and an rmedia job's `RUSTC_WRAPPER` pointing
into that directory died "never executed" mid-run (paiml/infra#775 class).
