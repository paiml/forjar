# Implementation receipt — PMAT-1000 — `github_runner` resource (PR #1001)

verdict: PASS at the change head named below, with the not-measured items listed. The resource registers, checks and removes self-hosted runner seats through a GitHub App; this receipt covers the review fix for the `sha256` shell injection found by all three review lanes at 8df9f3b.

## Identity

- ticket PMAT-1000 (forjar#1000, epic #697); kind: code
- PR #1001, branch `feat/1000-github-runner-resource`; reviewed head 8df9f3b, fix on top of it, merged with origin/main (CHANGELOG.md and roadmap.yaml conflicts resolved: main's [1.34.1] entries kept once, this branch's PMAT-1000 row added)
- files: `src/resources/github_runner/{script,mod}.rs`, `src/core/parser/github_runner_validate.rs`, `tests/falsification_github_runner.rs`, this receipt

## Before

`script.rs` put the manifest `sha256` inside a double-quoted `echo` (the mismatch message) and stripped only `'`. Validation checked only that the field was non-blank. A value such as `$(touch marker)` is never a valid digest, so the checksum always failed and the mismatch line, with the command substitution, always ran on the seat (as root when the seat script runs as root). One lane reproduced it with a bash probe.

## After

- Validation (`forjar validate`) refuses a `sha256` that is not exactly 64 hex characters, naming `sha256` (a still-templated value is judged after resolution).
- Resolution (`Seat::from_resource`) refuses the same, so a templated value cannot slip a payload in.
- The script builder carries a third guard: a digest that is not 64 lowercase hex characters yields a script that only prints the refusal and exits 1; the digest is never interpolated. The mismatch message now prints the digest through `sh_squote`, outside the double quotes.

## Falsifier

- `f6_sha256_must_be_64_hex_and_a_payload_never_runs` (tests/falsification_github_runner.rs): three bad values (`"$(touch M)"`, a short hex string, a backtick payload) are each refused naming `sha256`, and the marker file never appears.
- `a_non_hex_digest_never_reaches_the_script` (script.rs unit test): the built script contains neither the payload nor `sha256sum`.
- RED then GREEN: the new integration test was run against the 8df9f3b sources first (RED, result below), then against the fixed sources (GREEN).

Results (build host, CARGO_TARGET_DIR on the host): RED: `f6_sha256_must_be_64_hex_and_a_payload_never_runs` FAILED against the 8df9f3b sources (1 failed). GREEN on the fixed sources: the three github_runner integration targets pass (7 + 6 + 7), the 20 `github_runner` lib unit tests pass (19 before, plus the new script test), `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean (host t2yoga, x86_64).

## Not measured

- No live GitHub call: every test uses the local fake API and a fake tarball. `ACTIONS_RUNNER_INPUT_TOKEN` against a real runner is unverified.
- The `.quorum` bundle (Gate E) is not written: agy is out of credits. Three Opus fallback lanes reviewed 8df9f3b (`degraded: same-family`); all three FAILed on this injection, and this change answers it. A fresh review is requested at the new head.
- Gate content is unchanged by this change; the new test files are not yet in the `examples-validate` test list (see the PR comment for the exact change that would be needed).

IMPL-PMAT-1000-RECEIPT-END
