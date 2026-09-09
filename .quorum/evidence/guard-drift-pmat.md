# Quorum evidence — PMAT-215 — mechanical lane

`pmat work validate`: passes.

`analyze_vacuous_tests` over the touched test path: 0. Every case in the new suite drives the real binary through `CARGO_BIN_EXE_forjar` and asserts on its census lines, its findings and its exit code, and each is pinned by a mutation named in the receipt.

`pmat tdg`: no regression; `src/tripwire/drift/task_check.rs` is 238 lines, under the 500-line file-health limit, and the pre-commit gate passed on every commit without `--no-verify`.

`transcript-gate.sh` (I-3): `PASS attempted=11 denied=2 running_peak=3 slots=3`. Two hook denials this session, neither retried.

`kind-gate.sh`: `kind=code ticket=PMAT-215 files=2`.

`model-gate.sh`: `model=opus class=opus decision=admit basis=file`.

`cargo test --workspace`, the discovered `gate_cmd`: **RED**, one failure, `forjar-contracts` `coverage_map_enrichment`. Reproduced on `origin/main` at 54e36f13 from a path where a sibling `aprender` is visible, with none of this branch's changes present. Filed on forjar#452, recorded in `.pmat/jidoka.jsonl`, non-blocking. `discover.json` reports `gate_cmd_fallback=true`, so this command is a fallback rather than the repository's declared gate; the required check is `gate` and the repository's own pre-publish gate is `make dogfood`.
