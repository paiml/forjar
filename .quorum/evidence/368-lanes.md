# Quorum evidence — #363 / #368 / #378 — lane summaries

## probe lane
Built the branch and ran the three falsifiers plus `cargo test --lib`
(13370 passed, 0 failed), clippy `-D warnings` and `fmt --check` clean.
Before the fix: a phony resource made `apply --plan-file` refuse with
"config has changed since plan was created"; `--force` with a failing
resource aborted in debug and over-reported in release; `--refresh-only`
ignored `--operator`.

## crux lane
Terraform binds a saved plan to the state's lineage/serial and refuses a
stale one without prompting — the plan IS the review; it does not refresh.
Pulumi's `up --plan` constrains the engine and evaluates live state at
execution time, and prompts. Ansible has no saved-plan artefact; `--check`
and `--diff` are dry runs over live hosts. This branch puts forjar at the
Terraform default (hash-bound plan, gates run, prompt runs); the live probe
Pulumi runs is the gap recorded in #432.

## design lane
One `seal_config` snapshot feeds both hashes; one `run_plan_apply_gates`
holds the gate list in the order the interactive apply uses; the forced
count is reconciled against the run. The drift probe's omission is written
down where the gates are, not discovered later.

## judges
Two decisions scored: which config to hash, and where the forced count is
taken. See the judges file.

## agy /teamwork
Independent plan-mode review in a scrubbed HOME (no publish/push
credentials reachable) — see the agy file. Its refutation of the drift-gate
claim was accepted and filed as #432; its GH-208 finding was checked
against the merge-base and found pre-existing.
