# Claims — `build.rs` runtime `CARGO_MANIFEST_DIR` lookup & multi-`-p` tool-job guard (PMAT-194 / forjar#909)

Extracted from PR `#909` (`s-forjar-2/pack696`, Round 1 `6f655eac` -> Round 2 `8905e5a2`) and attacked across all 3 review lanes:

- **C1 (`build.rs` reads `CARGO_MANIFEST_DIR` at runtime, never `env!`)**: `build.rs:14` replaces compile-time `env!("CARGO_MANIFEST_DIR")` with `std::env::var("CARGO_MANIFEST_DIR")` so host build paths are never baked into the compiled build-script binary (`tests/falsification_build_rs_manifest_dir_runtime.rs:15`).
- **C2 (`build.rs` emits `cargo::rerun-if-env-changed=CARGO_MANIFEST_DIR`)**: `build.rs:22` declares `cargo::rerun-if-env-changed=CARGO_MANIFEST_DIR` so Cargo invalidates cached build-script outputs if the manifest directory moves (`tests/falsification_build_rs_manifest_dir_runtime.rs:38`).
- **C3 (`cargo_args_are_unfiltered` handles multi-`-p`, `--package=<pkg>`, and `--workspace`)**: `tests/falsification_tool_jobs_run_where_the_tools_are.rs:285` scans all package flags rather than returning early on the first `-p`, catching `cargo test -p forjar-contracts -p forjar` while narrowing `--package=forjar-contracts`.
- **C4 (`CHANGELOG.md` & `PMAT-194` roadmap traceability)**: `CHANGELOG.md:8` and `docs/roadmaps/roadmap.yaml` record `PMAT-194` cleanly.
- **R1 (Round 1 multi-`-p` bypass in `falsification_tool_jobs_run_where_the_tools_are.rs:289`)**: At `6f655eac`, `cargo_args_are_unfiltered` returned `false` on the first non-root `-p <pkg>`, misclassifying `cargo test -p forjar-contracts -p forjar` as narrowed.
- **R2 (Round 1 `--package=<pkg>` parsing bug at `6f655eac`)**: At `6f655eac`, `--package=forjar-contracts` (`=` form) was misclassified as unfiltered while `--package forjar-contracts` was classified as narrowed.
