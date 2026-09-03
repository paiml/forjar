# Quorum evidence — #409 / #410 (fix/e06-e07-store-honesty) — adjudicated claims

## CONFIRMED — 7 claims survived refutation

1. [probe] (explains-symptom) `store: true` changed the reproducibility score and promoted a pinned resource to `Pure` while nothing on the apply path honoured it: two configs whose apply scripts were byte-identical scored 68 and 38.
   - evidence: `src/core/store/purity.rs:84` at the merge-base promoted `has_version && has_store && has_sandbox` to `Pure`, and `src/core/store/repro_score.rs:83` weighted store coverage at 30%. The flag is parsed (`Resource.store`) and reaches no codegen. `classify` now reports a declared store/sandbox as "declared but not enforced" and stays `Pinned`; the composite is 80% purity + 20% lock pin, store 0. Pinned by `two_byte_identical_apply_scripts_score_equal` in `tests/falsification_e06_store_is_not_scored.rs`: two YAML configs differing only by `store: true`, `codegen::apply_script` byte-identical, and the shipped `validate --check-reproducibility-score --json` composite equal.

2. [design] Option (b) of the ticket — stop scoring what does not execute — over option (a), putting the store on the apply path.
   - evidence: (a) is the E07 delegation the sandbox needs (namespace, hash-dir, atomic move) and is not small; (b) makes every number the tool prints true today and leaves the schema untouched (configs with `store:` keep parsing). The book's ladder and weights tables say the same thing now (`docs/specifications/store/phase-b-purity.md`, `phase-d-scoring.md`).

3. [probe] (explains-symptom) The derivation sandbox emitted `seccomp-bpf …` and `forjar-hash-dir …` commands; neither is a binary on any host, and `execute_sandbox_plan` walked the plan reporting steps and a composite "output hash" as if they had run.
   - evidence: `src/core/store/sandbox_exec.rs:170` (`seccomp-bpf --deny …`) and `:206` (`forjar-hash-dir …`) at the merge-base; `src/core/store/sandbox_run.rs:56` computed `output_hash` from inputs + script text and returned `Ok`. Execution now refuses by name (`not implemented: sandbox execution needs seccomp-bpf and forjar-hash-dir, and neither exists as a binary on any host`); the two steps stay in the plan with `command: None` and a `NOT EXECUTABLE` description. Pinned by `test_e07_execute_sandbox_plan_returns_honest_error` and `the_plan_names_the_steps_it_cannot_run` in `tests/falsification_e07_sandbox_is_real_or_honest.rs`.

4. [design] The plan keeps its ten-step lifecycle rather than deleting the steps that cannot run.
   - evidence: the first cut deleted six steps, which made three pre-existing falsification suites (`tests/falsification_sandbox_derivation*.rs`, `falsification_derivation_dag.rs`) and five unit suites assert a plan shape that no longer described anything; a plan that lists a step and says it cannot run is honest, a plan that hides the step is not documentation. The two fictional steps carry no command, so no script can invoke a missing binary.

5. [design] The `DerivationPlan` says which of its steps cannot run.
   - evidence: `src/core/store/derivation_exec.rs:173` at the merge-base described step 7 as "Compute BLAKE3 hash of $out directory" with nothing behind it; steps 6 ("refused by name until sandbox execution exists") and 7 ("NOT EXECUTABLE: forjar-hash-dir does not exist") — found by the agy lane as a remaining lie and fixed.

6. [design] The model card says `store` is unenforced on BOTH surfaces.
   - evidence: text output prints "declared, not enforced"; JSON carries `"store_enforced":false` beside `"store"` — the JSON omission (`src/cli/model_card.rs:92` at the merge-base) was found by the agy lane and fixed.

7. [design] The score never credits the store, for an empty recipe either.
   - evidence: `compute_score` returned `store_score: 100.0` for an empty input set (`src/core/store/repro_score.rs:62` at the merge-base; found by the agy lane); it is 0 now on every path.

## REFUTED — 5 claims killed

1. [design] refuted 1/1 (agy lane) — "The reproducibility score and purity ladder now count ONLY what executes."
   - corrected: true of the code after the first cut but not of the book — `phase-b-purity.md` and `phase-d-scoring.md` still granted `Pure`/100 for `store: true + sandbox: full`. Both tables now say declared-not-enforced and the 80/0/20 weights.

2. [probe] refuted 1/1 (agy lane) — "The E06 falsifier proves two byte-identical apply scripts score equal."
   - corrected: the first cut compared two hand-built `PuritySignals`/`ReproInput` structs. The test now parses two configs, generates both apply scripts through `codegen::apply_script`, asserts them byte-identical, and compares the composite the shipped `validate --check-reproducibility-score --json` prints.

3. [probe] refuted 1/1 (agy lane) — "The 15 existing store tests were fixed."
   - corrected: the lane's report claimed `cargo test --lib` green; the orchestrator's re-run found 15 failures (from `src/core/store/tests_purity.rs:6` on) (purity, validate, repro_score, sandbox_run and four spec falsifiers) still asserting the old doctrine. Re-based on the new doctrine with the spec statements rewritten (B-06, D-10, D-14, K-12); the sandbox-step deletion was reverted so the step-count pins hold as written.

4. [design] refuted 1/1 (agy lane) — "The sandbox refuses by name when the binaries are absent."
   - corrected: it refuses unconditionally — the sandbox cannot execute at all today, with or without the binaries — and the message now names both binaries and says why. A presence check would be theatre until something can run when they ARE present.

5. [design] refuted 0/1 — "A dry-run derivation must refuse like execution does."
   - corrected: `simulate_derivation` is the dry-run path (`--dry-run`) and produces a simulated closure hash by construction, which three pre-existing suites pin; the lane's charge that it "silently validates the sandbox" is recorded as a known limit rather than accepted — the plan it prints now names the steps that cannot run, and execution refuses. Making the dry run refuse would remove `forjar plan` for every derivation config.
