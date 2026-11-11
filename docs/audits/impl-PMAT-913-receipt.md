# Implementation receipt — PMAT-913 — CI and release gate hygiene (train)

verdict: PASS — PR #978: the `examples-validate` job runs the same 20 non-sudo integration test targets, in the same order, in one `cargo test` invocation instead of 18 steps; no target is dropped and the live-sudo step stays separate.

## Identity

- ticket PMAT-913 (forjar#913, train "CI and release gate hygiene"; work requested under paiml/infra#3828); kind: ci
- PR #978, branch `chore/ci-dedupe-cargo`, change commit acc83726
- files: `.github/workflows/ci.yml` (+32 −123), `docs/roadmaps/roadmap.yaml` (row `GH-913` renamed to `PMAT-913`)

This receipt covers PR #978 only. Other PRs under the same train add their own section below when they land.

## PR #978 — one `cargo test` invocation for the non-sudo targets

Why: each of the 18 non-sudo steps paid its own cargo resolve and link plan on a shared runner pool that was already queued.

Contract: the set of `--test` targets executed by `examples-validate` is unchanged, and so is the order of the 20 non-sudo targets among themselves; `falsification_version_matches_manifest`, formerly the last step, now runs inside the merged invocation and so before the live-sudo target instead of after it; `falsification_sudo_transport_survives_closefrom` remains its own step gated on `FORJAR_REQUIRE_SUDO_TESTS == "1"`; `--locked` and the feature set are unchanged.

Falsifier: dropping or reordering one `--test` target fails the step-target bijection check (exit 1).

Evidence:

- Independent review, agy (Gemini 4 Carbon), run on t2fw13: `PASS @acc837269db9f9a1f6699f2376b8a28c30225451`. It parsed the job before (c5b7e4f) and after (acc8372), found the 20 targets 1:1 in order, the sudo step preserved, and killed the target-drop falsifier. File: `handoff/agy/review-forjar-978-acc8372.md` in the fleet inbox.
- Three-lane review at 20c4883 (agy; quorum receipt `.quorum/chore-ci-dedupe-cargo.json`, written by agy and committed unedited): all three PASS. Two claims of this receipt were refuted as written and are corrected above: the step count (18 non-sudo steps covering 20 targets, plus 1 live-sudo step, not 19 merged steps) and the order claim (the version-manifest target moved ahead of the live-sudo target).
- CI on acc83726: rollup SUCCESS, including `examples-validate`, `dogfood-guards` and `gate`. The merged step ran every target on the PR's own run.
- Counts from the PR body (non-comment lines of ci.yml, lint.yml, audit.yml): `cargo test` 21 → 4; `cargo clippy` and `cargo build/check/install` unchanged.

Not measured by forjar-cop on a build host: nothing was compiled for this receipt. The change is workflow YAML, and the PR's CI run is the execution of it.

IMPL-PMAT-913-RECEIPT-END
