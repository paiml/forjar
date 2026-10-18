# Judges — forjar#722: arch-neutral lint jobs and arch-keyed caches

Lanes and their findings: see PMAT-721-lanes.md. Line numbers are at head 37955c80.

## CONFIRMED

1. [fix] C1 — The three jobs run no x86-only step, so dropping X64 from their runs-on is safe on an aarch64 runner.
   - evidence: `.github/workflows/lint.yml:139` and `.github/workflows/msrv.yml:18` now read runs-on [self-hosted, clean-room]; the steps are checkout, rustup, cargo clippy, cargo check, fmt and a wc/git script, with no download, docker or shellcheck. The three round-4 lanes read the same steps.
2. [fix] C2 — Both cache keys carry runner.arch beside runner.os.
   - evidence: `.github/workflows/lint.yml:108` is key lint-runner.os-runner.arch-features-lockfile and `.github/workflows/msrv.yml:62` is key msrv-runner.os-runner.arch-lockfile; each file holds exactly one actions/cache step, so no key is left unkeyed.
3. [test] C3 — The guard and its eight-case Rust wrapper discriminate: the real tree and an untouched copy pass, every reverted or bypass shape fails.
   - evidence: `tests/falsification_722_cache_key_arch.rs:50` (real tree), `tests/falsification_722_cache_key_arch.rs:62` (the pre-722 keys are caught), `tests/falsification_722_cache_key_arch.rs:87` (key without arch beside restore-keys with it), `tests/falsification_722_cache_key_arch.rs:113` and `tests/falsification_722_cache_key_arch.rs:127` (block scalar, sequence), `tests/falsification_722_cache_key_arch.rs:140` (comment-only arch), `tests/falsification_722_cache_key_arch.rs:161` (missing file is a failure).
4. [falsified] C3 — With lint.yml and msrv.yml reverted to main and the guard restored, the path test goes red.
   - evidence: measured locally at fcf9b307: the reverted copy failed with a key-count mismatch and the unmodified copy passed; the failing assertion is `tests/falsification_722_cache_key_arch.rs:62`. A guard that dropped its comment strip or its restore-keys check failed the cases at `tests/falsification_722_cache_key_arch.rs:140` and `tests/falsification_722_cache_key_arch.rs:127`.
5. [fix] C4 — No required check is lost.
   - evidence: gate needs classify, ci, lockfile, examples-validate, no-default-features, doctests, dogfood-surface and dogfood-guards at `.github/workflows/ci.yml:420`; lint.yml and msrv.yml are not in it and are not required by the org ruleset, which names only gate.
6. [fix] C5 — The roadmap change is a pure append of the PMAT-721 entry.
   - evidence: git diff of docs/roadmaps/roadmap.yaml against b2918946 is one hunk, three context lines then nineteen added lines, none removed; the entry starts at `docs/roadmaps/roadmap.yaml:4995`.
7. [fix] C6 — CI runs the new test.
   - evidence: `.github/workflows/ci.yml:143` runs cargo test --locked --test falsification_722_cache_key_arch inside examples-validate, which gate lists in needs at `.github/workflows/ci.yml:420` and checks at `.github/workflows/ci.yml:481`.
8. [test] C7 — The fleet-arch test exempts exactly three named jobs and fails on a stale or re-pinned entry.
   - evidence: the allowlist is `tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:137`; `tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:178` marks a job seen only when it carries no X64 or ARM64 label; the stale assert follows at `tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:190`. Measured: 13 passed on 37955c80.

9. [test] C8 — The guard set is tied to ARCH_NEUTRAL_JOBS: a job from a file outside WORKFLOWS or the shell guard turns the new test red.
   - evidence: the allowlist is `tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:137`. Probe: bench.yml/benchmark added there with X64 dropped went RED (8 passed, 1 failed, bench.yml not in WORKFLOWS); restored, 9 and 13 passed. Round 7 PASS on all three lanes.

## REFUTED

1. [lane] R1 — Round 3 Argon on C7: an exempted job re-pinned to X64 was not reported stale.
   - corrected: true, and fixed in 37955c80 (`tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:178`); round 4 confirmed C7 on all three lanes.
2. [lane] R2 — Round 3 Argon on C7: matrix legs are flattened, so a multi-leg job that omits the architecture on one leg passes.
   - corrected: pre-existing in `tests/falsification_every_ci_job_runs_on_the_fleet/scan.rs` and untouched by this PR; the three exempted jobs are exempt by name, so this PR neither adds nor hides such a case. Left for its own ticket.
3. [lane] R3 — Round 2 Sonnet: quorum.yml does not run the new test.
   - corrected: quorum.yml runs scripts/quorum-gate.sh, which runs the falsification cargo target named in the receipt; ci.yml also runs it (C6).
4. [lane] R4 — Round 5 Argon and Flash: the new test ignored the guard exit status, matched ok by substring, and let the reader drop an unparsable line.
   - corrected: true, fixed in e5c0a224; the test now asserts the exit status, matches `ok <path>:` at line start, and panics on an unparsable line or a count mismatch.
5. [lane] R5 — Round 6 Sonnet, Argon and Flash: an empty ARCH_NEUTRAL_JOBS and an inline-commented path passed vacuously.
   - corrected: true, fixed in e642d9d8; the reader asserts a non-empty list and the guard-code check strips comments.

## Accepted, not changed

- Round 4 Argon: falsification_every_ci_job_runs_on_the_fleet is not run by ci.yml. It predates this PR; no workflow ran it before.
- Round 4 Argon: a double-quoted restore-keys holding an escaped newline would pass the line-based guard. No such key exists; the guard is a line check by design.
- Round 4 Argon: the guard workflow list was hardcoded and untied; closed in this revision (C8), the shell guard and WORKFLOWS stay two literal lists that the new test checks against ARCH_NEUTRAL_JOBS.
- Round 3 Sonnet: a flow-sequence restore-keys with brackets is a latent gap in the same class.
- Round 7 extras: the guard header still says lint.yml and msrv.yml; the parser needs the exact declaration text and fails loud otherwise.
- No aarch64 clean-room runner is reachable from this branch, so the aarch64 leg is argued from the steps, not run.
