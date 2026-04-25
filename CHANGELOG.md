# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.0] - 2026-04-25

### Added
- `forjar reseal` recovery subcommand for re-creating sidecar BLAKE3 integrity files (#118, #119)
- Contract trait enforcement expanded from 7/13 to 13/13 implementations
- `pv codegen` contract macros for build-time generation (Refs PMAT-120)
- Contract call-site instrumentation for `hash_data` + `execute_isolated` (Refs PMAT-122)
- `vendored-openssl` feature flag for cross-compilation reliability (Refs PMAT-067)
- AllImplemented enforcement policy — build now fails on contract gaps
- Sovereign-CI self-hosted runner adoption with PR authorization gate
- Nightly Criterion benchmarks via reusable workflow

### Changed
- Migrated from archived `provable-contracts` → `aprender-contracts` (#117, Refs PMAT-163)
- Bumped major dependency versions: `aprender-contracts(-macros) 0.30 → 0.31`, `bzip2 0.5 → 0.6`, `toml 0.8 → 1.1`, `criterion 0.5 → 0.8` (dev-dep)
- `cargo update`: tokio 1.50 → 1.52, indexmap 2.7 → 2.14, regex 1.x → 1.12, openssl 0.10.x → 0.10.78, async-trait 0.1.x → 0.1.89, plus dozens of compatible bumps across the tree
- Replaced deprecated `criterion::black_box` with `std::hint::black_box` in benches
- `pmat repo-score` compliance improved from 79.0 → 91.5
- README: added Features section, docs.rs badge, cookbook link, CI/crates.io badges, MSRV badge corrected to 1.89.0
- `.gitignore`: added `.claude/` (Claude Code session state) so it doesn't block clean publishes

### Fixed
- Sidecar BLAKE3 errors now propagate instead of being silently swallowed (#118, #119)
- Removed hardcoded `/mnt/nvme-raid0` path from `.cargo/config.toml` (#109, #110)
- Doctor SSH test handles missing `ssh` binary in CI containers
- `generate_installer` complexity reduced for CB-200 compliance (Refs PMAT-131)
- `generated_contracts.rs` is now a build artifact (gitignored, replaced 5858-line stale stub)
- 11 silently-ignored CLI flags now emit warnings
- `ingest_state_dir` errors are logged instead of silently discarded
- Security advisories: `tar 0.4.45` (RUSTSEC-2026-0067/0068), `rustls-webpki 0.103.10` (RUSTSEC-2026-0049)
- Contract-trait enforcement test added (provable-contracts §23)
- Parser whitelist now recognizes top-level `dist:` field — previously `forjar fmt → forjar validate --strict` failed and many commands logged spurious "unknown field 'dist'" warnings against dist-aware configs
- Race condition in `cli::colors` tests serialized via per-module `Mutex` — global `NO_COLOR` atomic could be flipped by a parallel test mid-assertion, causing intermittent CI failures

### Security
- Updated multiple deps for RUSTSEC-2026-{0007,0009,0041,0044-0049,0067,0068}
- `deny.toml`: explicit advisory ignores documented with reason + review date
- `RUSTSEC-2026-0104` (rustls-webpki CRL panic) acknowledged with mitigation note

## [1.2.1] - 2026-03-13

### Added
- `forjar dist` command and full release pipeline (FJ-3600)
- `github_release` resource type for nightly binary installation (FJ-034)
- WASM plugin runtime via `wasmi` (FJ-3404, #80)
- Watch daemon for filesystem-driven re-apply (FJ-3102)
- `aarch64-linux-gnu` to nightly build matrix
- All competitive feature gaps implemented (#77-#84)
- Refactored 26 oversized files under 500-line limit (Refs PMAT-029, PMAT-056)

### Fixed
- Vendor OpenSSL handling for cross-compile (later made opt-in via feature flag in 1.3.0)
- Template parameter resolution in `github_release` resource fields (Refs FJ-034)
- Asset filename preservation in `github_release` download
- Pre-apply drift check now passes machine context for container transports (Refs PMAT-058)

## [1.2.0] - 2026-03-10

### Added
- `forjar dist` command family (FJ-3600): generate distribution artifacts from YAML config
- 7 artifact generators: shell installer, Homebrew formula, cargo-binstall, Nix flake, GitHub Action, deb, rpm
- DistConfig type system with per-target libc variant support
- macOS targets (x86_64-apple-darwin, aarch64-apple-darwin) in release pipeline
- `cargo binstall forjar` support via `[package.metadata.binstall]`
- `.github/actions/setup-forjar` composite action for CI consumers
- `install.sh` at repo root for `curl | sh` installs
- `flake.nix` at repo root for `nix run github:paiml/forjar`
- Automated Homebrew tap publishing with real SHA256 checksums on release
- 56 Popperian falsification tests for dist generators
- `dist-forjar.yaml` — dogfood config for forjar's own distribution

## [1.1.1] - 2026-03-04

### Fixed
- Refactored 3 dispatch functions below TDG Grade A complexity threshold (CB-200)
- Removed 23 unwrap() calls from production code (kaizen RUST-UNWRAP-001)
- Fixed clippy warnings when building without encryption feature
- Pinned GitHub Actions reusable workflow to SHA (CB-953)

### Changed
- Made `age` encryption crate optional via `encryption` feature flag
- Reduced prod transitive dependencies from 305 to 253 (CB-081)
- Added `.pmat.yaml` with CB-954 suppression for `secrets: inherit`

## [1.1.0] - 2026-03-03

### Added
- Features #164-#166: Complexity analysis, impact analysis, drift prediction CLI commands
- Chapter 17: Operational Intelligence in the forjar book
- Chapter 18: Supply Chain Security & Resilience in the forjar book
- 7 new cookbook recipes (79-85) with A-grade quality scores
- 500+ new tests across 21 test files for 95%+ line coverage
- 3 cargo run examples: complexity, impact, drift prediction

### Fixed
- 5 stale entries in v2 spec falsification audit log (PMAT-042 through PMAT-046)

## [1.0.0] - 2026-03-01

### Added
- 163/163 v2 spec features complete
- Reproducible binary builds (FJ-095)
- Formal verification proofs: Kani + Verus
- State safety with BLAKE3 integrity chains
- MLOps/DataOps resource types (model, dataset, pipeline)
- Agent infrastructure (pull-based, registry, SBOM)
- Post-quantum signing (ML-DSA-65)
- GPU container support (NVIDIA + ROCm)
- Store system: content-addressable with GC
- Recipe system with expansion and validation
- 8000+ unit tests, 95%+ line coverage

## [0.2.0] - 2026-02-24

### Added
- SSH transport with batch mode and connection pooling (FJ-040)
- Container transport with Docker/Podman support (FJ-050)
- Rolling deployment with wave-based execution (FJ-060)
- Drift detection with anomaly scoring (FJ-070)
- Fleet status reporting with 200+ analytics flags (FJ-080)
- Lock file security auditing (FJ-090)
- Configuration validation with 30+ checks (FJ-100)
- Graph analysis: dependency visualization, impact, topology (FJ-110)

### Changed
- Upgraded to BLAKE3 1.8 for 15% faster hashing
- Switched to serde_yaml_ng for improved YAML parsing

## [0.1.0] - 2026-02-16

### Added
- YAML configuration parser with validation (FJ-001)
- Dependency resolver with Kahn's topological sort (FJ-003)
- Execution planner with BLAKE3 desired-state hashing (FJ-004)
- Script codegen for package, file, service, mount resources (FJ-005)
- File resource: create, directory, symlink, absent states (FJ-007)
- Package resource: apt, cargo, uv providers (FJ-006)
- Service resource: systemd start/stop/enable (FJ-009)
- Mount resource: NFS/bind mount/unmount (FJ-006)
- Local transport executor (FJ-010)
- SSH transport executor (FJ-011)
- Full apply orchestration with Jidoka failure policy (FJ-012)
- Atomic lock file persistence (FJ-013)
- BLAKE3 hashing for files, directories, strings (FJ-014)
- Append-only JSONL event log (FJ-015)
- Drift detection via hash comparison (FJ-016)
- CLI subcommands: init, validate, plan, apply, drift, status (FJ-017)
- Recipe system with typed inputs, validation, namespaced expansion (FJ-019)
- Provable contracts integration with 15 falsification tests (FJ-020)
- 254 unit tests across all modules
- Criterion benchmarks with 95% confidence intervals
