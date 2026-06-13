# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.6.0] - 2026-06-13

### Added

- **L3–L5 test-coverage persistence** (#155, FALSIFICATION-REPORT E9):
  `forjar test coverage` now reports beyond L2. Convergence/mutation/
  preservation runs append hash-stamped results to a `test-coverage.jsonl`
  log in the state dir; `cmd_test_coverage` reads them back and promotes
  each resource to the highest passing level whose `config_hash` still
  matches the resource's current desired-state hash — a changed resource
  falls back to its static L0–L2 level (no stale high-water marks). New
  `core::store::coverage_persist` + `cli::coverage_promote`.

### Security / Fixed — 28-defect deep audit (#154)

An adversarial bug-hunt (8 fault-dimension finders, every finding
double-refuted) surfaced 28 confirmed production defects; all are fixed:

- **apply no longer reports success on real failures** (#157): a failed
  `pre_apply:` hook now fails the resource (was reported `Skipped`, apply
  exited 0, dependents ran, rollback was skipped) on the sequential path;
  OCI registry push now checks HTTP status via `curl --fail-with-body`
  (401/404/413/5xx were reported as successful pushes); the coverage
  promotion gate distinguishes "llvm-cov absent" (advisory pass) from
  "llvm-cov ran and failed" (gate failure).
- **Shell-injection hardening** (#161): new `core::shell_escape::sh_squote`
  + identifier validators route every resource-handler data field through
  proper single-quote escaping. Fixes unescaped/unquoted interpolation in
  `build` (arbitrary command over ssh), `github_release` (repo/tag command
  substitution), `task`, `network` (ufw), the binary-cache rsync path, and
  the file/mount/package/cron/docker/model handlers.
- **Concurrency & resource leaks** (#159): mutation-runner sandbox dir now
  uses a process-wide atomic sequence (the #141 race, unpatched in the
  sibling); `acquire_process_lock` is now atomic via `O_EXCL` create_new
  (was a TOCTOU letting two `apply` runs both win); transport timeout now
  kills+reaps the child (was leaking a thread + orphan process); stdin-write
  failure reaps the child across all 7 transports (was leaking zombies);
  container build cleans up via RAII guards on every error path.
- **Storage integrity** (#158): OCI base-image blob paths validate the
  `sha256:<64-hex>` digest (a `..`-bearing digest allowed arbitrary
  read/write); state encryption writes atomically (temp+rename, was an
  in-place truncate that corrupted state on a mid-write crash); `lock
  defrag` refreshes the BLAKE3 `.b3` sidecar (was bricking the next apply's
  integrity check).
- **Planner/executor correctness** (#160): planner and `refresh-only` now
  resolve secrets with the same provider config the executor uses, ending
  a perpetual spurious-Update idempotency violation; the rolling-deploy
  `max_fail_percentage` gate uses integer math (was `as u8`-truncated at
  the boundary); `moved:` blocks reject collisions/chains at validate time
  (were silently overwriting lock state).
- **FAR archive + parser hardening** (#156): FAR decode bounds the
  manifest-length and chunk-count allocations from attacker-controlled
  headers (was unbounded → OOM/abort); the RFC-3339 and duration parsers
  validate ranges and use checked arithmetic (out-of-range month/day and
  multibyte/overflow inputs previously panicked).

## [1.5.0] - 2026-06-13

### Added

- **`forjar dist --verify` — Tier 1 static installer verification**
  (#146, FJ-3607/F-3609, Phase D of spec 25): generates artifacts to a
  temp dir and verifies `sh -n` parse, zero bashrs lint errors
  (in-process via the existing purifier wrapper), required
  checksum/arch-detection snippets, and download-URL structure.
  A deliberately broken asset template fails verification with a
  non-zero exit (F-3609). Tier 2 container execution is a follow-up.
- `forjar schema` now emits the `dist:` property block (mirrors
  DistConfig/targets/homebrew/nix, with a keep-in-sync pin test) —
  the spec claimed this existed; now it does (#146).
- `dist.source` validation: values other than `github_release`
  (local/url/s3) return a clear "not yet supported" error instead of
  generating artifacts with broken URLs (#146).

### Fixed

- **Coverage workflow de-flaked** (#147, closes #141): sandbox dirs in
  `run_convergence_test` were named from a wall-clock nanosecond read;
  concurrent threads on coarse-tick runners collided, and the first
  finisher's cleanup deleted the sibling's working dir mid-cycle
  (reproduced 32/240 trials). Names now include a process-wide atomic
  sequence — also hardens parallel `forjar check` in production.
- Release cargo cache keyed by runner image (#145): v1.4.4's
  x86_64-gnu leg failed linking 24.04-cached objects (glibc 2.38
  `__isoc23_*` symbols) on the new 22.04 baseline runner.

### Changed

- +69 lib tests closing the worst CLI coverage gaps (apply modes,
  canary/rolling fleet ops, infra dispatch, apply variants, check
  blocks, wave outcomes) — suite now 12,205 tests (#148).
- Spec 25 status reconciled from PROPOSED to per-phase reality
  (A-C implemented, D Tier 1 implemented, Tier 2 pending) (#146).

## [1.4.4] - 2026-06-12

### Fixed

- **`install.sh` works end-to-end** (#143) — verified live against the
  v1.4.3 release. The curl installer previously failed on every
  platform: wrong extraction path (archives contain a directory),
  unexpanded `~` fallback dir, hard dependency on a SHA256SUMS asset no
  release carried, `example.com` placeholder URLs, and a post-install
  version check that reported any pre-existing PATH binary. Generator
  now emits `$HOME`, directory-aware extraction, per-asset `.sha256`
  fallback, `"$DEST/$BINARY"`-anchored verification, `--prefix`
  traversal rejection, and lints clean (bashrs 16 errors → 0).
- **gnu binaries run on older distros again**: release builds moved to
  ubuntu-22.04 (glibc 2.35 baseline) — 24.04-built binaries demanded
  glibc ≥ 2.38.
- Regression tests pin the fixes for #85, #86, #88, #90 (#140); 15
  stale issues closed with evidence — open issues went from 18 to 1.
- Six F-grade files refactored to A-/B+ (#142, closes #116): zero
  F-grade files repo-wide; strict TDG pre-commit enforcement can be
  re-enabled via `pmat hooks refresh`.

### Added

- `binary-release.yml` uploads a combined **SHA256SUMS** asset per
  release (backfilled to v1.4.0/1/3); nightly **tag/release parity
  check**; all 8 historical tags now have GitHub releases with
  CHANGELOG-sourced notes, and v1.4.0/v1.4.1 received backfilled
  binaries.
- `tests/install_sh_parity.rs`: committed installer is byte-equal to
  generator output; workflow asset naming matches installer
  expectations.
- README documents the binary install path (installer one-liner,
  manual verify, cargo-binstall).
- `forjar dist` resolves **real checksums/versions** for Homebrew/Nix
  artifacts (#139, PMAT-080/F-3608/F-3610): `--version` + offline
  `--checksums-file`, hard errors instead of `PLACEHOLDER_CHECKSUM`.

## [1.4.3] - 2026-06-12

### Fixed

- **Three user-input-reachable panics** (#132, process aborts under
  `panic = "abort"`):
  - `unquote()` in when-expression evaluation sliced `&s[1..0]` for a
    lone quote character — `when: 'x == "'` panicked at plan time.
  - Secret-lint redaction byte-sliced `&matched[..12]` mid-UTF-8
    (e.g. `sshpass -p пароль`); now char-boundary-safe via the new
    shared `core::strutil` helpers, also adopted by `graph_svg`/`sbom`.
  - `forjar pin --check` sliced `&locked_hash[..16]` on hand-editable
    lockfile data; short/corrupt hashes no longer panic.
- **Auto-commit escaping its repository under git hooks** (#134, #137).
  forjar's git subprocesses inherited `GIT_DIR`/`GIT_WORK_TREE`/… which
  git exports to hook children, so `forjar apply` with `auto_commit`
  inside a git hook committed to the *hook's* repository. All git call
  sites now construct commands via the new `core::gitenv` module, which
  scrubs the nine repo-discovery variables.
- **Release pipeline repaired** (#131). The `Cargo.lock` version bump
  that every v1.4.x tag was missing is committed; a new CI
  `lockfile-preflight` job (`cargo package --locked --no-verify`) makes
  a stale lock fail PR CI instead of the tag; a committed dangling
  symlink (`.claude/worktrees/provable-contracts`) that broke
  `cargo package` on clean checkouts is removed; `.pmat-work/` and
  `.claude/` are excluded from the crate tarball.
- **Security Audit green again** (#131, closes #104, #98). Scoped
  cargo-deny `[[licenses.exceptions]]` for `libbz2-rs-sys` (SPDX
  `bzip2-1.0.6`); audit had been red on main since 2026-04-25.

### Added

- **Provable contracts for IaC convergence** (#136, closes #97):
  `idempotent-apply-v1` (converged lock ⇒ NoOp plan; f(f(x)) = f(x)),
  `plan-apply-equivalence-v1` (plan's predicted action set equals
  apply's executed set), `destroy-undo-roundtrip-v1` (undo restores the
  prior generation byte-for-byte) — 9 new `binding.yaml` entries, 22/22
  bound under `BindingPolicy::AllImplemented`, with `debug_assert!`
  call-sites and unit tests.
- **mdBook built in CI** (#135): new `docs.yml` builds the 102-chapter
  book with a SUMMARY.md resolution check; `tests/doc_cli_parity.rs`
  asserts all 154 CLI subcommands are documented; new CLI reference
  appendix documents 22 previously-undocumented commands; fixed
  `forjar completions` → `forjar completion` and stale MSRV/CLI counts.

### Changed

- **GitHub Releases are now created on hosted runners** (#131):
  `binary-release.yml` triggers on `v*` tag pushes, idempotently creates
  the release, and uploads the 4 Linux binaries — no self-hosted
  dependency in the critical path. Asset naming standardized to
  `forjar-<x.y.z>-<target>.tar.gz` (no `v` prefix) across both release
  workflows, matching homebrew patching and `install.sh`.
- Removed dead `provable-contracts` checkout/symlink/`pv codegen` steps
  from all 13 workflows (#131, #133) — `src/generated_contracts.rs` is
  git-tracked and `aprender-contracts` resolves from crates.io
  (closes #104's original cause; refs #112, #113).

## [1.4.2] - 2026-05-06

### Fixed

- `apply --force` now distinguishes "forced re-converge of an
  already-converged stack" from "legitimate re-converge after drift"
  in the apply summary (#129). Closes the gap that made claim **C3**
  (idempotency) unobservable through `--force`: a forced re-apply of
  a fully-converged stack used to print `N converged, 0 unchanged`,
  identical to a real drift-recovery run.

  When `--force` is used, the summary now adds:
  - **Text mode:** a yellow `note: --force re-ran N resource(s) the
    lock reported as unchanged (M actual change(s), N forced no-op(s))`
    line. `M = 0` is the C3-PASS demonstration.
  - **JSON mode:** new `forced_noop_count` and `actual_changes` fields
    in `summary{}`.
  - **Runtime contract:** `debug_assert!(forced_noop <= converged)` —
    aborts a debug build on planner / executor disagreement.

  Provable contract: `contracts/apply-summary-distinguishability-v1.yaml`.
  Integration test: `tests/test_fj129_force_distinguishability.rs`
  exercises all four step shapes from the contract's proof obligations.

  Surfaced downstream in [paiml/iac-from-zero](https://github.com/paiml/iac-from-zero)
  — the Coursera companion course had to drop `--force` from its
  C3 idempotency CI assertion to make the claim observable. With this
  fix, the demonstration works through `--force` too.

## [1.4.1] - 2026-05-02

### Fixed
- `apply_apt_latest` (`state: latest`) now tolerates `apt-get update` partial failures via `|| true` (Refs PMAT-161). Real hosts routinely have one or two unreachable third-party PPAs, masked PackageKit units, or stale arm64 entries on x86_64 boxes that make `apt-get update` exit non-zero even when the repos we actually care about refreshed cleanly. The subsequent `apt-get install` still fails loud if the requested package can't be resolved, so the `dpkg -l '^ii '` postcondition is preserved. Matches canonical Dockerfile / Ansible practice. Discovered when applying `state: latest` for `docker-ce` on lambda-labs (broken `mozillateam`, `obsproject`, etc. PPAs).

## [1.4.0] - 2026-05-02

### Added
- `state: latest` for the apt package provider (#125, Refs PMAT-161). Validation already accepted `latest` for Package types but `apply_script()` fell through to the unsupported arm, making it a no-op. The new `apply_apt_latest()` runs `apt-get update -qq` then `apt-get install -y -qq <pkgs>`, which installs missing packages or upgrades to the newest available version (no-op if already current). Closes the gap that made `apply_apt_present` (presence-only `dpkg -l` guard) unable to express "always latest" — adding `version:` to YAML was a no-op once a package was already installed at any older version. Postcondition verified via `dpkg -l "$pkg" | grep -q '^ii '` for each requested package.

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
