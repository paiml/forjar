# Claims — forjar#722: arch-neutral lint jobs and arch-keyed caches

Briefed to every lane with the full diff against main b2918946. C1–C7 were in every round; C6 and C7 were added after rounds 1 and 2 found the defects they answer.

- C1: dropping X64 from runs-on of lint.yml clippy (both legs), lint.yml file-health and msrv.yml msrv is safe on aarch64 runners: none of those jobs runs an x86-only step (binary download, x86 target triple, docker, shellcheck).
- C2: every actions/cache key in lint.yml and msrv.yml carries runner.arch, so x86_64 and aarch64 runners no longer share one target cache.
- C3: tests/cache-key-arch-guard.sh fails when any key: or restore-keys: line lacks runner.arch, or restore-keys is a block scalar or sequence, and passes otherwise; tests/falsification_722_cache_key_arch.rs proves it on the real tree, an unmodified copy, a fully reverted copy, a key-without-arch copy, block-scalar and sequence copies, a comment-only arch, and a missing workflow.
- C4: no required status check is lost: only gate is required and gate's needs do not include lint.yml or msrv.yml.
- C5: the roadmap.yaml change only adds one PMAT-721 entry and alters no existing ticket.
- C6: the Rust test runs in CI: ci.yml examples-validate has a named --test step, and gate needs examples-validate.
- C7: tests/falsification_every_ci_job_runs_on_the_fleet exempts exactly the three named arch-neutral jobs, fails on a stale entry (gone, not self-hosted, or re-pinned to an architecture), and still fails any other self-hosted job that names no architecture.
