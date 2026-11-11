# Implementation receipt — PMAT-848 — the cron check guard probes the crontab read, not `sudo -n true`

verdict: PASS — the cron check SKIPs only when sudo itself refuses the crontab read; under a crontab-only sudoers rule it observes the crontab, and a user with no crontab is `missing`, not SKIP.

## Identity

- ticket PMAT-848 (forjar#848); kind: code
- PR #903, branch `forjar-cop/packC` (shared with PMAT-685, receipt `docs/audits/impl-PMAT-685-receipt.md`)
- files: `src/resources/cron.rs`, `src/resources/tests_cron_exec.rs`, `docs/roadmaps/roadmap.yaml` (row GH-848 renamed PMAT-848)

## Root cause (five whys)

1. `forjar check` reported SKIP (exit 2) for a cron resource on a host where forjar could read the crontab.
2. `CRONTAB_CHECK_GUARD` exited 2 before the read.
3. The guard probed privilege with `sudo -n true`.
4. The host's sudoers rule allows `crontab` only, so `true` is refused while the real read is allowed.
5. The guard tested a command the check never runs instead of the read it does run.

## Contract and falsifier

Contract: as non-root, the guard runs `sudo -n crontab -u <user> -l` and exits 2 with `FORJAR_SKIP` only if that fails AND its stderr has a line starting `sudo:`. Every other outcome, "no crontab" included, falls through to the fail-closed read and verdict. Root skips the probe.

Falsifiers: `resources::tests_cron_exec::fj848_check_observes_under_a_crontab_only_sudoers_rule`, `fj848_no_crontab_is_missing_not_skip`, `fj848_sudo_refusing_the_read_is_still_a_skip`.

## Evidence (kfocus, `nice -n 19`, 2026-11-11, head e33a2e5 plus this receipt)

RED, with `src/resources/cron.rs` taken from main and the tests kept (`cargo test --lib fj848`):

    test resources::tests_cron_exec::fj848_check_observes_under_a_crontab_only_sudoers_rule ... FAILED
    test resources::tests_cron_exec::fj848_no_crontab_is_missing_not_skip ... FAILED
    test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 13770 filtered out; finished in 0.02s

`fj848_sudo_refusing_the_read_is_still_a_skip` passes on both sides, as it should: that behaviour is kept.

GREEN (`cargo test --lib cron`):

    test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 13656 filtered out; finished in 0.36s

`cargo fmt --all -- --check` rc=0; `cargo clippy --all-targets -- -D warnings` rc=0.

IMPL-PMAT-848-RECEIPT-END
