# Implementation receipt — PMAT-567 — lint was measuring a toolchain this repo never pinned

verdict: PASS — merged as PR #596 (770a4857). A rustup DIRECTORY OVERRIDE on some clean-room runners' workspaces outranked rust-toolchain.toml and resolved 1.89.0, which has no clippy, while the pin is 1.93.0 with clippy in its components. lint.yml now runs `rustup override unset` and refuses BY NAME ("ENVIRONMENT, not lint") via `cargo clippy --version` before linting. Positive control: `Lint: success` on main with both clean-room legs green, on intel-clean-room-15 AND yoga-build2.

## How this was implemented — stated, not inferred

Directly, in a forjar session, not through the paiml-implement harness; no
discover.json, route.sh verdict or dispatch ledger exists and none is invented.
Quorum: `.quorum/PMAT-567-lint-toolchain-override.json`.

## What was measured

    /home/noah/eph-build2/...             1.93.0  "overridden by rust-toolchain.toml"  PASS
    /home/noah/data/actions-runner-7/...  1.89.0  "directory override for ..."         FAIL

Two runners, one commit, both toolchains installed on the failing one. The
component was never missing; the toolchain was different. The issue's first
diagnosis ("some runners have the component and some do not") was wrong and is
corrected on #567.

## What the quorum changed

All three lanes refuted the first implementation, a version comparison that
`cut -d- -f1` got wrong for `channel = "stable"`, date-pinned nightlies and betas.
Replaced with a direct measurement. And re-running the test's own declared
mutation showed it was vacuous — it asserted a substring that the step's comments
contained. Now anchored to the command line.

## Gaps

The stale overrides on runner workspaces are fleet state, not fixable from this
repository (paiml/infra#813). `mutation.yml` also resolves 1.89.0 on
intel-clean-room-2 and fails installing cargo-mutants 27.1.0 (forjar#566); it does
not run `rustup override unset`, so it may be this same mechanism in a workflow
this PR did not touch. Unmeasured.

IMPL-PMAT-567-RECEIPT-END
