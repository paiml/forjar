# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.18.0] — 2026-08-24

**`apply` converges what drifted on the target.** 1.17.0 and every version before
it reported `unchanged` for a file changed behind forjar's back, and `drift` did
not report it either — so a drifted file was neither detected nor corrected by
normal operation. On the paiml fleet, **320 of 329 locked file resources were
invisible to drift detection.**

This release is unusual in that most of it is fixing the first attempt. The
convergence fix (#307) shipped five defects of its own, an adversarial release
gate returned **3/3 NO-GO**, and 1.18.0 is what came out the other side. The
NO-GO reasoning is worth keeping: 1.18.0-as-first-written was *strictly weaker*
than 1.17.0, because one `--dry-run` silenced drift for every resource type on a
machine — converting a file-only blindness into whole-machine, operator-triggerable
silence.

### The root cause (#305, #297)

forjar recorded **two** observables per file resource on every apply:

| | covers | computed |
|---|---|---|
| `live_hash` | content + owner + group + mode + existence | **on the target**, through the transport |
| `content_hash` | bytes only, and only when `content.is_some()` | **on the controller** |

`detect_nonfile_drift` excluded `ResourceType::File`, so the complete oracle was
written every run and **read by nothing**. The exclusion arrived with the comment
"already handled by detect_drift_impl", which was false when written — `source:`
support had landed 3h49m earlier without extending `build_resource_details`. A
later refactor deleted the comment, so the false premise stopped being visible.

`ResourceStatus::Drifted` was a defined enum variant that **nothing ever wrote**.

### Fixed

- **`apply` converges observed drift** rather than refusing it. `--force` was
  never a repair: it empties the lock map so every resource re-applies.
- **A managed directory does not drift because files were added inside it.**
  `stat` size tracks entry count (4096 → 12288 at 400 entries); folding that into
  the live hash would have marked every managed directory permanently drifted and
  permanently un-appliable.
- **`Drifted` means "needs work", not "stop looking"** (#310). The detectors
  re-check it. Without this the CI tripwire fired once and then reported clean
  forever over a still-tampered file.
- **`apply --dry-run` no longer writes the lock**, and a **refused** apply no
  longer writes before being refused.
- **Templates are resolved before comparison** — a regression of PMAT-197, which
  `cli/drift.rs` had carried the fix and the reason for all along. 112 templated
  paths and 23 templated task commands were re-applied on every run.
- **Drift asks the machine, not the controller.** `check_file_resource_drift`
  routed through the transport only for *container* transports; every other
  machine — including plain SSH — hashed the controller's filesystem and reported
  it as the remote host's state. Measured: a file present locally and absent on
  the target produced "No drift detected".
- **A docker resource's identity is declared intent**, not `docker inspect`'s
  full output, which carries `StartedAt` and changes on its own every few
  seconds. Every apply was tearing down and recreating every container.
- **A symlink at a managed path is replaced, not written through.** `>` and
  `chmod` follow links, so forjar wrote declared content and mode onto arbitrary
  paths with its own privileges; the dangling-link variant was a create primitive.
- **`group:` without `owner:`** emitted no ownership command at all, then reported
  converged.
- **The cargo package observable checks the binaries**, not just `.crates.toml`.
  Metadata survives a pruned `$CARGO_HOME/bin`, which is why six fleet-wide
  toolchain deletions produced zero drift findings.
- **Every transport call on the apply path is bounded.** One host that connects
  and then stalls used to hang the whole run with zero output.

### Contracts

`apply-converges-observed-drift-v1` is new. Its `apply_reads_what_drift_reads`
equation initially **encoded the raw-resources defect as its specification** —
directly above two invariants saying the opposite. A guard that specifies the bug
cannot fail on it. Corrected, plus `observation_is_bounded`.

`idempotent-apply-v1` gains a `scope_boundary` saying out loud what its equations
do **not** range over: they observe no machine, so a deleted file still plans NoOp
and correctly so. The defect violated no contract, which is the part worth fixing
in the corpus.

### Known limits of the verification

The release gate ran 166 agents across 8 competitive-research lanes and 10
adversarial falsification lanes — and **every lane used `addr: 127.0.0.1`**, where
controller and target are the same box. It structurally could not test the
distinction this release exists for. The transport defect above was found
afterwards, by hand, with two real hosts.

## [1.17.0] — 2026-08-23

Everything in the unpublished 1.16.0 internal build — six release blockers,
including arbitrary command execution on target machines via a `file` resource
whose content contained the heredoc delimiter — plus all 14 ledger regressions.

### The regressions

Defects already recorded in `docs/cli-defects.json` under `confirmed`, verified
against 1.12.3, still reproducing at 1.16.0. They survived 1.13, 1.14, 1.15 and
1.16 — not because they were hard, but because nothing ever re-ran the ledger's
own repros.

**Fixed**

- **`state-decrypt` accepted any passphrase.** It counted errors and returned
  `Ok(())` regardless, so a wrong passphrase exited 0. In the shipped build —
  which has no `encryption` feature — it reported skips and success for state it
  could never have read. It now refuses up front and names the missing feature.
- **`status --hash-verify` counted hashes instead of comparing them**
  ("1/1 resources *have* BLAKE3 hashes"). It now recomputes, reports
  match/MISMATCHED/unverifiable separately, and exits non-zero on a mismatch.
- **`drift --tripwire -m <unknown-machine>`** reported no drift over ZERO
  machines. A typo in a cron'd `drift --tripwire -m intel` silently stopped
  checking anything and reported healthy forever.
- **`lock-prune --yes`** announced a prune and left the lock byte-identical.
- **`stack-diff` was blind to twelve config fields**, including `name` and
  `policy`. The comparison enumerated a hand-written field list; it now
  destructures `ForjarConfig` exhaustively, so a new field is a compile error in
  the differ.
- **`validate --json` emitted zero bytes on failure** — the one case a machine
  consumer needs the structured errors.
- **`audit --json` Debug-printed the event into a string**, making `run_id`,
  `operator` and `config_hash` unreadable without re-parsing Rust syntax.
- **`run` executed unexpanded `{{params.*}}`** and ignored `--param`;
  **`run --json` never executed** and masked a failing task's exit code;
  **`trigger` reported actions fired and ran nothing.**
- **`agent --auto-apply` never remediated.** Two bugs: the effect was a boolean
  computed from the flag, and the agent carried a parallel drift detector that
  read a lock path forjar has never written.
- **`retry-failed` never saw a recorded failure.** It string-matched
  `"ApplyCompleted"` against a log serialised `"apply_completed"`.

**Rejected, deliberately**

- `force-and-rollback-report-zero-actual-changes` describes **intended,
  contract-specified behaviour**. `apply-summary-distinguishability-v1` defines
  `forced_noop_count` as lock-based, its invariant holds, and a previous fix at
  that site was reverted as a regression. Moved to `rejected`. Its genuinely
  misleading *wording* is fixed: the note now says "N differed from the lock"
  and names `forjar drift` for the live question.

### Added

- **`ledger-replay`**, a required proofs job that re-runs the ledger's own repros
  and fails when one still reproduces. It prints its denominator every run, fails
  when it replays zero entries, and treats a timeout or broken sandbox as an
  error rather than a skip.
- A **`contracts`** CI job that validates every contract and resolves every
  falsifier citation. `pv audit` counts declarations without resolving them, and
  no CI job ran `pv validate` at all — which is how an invalid contract shipped
  on the 1.16.0 branch.


## [1.16.0] - 2026-08-21 — INTERNAL BUILD, NOT PUBLISHED

**This version was deliberately never published to crates.io and never tagged.**
It was built and deployed to our own infrastructure only.

Six release blockers were found by the mandatory pre-release dogfood and fixed —
including arbitrary command execution on target machines driven by managed file
content. A follow-up audit then confirmed **14 regressions**: defects already in
`docs/cli-defects.json` under `confirmed`, verified against 1.12.3, still
reproducing. They had survived 1.13, 1.14, 1.15 and 1.16, because nothing ever
re-ran the ledger's own repros.

The judgement call was whether to publish anyway, since those 14 are equally
present in the published 1.15.0 and so are not a regression against what users
have. The decision was **no**: "not worse than last time" is not a release
standard, and putting known-defective behaviour in front of users is not excused
by the previous release having had it too. So 1.16.0 took the security fixes
internally, and 1.17.0 carries them to crates.io with the 14 cleared.

See the `1.17.0` entry for the full list of what shipped in both.


### Fixed

- **`--yes` no longer disables the BLAKE3 state-integrity gate** (FJ-1270).
  The pre-apply check was written `has_errors(&issues) && !yes`, and `--yes` is
  documented as "skip confirmation prompt (CI mode)" and is *mandatory* for any
  non-interactive apply. So every scheduled apply, every CI apply and every
  `ExecStart=… forjar apply --yes` unit ran with tamper detection off: over a
  lock file that did not match its `.b3` sidecar, apply printed
  `ERROR: integrity check failed`, converged anyway, and exited 0.

  Prompting and integrity are now separate concerns. `--yes` means "do not
  prompt" and nothing else; an integrity failure exits non-zero with or without
  it, and no host is touched. No apply flag lifts the gate — the check takes no
  override argument at all, so there is nothing to reach for.

  The refusal names the recovery instead: restore the state
  (`forjar snapshot restore` / `forjar generation`), or bless a known-good lock
  with `forjar reseal --all`, which was already the documented command for a
  lock and sidecar that have legitimately diverged. That path is now covered by
  a test, so the advice cannot rot into a gate with no way out. A replacement
  `--ignore-state-integrity` flag was considered and rejected: it asserts
  nothing about the state it waves through, and it is the kind of flag a CI job
  acquires permanently after one bad night — which is how this gate was lost the
  first time. It would also buy nothing for a corrupt-YAML lock, which fails to
  parse at load whether or not the check ran.

- **The published `install.sh` exited 127 on `--help`, and `dist --verify` said
  PASS.** The generator emitted the argument parser *above* the block defining
  `info`/`warn`/`die`/`usage`, so the very first thing a user ran hit
  `usage: not found`, and every error path before the definitions printed
  `die: not found` instead of its message. Generator output, the committed
  file, and the copy served from raw.githubusercontent.com were byte-identical,
  so this was live.

  `sh -n` cannot catch this by construction: a forward call is valid POSIX
  syntax that fails only at runtime. Every existing guard was a text or byte
  assertion, so nothing ever executed the script. `dist --verify` now **runs**
  it — `sh install.sh --help` must exit 0 and print usage, and an unknown flag
  must reach `die()` with exit 1 — inside a PATH sandbox whose first entry
  shims curl/wget/sudo/tar/install/cp/chmod/mktemp to refuse, so verifying can
  never install anything.


### Added

- **`nas_archive` resource type** ([#282](https://github.com/paiml/forjar/issues/282)).
  Disk *reclaim* has been a first-class resource since FJ-036; disk *archival* —
  its mirror, and the operation that **deletes originals** — was a shell script
  with its policy in a string literal. Five directories on lambda-labs were never
  archived because their names were not typed into that string, invisible to
  `plan` and `drift` because "a file exists with this sha" was perfectly
  converged over a wrong list.

  The type forbids the dangerous shapes: a destination inside or equal to the
  source (the move-onto-itself shape), an empty dir list, and a path where a
  directory name belongs. Containment is component-wise, so `/mnt/unas-old`
  beside `/mnt/unas` is still accepted.

  Six defects found reviewing the predecessor script are encoded once instead of
  per-copy: the verify fails **closed**, caches are dropped before comparing,
  inventories are compared as well as rsync output, `lsof` cannot abort the run,
  the source inventory is captured before the copy, and CIFS-hostile trees are
  refused.

  Admission is measured in **bytes held in small files**, not file count and not
  the small-file share of the count. Both proxies were tried and both misfire:
  `/home/noah/data/courses` is 755 G in 7,426 files, 46% under 64 KiB — but
  23.4 MB in small files against 754.9 GB in large ones, so the small files cost
  ~3 seconds inside a ~36-minute move. A file-count ceiling refused it outright;
  a 50% share threshold passed it for the wrong reason.

  `apply` installs the script and arms the timer; it does not move data. Running
  the script by hand is a dry run unless `ARCHIVE_EXECUTE=1`.

  Contract: `contracts/nas-archive-v1.yaml` — 6 equations, 14 proof obligations,
  14 falsification tests, 3 Kani harnesses, quorum-validated against git-annex,
  Nix gc roots and Terraform plan/apply.

### Fixed

- **forjar's own `if` collided with the condition it was given**
  ([#281](https://github.com/paiml/forjar/issues/281)). A `completion_check`
  written as a YAML folded scalar (`>-`) arrives collapsed onto one line, and
  `verdict::assert_that` inlined it into `if {condition}; then`, putting
  forjar's `if`/`then` on the same line as the condition's `do`/`done`. bashrs
  SC2136 and SC2135 read that as a malformed `if`; both are `SC2*`, which
  `validate_script` does not filter, and both are Error severity — so
  `forjar apply` aborted the resource and JIDOKA cascaded to its eight
  dependents, on a check whose source contains no `if` at all. The condition now
  gets a line of its own, verified running under both `dash` and `bash`.

- **An I8 rejection never showed what it rejected** (also #281). The sanitised
  script was built, linted and dropped, so the error named a rule and a line
  number for text nobody could see — and the script is *generated*, then
  rewritten again by `strip_data_payloads`. The rejection now prints that exact
  text, line-numbered.

### Changed

- bashrs 6.66.3 → **6.68.0**. 6.66.3 raised two SEC010 false positives on
  forjar's own generated scripts, so the I8 gate rejected output forjar
  produced. 6.68.0 also fixes SC1028/SC2104/SC1078 quote-blindness
  ([paiml/bashrs#243](https://github.com/paiml/bashrs/issues/243), #244, #245).
  Related: [#285](https://github.com/paiml/forjar/issues/285) — with 6.68.0
  linked, `purifier.rs`'s blanket `SC1*` exclusion can likely be retired.

## [1.15.0] - 2026-08-19

Same failure shape as 1.14.0 — **reporting a result that was never measured** —
found this time from the consuming side, during a live fleet outage.

Note: 1.14.0 was prepared but never published to crates.io, so this release
also carries everything listed under 1.14.0 below.

### Fixed

- **A check that could not run reported the config as clean.**
  `check_unknown_fields` re-parses raw YAML as a second pass and swallowed a
  parse failure as `Vec::new()` — indistinguishable from a clean config.
  paiml/infra's `machines/lambda-labs/forjar.yaml` reported `OK: 82 resources`
  while carrying `fs_type:` (the field is `fstype`), and the identical typo in
  a sibling config was correctly rejected. Same binary, opposite answers. The
  cause was a duplicate resource key 1200 lines away, which disabled
  unknown-field checking for the entire file.

- **Unresolved secret placeholders were emitted as credentials.**
  `resolve_or_fallback` returns the unresolved resource by design; nothing
  bounded the consequence. A `file` with `content: "API_KEY={{secrets.NAME}}"`
  wrote that literal string to disk. Guarded at codegen dispatch, so one
  unresolvable secret fails one resource rather than the whole machine.

- **A multi-line `completion_check` emitted invalid bash.** A YAML `|` block
  scalar keeps its trailing newline, so the appended `; }; then` landed on its
  own line — an empty statement, `syntax error near unexpected token ';'`.
  Fourteen resources on one host failed identically on a single apply.

- **The cargo check trusted cargo's install record, not the binary.**
  `cargo install --list` is a *record*; it is a different file from the
  binaries and nothing keeps them in agreement. After a CI cache-prune emptied
  a shared `~/.cargo/bin`, `apply -t stack-tools` reported 5 of 5 resources
  converged on a host with no rustup, no cargo and no rustc. The record is now
  used only for *which binaries a crate owns*; each one is then run.
  Includes a cargo-subcommand fallback (`cargo-X X --version`), without which
  `cargo-mutants` and `cargo-llvm-cov` are reported missing forever, and the
  version is matched against what the **binary** reports — a path-installed
  crate records its source dir in the header, and a record can be stale while
  the binary is correct.

- **`--refresh` was a dead flag.** Documented as "Re-run check scripts, only
  re-apply what fails" and read by nothing in production code. It returned in
  0.1s without contacting the host and reported `1 unchanged` over a binary
  that had been reduced to a dangling symlink. This is why a broken host could
  report converged: `apply` compares config to its lock and never looks at the
  machine. It now runs each in-scope resource's check on its host and evicts
  failing lock entries. Plain `apply` deliberately stays lock-based.

- **Repair could not overwrite a dangling symlink.** `cp` refuses
  ("not writing through dangling symlink"), and so does `cp -f` — which is the
  exact state a cache-prune leaves behind, so the one thing most needing
  repair was the one thing that could not be repaired. Placement now uses
  `install`, which is also portable to macOS unlike `cp --remove-destination`.

### Security

- **h2 0.4.13 → 0.4.16** for RUSTSEC-2026-0258 (unbounded empty DATA frames;
  unbounded memory growth or a panic on overflow). Transitive via
  hyper/reqwest; lockfile-only.

## [1.14.0] - 2026-08-17

A release about one failure shape: **a check that reported confidently on
something it had not measured.** Sixteen issues, all of that family.

### Fixed

- **`composite_hash` was not injective** (#235). NUL-separated components with
  no length framing collide: `["a\0b"]` and `["a", "b"]` hashed identically.
  This is the store's address function, so two different derivations could
  claim one store path. Now domain-separated and length-prefixed.

- **`prove` reported hash-determinism PASS for non-deterministic builds**
  (#248). It compared one pure function against itself — a tautology that
  could not fail. It now checks the codegen phases that can actually differ.

- **`execute_sync` reported the planned replay count as the executed count**
  (#249). A partial replay reported as complete.

- **Purity was reported but never enforced** (#241), and its monotonicity
  invariant was dead in production because `dep_levels` was always empty.

- **The store root was a hardcoded root-owned const** (#239) — unprivileged
  users could not use the store at all. Now resolved with a writability probe
  and a `FORJAR_STORE` override.

- **A task reported converged without reaching its declared state** (#254).
  `completion_check` gated whether the command *ran* and was never
  re-evaluated after, so "converged" meant "the command exited 0". On
  paiml/infra's `lean-toolchain`, `sudo: true` made `$HOME=/root`; every
  command succeeded, forjar reported `1 converged, 0 failed`, and
  `command -v lean` failed immediately afterwards.

- **The cargo package check asked the PATH, not cargo** (#257). `command -v`
  finds a binary whose name matches; it cannot tell you the crate is
  installed. Now `cargo install --list`.

- **`apply -r` prompted with a count it would not act on** (#253). The
  confirmation counted the *unscoped* plan: `plan -r X` promised 1 while apply
  offered 69. Execution was correctly scoped throughout — apply acted on 1 —
  but the prompt is the number an operator approves on, and
  `plan-apply-equivalence-v1` obliges the two to agree.

- **Two container tests asserted a wall-clock budget** (#259) — 4s in
  isolation, over 30s under the full suite, same commit. They measured how
  busy the machine was. They now assert dispatch provenance.

- **Five contracts had never validated** (#251), for two layers of reasons:
  proof-obligation types outside the schema vocabulary, then a flat
  `enforcement` block where the schema wants named rule structs, a unit
  smuggled into a u32 `bound`, and an unknown kani `strategy`.

### Added

- **A named library surface** (#240, #245) — `forjar::api` re-exports the 7
  functions and 4 types a consumer needs for content-hash staleness, instead
  of 1844 public items across 195 modules with no supported subset.

- **A CI job that runs the proofs, and proofs that can actually run** (#242).
  Kani harnesses and Lean proofs sat in the tree, cited by name as evidence in
  `contracts/*.yaml`, and nothing executed them — they did not even compile.
  Fixing the compilation only revealed the real state: of 21 harnesses, 7
  failed, 2 were never reached, and 2 were intractable (117 min, and 48 GB of
  RSS at 48 min).

  Two root causes, both now rules rather than one-off fixes:

  1. **Symbolic input reaching allocating code.** `format!` on a validator's
     error path drags `core::fmt` into the model, and CBMC models every path —
     not merely the one the property asserts on. Validators a harness drives
     now return a verdict (`classify_remote`, `hysteresis_holds`) and render
     the message in the caller. 117 min → 104 s; 48 min → 35 s.
  2. **A model checker cannot verify through a cryptographic hash.** Measured:
     blake3's default build fails outright (`foreign "C" function syscall`),
     and its portable `pure` build reached 29.1 GB of RSS still running at 36
     minutes. Nine harnesses reached a hash. Three were tautologies about the
     `blake3` crate; the rest are discharged executably, two of them by tests
     written to replace them because they had no coverage at all.

  A contract citing a proof that does not exist is the same defect one level
  up, so `every_harness_a_contract_names_actually_exists` now guards it — and
  found two pre-existing cases on its first run, where Lean theorems were
  declared under `kani_harnesses:`.

  Result: **20 harnesses, 20 verified, 0 failed.**

- `LICENSE-APACHE`, which `Cargo.toml` had claimed for some time (#243).

### Changed

- The README no longer claims "Pure Rust with zero C dependencies" (#238);
  there are 8 C-backed crates in the default tree.

- `dist-output/` is removed and gitignored (#256). Release generates into
  `/tmp/dist-output`; the copy committed at the repo root had drifted from the
  maintained `install.sh` and carried all 7 of the repo's bashrs SEC findings.

### Known limitations

Six issues remain open and are **features, not defects** — deliberately not
rushed into this release:

| # | |
|---|---|
| #236 | store has no output-content digest (no dedup, no corruption detection) |
| #244 | undeclared task inputs are undetectable |
| #246 | byte-identity is the only equivalence predicate |
| #247 | no regenerate-and-compare verification mode |
| #237 | no default feature set — `default-features = false` is a no-op |
| #228 | registry push still shells out to curl |


## [1.13.2] - 2026-08-16

### Added

- **`forjar dogfood` — exercise generated artifacts against reality** (FJ-038).

  Three releases in two days each fixed the previous one, and every one had
  passed 12,904 unit tests, a five-gate clean room and a 19-check CI run. The
  common cause was not missing tests: it was that the fixtures and the code
  shared an author, so each fixture confirmed the assumption it was meant to
  test. The rclone stub emitted whichever status characters the author believed
  in. The cargo fixture carried both marker files because the author believed
  both were present.

  **A test you author cannot falsify a premise you hold.** `forjar dogfood`
  invokes the real external tool and builds the on-disk shapes that really
  occur, then asserts reality agrees with what the code assumes:

  - `backup_sync` runs `rclone check --combined` against a four-case fixture and
    confirms `= * + -` mean what the coverage counters assume;
  - `disk_budget` builds all four real cargo layouts — repo target root, per-arch
    subdirectory, cargo registry, and a `cc`-style source dir named `target` —
    and confirms the first two are detected and the last two are not;
  - `file` and `cron` execute their emitted shell under **bash**, the interpreter
    every forjar transport actually uses.

  A missing external tool is a FAILURE, not a skip: dogfooding a resource built
  on a tool's output format, without that tool, proves nothing.

  Coverage is declared by an **exhaustive match** over `ResourceType` with no
  wildcard arm, so a new resource type fails to compile until its dogfood status
  is stated, and `NotApplicable` requires a written reason that is printed on
  every run. The previous `scripts/dogfood-use.sh` covered only `file` resources
  and still reported success while two new resource types shipped broken; a gate
  that can silently stop covering things is worse than none, because it reports
  GO with authority.

  Verified by mutation — reintroducing each shipped bug turns the gate RED:

  ```
  both-markers cargo rule (1.13.1) -> FAIL disk_budget: repo target root NOT detected
  inverted rclone +/-     (1.13.0) -> FAIL backup_sync: counter keyed on wrong character
  ```

- **`forjar codegen -r <resource> --phase apply|check|state-query`** — emit the
  shell a resource generates, resolved as `apply` would resolve it. A resource
  whose real payload is synthesised shell cannot be dogfooded, or debugged, if
  the artifact cannot be got at.

### Fixed

- **`disk_budget` matched no cargo target directory on a real fleet machine.**
  Detection required BOTH `CACHEDIR.TAG` and `.rustc_info.json`. Measured on
  lambda-labs across a 4.6 TB `targets/` tree: **zero of 16** marker-bearing
  directories carried the pair.

  ```
  targets/<repo>                 .rustc_info.json,  NO CACHEDIR.TAG
  targets/<repo>/<arch-triple>   CACHEDIR.TAG,      NO .rustc_info.json
  ```

  cargo writes `.rustc_info.json` at the target root and `CACHEDIR.TAG` in the
  per-arch subdirectories, so the conjunction is satisfied by neither. The
  reaper triggered at 94% used, enumerated nothing, reclaimed 0 bytes and
  reported `health=inert` — the exact silent-inertness the resource exists to
  prevent, reached by a different route.

  A directory is now a cargo target dir when it has `.rustc_info.json`
  (definitive), **or** `CACHEDIR.TAG` together with a `debug/` or `release/`
  subdirectory. That second clause is what still keeps the reaper out of
  `~/.cargo/registry`, which carries `CACHEDIR.TAG`, lacks `.rustc_info.json`,
  and whose children are `src/`, `cache/`, `index/` rather than build output.

  Both directions are pinned by falsification tests built from the measured
  layouts, and both were verified to turn RED under mutation: restoring the
  conjunction fails the per-arch test, and dropping the build-output
  requirement fails the registry-protection test.


## [1.13.1] - 2026-08-15

### Fixed

- **`backup_sync` read rclone's `--combined` status characters backwards, and
  the error inflated coverage.** They are not the intuitive way round:

  ```
  + path   missing on the DESTINATION  -> present locally, NOT backed up
  - path   missing on the SOURCE       -> only in the remote (stale)
  ```

  1.13.0 counted `-` as "missing from the remote". Files that were genuinely
  not backed up produce `+`, which nothing counted — so they fell out of the
  coverage denominator entirely and a backup **missing data reported higher
  coverage than one that had all of it**. That is the precise class of
  overstated-health failure the resource exists to prevent, shipped inside it.

  `+` now feeds the missing count. `-` is tracked separately as
  `stale_in_remote` and deliberately excluded from the denominator: a file
  present only in the remote means the local copy was deleted, not that
  anything is unprotected, and counting it would make every local deletion read
  as a backup fault until the next sync.

  Caught by running `rclone check --combined` against a real fixture rather
  than trusting the flag's name. The regression test now asserts the mapping
  explicitly, and the falsification stub emits rclone's real characters.

- **`a_missing_rclone_binary_stops_the_run` was host-dependent.** It relied on
  rclone not being installed on the machine running the tests, and broke the
  moment rclone was deployed. It now builds a hermetic PATH containing only the
  utilities the preflight needs.


## [1.13.0] - 2026-08-15

Two new resource types, both born from the same failure mode on one machine:
a guard that was deployed, enabled, reporting success, and doing nothing.

### Added

- **`disk_budget` — free space as declared machine state** (FJ-036).

  lambda-labs reached 100% on `/` (1.2 G free) while a reaper ran nightly on
  schedule and exited 0 every time. Over the preceding month it reclaimed 1.6 G
  total, across a slide from 370 G free to 1.2 G and through an earlier
  100%-full event. It was deployed, enabled, and `systemctl` reported it active
  throughout. Three independent defects, each individually sufficient:

  - a fixed 7-day idle TTL on a box whose build trees turn over in two days, so
    every candidate was legitimately "recent" and it correctly declined to
    delete anything, all the way to full;
  - build directories matched by **name** (`target|target-local|target-private`),
    so the 189 G living in `.target` was never even enumerated;
  - it never read `df`, so a run that reclaimed nothing at 100% pressure was
    indistinguishable from a healthy no-op.

  A `disk_budget` declares watermarks per filesystem. A high watermark triggers
  a reclaim pass; the pass runs until a low watermark is restored, oldest-first,
  and halts there rather than exhausting its candidates. The two thresholds
  cannot be collapsed into one — hysteresis is enforced at parse time, because a
  pass that stops while still above its trigger re-fires on every tick.

  Candidates are found **behaviourally, never by name**: cargo build directories
  by the markers cargo itself writes (`CACHEDIR.TAG` *and* `.rustc_info.json`),
  git worktrees by asking git. Requiring both cargo markers is what keeps the
  reaper out of `~/.cargo/registry`, which carries the tag and not the info file.

  Crucially, **a triggered pass that misses its target exits non-zero**, so an
  inert reaper becomes a failed unit and is visible to `forjar drift` instead of
  silently green. `state_query` publishes health *classes* on stdout and raw
  byte counts on stderr, so volatile values never enter the drift hash.

- **`backup_sync` — an offsite copy that must prove it exists** (FJ-037).

  The same machine held ~2.1 TB of irreplaceable media on a 4-wide RAID0 with no
  parity, and zero bytes of it anywhere off that array, while an hourly job
  reported `Backup complete` for months. It rsynced a directory to a symlink
  pointing back at that same directory, and its success metric ran `find` on
  that symlink without `-L` — printing `Files: 0` while 77 matching files sat
  there. Structurally zero on every input, not merely on an empty one.

  `backup_sync` rejects a destination that is not an rclone `remote:path`, and
  proves at runtime that the remote is *configured and reachable* before a byte
  moves — an unconfigured rclone remote silently degrades to a local path, which
  is the same self-referential failure by another route.

  Health is a count of files verified present in the remote **by checksum**
  (`rclone check --combined`), compared against the source. Zero examined or zero
  matched is a failure, not a pass. A run below the declared coverage threshold
  exits non-zero.

  `apply` deliberately does *not* run the sync, unlike `disk_budget`: seeding
  terabytes takes days under provider upload caps, and a deployer that runs the
  job also writes the status file that is supposed to be evidence the *service*
  ran. `apply` arms the timer; `state_query` reads the journal, which the
  deployer cannot forge.

  forjar owns the generated `rclone.conf` so the remote definition is declared
  state rather than a manual step that can go missing. Backend and options live
  in the repo; the OAuth token arrives through the secrets provider. A literal
  token is refused at parse time, an unresolved `{{secrets.x}}` at codegen, and
  the file is written under `umask 077` at 0600 with an atomic rename.

### Fixed

- **Removing a systemd resource no longer leaves an orphaned unit.** `state:
  absent` deleted unit files while the unit was still loaded, leaving it
  `Active: failed` with *"Unit to trigger vanished"* — invisible to an apply
  that reported converged. Teardown now stops, disables, removes, reloads, and
  clears the failed state, in that order.

- **`hash_desired_state` now covers scripts a handler generates.** A resource
  whose payload is synthesised is not fully described by its declaration: two
  forjar versions can emit different scripts from identical YAML. The planner
  compared only the declaration, reported `unchanged`, and left machines running
  the previous generated artifact. All three emitted scripts are hashed — folding
  in only `apply` would pin `apply`=unchanged against `drift`=drifted forever,
  with nothing re-recording state.

- **`RESOURCE_FIELDS` completeness is now enforced by reflection.** A field added
  to `Resource` but missing from the parser's hand-maintained allow-list was
  accepted by serde and then rejected by validation as `unknown field`, so a
  fully-implemented, fully-tested feature was undeclarable in YAML. A test walks
  the serialised struct, so the next omission fails a test instead of shipping.

- Templated values in reclaim roots and backup sources are resolved. An
  unexpanded `{{params.home}}` silently matches nothing, which for a reaper or a
  backup means "protects nothing" while reporting success.

### Notes for handler authors

Both handlers ship a test that runs every emitted script through
`purifier::validate_script` — the same call `forjar apply` makes. Its absence let
six I8 violations ship in `disk_budget`: the resource passed 12,000+ tests and
was rejected on every machine, because nothing in the suite exercised the
purification path production uses. If you add a resource type, add that test.

## [1.12.6] - 2026-08-12

### Fixed

- **`build --push` now names the missing binary instead of reporting an errno**
  ([#224](https://github.com/paiml/forjar/issues/224)). Every OCI registry
  request shells out to `curl`, an undeclared runtime dependency. On a host
  without it the first HEAD died as `curl HEAD: No such file or directory (os
  error 2)` — which names neither curl nor the fact that a required external
  binary is absent, and reads like a network or registry fault.

  A preflight at `push_image()` — the single funnel every push goes through —
  now fails early with an actionable message, before any partial upload begins.

  Found by infra's clean-room gate (a container holding only what the crate
  declares) while GitHub CI was green on the same commit, because the CI image
  happens to ship curl. A user running `cargo install forjar` on a minimal host
  would have hit the original error.

  This makes the dependency legible; it does not remove it. The crate already
  compiles `reqwest`, so doing the registry HEAD/PUT natively would drop the
  shell-out entirely — tracked in #224.

## [1.12.5] - 2026-08-12

### Changed

- **bashrs floor raised `6.64.0` → `6.66.3`** (lockfile moves 6.66.0 → 6.66.3).
  forjar surfaces bashrs' linter directly to users — `bashrs::linter::lint_shell`
  in the purifier, plus `forjar lint` and the MCP handlers — so bashrs' lint
  fixes are forjar's user-visible behaviour.

  This is a **floor** bump, not just a lockfile refresh, and deliberately so:
  bashrs 6.66.3 retires MAKE016, whose autofix **corrupted Makefiles**. A caret
  range still admitting 6.64.0 would let a consumer resolve back into a
  Makefile-corrupting autofix, which matters because v1.12's headline feature is
  Makefile ingest.

  What forjar users gain: no more spurious diagnostics inside quoted heredocs
  (`<<'EOF'` bodies are literal text, not shell — provisioning scripts and
  `command:` blocks are full of them), MAKE003 no longer trips over Make's `$$`
  escape, and MAKE010 recognises `|| exit` / `|| return` / `|| die` tails as
  error handling.

  No API changes were required: the full suite passes unmodified against 6.66.3
  (12776 passed, 0 failed).

## [1.12.4] - 2026-08-11

### Fixed

- **Dogfooding sweep of the published 1.12.3 across CLI / MCP / LSP / HTTP**
  ([#208](https://github.com/paiml/forjar/issues/208)): 101 confirmed defects,
  4 of them BLOCKERs. Resolved in
  [#216](https://github.com/paiml/forjar/pull/216) together with
  [#211](https://github.com/paiml/forjar/issues/211),
  [#212](https://github.com/paiml/forjar/issues/212),
  [#213](https://github.com/paiml/forjar/issues/213),
  [#214](https://github.com/paiml/forjar/issues/214) and
  [#215](https://github.com/paiml/forjar/issues/215):

  - **[A]** 15 flags were declared on the clap struct and never consumed —
    accepted on the command line and silently ignored. Notably every one of the
    eight dry-run spellings is now ORed fail-safe, so `--dry-run` cannot be
    dropped by a code path that only checked one of them.
  - **[C]** 26 defects where machine-readable output was malformed or leaked
    Rust `Debug` formatting into what callers parse as JSON.
  - **[D]** 5 selectors that filtered part of the output but not the rest,
    leaving precomputed counters disagreeing with the rows they summarise.
  - **[E]** 8 cases where invalid input was accepted or crashed instead of
    being rejected — including workspace names containing `..`, which are now
    refused before any filesystem access rather than after.
  - **[Z]** 19 assorted correctness defects, including MCP state-directory
    resolution, which resolved relative to the process working directory
    instead of the config file's directory.

  These were found by `cargo install`-ing the published crate and driving every
  interface, not by the test suite: two MCP handlers had been wrong since they
  were written and had passing tests that only asserted `Ok`.


- **`build --push` fabricated a push** ([#210](https://github.com/paiml/forjar/issues/210)).

  With a network it printed `Push complete: 3 uploaded` and exited 0 having
  uploaded nothing; with no network at all it printed
  `push skipped: registry unreachable` and **still exited 0**. The target was
  `docker.io/app:latest` whatever the resource declared:

  ```console
  $ unshare -rn forjar build --resource img --push   # no network whatsoever
    registry: docker.io
    name: app
    tag: latest
    push skipped: registry unreachable (no Location header in upload response)
  $ echo $?
  0
  ```

  Five defects, each sufficient on its own to make the success line false:

  1. Transport failures were swallowed into a "skipped" line and `Ok(())`.
  2. The push target was re-derived from `name`/`version` with a different
     default (`app`/`latest`) than the build used, and split at the first `/`,
     so `myorg/app` parsed as registry `myorg`. `tag:` on an image resource was
     parsed and dropped entirely. The push now reuses the exact reference the
     build stamped into the image, parsed by
     `core::store::image_ref::parse_image_ref`.
  3. Success was gated on the presence of a `Location:` header. `docker.io` is
     a website: it answers the upload POST with a 301 to the marketing site,
     whose `Location` was taken as an upload session — the blob was PUT at a
     web page, which returned 200. The status code is now the gate (202
     Accepted), and Docker Hub resolves to `registry-1.docker.io`.
  4. The manifest was uploaded as a *blob* and never PUT to the tag, so even a
     fully successful run created no pullable tag. It is now PUT to the tag.
  5. `?digest=` was concatenated onto session URLs that already carry a query
     string (`?_state=…`), which every real registry rejects with
     `BLOB_UPLOAD_INVALID`.

  `Push complete` is now printed only after forjar re-reads the tag from the
  registry and confirms it resolves to the manifest just pushed; every other
  outcome is a non-zero exit. HTTP 401 is reported as what it is — forjar
  implements no registry credentials, so an authenticated registry is refused
  with a pointer to `--load` + `docker push` or `--far`. Anonymous-write
  registries push and verify for real (`docker pull` of the pushed digest
  succeeds).


## [1.12.3] - 2026-08-10

### Fixed

- **`apply` deployed stale content while reporting "unchanged"** when a file
  resource's `source:` file changed ([#206](https://github.com/paiml/forjar/issues/206)).

  `hash_desired_state` hashes resource *field strings*. For `content:` that is
  correct — the content **is** the field. For `source:` the field is a **path**, so
  editing the referenced file left the hash identical, `determine_present_action`
  planned `NoOp`, and apply skipped the resource:

  ```console
  $ echo VERSION-ONE > payload.txt && forjar apply -f repro.yaml --yes
  Apply complete: 1 converged, 0 unchanged.
  $ echo VERSION-TWO > payload.txt && forjar apply -f repro.yaml --yes
  Apply complete: 0 converged, 1 unchanged.
  $ cat /tmp/deployed          # -> VERSION-ONE   (stale)
  ```

  `--force` was the only workaround. For a tool whose contract is "converge to
  declared state", silently not converging while printing success is the worst
  available failure mode. Observed live in paiml/infra PMAT-204, where an edited
  reconciler script reported "converged" three times while the machine kept
  executing the previous copy.

  The planner now folds the **content hash** of the `source:` file into the
  desired state. The component is **appended**, and only for resources that
  declare `source:`, so no recorded hash for any other resource on any machine is
  invalidated. Path identity is preserved (same bytes at different paths still
  hash differently), and a source that appears or disappears now changes the hash
  rather than staying pinned at "unchanged".

  Note: source-based file resources will show one `Update` on the first apply
  after upgrading, as their hash gains the new component. That re-apply is
  convergent and expected.

### Added

- `contracts/source-content-identity-v1.yaml` — the **completeness** leg of
  `idempotent-apply-v1`. That contract asserts "differing hash always plans
  Update", which is sound but vacuous if the hash cannot differ when the deployed
  artifact differs. 7 falsification tests, including an end-to-end reproduction.
- `src/core/planner/tests_hash_source.rs` — 5 tests covering content change,
  determinism, path identity, source appearance, and non-regression of
  source-less resources.

## [1.12.2] - 2026-07-29

Four design defects, each one a place where forjar reported on something other
than the thing that would actually run. They were found by a design review of
1.12.1 rather than by a failing test, because in every case the test suite and
the CLI output agreed with each other and both were wrong.

### Fixed

- **`apply` treated a zero exit code as proof the work happened.** A task
  declaring `output_artifacts` whose command exited 0 without producing them was
  recorded as converged, and the state lock then asserted an artifact that was
  not on disk. Apply now verifies declared outputs exist on local machines
  before recording success, and names the missing ones when they don't.
- **The script was readable as its own stdin.** The transport feeds the
  generated script to `bash` on stdin, so a task command that itself reads stdin
  (`cat > f`, `read`, `xargs`) consumed the remaining script lines. The task
  half-executed and reported success, having silently eaten its own tail. Every
  script is now wrapped `{ ... } < /dev/null` at the one point where scripts are
  executed, so a command's stdin can never be the script.
- **`forjar prove` proved the unresolved config.** It read `config.resources`
  before template expansion, so a proof about conflict-freedom examined
  `${var}/out` rather than the path two targets would really write. Two targets
  that genuinely collide after expansion were reported as
  `[PASS] I3 conflict-freedom: [CHECKED] 2 targets disjoint`. `prove` now
  resolves first.
- **`forjar lock` hashed the unresolved config.** The lockfile therefore
  fingerprinted the template text rather than the values, so two configs that
  expand to different infrastructure could share a hash. `lock` now resolves
  before hashing.

## [1.12.1] - 2026-07-28

Three interface defects found by installing 1.12.0 from crates.io and driving
every CLI, MCP and LSP surface against a real project. None was visible from a
schema, a `tools/list`, or a handler test that only asserted "returns Ok".

### Fixed

- **MCP `forjar_plan` reported every resource as a pending change.**
  `ExecutionPlan::changes` carries EVERY resource with its action, NoOp
  included; `cli::plan` filters those before counting and the MCP handler did
  not. A fully converged project reported all 6 of its resources as pending
  while the CLI reported `0 to change`. It also included phony resources, which
  are goal-only.
- **MCP `forjar_status` returned no machines, ever.** It scanned the state
  directory for files with a `.json` extension; a machine's state is a
  DIRECTORY, `state/<machine>/state.lock.yaml`. The CLI printed
  `Machine: local (localhost)` for the same project.
- **`forjar lsp` was an unrecognized subcommand.** The language server is
  complete and has 80 passing tests, but no `Commands` variant dispatched it,
  so no editor could start it. Now wired and documented.

### Known

`core::webhook_server` implements an HTTP endpoint with 16 tests including live
socket accept/reject, but nothing starts it — there is no `forjar webhook`
command. Exposing a listening socket needs bind-address and authentication
decisions, so it is a design question rather than a wiring fix; tracked in
PMAT-200 rather than added unilaterally in a patch release.

## [1.12.0] - 2026-07-28

forjar can now **replace and ingest** a trivial Makefile. Getting there required
fixing five defects, four of which shared one shape: a signal that reported
success without ever consulting the world.

### Added

**`forjar make [GOALS...]`** — builds each goal and its transitive
prerequisites, and nothing else. `resolver::goal_closure` walks `depends_on`
upward; the result is downward-closed by construction, so a pruned config can
never execute against an unconverged prerequisite. That is the property that
makes goal selection safe where `--subset`/`--exclude` pattern filters are not,
and it is what `-r` never had: `-r` is exact-match with NO closure, so
`apply -r link` runs link and silently skips the compile step it depends on,
linking whatever objects happen to be on disk. That is `make -o`, not `make`.

**`phony: true`** — a make-style target that names an ACTION, not a file. It is
excluded from bulk apply and plan entirely, and runs unconditionally when named
as a goal. Goal-only is the only reading that preserves idempotency: "runs on
every apply" would propagate dirtiness through its whole transitive closure and
stop `plan` ever reaching "0 to change"; "phony prerequisites auto-run when
reached" is not convergent, because a `clean` that `build` depends on deletes
the outputs that make `build` stale, forever.

**`forjar import-makefile`** — ingests a single-makefile, non-recursive build by
joining two streams from one `make` invocation: `-p` gives structure with
UNEXPANDED recipes, `--trace` gives the expanded commands. The join key is
`(recipe file, recipe line, target)` — the target name is load-bearing, because
`build/main.o` and `build/util.o` both trace as `Makefile:14` when they share a
pattern rule.

Two measured hazards shaped the invocation more than the parser. An up-to-date
tree emits NO commands (`Nothing to be done`), so `-B` is mandatory or the
import yields structure with no commands for exactly the targets that matter.
And pattern rules only instantiate during goal resolution, so import is two
passes: enumerate names, then ask for them all by name.

Recipes emit one **subshell per logical line** (after folding backslash
continuations), reproducing make's per-line shell isolation exactly — so
`cd build && ./app`, an idiom far too common to refuse, imports faithfully.
Order-only prerequisites become `depends_on` edges and never `task_inputs`;
hashing a directory as an input is what made 1.11.0 an idempotency pump.

Recursive make, `.ONESHELL`, double-colon rules, VPATH and GNU make < 4.0 are
**refused with reasons, writing nothing**. An importer that silently
mistranslates is worse than none: its output looks like your build and is not
one.

### Fixed

**`forjar check` reported `pass` for every resource, unconditionally** — for
every resource type, since at least 2026-02-27. Verified on the published
1.11.1 binary against a config that had never been applied, in an empty
directory: `2 pass, 0 fail, exit 0`. The cause was a protocol mismatch, not a
missing comparison: generators emitted their verdict as a stdout marker
(`<test> && echo exists || echo missing` — a branch whose arms are both `echo`
always exits 0) while the consumer read the exit code, and nothing anywhere
parsed the markers. `apply --check` shares the path, so its documented
"exit 2 = changes needed" was unreachable too.

The fix cannot live at the codegen boundary: the same marker means opposite
things depending on desired state — for `state: absent`, `missing:` IS
convergence. All 17 generators now report through `resources::verdict`. Also
corrected while converting: `service` asserted a fixed "active AND enabled", so
a `state: stopped` service would have become a permanent failure; gpu's rocm
path did `echo missing; exit 0`; a model checksum MISMATCH reported pass — the
most dangerous case, since the file exists and looks fine.

**24 templatable `Resource` fields were never resolved**, including
`task_inputs` — the field 1.11's entire incremental-build release is about,
while its sibling `output_artifacts` was resolved. A config that templated its
inputs got `Apply complete: 0 converged, 1 unchanged` over a stale artifact:
precisely the failure 1.11 shipped to eliminate. Also `scatter`/`gather` (spliced
into executed shell), `state` (selects the absent/directory/symlink branch), the
six `overlay_*` fields that configure the fleet's overlay IPs, and `stages`.

**`forjar destroy` executed unresolved templates** — generating
`rm -rf '{{params.x}}/...'` against a literal path, reporting success while the
real resource survived and its lock entry was removed. Third code path to make
this mistake. **And it RAN builds instead of removing them**: task, build and
wasm_bundle ignore `state`, so converging them to `absent` executed the command
— running a build or a deploy as the way of "removing" it.

**A selector matching nothing is now an error.** `apply -r <typo>` printed
`0 converged, 0 unchanged` and exited 0; in CI, where the exit code is often the
only signal read, a typo'd targeted apply looked like a completed deploy.

### Guards against recurrence

Both guards are constructed so that a future addition is covered without anyone
remembering to update them:

- `no_resource_type_generates_an_unfailable_check_script` EXECUTES each
  generated script against a real filesystem. Asserting on script TEXT is what
  let the check defect live for months — the text was always plausible.
- `every_string_field_on_resource_is_template_resolved` REFLECTS over the
  serialised `Resource` to discover which fields accept a string. A hand-written
  list of fields to check has the same failure mode as the hand-written list of
  fields to resolve.

Contract: `contracts/build-semantics-v1.yaml` (L3, 9 falsification tests).

### Known differences from make

- **Shell options.** forjar wraps a `command:` in `set -euo pipefail`; make sets
  none. Imported recipes restore make's semantics per line. An earlier draft of
  this entry called the difference "strictly stricter — it surfaces errors make
  swallows"; dogfooding the built binary falsified that. Under pipefail
  `seq 1 100000 | head -1` exits 141 on SIGPIPE where make returns 0, and
  `cmd | head` is a stock Makefile idiom, so the claim described a working build
  becoming a failing one as a safety improvement. A hand-written forjar
  `command:` still runs under `set -euo pipefail`.
- A `-` prefixed recipe line (ignore errors) imports as `... || true`. `--trace`
  strips the prefix, so it is read from the make database instead.
- A real file target that depends on a `.PHONY` target is **refused**: make runs
  the phony prerequisite when it reaches it, goal-only phony cannot, and the
  imported config would build without the action ever running.
- `include`, `$(shell …)` and `$(wildcard …)` are resolved by make before the
  import sees them, so their values are frozen at import time.
- Staleness is BLAKE3 content, not mtime: `touch` does not trigger a rebuild,
  and recompiling to identical bytes correctly does not relink.
- The staleness probe runs on the controller and skips remote resources rather
  than hashing the wrong host, so `forjar make` is a build system for LOCAL
  targets.

### What this release does NOT fix

v1.12 does not make the three read paths agree; it makes each one honest about
the level it observes. `check` is existence/state level — `rm build/demo` now
fails it, but editing a file's content in place does not. `plan` is config-hash
plus the build probe. `drift` remains the only content-level comparison.
Measured: tamper with a file resource's content and check says `1 pass`, plan
says `0 to change`, drift says `Drift detected`. Raising check to content level
means every generator hashing its artifact — a feature, not a bug fix.

Also outstanding, now recorded as `known_gaps` in the contract rather than left
implicit: the transport writes the script to bash's stdin, so a task that reads
stdin consumes the rest of its own script (pre-existing); the bashrs determinism
gate rejects idioms like `date +%s` at apply time, so such a Makefile imports
cleanly and then cannot run; and order-only edges participate in propagation
because there is no `order_only` field.

## [1.11.1] - 2026-07-27

### Fixed — two defects in 1.11.0, found by dogfooding it

**Directory `output_artifacts` created an idempotency pump.** Hashing a
directory's *contents* meant the canonical translation of make's `| build`
order-only prerequisite — `output_artifacts: ["build"]` — went stale the moment
the next rule wrote into it. Observed: apply #1 converged, apply #2 reported
`stale — output artifact modified` and **re-ran the entire graph**, and only
apply #3 settled. That violates `f(f(x)) = f(x)`, forjar's core idempotency
contract.

A directory artifact is now identified by **existence**, never by contents. Its
contents are the products of *other* rules; they are not the identity of the
rule that created it. Files declared alongside a directory are still hashed.

**The read paths were blind to the staleness `apply` acts on.** `planner::plan`
forwarded an EMPTY probe map, so after `rm build/demo`:

```
forjar plan  -> Plan: 0 to add, 0 to change, 0 to destroy, 3 unchanged
forjar check -> Check: 3 pass, 0 fail, 0 skip
forjar drift -> No drift detected
forjar apply -> stale — output artifact missing ... rebuilt
```

A planner that cannot predict its own apply is worse than one that is merely
conservative. `plan` now probes, so `plan`/`drift`/`observe` and `apply` give
one answer. `plan_with_probes` remains pure for unit tests.

### Known limitation

`forjar check` still over-reports. Its generated scripts echo a marker
(`task=completed` / `task=pending`) but exit 0 either way, and the CLI grades on
process success — so `check` currently means "a shell ran", not "the resource is
converged". Fixing it touches all 14 resource generators and changes what
`check` means for non-build resources, so it is deferred rather than rushed into
a patch release.


## [1.11.0] - 2026-07-27

### Added — incremental builds: forjar now plans from the world, not just the config

Until now forjar decided what to do by hashing the *desired config*. A task whose
`task_inputs` had changed on disk hashed identically as a desired state, so the
planner returned `NoOp`, the executor never ran, and forjar printed
`Apply complete: 0 converged, N unchanged` over a **stale artifact**. That is the
worst failure mode a build tool has: a wrong binary under a green summary.

The pre-existing `cache:` / `task_inputs` machinery could not fix it.
`check_task_input_cache` ran inside `apply_one_resource`, i.e. *downstream* of a
planner that had already said `NoOp` — structurally able to *suppress* work,
never to *schedule* it.

**What changed**

- New `core::task::probe`: before planning, forjar hashes each resource's
  declared `task_inputs` and `output_artifacts` and hands the result to the
  planner. The planner stays **pure** — it never touches the filesystem.
- A converged resource is now re-run when its inputs changed, an output artifact
  is missing, or an output was modified out of band. `rm build/demo` rebuilds;
  previously it reported `unchanged`.
- **Change propagation**: a rebuilt prerequisite now invalidates its dependents.
  `depends_on` was ordering-only, so a rebuilt object file left the link step
  converged and the binary stale. One forward sweep over the topological order.
- `output_hash` is now recorded alongside `input_hash`.

**Fixed along the way**

- I/O hashing resolved paths against `state_dir.parent()`, not `working_dir`.
  Since builds declare paths relative to their project root, every relative path
  hashed as absent — which silently disabled caching whenever `--state-dir` was
  relative. Added `hash_outputs_in(artifacts, base_dir)`.
- Hash tracking no longer requires `cache: true`. Tracking is what makes
  correctness possible; `cache` remains the switch for *skipping* work.

**Verified against `make`** on a 3-target C project: clean build, true no-op
re-apply, minimal rebuild on edit (2 of 3 targets — not rebuild-everything),
artifact restore after `rm`, and identical program output to `make` across
multiple source mutations.

### Fixed — `forjar drift` reported permanent false positives on templated resources

`drift` compared **raw** config resources against state the executor had stored
from **resolved** ones, so anything containing `{{params.*}}` drifted forever.
For `type: task` the state query is `echo "command=<command>"`, so the lock held
the resolved command while the probe regenerated the literal template.

This was not cosmetic: the apply-time drift gate is **global**, so one
self-drifting resource blocked every *targeted* apply on that machine. Downstream
fleets were running `forjar apply -t <tag> --no-tripwire` because of it.

This class had already appeared twice (planner FJ-154; drift), and `destroy`
still had it — so resolution now lives in exactly one place,
`resolver::resolve_or_fallback` / `resolve_all`, rather than being re-derived per
call site.

### Security

- `cargo update -p crossbeam-epoch` 0.9.18 → 0.9.20, clearing **RUSTSEC-2026-0204**
  (invalid pointer dereference in `fmt::Pointer` for `Atomic`/`Shared` when the
  underlying pointer is invalid). Transitive via `rayon` → `sysinfo`/`bashrs`.
  Lockfile-only; pre-existing, not introduced by this release, but a release
  should not ship a known advisory.

### Known limitations

forjar is **not** a general-purpose build system and this release does not claim
to be one. Specifically:

- Staleness is **content-hash** based, not mtime. `touch` alone does not trigger
  a rebuild — timestamp-only stamp-file idioms do not carry over, and
  round-trip tests against `make` must mutate content.
- Probing is **controller-local**. Resources targeting remote machines are not
  probed and keep the previous config-hash behaviour, rather than hashing the
  wrong host's files.
- No pattern rules, no `$@`/`$<` automatic variables, no `make <goal>` target
  invocation, no `.PHONY`. Deferred deliberately.


### Changed — copia delta is now TRUE ROLLING delta (the real fix)

forjar's large-file provisioning delta was fixed-block: a 1-byte insertion made
every later block differ, forcing a full re-transfer. It now computes a **true
rolling delta** via the copia crate (`copia::CopiaSync::delta` matches blocks at
ANY byte offset), so an insertion/deletion reuses the unchanged bulk of the file.

The elegant part: **no copia binary is staged on the receiver.** The receiver
reports a `copia::Signature` computed with `od`+`awk` (the Adler weak checksum,
verified bit-for-bit against `copia::RollingChecksum`) and `b3sum` (blake3 strong
hash) — all universal tools; forjar does the rolling match locally with the library.
A checksum disagreement (e.g. busybox) can only degrade to all-literal (correct
output), never corrupt — the strong hash and the whole-file blake3 are re-verified
before the atomic rename. `copia-provisioning-v1` gains FALSIFY-COPIA-005..007.


### Security — copia provisioning hardening (copia-provisioning-v1)
Fixes 5 defects a 6-lens quorum found in the generated remote delta-sync shell
(`src/copia`): perms (chown/chmod) now run on the temp file **before** the atomic
rename (closing a world-readable window for 0600 secrets); literal payloads stream
via a base64 **heredoc** (no `echo` ARG_MAX blowup); the reconstructed file's
**blake3 is verified** before commit; every interpolated path/owner/group/mode is
**shell-quoted** (injection guard); and a **cleanup trap** prevents temp-file litter.

### Added — 33 verified Lean 4 proofs (6 contracts to L4)
forjar's first Lean proofs: `blake3-state`, `dag-ordering`, `recipe-determinism`,
`execution-safety`, `codegen-dispatch`, `overlay-interface` each gain a machine-checked
proof of their decidable core (0 `sorry`), with honest verification_summary
(l4_lean_proved for the pure core, l4_not_applicable for genuine I/O).


### Added — Provable IaC: `forjar prove` invariant ladder (provable-iac-v1)

`forjar prove` is enhanced from a convergence proof into a **pv-style L1–L5
provability report over the forjar.yaml itself** — the honest analog of
`terraform validate` with machine-checked validators (NOT "safe to apply": the
remote shell that mutates the machine is outside the trusted computing base).

- **Three-state assurance** (never collapsed): `PROVED` (a theorem transfers to
  this config), `CHECKED` (a Kani-verified decision procedure ran and found
  nothing), `UNKNOWN` (an opaque/imperative resource the analysis can't see through).
- **Structural invariants** added alongside the existing convergence proofs:
  I2 dependency-completeness, I3 conflict-freedom (target-namespace disjointness),
  I6 protected/blast-radius, I9 input-purity (advisory) — plus a stable,
  order-independent `plan-hash` (the artifact `apply` will bind to).
- **No vacuous green**: any `command`/`cron`/`script`/`recipe` resource downgrades
  the invariants it touches to `UNKNOWN` — the gate never reports safe where it
  cannot see (a real risk this fleet's exec-heavy configs surface).
- **HARD invariants** (I1/I2/I3/I6) block apply on falsification; advisory ones warn.
- Backed by `contracts/provable-iac-v1.yaml` (6 falsification tests + a Kani-verified
  conflict-detector, `provable-iac-kani-001`); quorum-validated design in
  `docs/specifications/provable-iac.md` (6 world-class lenses).


### Added — `overlay_interface` resource type (FJ-035)

A new first-class resource type that provides a DNS/DHCP-independent fleet
overlay natively, replacing the `machines/fleet-hosts` shell installer. A
machine declares `type: overlay_interface` with `overlay_ip: "10.42.0.11/24"`
and forjar idempotently binds the static secondary IP on the default-route NIC
(auto-detected, or an explicit `overlay_iface`) and installs a self-healing
systemd `service` + `timer` — plus a NetworkManager dispatcher hook where NM
owns the NIC. Optional `overlay_hosts` (managed `/etc/hosts` block) and
`overlay_firewall` (ufw allow the /24) sub-features.

The implementation reproduces the validated self-heal mechanism exactly,
including its three hard-won anti-regressions: the service is a plain
`Type=oneshot` (never `RemainAfterExit=yes`), the timer uses
`OnCalendar=minutely` (never `OnUnitActiveSec`), and apply `daemon-reload`s then
**restarts** both units so a unit-content upgrade takes effect without a reboot.
`overlay_ip` / `overlay_iface` are strictly validated and every interpolation is
shell-injection hardened.

## [1.6.2] - 2026-06-13

### Fixed — dogfood QA findings (#177, #178, #179)

The new `dogfood` QA skill (run against v1.6.1) surfaced three
low-severity issues, all fixed:

- **`forjar check` gains `--state-dir`** (#178, #182): `check` was the
  only state-reading subcommand without it (`apply`/`destroy`/`drift`/
  `status`/`undo`/`undo-destroy`/`lock-verify`/`history` all have it), so
  a stack applied to a custom state dir couldn't be checked against it.
  `check` now also reports whether each resource is recorded in state
  (`! <res> — not recorded in state`; `"in_state"` in `--json`).
- **4 stale example configs now validate** (#179, #181): `agent-deployment`,
  `dogfood-gpu-training`, `dogfood-outputs`, and `multi-agent-fleet`
  failed `forjar validate` (unknown fields, missing `sudo: true`); fixed,
  and a new `examples-validate` CI gate + `tests/examples_validate.rs`
  prevent regressions (45/45 examples now validate).
- **README C9 dependency claim corrected** (#177, #180): the falsifiable
  claim said "<20 deps (17 runtime + 1 build)" but its own verify command
  reported 30 runtime deps; restated to the accurate 30 (27 always-on +
  3 optional: age/wasmi/dhat) + 3 build, and the comparison-table cell
  updated to match.

### Added

- **`dogfood` Claude Code skill** (#176): contract-first, read-only
  exhaustive QA — rebuild+install, full command grid, prove claims
  C1–C10 and the 10 `contracts/` against a sandboxed `/tmp`-only local
  stack, safe apply→reapply→destroy→undo lifecycle, loud-failure
  injection, `dist --verify` self-test, GO/WARN/FAIL verdict.

## [1.6.1] - 2026-06-13

### Fixed — new-code audit of the v1.6.0 changes (#165)

A focused audit of the ~10.6k lines merged for v1.6.0 (the fix/feature
implementations themselves, which the original bug-hunt predated) found 9
regressions/gaps, each double-refuted; all fixed:

- **Lock-acquire livelock (regression, #169):** the v1.6.0 `O_EXCL`
  rewrite of `acquire_process_lock` could spin at 100% CPU when a
  stale-but-undeletable lock persisted (read-only mount / cross-UID
  sticky-bit dir). The loop is now bounded (5 attempts, 50ms backoff) and
  propagates a clear error instead of looping; the stale-lock reap re-reads
  and byte-matches the dead-PID content before unlinking, so it can't
  delete a concurrently-acquired fresh lock (TOCTOU); and the transport
  timeout now spawns children in their own process group and kills the
  group gated on a worker-done flag, eliminating a PID-reuse kill of an
  unrelated process.
- **Shell-escape gaps (#166):** the v1.6.0 escaping sweep missed
  `mount.rs` mounted/unmounted echo labels and `file.rs` source-read /
  unsupported-state error messages — config-derived values are now
  escaped there too.
- **Planner/executor (#167):** `moved:` collision checks now also run
  *after* recipe/resource expansion (a `to` colliding with an expanded
  recipe key was previously missed); a failing `pre_apply` hook is no
  longer re-executed under `--retry` (the v1.6.0 Skipped→Failed change had
  made it retryable) via a `retryable` flag on the failed outcome.
- **Coverage demotion (#168):** L3–L5 promotion is now recency-aware —
  a failing run at the same config hash demotes the resource (failures are
  now persisted), instead of an old passing record surviving forever.
- **Tier-2 verify (#168):** `--verify-containers` now honors a custom
  `dist.checksums` filename (the offline shim previously hardcoded
  `SHA256SUMS`, silently skipping verification for any other name).

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
