# Implementation receipt — PMAT-660 — `forjar show` does not resolve `data:` sources

verdict: PASS — `cmd_show` resolves data sources right after parse-and-validate, as plan and apply do, so a manifest using `{{data.x}}` no longer fails `show` with "unknown data source"; one new test pins it and fails on main's `show.rs`

## Identity

- ticket PMAT-660 (forjar#660); kind: code
- PR #934, branch `reaper-w/forjar-660`, change commit 1e66fecb
- files: `src/cli/show.rs` (+2), `src/cli/tests_cov_show2.rs` (+19), `docs/roadmaps/roadmap.yaml` (row `GH-660` renamed to `PMAT-660`)

## Root cause (five whys)

1. `forjar show` failed with "unknown data source" on a manifest that `plan` and `apply` accept.
2. `{{data.x}}` was still unresolved when `show` reached template resolution.
3. `cmd_show` never called `resolver::resolve_data_sources`.
4. `plan` and `apply` get that call from `prepare_config`; `show` has its own, shorter path that does not go through it.
5. No test ran `show` over a manifest with a `data:` source, so the missing step was invisible.

## Contract and falsifier

Contract: `forjar show` resolves `data:` sources before templates, so a manifest whose templates read `{{data.*}}` renders under `show` as it does under `plan`. This does not extend to what only `plan`'s `prepare_config` adds (`--env-file` params, the `--workspace` param): `show` still skips those.

Falsifier: `cli::tests_cov_show2::show_resolves_data_sources`. It asserts that `cmd_show` succeeds on a `{{data.*}}` manifest (it failed before); it does not capture stdout, and the substituted value itself is pinned by `core::resolver::tests_data`.

## Evidence (kfocus, `nice -n 19`, 2026-11-11, `cargo test --lib tests_cov_show2`)

RED, with `src/cli/show.rs` taken from main:

    test cli::tests_cov_show2::show_resolves_data_sources ... FAILED
    test result: FAILED. 21 passed; 1 failed; 0 ignored; 0 measured; 13747 filtered out

GREEN at 1e66fecb (forced rebuild from this tree):

    test cli::tests_cov_show2::show_resolves_data_sources ... ok
    test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 13747 filtered out

A first GREEN run was discarded: it reported 20 tests, which showed it had reused another tree's test binary from a shared target directory. The run above was rebuilt from this tree.

## Not covered

- `show` still has its own path beside `prepare_config`; other steps that `prepare_config` performs and `show` skips were not audited here.

IMPL-PMAT-660-RECEIPT-END
