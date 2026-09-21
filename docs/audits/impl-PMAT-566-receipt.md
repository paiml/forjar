# Implementation receipt — PMAT-566 — the MSRV job left a toolchain override behind

verdict: PASS — msrv.yml selected its toolchain with `rustup override set 1.89.0`, a DIRECTORY OVERRIDE that outlives the job on the reused `data/actions-runner-N` workspaces and outranks rust-toolchain.toml; every later forjar job on that runner silently ran 1.89.0 instead of the pinned 1.93.0. It now selects the MSRV per job with `RUSTUP_TOOLCHAIN` and clears any override a previous run left, and mutation.yml clears one before `cargo install cargo-mutants`. A falsification test forbids any workflow from writing a directory override and ties the MSRV job to Cargo.toml's `rust-version`.

## How this was implemented — stated, not inferred

Directly, in a forjar session, not through the paiml-implement harness; no
discover.json, route.sh verdict or dispatch ledger exists and none is invented.
Quorum: `.quorum/PMAT-566-msrv-must-not-leave-an-override.json`, produced by the
quorum that reviewed this branch and committed with it.

## What was measured

mutation on main at 7bfb5720, landed on intel-clean-room-2:

    info: default toolchain set to stable -- rustc 1.98.1
    info: note that the toolchain '1.89.0' is currently in use
          (directory override for '/home/noah/data/actions-runner-2/_work/forjar/forjar')
    error: failed to compile `cargo-mutants v27.1.0`
      rustc 1.89.0 is not supported by the following packages: cargo-platform@0.3.3 requires rustc 1.91

The fleet was never short of a toolchain: dtolnay installed 1.98.1 and made it
the default, and the override pinned the workspace to 1.89.0 regardless. The
issue's title ("the fleet's 1.89 toolchain") names the symptom, not the cause.

## One cause, two symptoms, first diagnosed as two other things

- forjar#567 (`lint`): diagnosed first as "some runners lack clippy", then as a
  directory override. The override was right; where it came from was not asked.
- forjar#566 (`mutation`): diagnosed as the fleet's toolchain being too old.

Both are the MSRV job's override. #567's fix (`rustup override unset` in lint.yml)
treated the symptom in one workflow; this removes the source.

## Gaps

- Every other job that ran on a poisoned runner tested 1.89.0 while the repo pins
  1.93.0. Those results were not wrong so much as testing the wrong toolchain;
  nothing here re-runs them.
- Runners still carrying an old override are cleared the next time msrv, lint or
  mutation runs in that workspace. A job that reads the toolchain before any of
  those three runs on a given runner will still see 1.89.0 once.

IMPL-PMAT-566-RECEIPT-END
