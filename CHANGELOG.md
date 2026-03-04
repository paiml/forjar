# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
