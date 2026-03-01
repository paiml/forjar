# Forjar Cookbook

**Repo**: `forjar-cookbook` (separate repo — NOT part of forjar)
**Purpose**: Qualification suite that proves forjar works on real infrastructure
**Primary runner**: Self-hosted Intel (bare-metal, 32-core Xeon, 283 GB RAM, 2x AMD GPU)

---

## What This Repo Is

`forjar-cookbook` is a **separate Rust project** that exists to flush out bugs and missing features in forjar. It is NOT a documentation exercise — it is a living test harness.

Every recipe in this repo is a real forjar config that gets applied to real machines. When a recipe exposes a bug or missing feature in forjar, we:

1. **Stop** — record the gap in the qualification checklist
2. **Implement** — fix the bug or add the feature in the `forjar` repo
3. **Release** — publish the new forjar version
4. **Retry** — re-run the recipe and mark it qualified

This is the forjar equivalent of `apr-model-qa-playbook` — a systematic qualification framework where tests own the README checklist.

### What This Repo Is NOT

- Not a documentation-only spec
- Not part of the forjar repo
- Not aspirational — every recipe either passes or has a tracked blocker
- Not GitHub Actions-first — the self-hosted runner is the primary target

---

## Repo Structure

```
forjar-cookbook/
├── Cargo.toml                          # Rust workspace (edition 2024)
├── Makefile                            # All developer targets
├── README.md                           # Live qualification dashboard (test-managed)
├── CLAUDE.md                           # Project-specific instructions
├── crates/
│   ├── cookbook-runner/                 # Recipe execution harness
│   │   ├── src/lib.rs                  # Apply recipe, capture results, timing
│   │   └── src/main.rs                 # CLI: validate, qualify, score
│   ├── cookbook-qualify/                # README sync + qualification logic
│   │   ├── src/lib.rs                  # CSV parse, table generate, README update
│   │   └── src/main.rs                 # Binary: cookbook-readme-sync
│   └── cookbook-examples/               # cargo run --example targets
│       └── examples/
│           ├── validate_all.rs         # Validate every recipe config
│           ├── plan_all.rs             # Plan every recipe (dry-run)
│           ├── apply_container.rs      # Apply Tier 2 recipes in containers
│           ├── qualify_recipe.rs       # Full qualification cycle for one recipe
│           ├── idempotency_check.rs    # Two-apply idempotency test
│           └── score_all.rs           # Score all recipes, update CSV
├── recipes/                            # Forjar YAML configs (the actual recipes)
│   ├── 01-developer-workstation.yaml
│   ├── 02-web-server.yaml
│   ├── ...
│   └── 62-stack-cross-distro.yaml
├── configs/                            # Container-testable variants
│   ├── container-01-devbox.yaml
│   ├── container-02-webserver.yaml
│   └── ...
├── docs/
│   ├── certifications/
│   │   └── recipes.csv                 # Source of truth: recipe qualification status
│   └── book/                           # mdBook source
│       ├── book.toml
│       └── src/
│           ├── SUMMARY.md
│           ├── introduction.md
│           ├── getting-started.md
│           ├── recipes/                # One chapter per recipe category
│           │   ├── infrastructure.md
│           │   ├── nix-style.md
│           │   ├── rust-builds.md
│           │   ├── packages.md
│           │   ├── opentofu-patterns.md
│           │   ├── linux-admin.md
│           │   ├── failure-modes.md
│           │   └── composition.md
│           ├── qualification.md        # How the qualification process works
│           ├── runner-setup.md         # Self-hosted runner provisioning guide
│           └── troubleshooting.md
├── scripts/
│   ├── check-docs-consistency.sh       # Verify README vs CSV parity
│   ├── coverage-check.sh              # Enforce 95% threshold
│   └── qualify-all.sh                  # Run full qualification suite on runner
├── tests/
│   └── integration_tests.rs           # Workspace-level integration tests
└── .github/workflows/
    ├── ci.yml                          # fmt + clippy + test + coverage + score + docs
    ├── qualify-runner.yml              # Self-hosted runner qualification (primary)
    └── book.yml                        # mdBook build + deploy
```

---

## Quality Gates

This repo enforces the same standards as forjar itself:

| Gate | Tool | Threshold |
|------|------|-----------|
| **Test coverage** | `cargo llvm-cov` | >= 95% line coverage |
| **Lint** | `cargo clippy -- -D warnings` | Zero warnings |
| **Format** | `cargo fmt --check` | Zero diff |
| **Code health** | `pmat comply check` | All files pass |
| **Shell safety** | `bashrs` | All scripts + Makefile linted |
| **Complexity** | Pre-commit hooks | Cyclomatic <= 30, cognitive <= 25 |
| **File size** | Pre-commit hooks | No source file > 500 lines |
| **Docs consistency** | `./scripts/check-docs-consistency.sh` | README matches CSV |
| **Examples** | `cargo run --example validate_all` | All recipes validate |

### Makefile Targets

```makefile
check: fmt-check lint test docs-check              # Full gate chain
test:                                                # cargo test --workspace
lint:                                                # cargo clippy -- -D warnings
fmt-check:                                           # cargo fmt --check
coverage:                                            # cargo llvm-cov --workspace --lib --html
coverage-check:                                      # ./scripts/coverage-check.sh (>=95%)
docs-check:                                          # ./scripts/check-docs-consistency.sh
examples:                                            # cargo run --example validate_all && ...
update-qualifications:                               # cargo run --bin cookbook-readme-sync --quiet
score:                                                # cargo run --example score_all (updates CSV)
score-recipe RECIPE=01:                               # cookbook-runner score (static analysis)
qualify-recipe RECIPE=01:                             # cookbook-runner qualify (full cycle)
qualify-all:                                          # ./scripts/qualify-all.sh (self-hosted runner)
book:                                                # mdbook build docs/book
bashrs-lint:                                         # bashrs lint scripts/ Makefile
```

### cookbook-runner CLI

The `cookbook-runner` binary provides three subcommands:

```
cookbook-runner validate --file <recipe.yaml>     # Validate config (forjar validate + plan)
cookbook-runner qualify  --file <recipe.yaml>      # Full qualification: validate → plan → apply → idempotency → score
cookbook-runner score    --file <recipe.yaml>      # Static-only score (no apply) — SAF/OBS/DOC/RES/CMP dimensions
```

The `score` subcommand accepts `--status`, `--idempotency`, and `--budget-ms` flags for runtime context when scoring without a live apply. Without runtime data, COR/IDM/PRF dimensions are 0 (static-only mode).

The `qualify` subcommand automatically computes and appends the ForjarScore after qualification completes. It uses `runtime_data_from_qualify()` to build the runtime context from actual apply results.

---

## Qualification Dashboard (README.md)

The README contains a live qualification table bounded by HTML comment markers, generated from `docs/certifications/recipes.csv`. Tests own this table — it is never hand-edited.

### Markers

```markdown
<!-- QUALIFICATION_TABLE_START -->
**Qualification Summary** (updated: 2026-03-01)

| Status | Count |
|--------|-------|
| Qualified | 56 |
| Blocked   | 5  |

| # | Recipe | Status | Grade | Score | Blocker |
|---|--------|--------|-------|-------|---------|
| 1 | developer-workstation | QUALIFIED | A | 94 | — |
| 7 | rocm-gpu | BLOCKED | F | — | FJ-1126 |
| 53 | stack-dev-server | QUALIFIED | A | 94 | — |
...
<!-- QUALIFICATION_TABLE_END -->
```

### CSV Source of Truth

`docs/certifications/recipes.csv` (23 columns — extended with ForjarScore):

```csv
recipe_num,name,category,status,tier,idempotency,first_ms,idem_ms,blocker,blocker_desc,date,by,score,grade,cor,idm,prf,saf,obs,doc,res,cmp,ver
1,developer-workstation,infra,qualified,2+3,strong,1032,23,,,2026-03-01,cookbook-runner,94,A,100,100,95,97,90,80,80,85,1.0
7,rocm-gpu,gpu,blocked,3,strong,,,FJ-1126,ROCm userspace,,,0,F,0,0,0,0,0,0,0,0,1.0
```

### Sync Binary

`crates/cookbook-qualify/src/lib.rs`:

```rust
pub const START_MARKER: &str = "<!-- QUALIFICATION_TABLE_START -->";
pub const END_MARKER: &str = "<!-- QUALIFICATION_TABLE_END -->";

pub fn parse_csv(content: &str) -> Result<Vec<RecipeQualification>>;
pub fn generate_summary(recipes: &[RecipeQualification], timestamp: &str) -> String;
pub fn generate_table(recipes: &[RecipeQualification]) -> String;
pub fn update_readme(readme: &str, table_content: &str) -> Result<String>;
pub fn write_csv(recipes: &[RecipeQualification]) -> String;
```

Invoked via `make update-qualifications` → `cargo run --bin cookbook-readme-sync --quiet`.

### The Qualification Cycle

```
Recipe YAML written
        │
        ▼
  forjar validate          ← Tier 1: does it parse?
        │
        ▼
  forjar plan              ← Tier 1: does the DAG resolve?
        │
        ▼
  forjar apply (container) ← Tier 2: does it converge in a container?
        │
        ▼
  forjar apply (runner)    ← Tier 3: does it converge on bare metal?
        │
        ▼
  Second apply = 0 changes ← Idempotency proven
        │
        ▼
  Timing within budget     ← Performance proven
        │
        ▼
  ForjarScore computed     ← 8-dimension quality grade (A through F)
        │
        ▼
  Mark QUALIFIED in CSV    ← cookbook-runner updates CSV with score
        │
        ▼
  cookbook-readme-sync      ← README table regenerated with grades
```

### When a Recipe Fails

If a recipe fails because forjar is missing a feature or has a bug:

1. **Mark BLOCKED** in CSV with `blocker_ticket` (e.g., `FJ-XXXX`) and `blocker_description`
2. **File the issue** in the forjar repo
3. **Implement the fix** in forjar
4. **Bump forjar version** in cookbook's `Cargo.toml` (or `forjar` binary version)
5. **Re-run the recipe** — `make qualify-recipe RECIPE=NN`
6. **Mark QUALIFIED** when it passes — `cookbook-runner` updates CSV + README automatically

This feedback loop is the entire point of the repo. The cookbook is a forcing function for forjar quality.

---

## Self-Hosted Runner (Primary)

The Intel runner is the **primary qualification target**. GitHub Actions `ubuntu-latest` is secondary — useful for Tier 1/2 validation, but the real qualification happens on bare metal.

### Intel Runner (`ssh intel` — Mac Pro)

- **OS**: Ubuntu 22.04 (Jammy), kernel 6.8.0-101-generic
- **CPU**: 32-core Intel Xeon W
- **RAM**: 283 GB
- **Storage**: 3.6 TB NVMe RAID-0
- **GPU**: 2x AMD Radeon Pro W5700X (Navi 10), in-tree `amdgpu` driver
- **Docker**: 29.2.1
- **GPU devices**: `/dev/kfd`, `/dev/dri/renderD128`, `/dev/dri/renderD129`
- **ROCm userspace**: Not yet installed (FJ-1126)
- **Network**: Full (real UFW, SSH, NFS-capable)
- **Forjar**: Installed from source (`cargo install --path .`)

### Why Self-Hosted is Primary

| Concern | GitHub Actions | Self-Hosted Intel |
|---------|---------------|-------------------|
| Systemd | Stubbed/absent | Real systemd, real units |
| apt installs | Work but ephemeral | Work and persist for idempotency testing |
| Firewall (ufw) | Can't enforce | Real enforcement |
| GPU | None | 2x AMD Radeon Pro W5700X |
| Kernel modules | Limited | Full (amdgpu, nfs, cgroups v2) |
| NFS mounts | Impossible | Real NFS server + client |
| Pepita isolation | No cgroups v2 | Full kernel namespace support |
| Docker socket | DinD hacks | Native Docker daemon |
| Persistence | Destroyed after job | State survives for drift testing |

### Runner CI Workflow

`.github/workflows/qualify-runner.yml`:

```yaml
name: Qualify on Runner
on:
  workflow_dispatch:
    inputs:
      recipe:
        description: 'Recipe number (or "all")'
        default: 'all'
  push:
    paths: ['recipes/**', 'configs/**']

jobs:
  qualify:
    runs-on: self-hosted
    steps:
      - uses: actions/checkout@v4
      - name: Install forjar (latest)
        run: cargo install --path ../forjar --locked
      - name: Qualify recipes
        run: |
          if [ "${{ inputs.recipe }}" = "all" ]; then
            ./scripts/qualify-all.sh
          else
            make qualify-recipe RECIPE=${{ inputs.recipe }}
          fi
      - name: Update qualification dashboard
        run: make update-qualifications
      - name: Commit results
        run: |
          git add docs/certifications/recipes.csv README.md
          git diff --cached --quiet || git commit -m "qual: update recipe status"
```

### GitHub Actions CI Workflow (Secondary)

`.github/workflows/ci.yml`:

```yaml
name: CI
on: [push, pull_request]

jobs:
  check:                    # fmt + clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy --workspace --all-targets -- -D warnings

  test:                     # full test suite
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace

  coverage:                 # >= 95% line coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo install cargo-llvm-cov
      - run: cargo llvm-cov --workspace --lib --lcov --output-path lcov.info
      - run: ./scripts/coverage-check.sh

  score:                    # static-only scoring (no forjar binary needed)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo run --example score_all

  docs:                     # README/CSV consistency
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: ./scripts/check-docs-consistency.sh
```

**Note**: `validate_all` and `plan_all` require the `forjar` binary installed (`Command::new("forjar")`), so they run only on the self-hosted runner. The `score_all` example is pure library code and runs in CI without forjar.

---

## Testability Tiers

### Tier 1: YAML Validation

**What it tests**: config parsing, schema validation, template resolution, dependency graph, plan generation, codegen script output.

**How**: `forjar validate -f <config>` + `forjar plan -f <config> --state-dir /tmp/test`.

**Where**: GitHub Actions `ubuntu-latest` (secondary) + self-hosted runner.

**Coverage**: Tests that the *right shell scripts* are generated — does not test that they *work*.

### Tier 2: Container Transport

**What it tests**: full apply cycle inside a Docker container. `check_script` → `apply_script` → `state_query_script` → BLAKE3 hash → lock file.

**How**: `forjar apply -f <config> --state-dir /tmp/test --yes` where machines use `transport: container`.

**Where**: GitHub Actions `ubuntu-latest` (Docker pre-installed) + self-hosted runner (native Docker).

**Coverage**: file, package (apt), service (systemd-stub), user, cron, network (ufw), docker. Does NOT cover: mount, GPU, model, pepita.

### Tier 3: Bare-Metal Convergence (Self-Hosted Only)

**What it tests**: actual convergence on real hardware. Real apt, real systemd, real kernel modules, real GPU.

**How**: `forjar apply -f <config> --state-dir /tmp/test --yes` on the self-hosted runner (local transport).

**Where**: Self-hosted Intel runner ONLY.

**Coverage**: Everything — GPU, mount, model downloads, pepita kernel isolation, real firewall.

---

## Idempotency Contract

Every recipe MUST satisfy this contract, enforced by `cookbook-runner`:

```
Apply #1: converge from clean state → N changes applied
Apply #2: re-apply immediately    → 0 changes, exit code 0
Assert:   state hash #1 == state hash #2
```

### Test Protocol

```bash
# First apply — converge
forjar apply -f "$config" --state-dir /tmp/idem --yes 2>&1 | tee /tmp/apply1.log
HASH1=$(cat /tmp/idem/*/state.lock.yaml | b3sum | cut -d' ' -f1)

# Second apply — must be no-op
forjar apply -f "$config" --state-dir /tmp/idem --yes 2>&1 | tee /tmp/apply2.log
HASH2=$(cat /tmp/idem/*/state.lock.yaml | b3sum | cut -d' ' -f1)

# Assert idempotency
grep -q "0 changed" /tmp/apply2.log || { echo "FAIL: second apply made changes"; exit 1; }
[ "$HASH1" = "$HASH2" ] || { echo "FAIL: state hash drift"; exit 1; }
```

### Idempotency Violations to Watch For

| Pattern | Symptom | Root Cause |
|---------|---------|------------|
| File content with timestamps | Hash changes on every apply | Template contains `{{now}}` or similar |
| apt-get always runs | `NEED_INSTALL=1` every time | Check script doesn't detect installed package |
| Service restarts on re-apply | `restart_on` triggers fire | Config file hash changes due to whitespace/ordering |
| Cron jobs duplicated | Multiple crontab entries | No idempotent crontab management |
| Overlay remounted | Mount detected as missing | Check script doesn't survive reboot |

### Per-Recipe Idempotency Classification

| Class | Meaning | Example |
|-------|---------|---------|
| **Strong** | Deterministic — same inputs always produce identical state hash | File, package, user, network |
| **Weak** | Idempotent (zero changes on re-apply) but hash may vary due to external state | Service (PID changes), docker (container ID), model (remote checksum) |
| **Eventual** | May require multiple applies to converge (dependency resolution) | Recipe composition with cross-machine dependencies |

---

## Performance Budget

Every recipe has a convergence time target. These are enforced by `cookbook-runner` — exceeding the budget is a qualification failure.

### Convergence Time Targets

| Recipe Category | First Apply (clean) | Second Apply (idempotent) | Rationale |
|----------------|--------------------|-----------------------------|-----------|
| File-only (dotfiles, configs) | < 5s | < 1s | No package downloads |
| Package + file (devbox, baseline) | < 60s | < 2s | apt-get dominates first apply |
| Service lifecycle (web, postgres) | < 90s | < 3s | Package install + service start |
| Docker containers (redis, monitoring) | < 120s | < 5s | Image pull dominates first apply |
| Rust build (release, musl) | < 300s | < 5s | Cargo compile dominates |
| Model pipeline (APR compile) | < 600s | < 5s | Model download + conversion |
| GPU qualification | < 30s | < 2s | Driver check only |
| Linux admin (cron, users, sysctl) | < 10s | < 1s | File deploys, no package installs |
| Full stack composition | < 600s | < 10s | Sum of components |

### Measurement Protocol

```bash
# Measure first-apply time
START=$(date +%s%N)
forjar apply -f "$config" --state-dir /tmp/perf --yes
END=$(date +%s%N)
FIRST_MS=$(( (END - START) / 1000000 ))

# Measure idempotent-apply time
START=$(date +%s%N)
forjar apply -f "$config" --state-dir /tmp/perf --yes
END=$(date +%s%N)
IDEM_MS=$(( (END - START) / 1000000 ))

echo "first_apply_ms=$FIRST_MS idempotent_apply_ms=$IDEM_MS"
```

### Performance Regression Detection

`cookbook-runner` captures timing for each recipe and compares against the budget table. Exceeding the budget by >50% fails qualification. Timing history is stored in `docs/certifications/recipes.csv` for trend analysis.

---

## Forjar Score

Every recipe receives a **Forjar Score** — a multi-dimensional quality grade from A through F. The score is deterministic (same inputs always produce the same grade), automatically computed by `cookbook-qualify`, and designed so that A-grade is genuinely hard to achieve. All 62 recipes are designed as A-grade targets (56 currently qualified at A-grade, 5 blocked on hardware, 1 number reserved).

### Scoring Dimensions

| Dim | Code | Weight | What It Measures |
|-----|------|--------|------------------|
| Correctness | COR | 20% | Converges from clean state |
| Idempotency | IDM | 20% | Zero changes on re-apply, stable hashes |
| Performance | PRF | 15% | Within time budget, fast re-apply |
| Safety | SAF | 15% | No dangerous patterns (0777, curl\|bash, open ports) |
| Observability | OBS | 10% | State hashes, outputs, drift hooks, notification config |
| Documentation | DOC | 8% | Comment ratio >=15%, header metadata, description |
| Resilience | RES | 7% | Failure policy, dependency DAG, lifecycle hooks |
| Composability | CMP | 5% | Params, tags, templates, includes, multi-machine |

### Grade Gates

The composite score is the weighted sum of all dimensions (0–100). However, **minimum-per-dimension gates** prevent gaming by overperforming in easy dimensions while neglecting hard ones.

| Grade | Composite | Min Dimension | Meaning |
|-------|-----------|---------------|---------|
| A | >= 90 | >= 80 | Production-hardened, hand to any SRE |
| B | >= 75 | >= 60 | Solid, minor gaps |
| C | >= 60 | >= 40 | Functional but rough |
| D | >= 40 | any | Bare minimum |
| F | < 40 | any | Or: blocked/pending/never-qualified |

**Why A is hard**: A recipe scoring 100 in 7 dimensions but 70 in Documentation still gets B (min dimension < 80). Every dimension must be >= 80.

### Hard-Fail Conditions (Automatic F)

- `status == blocked` or `status == pending` (never qualified)
- Validation fails (`forjar validate` exit != 0)
- Plan fails (`forjar plan` exit != 0)
- First apply fails (`forjar apply` exit != 0)

Any hard-fail condition produces grade F regardless of dimension scores.

### Dimension Scoring Formulas

Each dimension scores 0–100. Points are additive within each dimension.

**COR — Correctness (20%)**
- validate_pass: +20
- plan_pass: +20
- first_apply_pass: +40
- all_resources_converged: +10
- state_lock_written: +10
- Penalty: -2 per warning (max -10)

**IDM — Idempotency (20%)**
- second_apply_pass: +30
- zero_changes_on_reapply: +30
- hash_stable: +20
- Idempotency class bonus: strong +20, weak +10, eventual +0
- Penalty: -10 per changed resource on 2nd apply

**PRF — Performance (15%)**
- First apply vs budget: <=50% → 50pts, <=75% → 40, <=100% → 30, <=150% → 15
- Idempotent timing: <=2s → 30pts, <=5s → 25, <=10s → 15
- Efficiency ratio (idem/first): <=5% → 20pts, <=10% → 15, <=25% → 10

**SAF — Safety (15%)** — Starts at 100, deductions:
- Critical: mode 0777 (-30), curl\|bash (-30), root+0666 (-20), wide-open ports (-15)
- Moderate: no explicit mode on file (-5), no explicit owner (-3), no version pin (-3)
- Hard cap at 40 if any critical violation exists

**OBS — Observability (10%)**
- tripwire_policy present: +15
- lock_file configured: +15
- outputs section: +10
- File mode coverage (% of files with explicit mode): up to +15 (full credit if no file resources)
- Owner coverage (% of files with explicit owner): up to +15 (full credit if no file resources)
- Notify hooks (on_success/on_failure/on_drift): up to +20

**DOC — Documentation (8%)**
- Comment ratio: >=15% → 40pts, >=10% → 30, >=5% → 20
- Header metadata (recipe#): +10
- Header metadata (tier): +10
- Header metadata (idempotency): +10
- description field present: +15
- Descriptive name (not generic): +5

**RES — Resilience (7%)**
- failure policy (continue_independent): +20
- ssh_retries > 1: +10
- Dependency DAG ratio: >=50% → 30pts, >=30% → 20
- pre_apply hook: +10
- post_apply hook: +10

**CMP — Composability (5%)**
- params with defaults: +20
- templates used: +10
- includes: +10
- tags on resources: +15
- resource_groups: +15
- multi-machine: +10
- recipe nesting: +15

### CSV Schema Extension

The qualification CSV extends from 12 to 23 columns. New fields:

| Column | Type | Description |
|--------|------|-------------|
| score | u32 | Composite score (0–100) |
| grade | String | Letter grade (A/B/C/D/F) |
| cor | u32 | Correctness dimension (0–100) |
| idm | u32 | Idempotency dimension (0–100) |
| prf | u32 | Performance dimension (0–100) |
| saf | u32 | Safety dimension (0–100) |
| obs | u32 | Observability dimension (0–100) |
| doc | u32 | Documentation dimension (0–100) |
| res | u32 | Resilience dimension (0–100) |
| cmp | u32 | Composability dimension (0–100) |
| score_version | String | Scoring algorithm version (e.g., "1.0") |

Backward compatibility: missing fields default to 0/empty when parsing older CSV files.

### README Table Rendering

The qualification table includes a **Grade** column with shields.io badges:
- A: `brightgreen`
- B: `blue`
- C: `yellow`
- D: `orange`
- F: `red`

The summary section includes a **Grade Distribution** row showing counts per grade.

### Worked Example

Consider Recipe #1 (developer-workstation) with full qualification data:

```
COR: 100 (all steps pass, converged, state lock written)
IDM: 100 (second apply passes, zero changes, hash stable, strong class)
PRF:  95 (first apply 7.5s vs 60s budget = 12.5%, idem 408ms, ratio 5.4%)
SAF:  97 (explicit mode/owner on all files, -3 for no version pin on packages)
OBS:  90 (tripwire, lock_file, outputs, file mode/owner coverage 100%, notify 3/3)
DOC:  80 (10%+ comments, header with recipe#/tier/idempotency, description, dash name)
RES:  80 (continue_independent, ssh_retries=3, DAG 50%+, pre_apply, post_apply)
CMP:  85 (params, templates, includes×2, tags, resource_groups)

Composite: 100×0.20 + 100×0.20 + 95×0.15 + 97×0.15 + 90×0.10 + 80×0.08 + 80×0.07 + 85×0.05
         = 20.0 + 20.0 + 14.25 + 14.55 + 9.0 + 6.4 + 5.6 + 4.25 = 94.05 → 94

Min dimension: DOC = 80 (>= 80 ✓)
Grade: A (composite 94 >= 90, min dimension 80 >= 80)
```

All 56 qualified recipes achieve A-grade through: shared includes (CMP+25), inline comments (DOC+10), dependency DAGs (RES+10), budget-aware PRF scoring, and full policy configuration (OBS+20).

---

## Cookbook Recipes

### 1. Developer Workstation

**Tier**: 1 (validate) + 2 (container apply for file/package subset)

Provision a development machine with tools, dotfiles, and directory structure. This is what `infra/forjar.yaml` already does for the Intel runner.

**Resources**: package (cargo, apt), file (dotfiles, directories)

**Testable in CI**:
- Tier 1: validate config, verify plan output, check codegen scripts
- Tier 2: apply inside container — apt packages install, files land at correct paths with correct permissions, template params resolve

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `user` | string | required | Unix username |
| `home` | path | `/home/{{inputs.user}}` | Home directory |
| `editor` | enum | `vim` | `vim` / `nano` / `emacs` |
| `shell_framework` | enum | `pzsh` | `pzsh` / `oh-my-zsh` / `none` |
| `cargo_tools` | string | `ripgrep,fd-find,bat` | Comma-separated cargo packages |

**Resources** (7):
- `dev-packages` — apt: build-essential, curl, git, tmux, htop
- `cargo-tools` — cargo: from `cargo_tools` input
- `home-dir` — directory: `{{inputs.home}}` with correct owner
- `gitconfig` — file: `.gitconfig` with user/editor
- `vimrc` — file: `.vimrc` (when editor=vim)
- `tmux-conf` — file: `.tmux.conf`
- `shell-rc` — file: `.zshrc` or `.bashrc` with framework eval

---

### 2. Web Application Server

**Tier**: 1 + 2 (full container apply)

Nginx reverse proxy + application config + firewall + TLS cert directories.

**Testable in CI**:
- Tier 1: validate, plan, codegen
- Tier 2: full container apply — apt install nginx, deploy config files, set up firewall rules (ufw), create TLS directories. Verify idempotency.

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `domain` | string | required | Domain name |
| `port` | int | `443` | Listen port |
| `upstream_port` | int | `8080` | App backend port |
| `tls_email` | string | `""` | Let's Encrypt email (empty = self-signed) |
| `log_level` | enum | `warn` | `error` / `warn` / `info` / `debug` |

**Resources** (8):
- `nginx-pkg` — apt: nginx
- `tls-dir` — directory: `/etc/nginx/ssl/{{inputs.domain}}`
- `nginx-site` — file: `/etc/nginx/sites-available/{{inputs.domain}}`
- `nginx-enable` — symlink: sites-enabled → sites-available
- `nginx-service` — service: nginx (running, enabled, restart_on: nginx-site)
- `firewall-https` — network: allow {{inputs.port}}/tcp
- `firewall-http` — network: allow 80/tcp (for ACME challenges)
- `firewall-deny-upstream` — network: deny {{inputs.upstream_port}}/tcp from 0.0.0.0/0

---

### 3. PostgreSQL Database Server

**Tier**: 1 + 2 (container apply with apt)

Production PostgreSQL with tuned config, backup cron, and firewall lockdown.

**Testable in CI**:
- Tier 1: validate, plan
- Tier 2: container apply — install postgresql, deploy pg_hba.conf and postgresql.conf, create backup script, set up cron, configure firewall

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `pg_version` | string | `16` | PostgreSQL major version |
| `listen_addresses` | string | `localhost` | Bind addresses |
| `max_connections` | int | `200` | Connection limit |
| `shared_buffers` | string | `256MB` | Shared memory |
| `backup_schedule` | string | `0 2 * * *` | Backup cron (2 AM daily) |
| `backup_dir` | path | `/var/backups/postgresql` | Backup destination |
| `allowed_cidr` | string | `127.0.0.1/32` | pg_hba.conf client CIDR |

**Resources** (8):
- `pg-pkg` — apt: postgresql-{{inputs.pg_version}}
- `pg-conf` — file: postgresql.conf (tuned settings)
- `pg-hba` — file: pg_hba.conf (auth rules)
- `pg-service` — service: postgresql (running, enabled, restart_on: pg-conf, pg-hba)
- `backup-dir` — directory: {{inputs.backup_dir}}
- `backup-script` — file: `/usr/local/bin/pg-backup.sh`
- `backup-cron` — cron: pg_basebackup on schedule
- `firewall-pg` — network: allow 5432/tcp from {{inputs.allowed_cidr}}

---

### 4. Monitoring Stack (Prometheus + Grafana)

**Tier**: 1 + 2 (container apply for config/directories; Docker resources for the services)

Lightweight monitoring: Prometheus scrapes node_exporter + app metrics, Grafana dashboards.

**Testable in CI**:
- Tier 1: validate, plan
- Tier 2: container apply — directory structure, config files, Docker container resources (prometheus, grafana, node_exporter), firewall rules

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `prometheus_port` | int | `9090` | Prometheus listen port |
| `grafana_port` | int | `3000` | Grafana listen port |
| `retention_days` | int | `30` | Metric retention |
| `scrape_targets` | string | `localhost:9100` | Comma-separated scrape endpoints |
| `grafana_admin_password` | string | `admin` | Initial Grafana password |

**Resources** (8):
- `prometheus-data` — directory: `/var/lib/prometheus`
- `prometheus-config` — file: `/etc/prometheus/prometheus.yml` (scrape config)
- `prometheus` — docker: prom/prometheus (ports, volumes, restart: always)
- `grafana-data` — directory: `/var/lib/grafana`
- `grafana` — docker: grafana/grafana (ports, volumes, env, restart: always)
- `node-exporter` — docker: prom/node-exporter (host network, restart: always)
- `firewall-grafana` — network: allow {{inputs.grafana_port}}/tcp
- `firewall-prometheus` — network: deny {{inputs.prometheus_port}}/tcp from 0.0.0.0/0

---

### 5. Redis Cache

**Tier**: 1 + 2

Redis with persistence config, memory limits, and firewall.

**Testable in CI**:
- Tier 2: container apply — Docker resource for Redis container, config file, firewall

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `port` | int | `6379` | Redis listen port |
| `maxmemory` | string | `256mb` | Memory limit |
| `maxmemory_policy` | enum | `allkeys-lru` | Eviction policy |
| `appendonly` | bool | `true` | AOF persistence |

**Resources** (4):
- `redis-data` — directory: `/var/lib/redis`
- `redis-config` — file: `/etc/redis/redis.conf`
- `redis` — docker: redis:7-alpine (ports, volumes, command, restart: always)
- `firewall-redis` — network: deny {{inputs.port}}/tcp from 0.0.0.0/0

---

### 6. CI Runner (GitHub Actions Self-Hosted)

**Tier**: 1 + 3 (validated in CI, converged on real runner hardware)

Provision a self-hosted GitHub Actions runner with Docker, build tools, and Rust toolchain. This is the recipe that bootstraps the Intel runner itself.

**Testable in CI**:
- Tier 1: validate, plan, codegen review
- Tier 3: apply on Intel runner — installs runner agent, Docker, Rust, registers with GitHub

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `runner_name` | string | required | GitHub runner name |
| `runner_labels` | string | `self-hosted,linux,x86_64` | Comma-separated labels |
| `github_org` | string | required | GitHub org or user |
| `github_repo` | string | `""` | Specific repo (empty = org-level) |
| `runner_user` | string | `runner` | Unix user for the runner |
| `work_dir` | path | `/opt/actions-runner/_work` | Runner work directory |
| `docker_compose` | bool | `true` | Install docker-compose |

**Resources** (9):
- `runner-user` — user: system account for runner agent
- `runner-deps` — package (apt): curl, jq, git, build-essential, pkg-config, libssl-dev
- `docker-pkg` — package (apt): docker-ce, docker-ce-cli, containerd.io
- `docker-group` — user config: add runner_user to docker group
- `docker-service` — service: docker (running, enabled)
- `runner-dir` — directory: `/opt/actions-runner`
- `runner-install` — file: install script (download + configure actions-runner)
- `runner-service` — service: actions.runner (running, enabled)
- `firewall-outbound` — network: allow 443/tcp (GitHub API)

---

### 7. ROCm GPU Workstation

**Tier**: 1 + 3 (validated in CI, converged on Intel runner)

AMD GPU development environment: ROCm userspace, kernel driver verification, HIP toolkit.

**Testable in CI**:
- Tier 1: validate, plan, verify check_script/apply_script codegen
- Tier 3: apply on Intel runner — verify amdgpu driver, install rocminfo, verify GPU device access

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `gpu_name` | string | `gpu0` | GPU resource name |
| `driver_version` | string | `""` | Expected driver version or `kernel-*` for in-tree |
| `rocm_version` | string | `""` | ROCm version to install (empty = latest) |
| `install_hip` | bool | `false` | Install HIP development toolkit |
| `user` | string | required | User to add to video/render groups |

**Resources** (5):
- `amd-gpu` — gpu: backend=rocm, driver_version, rocm_version
- `gpu-user-groups` — user: add {{inputs.user}} to video, render groups
- `rocm-tools` — package (apt): rocminfo, rocm-smi-lib
- `hip-dev` — package (apt): hip-dev (when install_hip=true)
- `gpu-health-cron` — cron: `rocm-smi --showtemp` every 6h to syslog

---

### 8. NVIDIA GPU + CUDA Server

**Tier**: 1 + 3 (needs NVIDIA hardware — not available on Intel runner)

NVIDIA GPU with CUDA toolkit, persistence mode, compute mode lockdown.

**Testable in CI**:
- Tier 1: validate, plan, codegen review
- Tier 3: requires a runner with NVIDIA GPU (blocked: FJ-1127)

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `driver_version` | string | `550` | NVIDIA driver version |
| `cuda_version` | string | `12.4` | CUDA toolkit version |
| `compute_mode` | enum | `default` | `default` / `exclusive_process` / `prohibited` |
| `persistence_mode` | bool | `true` | Enable nvidia-persistenced |
| `gpu_devices` | string | `all` | GPU indices or "all" |

**Resources** (4):
- `nvidia-gpu` — gpu: backend=nvidia, driver, cuda, compute_mode, persistence
- `nvidia-container` — package (apt): nvidia-container-toolkit (for Docker GPU passthrough)
- `gpu-health-cron` — cron: `nvidia-smi --query-gpu=...` every 6h
- `firewall-deny-gpu-api` — network: deny 8080/tcp from 0.0.0.0/0

---

### 9. Secure Baseline

**Tier**: 1 + 2

Minimal security hardening: SSH config, firewall defaults, fail2ban, automatic updates.

**Testable in CI**:
- Tier 2: container apply — SSH config deployed, UFW rules applied, fail2ban config landed

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `ssh_port` | int | `22` | SSH listen port |
| `allowed_ssh_cidr` | string | `0.0.0.0/0` | Allowed SSH source CIDR |
| `auto_updates` | bool | `true` | Enable unattended-upgrades |
| `fail2ban_maxretry` | int | `5` | Login attempts before ban |

**Resources** (7):
- `security-pkgs` — package (apt): fail2ban, unattended-upgrades, ufw
- `sshd-config` — file: `/etc/ssh/sshd_config` (no root login, key-only auth, custom port)
- `sshd-service` — service: sshd (running, enabled, restart_on: sshd-config)
- `fail2ban-config` — file: `/etc/fail2ban/jail.local`
- `fail2ban-service` — service: fail2ban (running, enabled, restart_on: fail2ban-config)
- `firewall-ssh` — network: allow {{inputs.ssh_port}}/tcp from {{inputs.allowed_ssh_cidr}}
- `firewall-default-deny` — network: deny 0/tcp from 0.0.0.0/0

---

### 10. NFS File Server

**Tier**: 1 + 3 (mount resources need real kernel)

NFS exports from a file server + NFS mounts on client machines.

**Testable in CI**:
- Tier 1: validate, plan, codegen
- Tier 3: apply on Intel runner (both NFS server exports and client mounts)

**Recipe inputs** (server):
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `export_path` | path | required | Directory to export |
| `allowed_network` | string | `192.168.0.0/24` | NFS client CIDR |
| `options` | string | `rw,sync,no_subtree_check` | Export options |

**Recipe inputs** (client):
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `server_addr` | string | required | NFS server IP/hostname |
| `remote_path` | path | required | Server export path |
| `mount_point` | path | required | Local mount point |

**Resources** (server — 4):
- `nfs-pkg` — package (apt): nfs-kernel-server
- `export-dir` — directory: {{inputs.export_path}}
- `exports-config` — file: `/etc/exports`
- `nfs-service` — service: nfs-kernel-server (running, enabled, restart_on: exports-config)

**Resources** (client — 3):
- `nfs-client-pkg` — package (apt): nfs-common
- `mount-point` — directory: {{inputs.mount_point}}
- `nfs-mount` — mount: {{inputs.server_addr}}:{{inputs.remote_path}} on {{inputs.mount_point}}

---

## CI Integration Plan

CI is split across two workflows. The self-hosted runner workflow is the primary qualification pipeline.

### `ci.yml` — Code Quality (GitHub Actions `ubuntu-latest`)

Runs on every push and PR. Guards code quality, not recipe qualification.

| Job | What it does |
|-----|-------------|
| `check` | `cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings` |
| `test` | `cargo test --workspace` |
| `coverage` | `cargo install cargo-llvm-cov` + `cargo llvm-cov` >= 95% threshold |
| `score` | `cargo run --example score_all` — static-only scoring (no forjar binary) |
| `docs` | `./scripts/check-docs-consistency.sh` |

### `qualify-runner.yml` — Recipe Qualification (Self-Hosted Intel)

The primary pipeline. Runs on push to `recipes/**` or manual dispatch.

```yaml
name: Qualify on Runner
on:
  workflow_dispatch:
    inputs:
      recipe:
        description: 'Recipe number (or "all")'
        default: 'all'
  push:
    paths: ['recipes/**', 'configs/**']

jobs:
  qualify:
    runs-on: self-hosted
    steps:
      - uses: actions/checkout@v4

      - name: Install forjar (latest from source)
        run: cargo install --path ../forjar --locked

      - name: Qualify recipes
        run: |
          if [ "${{ inputs.recipe }}" = "all" ]; then
            ./scripts/qualify-all.sh
          else
            make qualify-recipe RECIPE=${{ inputs.recipe }}
          fi

      - name: Update qualification dashboard
        run: make update-qualifications

      - name: Commit results
        run: |
          git add docs/certifications/recipes.csv README.md
          git diff --cached --quiet || git commit -m "qual: update recipe status"
```

### `book.yml` — Documentation (GitHub Actions `ubuntu-latest`)

Builds and deploys mdBook on push to `docs/book/**`.

---

## Naming Conventions

| Pattern | Purpose | Example |
|---------|---------|---------|
| `recipes/NN-<name>.yaml` | Recipe config (numbered) | `recipes/01-developer-workstation.yaml` |
| `configs/container-NN-<name>.yaml` | Container-testable variant (Tier 2) | `configs/container-01-devbox.yaml` |

---

## Implementation Priority

All 9 phases are **complete**. 56 of 61 recipes are qualified at A-grade. 5 recipes are blocked on hardware requirements (GPU, NFS, secrets infrastructure, GPG keys).

### Phase Summary

| Phase | Recipes | Status |
|-------|---------|--------|
| 1: Core Infrastructure | #1-6, #9 | **Complete** — all A-grade |
| 2: GPU & Hardware | #7, #8, #10 | **Blocked** — #7 FJ-1126 ROCm, #8 FJ-1127 NVIDIA, #10 FJ-1128 NFS |
| 3: Nix-Style | #11-15 | **Complete** — all A-grade |
| 4: Rust Build Pipelines | #16-21 | **Complete** — all A-grade |
| 5: Package Distribution | #25-29 | **Complete** — #25 blocked (FJ-1130 GPG), rest A-grade |
| 6: Operational Maturity | #22-24 | **Complete** — #22 blocked (FJ-1129 secrets), rest A-grade |
| 7: Linux Administration | #40-49 | **Complete** — all A-grade |
| 8: OpenTofu Patterns | #30-39 | **Complete** — all A-grade |
| 9: Resilience & Composition | #50-62 | **Complete** — all A-grade |

### Blocked Recipes

| # | Recipe | Blocker | Requirement |
|---|--------|---------|-------------|
| 7 | ROCm GPU | FJ-1126 | AMD GPU with ROCm driver |
| 8 | NVIDIA GPU | FJ-1127 | NVIDIA GPU with CUDA driver |
| 10 | NFS Server | FJ-1128 | Bare-metal with NFS kernel modules |
| 22 | Secrets Lifecycle | FJ-1129 | age encryption infrastructure |
| 25 | Third-Party APT Repo | FJ-1130 | GPG signing key setup |

---

## Nix-Style Recipes

Declarative, reproducible, isolated environments using forjar's kernel primitives instead of the Nix store.

### How It Maps

| Nix Concept | Forjar Equivalent | Mechanism |
|---|---|---|
| `nix develop` / `nix-shell` | Pepita transport machine | Kernel namespaces + overlayfs |
| `/nix/store` layers | `overlay_lower` / `overlay_upper` | OverlayFS copy-on-write |
| `flake.lock` | `state.lock.yaml` | BLAKE3 content-addressed hashes |
| NixOS modules | Recipes with typed inputs | YAML + `{{inputs.*}}` templates |
| `nix-collect-garbage` | `state: absent` + destroy overlay | Remove upper layer, done |
| Rollback / generations | State lock history + git | `git log` on forjar configs |
| Home Manager | File resources | Already working (dotfiles, configs) |
| `nix build` (hermetic) | Pepita ephemeral sandbox | Overlay + netns isolation + cgroups |
| `nix profile install` | Package resources with versions | apt/cargo with version pinning |

### What Forjar Adds Beyond Nix

- **Multi-machine** — `nix develop` is local-only; forjar orchestrates fleets
- **Drift detection** — Nix has no runtime drift detection; forjar watches for unauthorized changes
- **GPU-aware** — Nix has poor GPU/driver support; forjar has first-class GPU resources
- **No new package manager** — uses apt/cargo/uv, not a parallel universe of packages

### What Nix Does Better (and we don't try to replicate)

- Content-addressed per-package isolation (each package in its own `/nix/store/<hash>-name/`)
- Hermetic builds with no network access
- Atomic profile switching via symlink generations
- Massive package repository (nixpkgs)

---

### 11. Dev Shell (like `nix develop`)

**Tier**: 1 + 3 (needs real kernel namespaces + overlayfs)

Per-project isolated development environment. The machine uses pepita transport with overlayfs — packages install into the overlay, not the host. Set `ephemeral: false` to persist across sessions (like `nix develop`), or `ephemeral: true` for throwaway shells (like `nix-shell --pure`).

**Testable in CI**:
- Tier 1: validate config, verify plan, review codegen scripts
- Tier 3: apply on Intel runner — creates namespace, mounts overlay, installs packages into overlay, drops you into isolated shell

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | required | Environment name (used for namespace + overlay paths) |
| `packages_apt` | string | `""` | Comma-separated apt packages |
| `packages_cargo` | string | `""` | Comma-separated cargo packages |
| `env_vars` | string | `""` | `KEY=VALUE` pairs, comma-separated |
| `memory_mb` | int | `4096` | cgroup memory limit |
| `cpus` | string | `""` | CPU affinity (e.g., `0-3`), empty = all |
| `network` | enum | `host` | `host` (shared network) or `isolated` |
| `ephemeral` | bool | `false` | Destroy after apply |
| `shell_hook` | string | `""` | Command to run on shell entry (like Nix's `shellHook`) |

**Machine config** (pepita transport):
```yaml
machines:
  devshell:
    hostname: "devshell-{{inputs.name}}"
    addr: pepita
    transport: pepita
    pepita:
      rootfs: /                              # host root as base layer
      memory_mb: "{{inputs.memory_mb}}"
      network: "{{inputs.network}}"
      filesystem: overlay                    # CoW — installs don't touch host
      ephemeral: "{{inputs.ephemeral}}"
```

**Resources** (4):
- `dev-packages-apt` — package (apt): from `packages_apt` input, installed into overlay
- `dev-packages-cargo` — package (cargo): from `packages_cargo` input
- `env-file` — file: `/etc/profile.d/devshell.sh` with env vars + shell hook
- `shell-entry` — file: `/etc/motd` with environment name + loaded packages

**Key difference from Nix**: packages are real apt/cargo installs but isolated in the overlay. No new package format, no learning curve. Delete the overlay upper dir and you're back to clean host.

---

### 12. Toolchain Pin (like `nix profile`)

**Tier**: 1 + 2 (container apply for version verification)

Declare exact versions of compilers and runtimes. Converge to those versions. Drift detection catches unexpected upgrades (e.g., `apt upgrade` bumps Python from 3.11 to 3.12).

**Testable in CI**:
- Tier 1: validate, plan
- Tier 2: container apply — install pinned packages, verify versions via state query, test drift detection against version changes

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `rust_version` | string | `""` | Rust toolchain (e.g., `1.82.0`), empty = skip |
| `python_version` | string | `""` | Python version (e.g., `3.11`), empty = skip |
| `node_version` | string | `""` | Node.js major version (e.g., `20`), empty = skip |
| `go_version` | string | `""` | Go version (e.g., `1.22`), empty = skip |

**Resources** (up to 8, conditional via `when:`):
- `rust-toolchain` — package (cargo): rustup + `rustup default {{inputs.rust_version}}`
- `rust-version-check` — file: `/etc/forjar/toolchain.d/rust` (version pin marker for drift)
- `python-pkg` — package (apt): `python{{inputs.python_version}}`
- `python-version-check` — file: `/etc/forjar/toolchain.d/python`
- `node-pkg` — package (apt): `nodejs` (via nodesource repo for version pinning)
- `node-version-check` — file: `/etc/forjar/toolchain.d/node`
- `go-pkg` — file: `/usr/local/go` (binary tarball install for exact version)
- `go-version-check` — file: `/etc/forjar/toolchain.d/go`

**Drift detection**: The version-check files contain the expected version string. `forjar drift` hashes these files + the actual binary output of `rustc --version`, `python3 --version`, etc. via state queries. If someone runs `apt upgrade` and Python moves to 3.12, drift fires.

---

### 13. Ephemeral Build Sandbox (like `nix build`)

**Tier**: 1 + 3 (needs real overlayfs + namespaces)

Throwaway build environment: overlay on `/`, install build deps, run build, extract artifacts, destroy. Network-isolated to prevent builds from fetching undeclared dependencies (like Nix's `--sandbox`).

**Testable in CI**:
- Tier 1: validate, plan
- Tier 3: apply on Intel runner — full lifecycle: create overlay → install deps → build → extract → destroy

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | required | Build name |
| `build_deps` | string | required | Comma-separated apt packages |
| `build_command` | string | required | Shell command to run |
| `artifact_paths` | string | required | Comma-separated paths to extract from overlay |
| `output_dir` | path | `/tmp/build-output` | Where to copy artifacts on the host |
| `memory_mb` | int | `8192` | Build memory limit |
| `cpuset` | string | `""` | CPU affinity |
| `network` | enum | `isolated` | `isolated` (hermetic) or `host` (can fetch) |

**Machine config** (pepita transport, ephemeral):
```yaml
machines:
  builder:
    hostname: "build-{{inputs.name}}"
    addr: pepita
    transport: pepita
    pepita:
      rootfs: /
      memory_mb: "{{inputs.memory_mb}}"
      network: "{{inputs.network}}"
      filesystem: overlay
      ephemeral: true                        # destroy after apply
```

**Resources** (4):
- `build-deps` — package (apt): from `build_deps` input
- `build-run` — cron/command: execute `build_command` (one-shot via `pre_apply` hook)
- `extract-artifacts` — file: `post_apply` hook copies `artifact_paths` to host `output_dir`
- `build-manifest` — file: `/build-manifest.json` with inputs, timestamp, BLAKE3 hash of artifacts

**Key difference from Nix**: no need for a derivation language or Nix store. It's just "spin up overlay, apt install, run command, grab files, tear down." The `ephemeral: true` flag handles cleanup automatically.

---

### 14. System Profile (like NixOS `configuration.nix`)

**Tier**: 1 + 3

The full-machine declarative profile. Composes Dev Shell + Toolchain Pin + infrastructure recipes into one config. This is what `infra/forjar.yaml` already does — formalized as a composable recipe that other teams can fork.

**Testable in CI**:
- Tier 1: validate, plan, dependency graph analysis
- Tier 3: apply on Intel runner — full machine convergence

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `user` | string | required | Primary user |
| `home` | path | `/home/{{inputs.user}}` | Home directory |
| `role` | enum | `dev` | `dev` / `gpu-compute` / `ci-runner` / `server` |
| `editor` | enum | `vim` | `vim` / `nano` / `emacs` |
| `shell_framework` | enum | `pzsh` | `pzsh` / `oh-my-zsh` / `none` |
| `cargo_tools` | string | `ripgrep,fd-find,bat,hyperfine` | Cargo-installed CLI tools |
| `gpu_backend` | enum | `none` | `none` / `nvidia` / `rocm` / `cpu` |
| `monitoring` | bool | `true` | Enable Prometheus + node_exporter |

**Composes these recipes** (via `type: recipe`):
- Developer Workstation (recipe #1) — always
- Secure Baseline (recipe #9) — always
- Toolchain Pin (recipe #12) — always
- ROCm GPU Workstation (recipe #7) — when `gpu_backend=rocm`
- NVIDIA GPU + CUDA (recipe #8) — when `gpu_backend=nvidia`
- Monitoring Stack (recipe #4) — when `monitoring=true`

**Resources** (composition, ~15-30 depending on role):
```yaml
resources:
  base:
    type: recipe
    machine: target
    recipe: recipes/devbox.yaml
    inputs:
      user: "{{inputs.user}}"
      editor: "{{inputs.editor}}"

  security:
    type: recipe
    machine: target
    recipe: recipes/secure-baseline.yaml
    depends_on: [base]

  toolchain:
    type: recipe
    machine: target
    recipe: recipes/toolchain-pin.yaml
    inputs:
      rust_version: "1.82.0"
    depends_on: [base]

  gpu:
    type: recipe
    machine: target
    recipe: recipes/rocm-gpu.yaml
    when: "{{inputs.gpu_backend}} == rocm"
    depends_on: [base]
```

This is the Nix equivalent of importing modules in `configuration.nix` — each recipe is a module with typed inputs, and the system profile composes them.

---

### 15. Multi-Project Workspace (like Nix flake workspaces)

**Tier**: 1 + 3

Multiple dev shells sharing a common base overlay but with project-specific upper layers. Like `nix develop .#project-a` vs `.#project-b` — same Rust toolchain, different project deps.

**Testable in CI**:
- Tier 1: validate all workspace configs
- Tier 3: apply on Intel runner — verify shared base layer, independent upper layers

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `workspace_name` | string | required | Workspace name |
| `base_packages` | string | `build-essential,curl,git` | Shared base layer packages |
| `projects` | string | required | Comma-separated project names |

**Architecture**:
```
overlay_lower: /                     ← host OS (read-only)
  └─ overlay "base":
       upper: /var/forjar/ws/base/   ← shared packages (rust, python, etc.)
       └─ overlay "project-a":
            upper: /var/forjar/ws/project-a/  ← project-specific deps
       └─ overlay "project-b":
            upper: /var/forjar/ws/project-b/  ← different deps
```

Each project gets its own pepita machine with overlayfs stacked on the shared base:

```yaml
machines:
  base:
    hostname: "ws-{{inputs.workspace_name}}-base"
    addr: pepita
    transport: pepita
    pepita:
      rootfs: /
      filesystem: overlay
      ephemeral: false

  project-a:
    hostname: "ws-{{inputs.workspace_name}}-project-a"
    addr: pepita
    transport: pepita
    pepita:
      rootfs: /var/forjar/ws/base/merged   # stacks on base overlay
      filesystem: overlay
      ephemeral: false
```

**Resources per project** (3):
- `project-deps` — package: project-specific apt/cargo packages
- `project-env` — file: `/etc/profile.d/project.sh` with project env vars
- `project-config` — file: project-specific tool configs

**Key advantage over Nix**: multi-machine. The workspace can span multiple hosts — e.g., a GPU project gets its dev shell on the Intel runner, a web project gets its dev shell on a cloud VM, both share the same forjar config.

---

## Nix-Style Recipe Testability Summary

| # | Recipe | Tier | GitHub Actions | Intel Runner |
|---|--------|------|----------------|--------------|
| 11 | Dev Shell | 1+3 | validate + plan | full apply (namespace + overlay) |
| 12 | Toolchain Pin | 1+2 | validate + container apply | full apply |
| 13 | Build Sandbox | 1+3 | validate + plan | full lifecycle (create → build → extract → destroy) |
| 14 | System Profile | 1+3 | validate + plan | full machine convergence |
| 15 | Multi-Project Workspace | 1+3 | validate + plan | stacked overlays, shared base |

**Toolchain Pin is the only Nix-style recipe fully testable in GitHub Actions** (Tier 2) because it just uses apt/cargo packages — no kernel namespace or overlayfs needed. The rest require Tier 3 (Intel runner) for the pepita transport.

---

## Rust Build Recipes

Declarative Rust compilation pipelines — from `cargo install` one-liners to multi-stage static binary builds producing minimal deploy artifacts.

### Current State

Forjar's cargo provider (`provider: cargo`) does `cargo install --force '<crate>'`. This is fine for dev tooling but insufficient for production builds:

- No control over release profile, target triple, or RUSTFLAGS
- No musl static linking
- No binary size optimization beyond what Cargo.toml specifies
- No multi-stage: build deps stay on the machine forever
- No cross-compilation
- No artifact extraction or BLAKE3 verification

The sovereign AI stack binaries on the Intel runner today:

| Binary | Size | Linked | Notes |
|--------|------|--------|-------|
| `forjar` | 12 MB | dynamic (libssl, libc) | `lto=true, strip=true, codegen-units=1` |
| `pmat` | 39 MB | dynamic | Includes sqlite + full index engine |
| `batuta` | 11 MB | dynamic | Mutation testing engine |
| `pzsh` | 961 KB | dynamic | Already tiny — shell framework |

Forjar has no openssl/native-tls dependency (uses `age` crate with pure Rust crypto), so musl static builds should work cleanly for fully self-contained binaries.

### Size Reduction Techniques

| Technique | Mechanism | Typical Savings |
|-----------|-----------|-----------------|
| `--release` | Optimized codegen | 5-10x vs debug |
| `lto = true` | Link-time optimization across crates | 10-30% |
| `codegen-units = 1` | Better LTO (single codegen unit) | 5-10% |
| `strip = true` | Remove debug symbols | 20-40% |
| `opt-level = "z"` | Optimize for size over speed | 10-20% (slower runtime) |
| `panic = "abort"` | Remove unwinding tables | 5-10% |
| musl static | Eliminate libc/libssl dynamic deps | +size but fully portable |
| `upx --best` | Binary compression | 50-70% (slower startup) |
| Multi-stage | Build deps don't ship | N/A (deployment concern) |

---

### 16. Rust Release Build

**Tier**: 1 + 2 (container apply) + 3 (Intel runner for real builds)

Compile a Rust crate with production release settings. Generates a `pre_apply` hook that runs the actual cargo build, then deploys the binary to the target path.

**Testable in CI**:
- Tier 1: validate, plan, codegen review
- Tier 2: container apply — install Rust toolchain, clone repo, build release binary, verify binary exists at target path
- Tier 3: Intel runner — real build with all optimizations, verify binary size and functionality

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `crate_name` | string | required | Crate name or path |
| `source` | enum | `crates-io` | `crates-io` / `git` / `path` |
| `source_url` | string | `""` | Git URL or local path (when source != crates-io) |
| `version` | string | `""` | Crate version (crates-io) or git ref |
| `target_path` | path | `/usr/local/bin/{{inputs.crate_name}}` | Where to install the binary |
| `opt_level` | enum | `3` | `3` (speed) / `s` (size) / `z` (min size) |
| `lto` | bool | `true` | Link-time optimization |
| `strip` | bool | `true` | Strip debug symbols |
| `panic` | enum | `unwind` | `unwind` / `abort` |
| `extra_rustflags` | string | `""` | Additional RUSTFLAGS |

**Resources** (4):
- `rust-toolchain` — package (cargo): ensure rustc/cargo present (bootstraps rustup if missing)
- `build-deps` — package (apt): build-essential, pkg-config, libssl-dev
- `binary` — file: `{{inputs.target_path}}`, `pre_apply` runs the cargo build:
  ```bash
  CARGO_PROFILE_RELEASE_OPT_LEVEL={{inputs.opt_level}} \
  CARGO_PROFILE_RELEASE_LTO={{inputs.lto}} \
  CARGO_PROFILE_RELEASE_STRIP={{inputs.strip}} \
  CARGO_PROFILE_RELEASE_PANIC={{inputs.panic}} \
  RUSTFLAGS="{{inputs.extra_rustflags}}" \
  cargo install --root /tmp/forjar-build --force '{{inputs.crate_name}}'
  cp /tmp/forjar-build/bin/{{inputs.crate_name}} {{inputs.target_path}}
  ```
- `binary-verify` — cron: health check `{{inputs.target_path}} --version` (optional, schedule: empty = disabled)

---

### 17. Static Binary Build (musl)

**Tier**: 1 + 2 + 3

Fully static Linux binary via `x86_64-unknown-linux-musl`. No libc, no libssl, no dynamic deps. Copy to any Linux machine and it runs. This is the Rust equivalent of Go's static binaries.

**Testable in CI**:
- Tier 1: validate, plan
- Tier 2: container apply — install musl toolchain, build, verify `file` reports "statically linked", verify `ldd` reports "not a dynamic executable"
- Tier 3: Intel runner — build + deploy + verify portability

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `crate_name` | string | required | Crate name |
| `source` | enum | `crates-io` | `crates-io` / `git` / `path` |
| `source_url` | string | `""` | Git URL or path |
| `version` | string | `""` | Version or git ref |
| `target_path` | path | `/usr/local/bin/{{inputs.crate_name}}` | Install destination |
| `upx` | bool | `false` | Compress with UPX after build |
| `opt_level` | enum | `z` | Default to size-optimized for static builds |
| `panic` | enum | `abort` | Default to abort for static builds (smaller) |

**Resources** (5):
- `musl-toolchain` — package (apt): `musl-tools`
- `rust-musl-target` — file: `pre_apply` runs `rustup target add x86_64-unknown-linux-musl`
- `upx-pkg` — package (apt): `upx` (when `inputs.upx=true`)
- `build-static` — file: `{{inputs.target_path}}`, `pre_apply` runs:
  ```bash
  CARGO_PROFILE_RELEASE_OPT_LEVEL={{inputs.opt_level}} \
  CARGO_PROFILE_RELEASE_LTO=true \
  CARGO_PROFILE_RELEASE_STRIP=true \
  CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
  CARGO_PROFILE_RELEASE_PANIC={{inputs.panic}} \
  cargo install --target x86_64-unknown-linux-musl \
    --root /tmp/forjar-build --force '{{inputs.crate_name}}'
  BIN=/tmp/forjar-build/bin/{{inputs.crate_name}}
  {{#if inputs.upx}}upx --best --lzma "$BIN"{{/if}}
  cp "$BIN" {{inputs.target_path}}
  ```
- `verify-static` — file: `post_apply` verifies static linking:
  ```bash
  file {{inputs.target_path}} | grep -q "statically linked"
  echo "BLAKE3: $(b3sum {{inputs.target_path}} | cut -d' ' -f1)"
  ls -lh {{inputs.target_path}} | awk '{print "SIZE:", $5}'
  ```

**Expected sizes** (forjar as reference):

| Config | Size | Notes |
|--------|------|-------|
| Dynamic release (current) | 12 MB | lto + strip |
| Static musl (opt-level=3) | ~14 MB | Slightly larger (libc inlined) |
| Static musl (opt-level=z) | ~11 MB | Size-optimized |
| Static musl + panic=abort + opt-z | ~10 MB | Minimal |
| Static musl + opt-z + UPX | ~3-4 MB | Compressed, slower startup |

---

### 18. Multi-Stage Build Pipeline

**Tier**: 1 + 3 (needs pepita overlayfs for true multi-stage)

The Nix-style build: compile in an ephemeral overlay (build deps install into overlay, not host), extract the binary, destroy the build environment. The host machine never sees build-essential, pkg-config, or 2GB of Cargo registry — only the final binary.

This is Docker multi-stage builds without Docker.

**Testable in CI**:
- Tier 1: validate, plan
- Tier 3: Intel runner — full multi-stage lifecycle

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `crate_name` | string | required | Crate name |
| `source` | enum | `crates-io` | `crates-io` / `git` / `path` |
| `source_url` | string | `""` | Git URL or path |
| `version` | string | `""` | Version or git ref |
| `target_path` | path | `/usr/local/bin/{{inputs.crate_name}}` | Final binary location on host |
| `static_musl` | bool | `true` | Build static musl binary |
| `upx` | bool | `false` | UPX compress |
| `build_memory_mb` | int | `8192` | Build sandbox memory limit |
| `build_cpuset` | string | `""` | Build sandbox CPU affinity |

**Two machines** — build sandbox (ephemeral) + deploy target:

```yaml
machines:
  # Stage 1: Build environment (ephemeral overlay)
  builder:
    hostname: build-{{inputs.crate_name}}
    addr: pepita
    transport: pepita
    pepita:
      rootfs: /
      memory_mb: "{{inputs.build_memory_mb}}"
      network: host                          # needs crates.io access
      filesystem: overlay
      ephemeral: true                        # destroy after build

  # Stage 2: Deploy target (real machine)
  target:
    hostname: "{{machine_hostname}}"
    addr: "{{machine_addr}}"
```

**Resources — Stage 1 (builder machine)** (4):
- `build-deps` — package (apt): build-essential, pkg-config, musl-tools, libssl-dev
- `rust-toolchain` — package (cargo): rustc + cargo (+ musl target if static)
- `compile` — file: `/tmp/artifact/{{inputs.crate_name}}`, `pre_apply` runs full build
- `checksum` — file: `/tmp/artifact/{{inputs.crate_name}}.b3`, BLAKE3 hash of binary

**Resources — Stage 2 (target machine)** (2):
- `deploy-binary` — file: `{{inputs.target_path}}`, source from builder artifact
- `verify-deploy` — file: `post_apply` verifies `{{inputs.target_path}} --version`

**After apply**: builder overlay is destroyed (ephemeral). Target has only the binary — no build-essential, no cargo registry, no intermediate artifacts. Clean machine.

**Multi-stage size comparison**:
```
Builder overlay (during build):  ~2-4 GB (rustc + cargo + deps + build artifacts)
Builder overlay (after apply):   0 B    (ephemeral: true → destroyed)
Target machine (after deploy):   ~12 MB (just the binary)
```

---

### 19. Cross-Compilation Pipeline

**Tier**: 1 + 3

Build for a different architecture than the build host. Compile on the Intel runner (x86_64) for aarch64 targets (Raspberry Pi, Jetson, Graviton). Uses Rust's cross-compilation with `cross` or native `rustup target add`.

**Testable in CI**:
- Tier 1: validate, plan, codegen review
- Tier 3: Intel runner — build for aarch64, verify with `file` command

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `crate_name` | string | required | Crate name |
| `source` | enum | `crates-io` | `crates-io` / `git` / `path` |
| `source_url` | string | `""` | Git URL or path |
| `version` | string | `""` | Version or git ref |
| `target_triple` | string | required | Rust target (e.g., `aarch64-unknown-linux-musl`) |
| `linker` | string | `""` | Cross-linker (e.g., `aarch64-linux-gnu-gcc`), empty = auto-detect |
| `target_path` | path | `/tmp/cross-build/{{inputs.crate_name}}` | Output path |
| `static_musl` | bool | `true` | Use musl for static binary |

**Resources** (5):
- `cross-toolchain` — package (apt): `gcc-aarch64-linux-gnu` (or appropriate cross-compiler)
- `musl-cross` — package (apt): musl-tools for cross target (when static)
- `rust-target` — file: `pre_apply` runs `rustup target add {{inputs.target_triple}}`
- `cargo-config` — file: `~/.cargo/config.toml` with `[target.{{inputs.target_triple}}]` linker setting
- `build-cross` — file: `{{inputs.target_path}}`, `pre_apply` runs:
  ```bash
  CARGO_TARGET_{{inputs.target_triple | upper | replace("-","_")}}_LINKER={{inputs.linker}} \
  cargo install --target {{inputs.target_triple}} \
    --root /tmp/cross-out --force '{{inputs.crate_name}}'
  cp /tmp/cross-out/bin/{{inputs.crate_name}} {{inputs.target_path}}
  file {{inputs.target_path}}  # verify: "ELF 64-bit ... aarch64"
  ```

**Useful target triples**:

| Target | Use Case |
|--------|----------|
| `x86_64-unknown-linux-musl` | Static x86_64 Linux (any distro) |
| `aarch64-unknown-linux-musl` | Static ARM64 Linux (Pi, Jetson, Graviton) |
| `aarch64-unknown-linux-gnu` | Dynamic ARM64 Linux |
| `x86_64-unknown-linux-gnu` | Dynamic x86_64 (default, current) |

---

### 20. Sovereign Stack Release Pipeline

**Tier**: 1 + 3

Build all sovereign AI stack binaries in a single config — forjar, pmat, batuta, pzsh. Multi-stage: build in ephemeral overlay, extract binaries, deploy to target, verify all tools work together.

This is the dogfood recipe: **forjar building and deploying itself and its siblings**.

**Testable in CI**:
- Tier 1: validate, plan
- Tier 3: Intel runner — full pipeline, end-to-end

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `install_dir` | path | `/usr/local/bin` | Binary install directory |
| `source_dir` | path | `/home/noah/src` | Root of source checkouts |
| `static_musl` | bool | `false` | Build static binaries |
| `upx` | bool | `false` | UPX compress |
| `parallel` | bool | `true` | Build crates in parallel |

**Resources** (10):
- `build-deps` — package (apt): build-essential, pkg-config, musl-tools
- `forjar-build` — file: `{{inputs.install_dir}}/forjar`, `pre_apply` builds from `{{inputs.source_dir}}/forjar`
- `pmat-build` — file: `{{inputs.install_dir}}/pmat`, `pre_apply` builds from `{{inputs.source_dir}}/pmat`
- `batuta-build` — file: `{{inputs.install_dir}}/batuta`, `pre_apply` builds from `{{inputs.source_dir}}/batuta`
- `pzsh-build` — file: `{{inputs.install_dir}}/pzsh`, `pre_apply` builds from `{{inputs.source_dir}}/pzsh`
- `verify-forjar` — cron/file: `post_apply` runs `forjar --version`
- `verify-pmat` — cron/file: `post_apply` runs `pmat --version`
- `verify-batuta` — cron/file: `post_apply` runs `batuta --version`
- `verify-pzsh` — cron/file: `post_apply` runs `pzsh --version`
- `release-manifest` — file: `/etc/forjar/release-manifest.json` with version, BLAKE3 hash, size, build timestamp for each binary

**Build order** (respects dependencies):
```
build-deps
  ├── forjar-build ──→ verify-forjar ─┐
  ├── pmat-build ────→ verify-pmat ───┤
  ├── batuta-build ──→ verify-batuta ──┼──→ release-manifest
  └── pzsh-build ───→ verify-pzsh ────┘
```

With `parallel: true`, all four builds run concurrently (forjar's multi-machine parallel execution). With `static_musl: true`, produces four fully portable binaries totaling ~60 MB (or ~15 MB with UPX).

---

### 21. Compiled APR Model Binary

**Tier**: 1 + 2 (CI with tiny model) + 3 (Intel runner with GPU inference)

The full APR model compilation pipeline: pull a model from HuggingFace, convert to APR's native compiled format, build a self-contained inference binary that bundles model weights + runtime, then serve it. The compiled binary is a single executable — no Python, no runtime deps, no model files to manage separately.

This is the "compile your model into a binary" pattern — like Go embedding assets, but for neural networks.

**Pipeline stages**:
```
HuggingFace (GGUF/safetensors)
  → apr pull (download + cache)
    → apr convert (to .apr format)
      → apr compile (model + runtime → single binary)
        → deploy binary
          → apr serve (or just run the compiled binary)
```

**Testable in CI**:
- Tier 1: validate config, plan, codegen review
- Tier 2: container apply with a tiny model — download [SmolLM2-135M](https://huggingface.co/QuantFactory/SmolLM2-135M-GGUF) (88 MB Q2_K) or [TinyLLama-v0](https://huggingface.co/mav23/TinyLLama-v0-GGUF) (4.4 MB Q2_K), run conversion + compilation pipeline, verify binary exists and responds to `--version`
- Tier 3: Intel runner with ROCm — full GPU-accelerated inference serving with real model

**CI model candidates** (small enough for GitHub Actions):

| Model | Params | Q2_K Size | Use Case |
|-------|--------|-----------|----------|
| [TinyLLama-v0](https://huggingface.co/mav23/TinyLLama-v0-GGUF) | 4.6M | 4.4 MB | Smoke test — barely a model, but proves the pipeline |
| [SmolLM2-135M](https://huggingface.co/QuantFactory/SmolLM2-135M-GGUF) | 135M | 88 MB | Real model — generates coherent text, fits in CI |
| [SmolLM2-135M-Instruct](https://huggingface.co/bartowski/SmolLM2-135M-Instruct-GGUF) | 135M | 88 MB | Instruction-tuned variant, better for chat/serve testing |

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `model_source` | string | required | HuggingFace repo ID (e.g., `QuantFactory/SmolLM2-135M-GGUF`) |
| `model_file` | string | `""` | Specific GGUF file from repo (e.g., `SmolLM2-135M.Q2_K.gguf`), empty = auto-detect |
| `model_format` | enum | `gguf` | Source format: `gguf` / `safetensors` |
| `quantization` | string | `q4_k_m` | Quantization level for conversion |
| `compile_target` | enum | `binary` | `binary` (self-contained) / `library` (shared .so) |
| `serve_port` | int | `8080` | Inference HTTP port |
| `gpu_backend` | enum | `cpu` | `cpu` / `nvidia` / `rocm` |
| `gpu_device` | int | `0` | GPU device index |
| `user` | string | `apr` | Service account |
| `install_dir` | path | `/opt/apr` | Base install directory |

**Resources** (12):

```yaml
resources:
  # ── Stage 1: Prerequisites ──
  apr-user:
    type: user
    name: "{{inputs.user}}"
    shell: /usr/sbin/nologin
    home: "{{inputs.install_dir}}"
    system_user: true

  apr-dirs:
    type: file
    state: directory
    path: "{{inputs.install_dir}}"
    owner: "{{inputs.user}}"
    mode: "0755"
    depends_on: [apr-user]

  apr-cli:
    type: package
    provider: cargo
    packages: [aprender]
    depends_on: []

  # ── Stage 2: Download from HuggingFace ──
  model-download:
    type: model
    name: "{{inputs.model_source}}"
    source: "{{inputs.model_source}}"
    format: "{{inputs.model_format}}"
    quantization: "{{inputs.quantization}}"
    path: "{{inputs.install_dir}}/models/source.{{inputs.model_format}}"
    cache_dir: "{{inputs.install_dir}}/cache"
    owner: "{{inputs.user}}"
    depends_on: [apr-dirs, apr-cli]

  # ── Stage 3: Convert to APR format ──
  model-convert:
    type: file
    path: "{{inputs.install_dir}}/models/model.apr"
    owner: "{{inputs.user}}"
    pre_apply: |
      apr convert \
        --input "{{inputs.install_dir}}/models/source.{{inputs.model_format}}" \
        --output "{{inputs.install_dir}}/models/model.apr" \
        --quantization "{{inputs.quantization}}" \
        --format apr
      echo "BLAKE3: $(b3sum {{inputs.install_dir}}/models/model.apr | cut -d' ' -f1)"
    depends_on: [model-download]

  # ── Stage 4: Compile model into binary ──
  model-compile:
    type: file
    path: "{{inputs.install_dir}}/bin/model-server"
    owner: "{{inputs.user}}"
    mode: "0755"
    pre_apply: |
      apr compile \
        --model "{{inputs.install_dir}}/models/model.apr" \
        --output "{{inputs.install_dir}}/bin/model-server" \
        --target "{{inputs.compile_target}}" \
        --backend "{{inputs.gpu_backend}}"
      ls -lh "{{inputs.install_dir}}/bin/model-server"
      file "{{inputs.install_dir}}/bin/model-server"
      echo "BLAKE3: $(b3sum {{inputs.install_dir}}/bin/model-server | cut -d' ' -f1)"
    depends_on: [model-convert]

  # ── Stage 5: GPU driver (conditional) ──
  gpu-driver:
    type: gpu
    name: gpu0
    gpu_backend: "{{inputs.gpu_backend}}"
    when: "{{inputs.gpu_backend}} != cpu"
    depends_on: []

  # ── Stage 6: Systemd service ──
  model-service-unit:
    type: file
    path: /etc/systemd/system/apr-model.service
    content: |
      [Unit]
      Description=APR Compiled Model Server
      After=network.target

      [Service]
      Type=simple
      User={{inputs.user}}
      ExecStart={{inputs.install_dir}}/bin/model-server \
        --port {{inputs.serve_port}} \
        --device {{inputs.gpu_device}}
      Restart=on-failure
      RestartSec=10
      Environment=APR_GPU_BACKEND={{inputs.gpu_backend}}

      [Install]
      WantedBy=multi-user.target
    owner: root
    mode: "0644"
    depends_on: [model-compile]

  model-service:
    type: service
    name: apr-model
    state: running
    enabled: true
    restart_on: [model-compile, model-service-unit]
    depends_on: [model-service-unit, gpu-driver]

  # ── Stage 7: Network + health ──
  firewall-model:
    type: network
    port: "{{inputs.serve_port}}"
    protocol: tcp
    action: allow
    depends_on: [model-service]

  health-check:
    type: cron
    name: apr-model-health
    schedule: "0 */6 * * *"
    command: "curl -sf http://localhost:{{inputs.serve_port}}/health || systemctl restart apr-model"
    user: root
    depends_on: [model-service]

  # ── Manifest ──
  build-manifest:
    type: file
    path: "{{inputs.install_dir}}/manifest.json"
    content: |
      {
        "model_source": "{{inputs.model_source}}",
        "model_format": "{{inputs.model_format}}",
        "quantization": "{{inputs.quantization}}",
        "compile_target": "{{inputs.compile_target}}",
        "gpu_backend": "{{inputs.gpu_backend}}",
        "binary_path": "{{inputs.install_dir}}/bin/model-server"
      }
    owner: "{{inputs.user}}"
    mode: "0644"
    depends_on: [model-compile]
```

**DAG execution order**:
```
apr-user → apr-dirs ─┐
apr-cli ─────────────┼→ model-download → model-convert → model-compile ─┬→ model-service-unit → model-service → firewall-model
gpu-driver ──────────┼──────────────────────────────────────────────────┘                                      → health-check
                     └→ build-manifest
```

**Container-testable variant** (Tier 2 — CI with tiny model):

```yaml
# cookbook-apr-compile-container.yaml
version: "1.0"
name: apr-compile-smoke-test

machines:
  target:
    hostname: target
    addr: localhost
    transport: container
    container:
      image: forjar-test-target
      name: forjar-apr-compile

resources:
  # Use TinyLLama-v0 (4.4 MB) for CI smoke test
  apr-compile-test:
    type: recipe
    machine: target
    recipe: recipes/apr-compile.yaml
    inputs:
      model_source: "mav23/TinyLLama-v0-GGUF"
      model_file: "TinyLLama-v0.Q2_K.gguf"
      model_format: gguf
      quantization: q2_k
      gpu_backend: cpu
      compile_target: binary
      serve_port: 8080
      user: root
```

**Intel runner variant** (Tier 3 — real GPU):

```yaml
# cookbook-apr-compile-rocm.yaml
version: "1.0"
name: apr-compile-rocm

machines:
  intel:
    hostname: intel
    addr: intel

resources:
  apr-compile-gpu:
    type: recipe
    machine: intel
    recipe: recipes/apr-compile.yaml
    inputs:
      model_source: "QuantFactory/SmolLM2-135M-GGUF"
      model_file: "SmolLM2-135M.Q4_K_M.gguf"
      model_format: gguf
      quantization: q4_k_m
      gpu_backend: rocm
      compile_target: binary
      serve_port: 8080
      user: noah
```

**Why this matters**: The compiled binary pattern eliminates the #1 operational headache with ML deployments — managing model files separately from the serving runtime. A single `scp model-server target:/usr/local/bin/` deploys everything. Drift detection watches the binary hash, not a sprawl of model shards. Rollback is `git revert` on the forjar config + re-apply.

---

### Rust Build Recipe Testability Summary

| # | Recipe | Tier | GitHub Actions | Intel Runner |
|---|--------|------|----------------|--------------|
| 16 | Rust Release Build | 1+2+3 | validate + container build | real optimized build |
| 17 | Static Binary (musl) | 1+2+3 | validate + container musl build | real static build + size verification |
| 18 | Multi-Stage Pipeline | 1+3 | validate + plan | ephemeral overlay build → deploy |
| 19 | Cross-Compilation | 1+3 | validate + plan | x86_64 → aarch64 cross build |
| 20 | Sovereign Stack Release | 1+3 | validate + plan | full stack build + deploy + verify |
| 21 | Compiled APR Model | 1+2+3 | validate + container build w/ 4.4 MB tiny model | ROCm GPU inference on Intel runner |

**Recipes 16, 17, and 21 are fully testable in GitHub Actions** (Tier 2) using the container transport. Recipe 21 uses [TinyLLama-v0](https://huggingface.co/mav23/TinyLLama-v0-GGUF) (4.4 MB Q2_K) for CI smoke testing — small enough to download and compile in seconds, but exercises the full pipeline.

---

## Secrets & TLS Lifecycle Recipes

### 22. Secrets Lifecycle (Age Encryption)

**Tier**: 1 + 2 (full container apply with encrypted secrets)

End-to-end secret management: key generation, encryption, deployment, rotation, and emergency revocation. Forjar already supports `ENC[age,...]` markers (FJ-200) and secret rotation (FJ-201) — this recipe formalizes the full lifecycle into a testable, repeatable pattern.

**Testable in CI**:
- Tier 1: validate, plan
- Tier 2: container apply — generate age keypair, encrypt test secrets, deploy config files with `ENC[age,...]` markers, verify decryption at apply time, rotate secrets, verify old secrets are replaced

**Lifecycle stages**:
```
keygen → encrypt → deploy → verify → rotate → revoke
  │         │         │         │        │        │
  │         │         │         │        │        └─ Emergency: re-encrypt all with new key,
  │         │         │         │        │           remove old identity, re-apply
  │         │         │         │        └─ Periodic: re-encrypt changed secrets,
  │         │         │         │           re-apply, verify no plaintext in state
  │         │         │         └─ forjar drift: detect if decrypted file was tampered
  │         │         └─ forjar apply: ENC[age,...] → plaintext at deploy time
  │         └─ forjar secrets encrypt "<value>" -r <public-key>
  └─ forjar secrets keygen > identity.txt
```

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `app_name` | string | required | Application name |
| `identity_path` | path | `~/.config/forjar/identity.txt` | Age identity (private key) file |
| `recipient` | string | required | Age public key for encryption |
| `secrets` | string | required | Comma-separated secret names (e.g., `api_key,db_password,jwt_secret`) |
| `config_dir` | path | `/etc/{{inputs.app_name}}` | Config file directory |
| `rotation_schedule` | string | `0 0 1 * *` | Cron schedule for rotation reminders (monthly default) |

**Resources** (7):
- `secrets-dir` — directory: `{{inputs.config_dir}}` (mode 0700)
- `identity-check` — file: `pre_apply` verifies identity file exists and is readable
- `app-config` — file: `{{inputs.config_dir}}/config.env` with `ENC[age,...]` markers for each secret, mode 0600
- `db-credentials` — file: `{{inputs.config_dir}}/db.env` with `ENC[age,...]` database credentials, mode 0600
- `secret-audit-log` — file: `{{inputs.config_dir}}/audit.log`, `post_apply` appends rotation timestamp
- `rotation-reminder` — cron: reminder to rotate secrets on schedule
- `plaintext-guard` — file: `post_apply` verifies no plaintext secrets appear in state lock file:
  ```bash
  # Verify: state lock must NOT contain decrypted secret values
  grep -r "sk-\|password\|secret" /tmp/state/*/state.lock.yaml && \
    { echo "FAIL: plaintext secret in state lock"; exit 1; } || true
  ```

**Container-testable variant**:
```yaml
# cookbook-secrets-container.yaml
version: "1.0"
name: cookbook-secrets-test

machines:
  target:
    hostname: target
    addr: container
    transport: container
    container:
      image: forjar-test-target
      name: forjar-cookbook-secrets

resources:
  secrets-lifecycle:
    type: recipe
    machine: target
    recipe: recipes/secrets-lifecycle.yaml
    inputs:
      app_name: test-app
      recipient: "age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p"
      secrets: "api_key,db_password"
```

**Security invariants** (verified in CI):
1. No plaintext secrets in `state.lock.yaml` — only BLAKE3 hashes of deployed files
2. Identity file never copied to target machine — decryption happens locally before deploy
3. `mode: 0600` on all secret-containing files
4. Drift detection fires if secret files are modified outside forjar

---

### 23. TLS Certificate Lifecycle

**Tier**: 1 + 2 (container apply for config/directory structure) + 3 (Intel runner for real ACME)

TLS certificate management: self-signed cert generation for dev/test, ACME/Let's Encrypt for production, automated renewal via cron, expiry monitoring, and emergency re-issuance.

**Testable in CI**:
- Tier 1: validate, plan
- Tier 2: container apply — generate self-signed cert via openssl, deploy to correct paths, verify permissions, test renewal script exists, verify nginx config references correct cert paths
- Tier 3: Intel runner — real ACME challenge against staging Let's Encrypt (if domain available)

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `domain` | string | required | Primary domain |
| `alt_domains` | string | `""` | Comma-separated SANs |
| `cert_mode` | enum | `self-signed` | `self-signed` / `acme` |
| `acme_email` | string | `""` | Let's Encrypt email (required for acme mode) |
| `acme_server` | enum | `staging` | `staging` / `production` |
| `cert_dir` | path | `/etc/ssl/forjar` | Certificate directory |
| `renewal_days` | int | `30` | Days before expiry to renew |
| `key_type` | enum | `ec` | `ec` (ECDSA P-256) / `rsa` (RSA 2048) |

**Resources** (8):
- `cert-dir` — directory: `{{inputs.cert_dir}}/{{inputs.domain}}` (mode 0755)
- `private-key-dir` — directory: `{{inputs.cert_dir}}/{{inputs.domain}}/private` (mode 0700)
- `certbot-pkg` — package (apt): certbot (when cert_mode=acme)
- `self-signed-cert` — file: `pre_apply` generates self-signed cert via openssl (when cert_mode=self-signed):
  ```bash
  openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 \
    -keyout {{inputs.cert_dir}}/{{inputs.domain}}/private/key.pem \
    -out {{inputs.cert_dir}}/{{inputs.domain}}/cert.pem \
    -days 365 -nodes -subj "/CN={{inputs.domain}}"
  ```
- `acme-cert` — file: `pre_apply` runs certbot (when cert_mode=acme):
  ```bash
  certbot certonly --standalone --non-interactive \
    --agree-tos --email {{inputs.acme_email}} \
    --server https://acme-{{inputs.acme_server}}-v02.api.letsencrypt.org/directory \
    -d {{inputs.domain}} {{inputs.alt_domains | split(",") | map("prepend", "-d ") | join(" ")}} \
    --cert-path {{inputs.cert_dir}}/{{inputs.domain}}/cert.pem \
    --key-path {{inputs.cert_dir}}/{{inputs.domain}}/private/key.pem
  ```
- `renewal-cron` — cron: `certbot renew --deploy-hook "systemctl reload nginx"` (daily check)
- `expiry-monitor` — cron: check cert expiry, alert if < `renewal_days`:
  ```bash
  EXPIRY=$(openssl x509 -enddate -noout -in {{inputs.cert_dir}}/{{inputs.domain}}/cert.pem | cut -d= -f2)
  DAYS_LEFT=$(( ( $(date -d "$EXPIRY" +%s) - $(date +%s) ) / 86400 ))
  [ "$DAYS_LEFT" -lt {{inputs.renewal_days}} ] && \
    echo "WARN: {{inputs.domain}} cert expires in $DAYS_LEFT days" | logger -t forjar-tls
  ```
- `cert-permissions` — file: `post_apply` ensures key is 0600, cert is 0644

**Drift detection**: forjar hashes the cert and key files. If someone manually replaces a cert (e.g., via certbot outside forjar), drift fires. This is intentional — forjar should be the single source of truth for cert state.

---

## Fleet-Scale Recipes

### 24. Fleet Provisioning (N-Machine Scale)

**Tier**: 1 + 2 (container with simulated fleet) + 3 (Intel runner, real multi-machine)

All previous recipes target 1-3 machines. This recipe demonstrates forjar's `for_each:` and `count:` features at fleet scale — provisioning 10+ machines from a single config with per-machine customization.

**Testable in CI**:
- Tier 1: validate config with 20 simulated machines, verify plan shows correct DAG
- Tier 2: container apply with `count: 5` to create 5 container "machines", deploy files + packages to each, verify per-machine state locks
- Tier 3: Intel runner — provision 3+ real machines via SSH

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `fleet_name` | string | required | Fleet name |
| `machine_count` | int | `5` | Number of machines to provision |
| `base_hostname` | string | `node` | Hostname prefix (node-0, node-1, ...) |
| `base_packages` | string | `curl,jq,htop,git` | Packages for every machine |
| `ssh_authorized_keys` | string | `""` | SSH public keys for deploy user |
| `monitoring_endpoint` | string | `""` | Prometheus push gateway URL |
| `tags` | string | `fleet` | Comma-separated resource tags |

**Pattern A: Homogeneous fleet** — same resources on every machine using `count:`:

```yaml
version: "1.0"
name: "fleet-{{inputs.fleet_name}}"

machines:
  # In real deployment, these would be SSH targets.
  # For CI testing, use container transport with count.
  fleet-node:
    hostname: "{{inputs.base_hostname}}"
    addr: container
    transport: container
    container:
      image: forjar-test-target
      name: "forjar-fleet-{{inputs.fleet_name}}"

resources:
  # Base packages on every node
  base-packages:
    type: package
    machine: fleet-node
    provider: apt
    packages: [curl, jq, htop, git]
    tags: [fleet, base]

  # Per-node identity file
  node-identity:
    type: file
    machine: fleet-node
    path: "/etc/forjar/node-identity.conf"
    content: |
      fleet={{inputs.fleet_name}}
      node_index={{index}}
      hostname={{inputs.base_hostname}}-{{index}}
    count: "{{inputs.machine_count}}"
    tags: [fleet, identity]

  # Per-node monitoring registration
  node-exporter-config:
    type: file
    machine: fleet-node
    path: "/etc/forjar/node-exporter-{{index}}.yaml"
    content: |
      scrape_configs:
        - job_name: "{{inputs.base_hostname}}-{{index}}"
          static_configs:
            - targets: ["localhost:9100"]
              labels:
                fleet: "{{inputs.fleet_name}}"
                node: "{{index}}"
    count: "{{inputs.machine_count}}"
    depends_on: [base-packages]
    tags: [fleet, monitoring]

  # Fleet manifest — single file listing all nodes
  fleet-manifest:
    type: file
    machine: fleet-node
    path: "/etc/forjar/fleet-manifest.json"
    content: |
      {
        "fleet": "{{inputs.fleet_name}}",
        "node_count": {{inputs.machine_count}},
        "base_packages": "{{inputs.base_packages}}",
        "provisioned_by": "forjar"
      }
    tags: [fleet, manifest]
```

**Pattern B: Heterogeneous fleet** — role-based provisioning using `for_each:`:

```yaml
resources:
  # Different vhost configs per service role
  vhost-configs:
    type: file
    machine: fleet-node
    path: "/etc/nginx/sites-available/{{item}}.conf"
    content: |
      server {
          server_name {{item}}.{{inputs.fleet_name}}.internal;
          location / { proxy_pass http://127.0.0.1:{{item | port_for_service}}; }
      }
    for_each: [api, web, worker, scheduler]
    tags: [fleet, nginx]

  # Per-environment firewall rules
  firewall-rules:
    type: network
    machine: fleet-node
    port: "{{item}}"
    protocol: tcp
    action: allow
    for_each: ["80", "443", "9090", "9100"]
    tags: [fleet, firewall]
```

**Fleet-scale verification**:
- `forjar plan` outputs correct DAG with `machine_count * resources_per_machine` total resources
- `forjar apply` converges all machines in parallel (default behavior)
- `forjar status` shows per-machine convergence state
- `forjar drift` detects per-machine drift independently
- State locks: one `state.lock.yaml` per machine under `state-dir/<machine>/`

**Scale targets**:

| Fleet Size | Plan Time | First Apply | Idempotent Apply | State Lock Size |
|-----------|-----------|-------------|------------------|-----------------|
| 5 machines | < 500ms | < 120s | < 5s | ~5 KB per machine |
| 20 machines | < 2s | < 300s | < 10s | ~5 KB per machine |
| 100 machines | < 10s | < 600s | < 30s | ~5 KB per machine |

---

## Package Build & Distribution Recipes

The missing link between "build" and "deploy at scale." Today, forjar installs packages via `apt` or `cargo install`. But production teams don't run `cargo install` on every machine — they build a `.deb` once, host it in a private repo, and `apt install` it across the fleet. These recipes formalize that pipeline.

### Current Package Provider Landscape

| Provider | Status | Distro Family | Check | Install | State Query |
|----------|--------|--------------|-------|---------|-------------|
| `apt` | Implemented (FJ-006) | Debian/Ubuntu | `dpkg -l` | `apt-get install` | `dpkg-query -W` |
| `cargo` | Implemented (FJ-006) | Any (Rust) | `command -v` | `cargo install` | `command -v` |
| `uv` | Implemented (FJ-006) | Any (Python) | `uv tool list` | `uv tool install` | `uv tool list` |
| `dnf` | Not implemented | RHEL/Fedora/Rocky | — | — | — |
| `apk` | Not implemented | Alpine | — | — | — |

### The Production Distribution Pipeline

```
Source code
  → Build (recipe #16-17: cargo build --release)
    → Package (.deb or .rpm with metadata, systemd units, conffiles)
      → Sign (GPG signature for authenticity)
        → Upload to private repo (apt repo served by nginx)
          → Fleet deploy (forjar apply with provider: apt, version pinned)
            → Drift detection (BLAKE3 hash of installed package)
```

Every stage is a forjar resource. Every stage is testable.

---

### 25. Third-Party APT Repository Management

**Tier**: 1 + 2 (fully container-testable)

Adding third-party apt repositories is the #1 thing every Ubuntu server needs after base install — Docker, PostgreSQL PGDG, NodeSource, GitHub CLI, HashiCorp, etc. Today this is manual shell scripting (GPG key download → keyring → sources.list.d → apt update). This recipe makes it declarative.

**Testable in CI**:
- Tier 2: container apply — add Docker repo to container, verify sources.list.d, verify keyring, `apt-get update` succeeds, install package from new repo

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `repo_name` | string | required | Repository identifier (e.g., `docker`, `nodesource`) |
| `gpg_key_url` | string | required | URL to GPG key (ASCII-armored or binary) |
| `repo_url` | string | required | APT repository URL |
| `repo_suite` | string | required | Distribution suite (e.g., `jammy`, `stable`) |
| `repo_component` | string | `main` | Repository component |
| `arch` | string | `amd64` | Architecture filter |
| `packages` | string | `""` | Comma-separated packages to install from this repo |
| `pin_priority` | int | `0` | APT pinning priority (0 = no pin, 500 = default, 1000 = force) |

**Resources** (5):
- `keyring-dir` — file: ensure `/etc/apt/keyrings` exists (mode 0755)
- `gpg-key` — file: `/etc/apt/keyrings/{{inputs.repo_name}}.gpg`, `pre_apply` downloads and dearmors GPG key:
  ```bash
  curl -fsSL '{{inputs.gpg_key_url}}' | gpg --dearmor -o /etc/apt/keyrings/{{inputs.repo_name}}.gpg
  chmod 644 /etc/apt/keyrings/{{inputs.repo_name}}.gpg
  ```
- `sources-list` — file: `/etc/apt/sources.list.d/{{inputs.repo_name}}.list`
  ```
  deb [arch={{inputs.arch}} signed-by=/etc/apt/keyrings/{{inputs.repo_name}}.gpg] {{inputs.repo_url}} {{inputs.repo_suite}} {{inputs.repo_component}}
  ```
- `apt-pin` — file: `/etc/apt/preferences.d/{{inputs.repo_name}}` (when pin_priority > 0):
  ```
  Package: *
  Pin: origin {{inputs.repo_url | hostname}}
  Pin-Priority: {{inputs.pin_priority}}
  ```
- `repo-packages` — package (apt): install packages from the new repo (depends_on: sources-list)

**Common repo presets** (can be parameterized):

| Repo | `gpg_key_url` | `repo_url` | Packages |
|------|---------------|------------|----------|
| Docker CE | `https://download.docker.com/linux/ubuntu/gpg` | `https://download.docker.com/linux/ubuntu` | docker-ce, docker-ce-cli, containerd.io |
| PostgreSQL PGDG | `https://www.postgresql.org/media/keys/ACCC4CF8.asc` | `http://apt.postgresql.org/pub/repos/apt` | postgresql-16 |
| NodeSource 20 | `https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key` | `https://deb.nodesource.com/node_20.x` | nodejs |
| GitHub CLI | `https://cli.github.com/packages/githubcli-archive-keyring.gpg` | `https://cli.github.com/packages` | gh |
| HashiCorp | `https://apt.releases.hashicorp.com/gpg` | `https://apt.releases.hashicorp.com` | terraform, vault, consul |
| ROCm 6.3 | `https://repo.radeon.com/rocm/rocm.gpg.key` | `https://repo.radeon.com/rocm/apt/6.3` | rocminfo, rocm-smi-lib |

**Container-testable variant**:
```yaml
# cookbook-apt-repo-container.yaml — add Docker repo in container
version: "1.0"
name: cookbook-apt-repo-test

machines:
  target:
    hostname: target
    addr: container
    transport: container
    container:
      image: forjar-test-target
      name: forjar-apt-repo

resources:
  docker-repo:
    type: recipe
    machine: target
    recipe: recipes/apt-repo.yaml
    inputs:
      repo_name: docker
      gpg_key_url: "https://download.docker.com/linux/ubuntu/gpg"
      repo_url: "https://download.docker.com/linux/ubuntu"
      repo_suite: jammy
      repo_component: stable
      packages: "docker-ce-cli"
```

---

### 26. Build .deb Package from Binary

**Tier**: 1 + 2 + 3

Take a compiled binary (from recipes #16-17) and package it as a proper `.deb` with systemd unit, conffiles, pre/post install scripts, and version metadata. The output is a `.deb` file that can be installed with `dpkg -i` or hosted in a private repo.

Uses `dpkg-deb` (available everywhere on Debian/Ubuntu) — no external tooling like fpm required.

**Testable in CI**:
- Tier 2: container apply — build a tiny Rust binary, package as .deb, install with `dpkg -i`, verify `dpkg -l` shows it installed, verify binary runs, verify systemd unit is correct, uninstall cleanly

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `pkg_name` | string | required | Package name (e.g., `forjar`) |
| `pkg_version` | string | required | Package version (e.g., `1.0.0`) |
| `pkg_description` | string | required | One-line description |
| `pkg_maintainer` | string | required | Maintainer email |
| `binary_path` | path | required | Path to compiled binary |
| `install_path` | path | `/usr/local/bin/{{inputs.pkg_name}}` | Where binary installs |
| `systemd_unit` | string | `""` | Systemd unit file content (empty = no service) |
| `conffiles` | string | `""` | Comma-separated config file paths (preserved on upgrade) |
| `depends` | string | `""` | Comma-separated .deb dependencies |
| `output_dir` | path | `/tmp/deb-output` | Where to write the .deb file |

**Resources** (7):
- `staging-dir` — file: create staging tree `/tmp/deb-build/{{inputs.pkg_name}}`
- `control-file` — file: `DEBIAN/control`
  ```
  Package: {{inputs.pkg_name}}
  Version: {{inputs.pkg_version}}
  Architecture: amd64
  Maintainer: {{inputs.pkg_maintainer}}
  Description: {{inputs.pkg_description}}
  Depends: {{inputs.depends}}
  ```
- `conffiles-file` — file: `DEBIAN/conffiles` (when conffiles is non-empty)
- `binary-stage` — file: copy binary into staging tree at correct install path
- `systemd-stage` — file: `lib/systemd/system/{{inputs.pkg_name}}.service` (when systemd_unit non-empty)
- `build-deb` — file: `{{inputs.output_dir}}/{{inputs.pkg_name}}_{{inputs.pkg_version}}_amd64.deb`, `pre_apply`:
  ```bash
  dpkg-deb --build /tmp/deb-build/{{inputs.pkg_name}} \
    {{inputs.output_dir}}/{{inputs.pkg_name}}_{{inputs.pkg_version}}_amd64.deb
  echo "BLAKE3: $(b3sum {{inputs.output_dir}}/*.deb | cut -d' ' -f1)"
  dpkg-deb --info {{inputs.output_dir}}/*.deb
  ```
- `verify-install` — package (apt): `post_apply` installs the .deb and verifies:
  ```bash
  dpkg -i {{inputs.output_dir}}/{{inputs.pkg_name}}_{{inputs.pkg_version}}_amd64.deb
  dpkg -l {{inputs.pkg_name}}
  {{inputs.install_path}} --version
  ```

**Output**: a `.deb` file ready for `dpkg -i`, private repo upload, or S3 distribution.

---

### 27. Private APT Repository

**Tier**: 1 + 2 + 3

Host your own apt repository — serve `.deb` files via nginx with GPG signing. Fleet machines point at this repo and install internal packages with standard `apt-get install`. No Artifactory, no Nexus, no SaaS — just nginx + `dpkg-scanpackages` + GPG.

**Testable in CI**:
- Tier 2: container apply — create repo structure, add a test .deb, generate Packages index, sign with GPG, configure nginx, verify `apt-get update` from the repo works
- Tier 3: Intel runner — serve real .debs, install on same or remote machine

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `repo_name` | string | required | Repository name |
| `repo_dir` | path | `/var/lib/apt-repo/{{inputs.repo_name}}` | Package storage directory |
| `gpg_key_email` | string | required | GPG key identity for signing |
| `serve_port` | int | `8090` | Nginx listen port |
| `architectures` | string | `amd64` | Comma-separated architectures |
| `codename` | string | `jammy` | Distribution codename |
| `component` | string | `main` | Repository component |

**Resources** (9):
- `repo-deps` — package (apt): dpkg-dev, gpg, nginx
- `repo-dir` — directory: `{{inputs.repo_dir}}/pool/{{inputs.component}}`
- `repo-conf-dir` — directory: `{{inputs.repo_dir}}/dists/{{inputs.codename}}/{{inputs.component}}/binary-{{inputs.architectures}}`
- `gpg-key` — file: `pre_apply` generates GPG key if not exists:
  ```bash
  gpg --list-keys '{{inputs.gpg_key_email}}' 2>/dev/null || \
    gpg --batch --gen-key <<GPGEOF
      Key-Type: RSA
      Key-Length: 4096
      Name-Email: {{inputs.gpg_key_email}}
      Expire-Date: 0
      %no-protection
  GPGEOF
  gpg --export --armor '{{inputs.gpg_key_email}}' > {{inputs.repo_dir}}/repo.gpg.key
  ```
- `update-script` — file: `/usr/local/bin/apt-repo-update-{{inputs.repo_name}}`, executable script:
  ```bash
  #!/bin/bash
  set -euo pipefail
  cd {{inputs.repo_dir}}
  dpkg-scanpackages pool/ > dists/{{inputs.codename}}/{{inputs.component}}/binary-{{inputs.architectures}}/Packages
  gzip -kf dists/{{inputs.codename}}/{{inputs.component}}/binary-{{inputs.architectures}}/Packages
  apt-ftparchive release dists/{{inputs.codename}} > dists/{{inputs.codename}}/Release
  gpg --batch --yes --armor --detach-sign -o dists/{{inputs.codename}}/Release.gpg dists/{{inputs.codename}}/Release
  gpg --batch --yes --clearsign -o dists/{{inputs.codename}}/InRelease dists/{{inputs.codename}}/Release
  echo "Repo updated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  ```
- `nginx-site` — file: `/etc/nginx/sites-available/apt-repo-{{inputs.repo_name}}`
  ```nginx
  server {
      listen {{inputs.serve_port}};
      root {{inputs.repo_dir}};
      autoindex on;
      location / {
          allow all;
      }
  }
  ```
- `nginx-enable` — file: symlink `sites-enabled` → `sites-available`
- `nginx-service` — service: nginx (running, enabled, restart_on: nginx-site)
- `firewall` — network: allow {{inputs.serve_port}}/tcp

**Usage pipeline** (combines recipes #17, #26, #27):
```yaml
resources:
  # Step 1: Build static binary
  build:
    type: recipe
    machine: build-box
    recipe: recipes/rust-static-build.yaml
    inputs:
      crate_name: forjar
      source: path
      source_url: /home/noah/src/forjar

  # Step 2: Package as .deb
  package:
    type: recipe
    machine: build-box
    recipe: recipes/deb-build.yaml
    inputs:
      pkg_name: forjar
      pkg_version: "1.0.0"
      pkg_description: "Declarative infrastructure convergence"
      pkg_maintainer: "noah@paiml.com"
      binary_path: /usr/local/bin/forjar
      systemd_unit: ""
    depends_on: [build]

  # Step 3: Upload to private repo
  upload:
    type: file
    machine: repo-server
    path: /var/lib/apt-repo/internal/pool/main/forjar_1.0.0_amd64.deb
    source: /tmp/deb-output/forjar_1.0.0_amd64.deb
    post_apply: /usr/local/bin/apt-repo-update-internal
    depends_on: [package]

  # Step 4: Install from repo on fleet
  deploy:
    type: package
    machine: [node-1, node-2, node-3]
    provider: apt
    packages: [forjar]
    version: "1.0.0"
    depends_on: [upload]
```

---

### 28. RPM Package Provider & Build

**Tier**: 1 + 2 (with Fedora/Rocky container) + 3

Extends forjar to RHEL/Fedora/Rocky/Alma Linux via `provider: dnf`. Also provides an RPM build recipe parallel to the .deb recipe (#26).

**Code change required**: Add `provider: dnf` to `src/resources/package.rs`:

```rust
"dnf" => {
    let checks: Vec<String> = packages.iter()
        .map(|p| format!(
            "rpm -q '{}' >/dev/null 2>&1 && echo 'installed:{}' || echo 'missing:{}'",
            p, p, p
        ))
        .collect();
    checks.join("\n")
}
```

**New test container** for CI: `tests/Dockerfile.test-target-rpm`
```dockerfile
FROM rockylinux:9
RUN dnf install -y bash coreutils sudo curl ca-certificates \
    rpm-build gcc make && dnf clean all
CMD ["sleep", "infinity"]
```

**Recipe inputs** (RPM build):
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `pkg_name` | string | required | Package name |
| `pkg_version` | string | required | Version |
| `pkg_release` | string | `1` | Release number |
| `pkg_summary` | string | required | Summary line |
| `pkg_license` | string | `MIT` | License |
| `binary_path` | path | required | Compiled binary |
| `install_path` | path | `/usr/local/bin/{{inputs.pkg_name}}` | Install destination |
| `systemd_unit` | string | `""` | Systemd unit content |
| `output_dir` | path | `/tmp/rpm-output` | Output directory |

**Resources** (6):
- `rpm-deps` — package (dnf): rpm-build, rpmdevtools
- `rpmbuild-tree` — file: `pre_apply` runs `rpmdev-setuptree`
- `spec-file` — file: `~/rpmbuild/SPECS/{{inputs.pkg_name}}.spec`:
  ```spec
  Name:    {{inputs.pkg_name}}
  Version: {{inputs.pkg_version}}
  Release: {{inputs.pkg_release}}%{?dist}
  Summary: {{inputs.pkg_summary}}
  License: {{inputs.pkg_license}}

  %description
  {{inputs.pkg_summary}}

  %install
  mkdir -p %{buildroot}{{inputs.install_path | dirname}}
  cp {{inputs.binary_path}} %{buildroot}{{inputs.install_path}}

  %files
  {{inputs.install_path}}
  ```
- `build-rpm` — file: `pre_apply` runs `rpmbuild -bb ~/rpmbuild/SPECS/{{inputs.pkg_name}}.spec`
- `copy-output` — file: copy built RPM to `{{inputs.output_dir}}`
- `verify-rpm` — file: `post_apply` runs `rpm -qip {{inputs.output_dir}}/*.rpm`

**Cross-format build** — combine recipes #26 and #28 to produce both `.deb` and `.rpm` from the same binary:

```yaml
resources:
  build-binary:
    type: recipe
    machine: build-box
    recipe: recipes/rust-static-build.yaml
    inputs: { crate_name: forjar, static_musl: true }

  package-deb:
    type: recipe
    machine: build-box
    recipe: recipes/deb-build.yaml
    inputs: { pkg_name: forjar, pkg_version: "1.0.0", binary_path: /usr/local/bin/forjar }
    depends_on: [build-binary]

  package-rpm:
    type: recipe
    machine: build-box-rpm       # Rocky Linux container
    recipe: recipes/rpm-build.yaml
    inputs: { pkg_name: forjar, pkg_version: "1.0.0", binary_path: /usr/local/bin/forjar }
    depends_on: [build-binary]
```

---

### 29. Package Distribution Pipeline (Build → Sign → Repo → Fleet)

**Tier**: 1 + 2 + 3

The complete end-to-end pipeline: build a Rust binary, package as `.deb`, sign, upload to private apt repo, deploy to fleet via `apt install`, verify via drift detection. This is the recipe that connects all the pieces.

**Testable in CI**:
- Tier 2: container apply — full pipeline in a single container (build a tiny Rust crate → .deb → local repo → apt install → verify)
- Tier 3: Intel runner — build forjar itself, .deb, serve from private repo, install on same machine

**Recipe inputs**:
| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `crate_name` | string | required | Rust crate to build |
| `crate_source` | enum | `path` | `path` / `crates-io` / `git` |
| `crate_path` | string | `""` | Source path or URL |
| `pkg_version` | string | required | Package version |
| `pkg_maintainer` | string | required | Maintainer email |
| `static_musl` | bool | `true` | Build static for portability |
| `repo_server` | string | required | Machine name hosting apt repo |
| `repo_port` | int | `8090` | Repo nginx port |
| `fleet_machines` | string | required | Comma-separated target machine names |

**DAG**:
```
rust-static-build (#17)
  → deb-build (#26)
    → sign (.deb GPG signature)
      → upload to private-apt-repo (#27)
        → apt-repo-update (regenerate Packages index)
          → fleet-deploy (provider: apt, version pinned)
            → verify (dpkg -l + binary --version on each machine)
              → drift-baseline (BLAKE3 hash of installed binary)
```

**This is forjar's answer to**: "How do you go from `git push` to running on 50 machines?" One YAML file, one `forjar apply`.

---

### Package Recipe Testability Summary

| # | Recipe | Tier | GitHub Actions | Intel Runner |
|---|--------|------|----------------|--------------|
| 25 | Third-Party APT Repo | 1+2 | validate + container apply (add Docker repo, install docker-ce-cli) | — |
| 26 | Build .deb Package | 1+2+3 | validate + container (build tiny binary → .deb → dpkg -i) | build forjar .deb |
| 27 | Private APT Repository | 1+2+3 | validate + container (repo + nginx + apt-get from local) | serve real .debs |
| 28 | RPM Provider & Build | 1+2 | validate + Rocky Linux container (dnf + rpmbuild) | — |
| 29 | Distribution Pipeline | 1+2+3 | validate + container (full pipeline end-to-end) | build forjar → .deb → repo → install |

**All five recipes are at least Tier 2 testable.** Recipe 28 requires a Rocky Linux container image (`tests/Dockerfile.test-target-rpm`) but no special hardware. Recipe 29 is the integration test that proves all the pieces compose correctly.

---

## Failure Mode Testing

Production infrastructure fails. World-class tools handle partial failures gracefully — resume where they left off, don't corrupt state, and provide clear diagnostics. This section defines failure scenarios that every recipe must survive.

### Failure Categories

| Category | Example | Expected Behavior |
|----------|---------|-------------------|
| **Transient** | apt repo temporarily unreachable | Retry-safe: re-run `forjar apply`, converges without duplication |
| **Partial apply** | 3 of 5 resources succeed, resource 4 fails | State lock records 3 successes; re-apply starts from resource 4 |
| **State corruption** | `state.lock.yaml` deleted or truncated | Full re-convergence from scratch (treats all resources as missing) |
| **Transport failure** | SSH connection drops mid-apply | No half-written files; atomic script execution via pipe-to-bash |
| **Dependency failure** | Package install succeeds but service won't start | Service resource reports failure; dependent resources skipped |
| **Resource conflict** | Two resources write to same file path | Detected at validate time (plan-time conflict detection) |
| **Disk full** | Model download fills disk | Script `set -euo pipefail` catches write failure; partial file cleaned up |

### Failure Test Recipes

These are intentionally broken configs that verify forjar's error handling:

#### `cookbook-failure-partial-apply.yaml` (Tier 2)

```yaml
# Intentional failure: resource 3 of 4 fails, verify state is consistent
version: "1.0"
name: failure-partial-apply

machines:
  target:
    hostname: target
    addr: container
    transport: container
    container:
      image: forjar-test-target
      name: forjar-failure-test

resources:
  # Resource 1: succeeds
  good-file-1:
    type: file
    machine: target
    path: /tmp/good1.txt
    content: "success"

  # Resource 2: succeeds
  good-file-2:
    type: file
    machine: target
    path: /tmp/good2.txt
    content: "success"
    depends_on: [good-file-1]

  # Resource 3: FAILS — installs nonexistent package
  bad-package:
    type: package
    machine: target
    provider: apt
    packages: [this-package-does-not-exist-forjar-test]
    depends_on: [good-file-2]

  # Resource 4: should be SKIPPED (depends on failed resource)
  post-failure-file:
    type: file
    machine: target
    path: /tmp/should-not-exist.txt
    content: "should never be written"
    depends_on: [bad-package]
```

**Verification**:
```bash
# Apply — should fail on resource 3
forjar apply -f cookbook-failure-partial-apply.yaml --state-dir /tmp/fail --yes || true

# Verify: resources 1 and 2 succeeded
[ -f /tmp/fail/target/state.lock.yaml ] || { echo "FAIL: no state lock"; exit 1; }
grep -q "good-file-1" /tmp/fail/target/state.lock.yaml
grep -q "good-file-2" /tmp/fail/target/state.lock.yaml

# Verify: resource 4 was NOT applied (dependency on failed resource)
docker exec forjar-failure-test test ! -f /tmp/should-not-exist.txt

# Recovery: fix the config (remove bad-package), re-apply
# Should converge resource 4 without re-applying 1 and 2
```

#### `cookbook-failure-state-recovery.yaml` (Tier 2)

```yaml
# Test: delete state lock mid-lifecycle, verify full re-convergence
version: "1.0"
name: failure-state-recovery

machines:
  target:
    hostname: target
    addr: container
    transport: container
    container:
      image: forjar-test-target
      name: forjar-state-recovery

resources:
  file-a:
    type: file
    machine: target
    path: /tmp/recovery-a.txt
    content: "file-a-content"

  file-b:
    type: file
    machine: target
    path: /tmp/recovery-b.txt
    content: "file-b-content"
    depends_on: [file-a]
```

**Verification**:
```bash
# Apply successfully
forjar apply -f cookbook-failure-state-recovery.yaml --state-dir /tmp/sr --yes

# Corrupt state — delete lock file
rm /tmp/sr/target/state.lock.yaml

# Re-apply — should fully re-converge (not error out)
forjar apply -f cookbook-failure-state-recovery.yaml --state-dir /tmp/sr --yes

# Verify: files still correct, new state lock created
[ -f /tmp/sr/target/state.lock.yaml ]
```

#### `cookbook-failure-idempotent-after-crash.yaml` (Tier 2)

Tests that a recipe converges cleanly even if a previous apply was interrupted. Simulates a crash by applying a config that creates a file, then applying an updated config that changes the file content. Verifies no stale state leaks.

### Failure Test CI Job

```yaml
cookbook-failure:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - uses: Swatinem/rust-cache@v2
    - run: docker build -t forjar-test-target -f tests/Dockerfile.test-target .
    - name: Partial apply recovery
      run: |
        cargo run -- apply -f examples/cookbook-failure-partial-apply.yaml \
          --state-dir /tmp/fail --yes 2>&1 || true
        # Verify partial state is consistent
        test -f /tmp/fail/target/state.lock.yaml
    - name: State lock recovery
      run: |
        cargo run -- apply -f examples/cookbook-failure-state-recovery.yaml \
          --state-dir /tmp/sr --yes
        rm /tmp/sr/target/state.lock.yaml
        cargo run -- apply -f examples/cookbook-failure-state-recovery.yaml \
          --state-dir /tmp/sr --yes
        test -f /tmp/sr/target/state.lock.yaml
```

---

## Composability Matrix

Which recipes layer on top of which, which combos are tested, and which conflict.

### Composition Rules

1. **Base layer first** — Secure Baseline (#9) or Developer Workstation (#1) should be the first recipe in any stack
2. **GPU is additive** — ROCm (#7) or NVIDIA (#8) adds to any base, never conflicts
3. **Monitoring is always-on** — Monitoring Stack (#4) layers on top of anything
4. **Secrets are orthogonal** — Secrets Lifecycle (#22) applies to any recipe that deploys config files
5. **TLS wraps services** — TLS Lifecycle (#23) wraps Web Server (#2) or any service with an HTTP port
6. **Nix-style is exclusive** — Dev Shell (#11) / Build Sandbox (#13) use pepita transport, which replaces the machine's transport. Don't combine with container-transport recipes on the same machine.

### Tested Stacks

Compositions that are validated end-to-end (config exists + CI test):

| # | Stack Name | Recipes | Machines | Total Resources | Tier | Grade |
|---|-----------|---------|----------|-----------------|------|-------|
| 53 | **Dev Server** | 1 + 9 + 12 | 1 | ~20 | 1+2 | A (94) |
| 54 | **Web Production** | 2 + 3 + 4 + 9 + 22 + 23 | 2-3 | ~40 | 1+2 | A (94) |
| 55 | **GPU Lab** | 1 + 7 + 4 + 9 | 1 | ~25 | 1+2 | A (94) |
| 56 | **Build Farm** | 13 + 17 + 19 | 1-3 | ~15 | 1+2 | A (94) |
| 57 | **Package Pipeline** | 17 + 26 + 27 + 25 + 24 | 2-5 | ~30 | 1+2 | A (94) |
| 58 | **ML Inference** | 7 + 21 + 4 + 9 + 22 | 1-2 | ~35 | 1+2 | A (93) |
| 59 | **CI Infrastructure** | 6 + 9 + 4 | 1 | ~25 | 1+2 | A (94) |
| 60 | **Sovereign AI** | 1 + 7 + 21 + 4 + 9 + 22 | 3 | ~60 | 1+2 | A (94) |
| 61 | **Fleet Baseline** | 9 + 24 + 4 | 5-20 | 5-20 per machine | 1+2 | A (94) |
| 62 | **Cross-Distro Release** | 17 + 26 + 28 + 29 | 2-3 | ~20 | 1+2 | A (94) |

### Conflict Matrix

Combinations that are invalid or require special handling:

| Recipe A | Recipe B | Conflict | Resolution |
|----------|----------|----------|------------|
| Dev Shell (#11) | Container transport recipes | Transport collision — pepita vs container | Use separate machines |
| NVIDIA GPU (#8) | ROCm GPU (#7) | Same machine, different backends | Use `when:` conditions or separate machines |
| NFS Server (#10) | NFS Client (#10) | Same machine can't be both | Use separate machines |
| Build Sandbox (#13) | Any non-ephemeral recipe | Sandbox is ephemeral — state lost after apply | Extract artifacts before sandbox destruction |
| Self-signed TLS (#23) | ACME TLS (#23) | Same cert_dir, different cert sources | Use different `cert_mode` per domain, not per machine |

### Composition Pattern

System Profile (#14) is the canonical composition pattern:

```yaml
resources:
  # Layer 1: Security baseline (always)
  security:
    type: recipe
    recipe: recipes/secure-baseline.yaml
    machine: target

  # Layer 2: Developer tools (always)
  devtools:
    type: recipe
    recipe: recipes/devbox.yaml
    machine: target
    depends_on: [security]

  # Layer 3: GPU (conditional)
  gpu:
    type: recipe
    recipe: recipes/rocm-gpu.yaml
    machine: target
    when: "{{inputs.gpu_backend}} == rocm"
    depends_on: [devtools]

  # Layer 4: Application (conditional)
  model-server:
    type: recipe
    recipe: recipes/apr-compile.yaml
    machine: target
    when: "{{inputs.role}} == inference"
    depends_on: [gpu]

  # Layer 5: Monitoring (always)
  monitoring:
    type: recipe
    recipe: recipes/monitoring.yaml
    machine: target
    depends_on: [devtools]

  # Layer 6: Secrets (always)
  secrets:
    type: recipe
    recipe: recipes/secrets-lifecycle.yaml
    machine: target
    depends_on: [security]
```

Layers execute in DAG order. Each layer's resources are fully independent — no resource in layer 3 can depend on a specific resource inside layer 1 (only on the layer-1 recipe as a whole). This is the forjar equivalent of NixOS module composition.

---

## Container Transport Config Pattern

For Tier 2 recipes, the container-testable variant replaces the SSH machine with a container:

```yaml
# cookbook-webserver-container.yaml
version: "1.0"
name: cookbook-webserver-container-test

machines:
  target:
    hostname: target
    addr: localhost
    transport: container
    container:
      image: forjar-test-target
      name: forjar-cookbook-webserver

resources:
  # Same resources as the recipe, but machine: target
  nginx-pkg:
    type: package
    machine: target
    provider: apt
    packages: [nginx]
  # ...
```

This pattern lets every Tier 1 recipe gain a Tier 2 container-testable companion that exercises real shell execution in CI without any hardware dependencies.

---

## OpenTofu-Inspired Patterns

Patterns borrowed from OpenTofu/Terraform, adapted to forjar's sovereign (no cloud backend), bare-metal-first model. Where OpenTofu uses HCL and cloud APIs, forjar uses YAML and SSH/shell.

### What Forjar Already Has

| OpenTofu Feature | Forjar Equivalent | Status |
|-----------------|------------------|--------|
| Workspaces | `forjar workspace new/list/select/delete`, `-w` flag (FJ-210) | Done |
| `terraform state mv` | `forjar state-mv <old> <new>` (FJ-212) | Done |
| `terraform state rm` | `forjar state-rm <id>` (FJ-213) | Done |
| `terraform import` | `forjar import --scan-types` (FJ-065, FJ-084) | Done |
| Data sources | `data:` block — file, command, dns types (FJ-223) | Done |
| Provisioners | Native — forjar IS the provisioner (codegen + transport) | N/A |
| Variable validation | Recipe input typing + validation | Done |
| Sensitive values | `ENC[age,...]` markers + age encryption (FJ-200) | Done |
| `terraform plan` | `forjar plan` — DAG-ordered diff | Done |
| Drift detection | `forjar drift` — BLAKE3 content hash | Done |
| Graph visualization | `forjar graph` — Mermaid + DOT output | Done |
| Outputs | `forjar output` — export values from a config | Done |

### What Forjar Should Adopt

The patterns below represent the highest-value OpenTofu features that forjar doesn't yet have. Each is specified as a recipe or capability.

---

### 30. Saved Plan Files (TOCTOU Safety)

**OpenTofu equivalent**: `tofu plan -out=plan.bin && tofu apply plan.bin`

**The problem**: Currently `forjar apply` re-plans internally. In CI/CD, conditions may change between the plan shown in PR review and the actual apply. This is a TOCTOU (time-of-check to time-of-use) vulnerability.

**Tier**: 1 (validate) + 2 (container apply)

**Design**:

```bash
# Save plan to file (CI: PR review step)
forjar plan -f config.yaml --state-dir ./state --out plan.json

# Apply exact saved plan (CI: merge step)
forjar apply --plan plan.json --state-dir ./state --yes

# Human-readable plan review
forjar show plan.json

# Machine-readable plan for policy engines
forjar show --json plan.json
```

**Plan file format** (YAML, git-friendly):

```yaml
# plan.json — forjar saved plan
version: "1.0"
config_hash: "b3:abc123..."     # BLAKE3 of source config
state_hash: "b3:def456..."      # BLAKE3 of state at plan time
created_at: "2026-03-01T12:00:00Z"
workspace: "default"
actions:
  - resource: nginx-pkg
    machine: web
    action: create
    details:
      type: package
      provider: apt
      packages: [nginx]
  - resource: nginx-conf
    machine: web
    action: create
    depends_on: [nginx-pkg]
    details:
      type: file
      dest: /etc/nginx/sites-available/default
      content_hash: "b3:789abc..."
```

**Safety checks on `apply --plan`**:

1. Config hash must match current config (fail if config was edited after plan)
2. State hash must match current state (fail if another apply ran between plan and apply)
3. Plan file signature verified (optional: age-signed)

**Idempotency**: Strong — saved plan is a static artifact

**Convergence budget**: < 1s for plan save, same as regular apply for apply

**Test config** (`dogfood-saved-plan.yaml`):

```yaml
version: "1.0"
name: saved-plan-test
machines:
  target:
    hostname: target
    addr: localhost
    transport: container
    container:
      image: forjar-test-target
      name: forjar-saved-plan
resources:
  test-file:
    type: file
    machine: target
    dest: /tmp/plan-test.txt
    content: "planned content"
```

**CI verification**:

```bash
# Plan → save → apply saved → verify
forjar plan -f dogfood-saved-plan.yaml --state-dir /tmp/sp --out /tmp/plan.json
forjar apply --plan /tmp/plan.json --state-dir /tmp/sp --yes
# Tamper config and verify plan rejection
echo "  extra: field" >> dogfood-saved-plan.yaml
forjar apply --plan /tmp/plan.json --state-dir /tmp/sp --yes 2>&1 | grep -q "config changed since plan"
```

---

### 31. JSON Plan Format (Machine-Readable Plans)

**OpenTofu equivalent**: `tofu show -json plan.bin | jq .`

**The problem**: `forjar plan` output is human-readable only. Policy engines (OPA/Rego), cost estimators, PR comment bots, and audit systems need structured data.

**Tier**: 1 (validate JSON schema)

**Design**:

```bash
# JSON plan to stdout
forjar plan -f config.yaml --state-dir ./state --json

# JSON plan to file
forjar plan -f config.yaml --state-dir ./state --json --out plan.json

# Pipe to policy check
forjar plan -f config.yaml --state-dir ./state --json | opa eval -d policy.rego 'data.forjar.allow'
```

**JSON plan schema**:

```json
{
  "format_version": "1.0",
  "forjar_version": "0.9.0",
  "config": {
    "name": "web-stack",
    "hash": "b3:abc...",
    "resource_count": 5,
    "machine_count": 2
  },
  "state": {
    "hash": "b3:def...",
    "workspace": "production"
  },
  "changes": [
    {
      "resource": "nginx-pkg",
      "machine": "web",
      "action": "create",
      "type": "package",
      "provider": "apt",
      "before": null,
      "after": {
        "packages": ["nginx"],
        "version": null
      }
    },
    {
      "resource": "nginx-conf",
      "machine": "web",
      "action": "update",
      "type": "file",
      "before": {
        "content_hash": "b3:old..."
      },
      "after": {
        "content_hash": "b3:new...",
        "dest": "/etc/nginx/sites-available/default"
      }
    }
  ],
  "summary": {
    "create": 3,
    "update": 1,
    "destroy": 0,
    "unchanged": 1,
    "total": 5
  }
}
```

**OPA policy example** (`policy.rego`):

```rego
package forjar

default allow = true

# Block destroy actions in production
deny[msg] {
  input.state.workspace == "production"
  change := input.changes[_]
  change.action == "destroy"
  msg := sprintf("cannot destroy %s in production", [change.resource])
}

# Require version pinning for packages
deny[msg] {
  change := input.changes[_]
  change.type == "package"
  change.after.version == null
  msg := sprintf("package %s must have pinned version", [change.resource])
}

allow = false { count(deny) > 0 }
```

---

### 32. Check Blocks (Post-Apply Health Assertions)

**OpenTofu equivalent**: `check "name" { data { ... } assert { ... } }`

**The problem**: Forjar verifies that resources converged to desired state (file exists, package installed), but doesn't verify functional outcomes (the app responds, the port is open, the cert is valid). These are different questions: "is nginx installed?" vs "does the website return 200?"

**Tier**: 2 (container transport)

**Design**:

```yaml
version: "1.0"
name: web-with-checks
machines:
  web:
    hostname: web
    addr: web-server
resources:
  nginx-pkg:
    type: package
    machine: web
    packages: [nginx]
  nginx-conf:
    type: file
    machine: web
    dest: /etc/nginx/sites-available/default
    content: |
      server { listen 80; root /var/www/html; }
    depends_on: [nginx-pkg]
  nginx-svc:
    type: service
    machine: web
    service_name: nginx
    desired_state: running
    depends_on: [nginx-conf]

# Post-apply health checks — run after all resources converge
checks:
  http-health:
    machine: web
    command: "curl -sf http://localhost/ >/dev/null"
    expect_exit: 0
    description: "Nginx serves HTTP on port 80"
  port-open:
    machine: web
    command: "ss -tlnp | grep -q ':80 '"
    expect_exit: 0
    description: "Port 80 is listening"
  config-valid:
    machine: web
    command: "nginx -t 2>&1"
    expect_exit: 0
    description: "Nginx config syntax is valid"
```

**Behavior**:

- `checks:` blocks run AFTER all resources converge (post-apply)
- Failures are **warnings** by default (like OpenTofu check blocks) — they don't roll back the apply
- `--checks=fail` flag makes check failures exit non-zero (CI mode)
- `forjar plan` shows checks that will run
- `forjar check -f config.yaml` runs checks without applying (validation-only mode)
- Check results appear in the provenance event log

**CI verification**:

```bash
forjar apply -f config.yaml --state-dir /tmp/state --yes --checks=fail
# Exit code 0 only if all resources converge AND all checks pass
```

**Idempotency**: N/A — checks are read-only assertions, not convergent resources

---

### 33. Lifecycle Protection Rules

**OpenTofu equivalent**: `lifecycle { prevent_destroy = true; ignore_changes = [field]; create_before_destroy = true }`

**The problem**: Forjar treats all resources equally. A `forjar destroy` will happily delete a PostgreSQL data directory alongside a log rotation config. There's no way to mark critical resources as protected, suppress known-harmless drift, or control replacement ordering.

**Tier**: 1 (validate) + 2 (container apply)

**Design**:

```yaml
version: "1.0"
name: protected-stack
resources:
  postgres-data:
    type: file
    machine: db
    dest: /var/lib/postgresql/15/main
    mode: "0700"
    owner: postgres
    lifecycle:
      prevent_destroy: true          # forjar destroy will skip + warn

  nginx-conf:
    type: file
    machine: web
    dest: /etc/nginx/nginx.conf
    lifecycle:
      create_before_destroy: true    # write new config before removing old

  app-config:
    type: file
    machine: web
    dest: /etc/app/config.yaml
    lifecycle:
      ignore_drift: [content]        # external process manages content; only track existence
```

**Three lifecycle rules**:

| Rule | Behavior |
|------|----------|
| `prevent_destroy: true` | `forjar destroy` skips this resource with a warning. Removing it from config triggers a plan error ("resource X is protected; remove lifecycle.prevent_destroy first"). |
| `create_before_destroy: true` | When a resource must be replaced (content changed), write the new version before removing the old. Relevant for config files with restarts — avoids a window where the config doesn't exist. |
| `ignore_drift: [fields]` | `forjar drift` reports drift on these fields as "suppressed" rather than "detected". `forjar apply` skips convergence for suppressed fields. Useful for fields managed by external systems (autoscaler-managed replica counts, externally-rotated passwords). |

**CLI interaction**:

```bash
# Destroy with protected resources — warns but continues for unprotected
forjar destroy -f config.yaml --state-dir ./state --yes
# Output: "SKIP: postgres-data (prevent_destroy); DESTROY: nginx-conf, app-config"

# Override protection (explicit flag)
forjar destroy -f config.yaml --state-dir ./state --yes --force-destroy
# Output: "DESTROY: postgres-data (PROTECTION OVERRIDDEN), nginx-conf, app-config"
```

---

### 34. Moved Blocks (Declarative Refactoring)

**OpenTofu equivalent**: `moved { from = old_name; to = new_name }`

**The problem**: `forjar state-mv` is an imperative CLI command. If you rename a resource in YAML (e.g., `webserver` → `nginx-web`), forjar sees a destroy + create. The `state-mv` command fixes this, but it requires manual intervention and doesn't survive `forjar plan` in CI.

**Design**:

```yaml
version: "1.0"
name: refactored-stack

# Declarative renames — processed before planning
moved:
  - from: webserver
    to: nginx-web
  - from: db
    to: postgres-primary

resources:
  nginx-web:       # was "webserver"
    type: package
    machine: web
    packages: [nginx]
  postgres-primary: # was "db"
    type: package
    machine: db
    packages: [postgresql-15]
```

**Behavior**:

- `moved:` entries are processed during planning, before the diff
- State is updated in-place (rename key in lock file)
- Plan shows "moved" action instead of destroy + create
- After the first successful apply, the `moved:` block can be removed (it's a one-time migration)
- Circular moves are rejected at validation time
- Moving to a name that already exists in state is rejected

**Tier**: 1 (validate moved blocks) + 2 (apply with renamed resources in container)

---

### 35. Refresh-Only Mode (Drift Acceptance)

**OpenTofu equivalent**: `tofu apply -refresh-only`

**The problem**: `forjar drift` detects drift but the only option is `forjar apply` to reconcile. Sometimes drift is intentional (manual hotfix, autoscaler change) and you want to accept it — update state to match reality without re-converging.

**Design**:

```bash
# Detect drift (existing)
forjar drift -f config.yaml --state-dir ./state
# Output: "DRIFT: nginx-conf content changed (b3:old → b3:new)"

# Accept drift — update state to match current machine state
forjar apply -f config.yaml --state-dir ./state --refresh-only --yes
# Output: "REFRESH: nginx-conf state updated (b3:old → b3:new), 0 changes applied"

# Selective refresh — only accept drift for specific resources
forjar apply -f config.yaml --state-dir ./state --refresh-only --target nginx-conf --yes
```

**Behavior**:

- Re-runs state query scripts on all (or targeted) resources
- Updates state lock hashes to match current reality
- Does NOT run converge scripts — no changes to the machine
- Event log records `refresh` event type (distinct from `apply`)
- Shows diff of what changed in state

**Idempotency**: Strong — refresh-only is itself idempotent (running twice produces same state)

---

### 36. Resource Targeting (`--target` Flag)

**OpenTofu equivalent**: `tofu apply -target=aws_instance.app`

**The problem**: `forjar apply` converges all resources. When debugging, recovering from partial failure, or bootstrapping, you need to apply a single resource without touching others.

**Design**:

```bash
# Apply single resource
forjar apply -f config.yaml --state-dir ./state --target nginx-pkg --yes

# Apply multiple targets
forjar apply -f config.yaml --state-dir ./state --target nginx-pkg --target nginx-conf --yes

# Plan for single resource
forjar plan -f config.yaml --state-dir ./state --target nginx-pkg

# Destroy single resource
forjar destroy -f config.yaml --state-dir ./state --target old-config --yes
```

**Dependency behavior**:

- `--target X` implicitly includes X's dependencies (upstream)
- Does NOT include X's dependents (downstream) — they may be in an inconsistent state
- Warning emitted: "Targeted apply may leave dependents in an inconsistent state"
- `--target X --include-dependents` includes downstream resources too

**Tier**: 2 (container — apply single resource, verify others unchanged)

---

### 37. Testing DSL (Native Test Runner)

**OpenTofu equivalent**: `.tftest.hcl` files with `run` blocks, `assert` blocks, `mock_provider`

**The problem**: Forjar recipe testing currently requires hand-written shell scripts and CI workflow YAML. There's no way to co-locate tests with recipes, run them with a single command, or mock transports.

**Design**:

Test files use `.forjar-test.yaml` extension, live alongside the recipe:

```yaml
# recipes/webserver.forjar-test.yaml
name: webserver-recipe-tests
recipe: recipes/webserver.yaml

# Test 1: Plan produces expected resources
tests:
  - name: plan-produces-nginx
    command: plan
    inputs:
      domain: test.local
      port: 8080
    assert:
      - resource_exists: nginx-pkg
      - resource_exists: nginx-conf
      - resource_count: 3

  # Test 2: Apply converges
  - name: apply-converges
    command: apply
    transport: container
    container:
      image: forjar-test-target
    inputs:
      domain: test.local
      port: 8080
    assert:
      - exit_code: 0
      - resource_state: nginx-pkg
        status: present
      - file_contains:
          path: /etc/nginx/sites-available/default
          pattern: "server_name test.local"

  # Test 3: Idempotency
  - name: idempotent-reapply
    command: apply
    depends_on: [apply-converges]   # reuses state from previous test
    assert:
      - changes: 0
      - exit_code: 0

  # Test 4: Bad input rejected
  - name: reject-empty-domain
    command: validate
    inputs:
      domain: ""
      port: 8080
    expect_failure: "domain must not be empty"
```

**CLI**:

```bash
# Run all tests for a recipe
forjar test recipes/webserver.forjar-test.yaml

# Run all tests in a directory
forjar test recipes/

# Run specific test by name
forjar test recipes/webserver.forjar-test.yaml --run plan-produces-nginx

# JSON output for CI
forjar test recipes/ --json
```

**Assertions available**:

| Assertion | What it checks |
|-----------|---------------|
| `exit_code: N` | Apply/plan exit code |
| `changes: N` | Number of resources changed |
| `resource_exists: name` | Resource appears in plan |
| `resource_count: N` | Total resources in plan |
| `resource_state: name` + `status` | Resource converged to expected state |
| `file_contains: path + pattern` | File on target contains pattern |
| `file_absent: path` | File does not exist on target |
| `command_succeeds: cmd` | Shell command exits 0 on target |
| `expect_failure: msg` | Operation fails with expected message |
| `state_hash_stable` | Second apply produces same state hash |

---

### 38. State Encryption (Defense in Depth)

**OpenTofu equivalent**: Client-side state encryption with passphrase or KMS

**The problem**: Forjar encrypts secret values with `ENC[age,...]` markers, but the state lock file (`state.lock.yaml`) contains resource hashes, metadata, and potentially sensitive details (file paths, package names, service configurations) in plaintext. If state files are committed to a shared repo, this leaks infrastructure topology.

**Design**:

```bash
# Encrypt state at rest (first time — generates age key)
forjar apply -f config.yaml --state-dir ./state --encrypt-state --yes

# State files are now age-encrypted YAML
cat ./state/web/state.lock.yaml
# age-encryption.org/v1
# -> X25519 ...
# --- ...

# Subsequent applies auto-decrypt/re-encrypt with the same key
forjar apply -f config.yaml --state-dir ./state --yes
# (auto-detects encrypted state, decrypts with ~/.config/forjar/state.key)

# Decrypt for debugging
forjar state-list --state-dir ./state --machine web
# (decrypts in memory, displays plaintext, never writes plaintext to disk)
```

**Key management**:

- State key stored at `~/.config/forjar/state.key` (age secret key)
- Public key stored in config or env var: `FORJAR_STATE_KEY=age1...`
- CI: state key injected via GitHub Actions secret
- Multiple recipients: `--state-recipients age1...,age1...` (team access)
- Key rotation: `forjar state-rekey --new-key age1...` (re-encrypts all state files)

**Tier**: 1 (validate encrypted state loads) + 2 (container apply with encrypted state round-trip)

---

### 39. Cross-Config Outputs (Multi-Stack Composition)

**OpenTofu equivalent**: `terraform_remote_state` data source + output values

**The problem**: Forjar configs are self-contained. A network config can't export a subnet CIDR for an application config to consume. Teams split infrastructure into stacks (network, database, app) but have no way to pass values between them without hardcoding.

**Design**:

```yaml
# network-stack.yaml
version: "1.0"
name: network-stack

outputs:
  subnet_cidr: "10.0.1.0/24"
  gateway_ip: "10.0.1.1"

resources:
  network-config:
    type: file
    machine: router
    dest: /etc/network/interfaces.d/vlan10
    content: |
      auto eth0.10
      iface eth0.10 inet static
        address {{outputs.gateway_ip}}
        netmask 255.255.255.0
```

```yaml
# app-stack.yaml
version: "1.0"
name: app-stack

data:
  network:
    type: forjar-state
    state_dir: ./state          # reads network-stack's state
    config: network-stack       # config name to import from
    outputs: [subnet_cidr, gateway_ip]

resources:
  app-config:
    type: file
    machine: app
    dest: /etc/app/network.conf
    content: |
      SUBNET={{data.network.subnet_cidr}}
      GATEWAY={{data.network.gateway_ip}}
```

**How it works**:

1. `network-stack` defines `outputs:` — values exported after apply
2. Outputs are stored in state: `./state/<machine>/outputs.yaml`
3. `app-stack` uses `data.type: forjar-state` to read outputs from another config's state
4. Template resolution substitutes `{{data.network.subnet_cidr}}` with the real value
5. If the upstream state doesn't exist or is missing an output, plan fails with a clear error

**Tier**: 2 (container — two-config apply sequence, verify output propagation)

---

### Implementation Phase 8: OpenTofu-Inspired Patterns

| # | Recipe/Feature | Tier | Why |
|---|---------------|------|-----|
| 30 | Saved Plan Files | 1+2 | TOCTOU safety for CI/CD pipelines |
| 31 | JSON Plan Format | 1 | Machine-readable plans for policy engines |
| 32 | Check Blocks | 2 | Post-apply functional health assertions |
| 33 | Lifecycle Protection | 1+2 | Prevent accidental destruction of stateful resources |
| 34 | Moved Blocks | 1+2 | Safe declarative refactoring without destroy/create |
| 35 | Refresh-Only Mode | 2 | Accept intentional drift without re-converging |
| 36 | Resource Targeting | 2 | Surgical apply for debugging and recovery |
| 37 | Testing DSL | 1+2 | Co-located, declarative recipe tests |
| 38 | State Encryption | 1+2 | Defense-in-depth for state files at rest |
| 39 | Cross-Config Outputs | 2 | Multi-stack composition without hardcoding |

### Implementation Order (Completed)

All 10 recipes implemented and qualified at A-grade. Implementation order was:

1. **Saved Plan Files (#30) + JSON Plan (#31)** — Foundation for CI/CD safety
2. **Check Blocks (#32)** — Validates outcomes not just convergence
3. **Lifecycle Protection (#33)** — Production safety
4. **Resource Targeting (#36)** — Operational debugging
5. **Moved Blocks (#34)** — Safe declarative refactoring
6. **Refresh-Only (#35)** — Operational flexibility
7. **Testing DSL (#37)** — Ecosystem maturity
8. **State Encryption (#38)** — Defense in depth via age
9. **Cross-Config Outputs (#39)** — Multi-stack composition

### OpenTofu Features Deliberately NOT Adopted

| Feature | Reason |
|---------|--------|
| Remote state backends (S3, Consul) | Violates sovereign principle — state lives in git, not cloud services |
| Provider plugin system | Forjar's resource types are compiled in; no runtime plugin loading needed for bare-metal |
| HCL language | YAML is simpler, more portable, already chosen |
| `count` meta-argument | Forjar uses `for_each:` which is strictly better (no index shifting) |
| `dynamic` blocks | Forjar's template system (`{{inputs.*}}`) handles this more simply |
| Sentinel/OPA integration | OPA can consume JSON plan (#31) directly; no SDK needed |
| Cloud provider APIs | Forjar manages machines via SSH, not cloud APIs — different paradigm |

---

## Linux System Administration Recipes

Recipes for day-to-day Linux operations — the tasks sysadmins do on every machine. These compose forjar's cron, user, service, file, network, and package resource types into reusable patterns.

### How It Maps

| Sysadmin Task | Forjar Resource Types | Converge Pattern |
|--------------|----------------------|-----------------|
| Cron job management | cron + file | Idempotent crontab entries with wrapper scripts |
| User/group provisioning | user + file | System accounts, SSH keys, sudoers |
| Kernel tuning | file + service | sysctl.d drop-ins + `sysctl --system` reload |
| Log management | package + file + service | logrotate configs, journald tuning |
| Time sync | package + file + service | chrony/NTP config + service lifecycle |
| DNS resolution | file + service | resolved config or /etc/resolv.conf |
| Swap management | file (script) + cron | Swapfile creation, sysctl vm.swappiness |
| Systemd units | file + service | Custom unit files + daemon-reload + enable |
| Hostname/locale | file | /etc/hostname, /etc/locale.gen, localectl |
| Resource limits | file | limits.d drop-ins, sysctl |
| Automated patching | package + cron + file | unattended-upgrades + reboot schedule |

---

### 40. Scheduled Task Management (Cron Recipes)

**Tier**: 1 + 2

Demonstrates every cron pattern — periodic scripts, log cleanup, health checks, backup scheduling, and cron environment management.

**Testable in CI**:
- Tier 1: validate, plan
- Tier 2: container apply — crontab entries written, wrapper scripts deployed, permissions correct

**Recipe inputs**:

| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `backup_dir` | path | `/var/backups` | Backup destination |
| `backup_schedule` | string | `0 2 * * *` | Backup cron schedule |
| `log_retention_days` | int | `30` | Days to keep old logs |
| `health_check_url` | string | `""` | URL to ping after backup (empty = skip) |
| `mailto` | string | `root` | Cron MAILTO for error notifications |

**Resources** (8):

- `backup-dir` — file (directory): `{{inputs.backup_dir}}`, mode 0750
- `backup-script` — file: `/usr/local/bin/forjar-backup.sh`
  ```bash
  #!/bin/bash
  set -euo pipefail
  DEST="{{inputs.backup_dir}}/$(date +%Y%m%d-%H%M%S)"
  mkdir -p "$DEST"
  # Configurable backup commands go here
  tar czf "$DEST/etc.tar.gz" /etc/
  dpkg --get-selections > "$DEST/packages.list"
  {{#if inputs.health_check_url}}
  curl -fsS "{{inputs.health_check_url}}" || true
  {{/if}}
  ```
- `backup-cron` — cron: `{{inputs.backup_schedule}}` → `/usr/local/bin/forjar-backup.sh`, owner: root
- `log-cleanup-script` — file: `/usr/local/bin/forjar-log-cleanup.sh`
  ```bash
  #!/bin/bash
  find /var/log -name "*.gz" -mtime +{{inputs.log_retention_days}} -delete
  find /var/log -name "*.old" -mtime +{{inputs.log_retention_days}} -delete
  journalctl --vacuum-time={{inputs.log_retention_days}}d 2>/dev/null || true
  ```
- `log-cleanup-cron` — cron: `0 3 * * 0` (weekly Sunday 3 AM) → log cleanup script
- `cron-env-file` — file: `/etc/cron.d/forjar-env`
  ```
  SHELL=/bin/bash
  PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
  MAILTO={{inputs.mailto}}
  ```
- `dead-cron-audit-script` — file: `/usr/local/bin/forjar-cron-audit.sh`
  ```bash
  #!/bin/bash
  # Audit: find cron jobs pointing to missing scripts
  for user in $(cut -d: -f1 /etc/passwd); do
    crontab -l -u "$user" 2>/dev/null | grep -v '^#' | while read -r line; do
      cmd=$(echo "$line" | awk '{print $6}')
      [ -n "$cmd" ] && [ ! -x "$cmd" ] && echo "DEAD: user=$user cmd=$cmd"
    done
  done
  ```
- `dead-cron-audit` — cron: `0 6 * * 1` (weekly Monday 6 AM) → audit script

**Idempotency**: Strong — cron entries are tagged with forjar identifiers, scripts are content-hashed

**Convergence budget**: < 5s first apply, < 1s idempotent

---

### 41. User & Group Provisioning

**Tier**: 1 + 2

Comprehensive user lifecycle — system accounts, developer accounts, SSH key deployment, sudoers management, group membership.

**Testable in CI**:
- Tier 2: container apply — users created, groups assigned, SSH keys deployed, sudoers written

**Recipe inputs**:

| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `admin_users` | list | required | List of admin usernames |
| `service_user` | string | `"forjar"` | Service account name |
| `ssh_keys` | map | `{}` | Map of username → SSH public key |
| `sudo_nopasswd` | bool | `false` | Allow passwordless sudo for admins |
| `shell` | string | `/bin/bash` | Default shell |
| `home_base` | path | `/home` | Base path for home directories |

**Resources** (per-user via `for_each`, plus shared):

- `admin-group` — user: group `forjar-admins`, state: present
- `service-account` — user: `{{inputs.service_user}}`, system: true, shell: `/usr/sbin/nologin`, home: `/var/lib/{{inputs.service_user}}`
- `admin-user-{name}` — user (for_each `admin_users`): create user, groups: [forjar-admins, sudo], shell: `{{inputs.shell}}`
- `ssh-key-{name}` — file (for_each `ssh_keys`): `/home/{name}/.ssh/authorized_keys`, mode 0600, owner: `{name}`
- `sudoers-forjar` — file: `/etc/sudoers.d/forjar-admins`, mode 0440
  ```
  # Managed by forjar — do not edit
  %forjar-admins ALL=(ALL) {{#if inputs.sudo_nopasswd}}NOPASSWD:{{/if}} ALL
  ```
- `sudoers-validate` — file: `/usr/local/bin/forjar-sudoers-check.sh`
  ```bash
  #!/bin/bash
  visudo -c -f /etc/sudoers.d/forjar-admins
  ```

**Idempotency**: Strong — users and groups are checked by name, SSH keys by content hash

**Convergence budget**: < 5s first apply, < 1s idempotent

---

### 42. Kernel Tuning (sysctl)

**Tier**: 1 + 2

Production kernel parameter tuning via sysctl.d drop-in files. Covers network performance, memory management, security hardening, and file descriptor limits.

**Testable in CI**:
- Tier 2: container apply — sysctl.d files deployed (sysctl --system may not work in unprivileged containers, but file content is verified)

**Recipe inputs**:

| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `profile` | enum | `general` | Tuning profile: `general`, `web`, `database`, `gpu-compute` |
| `max_connections` | int | `65535` | `net.core.somaxconn` |
| `swappiness` | int | `10` | `vm.swappiness` (0–100) |
| `file_max` | int | `2097152` | `fs.file-max` |
| `custom_params` | map | `{}` | Additional sysctl key=value pairs |

**Resources** (5):

- `sysctl-network` — file: `/etc/sysctl.d/60-forjar-network.conf`
  ```ini
  # Managed by forjar — network tuning ({{inputs.profile}} profile)
  net.core.somaxconn = {{inputs.max_connections}}
  net.ipv4.tcp_max_syn_backlog = {{inputs.max_connections}}
  net.ipv4.ip_local_port_range = 1024 65535
  net.ipv4.tcp_tw_reuse = 1
  net.ipv4.tcp_fin_timeout = 15
  net.core.netdev_max_backlog = 65536
  net.core.rmem_max = 16777216
  net.core.wmem_max = 16777216
  ```
- `sysctl-memory` — file: `/etc/sysctl.d/60-forjar-memory.conf`
  ```ini
  vm.swappiness = {{inputs.swappiness}}
  vm.dirty_ratio = 15
  vm.dirty_background_ratio = 5
  vm.overcommit_memory = 0
  ```
- `sysctl-security` — file: `/etc/sysctl.d/60-forjar-security.conf`
  ```ini
  # ICMP hardening
  net.ipv4.icmp_echo_ignore_broadcasts = 1
  net.ipv4.icmp_ignore_bogus_error_responses = 1
  # SYN flood protection
  net.ipv4.tcp_syncookies = 1
  # Source routing disabled
  net.ipv4.conf.all.accept_source_route = 0
  net.ipv4.conf.default.accept_source_route = 0
  # ASLR
  kernel.randomize_va_space = 2
  ```
- `sysctl-filelimits` — file: `/etc/sysctl.d/60-forjar-filelimits.conf`
  ```ini
  fs.file-max = {{inputs.file_max}}
  fs.inotify.max_user_watches = 524288
  ```
- `sysctl-reload` — service: oneshot script `sysctl --system`, restart_on: [sysctl-network, sysctl-memory, sysctl-security, sysctl-filelimits]

**Profile presets**:

| Profile | somaxconn | swappiness | file_max | Extra |
|---------|-----------|-----------|----------|-------|
| `general` | 65535 | 10 | 2097152 | — |
| `web` | 65535 | 10 | 2097152 | `net.ipv4.tcp_keepalive_time=60` |
| `database` | 65535 | 1 | 2097152 | `vm.dirty_background_ratio=3`, `vm.overcommit_memory=2` |
| `gpu-compute` | 4096 | 1 | 2097152 | `vm.nr_hugepages=1024` |

**Idempotency**: Strong — file content determines convergence, sysctl reload only on change

**Convergence budget**: < 3s first apply, < 1s idempotent

---

### 43. Log Management (Journald + Logrotate)

**Tier**: 1 + 2

Centralized log configuration — journald tuning, logrotate for application logs, disk usage monitoring.

**Testable in CI**:
- Tier 2: container apply — config files deployed, logrotate config validated

**Recipe inputs**:

| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `journal_max_disk` | string | `500M` | Max persistent journal size |
| `journal_max_age` | string | `30d` | Journal retention period |
| `app_log_paths` | list | `[]` | Application log paths for logrotate |
| `rotate_count` | int | `7` | Logrotate file count |
| `rotate_schedule` | enum | `daily` | daily, weekly, monthly |
| `compress` | bool | `true` | Compress rotated logs |

**Resources** (6):

- `logrotate-pkg` — package (apt): logrotate
- `journald-config` — file: `/etc/systemd/journald.conf.d/forjar.conf`
  ```ini
  [Journal]
  SystemMaxUse={{inputs.journal_max_disk}}
  MaxRetentionSec={{inputs.journal_max_age}}
  MaxFileSec=1day
  Compress=yes
  ForwardToSyslog=no
  ```
- `journald-restart` — service: systemd-journald (restart_on: journald-config)
- `app-logrotate` — file: `/etc/logrotate.d/forjar-apps`
  ```
  {{#each inputs.app_log_paths}}
  {{this}} {
      {{inputs.rotate_schedule}}
      rotate {{inputs.rotate_count}}
      {{#if inputs.compress}}compress{{/if}}
      delaycompress
      missingok
      notifempty
      create 0640 root adm
  }
  {{/each}}
  ```
- `logrotate-test-cron` — cron: `0 */6 * * *` → `logrotate -d /etc/logrotate.d/forjar-apps 2>&1 | logger -t forjar-logrotate` (dry-run audit every 6h)
- `disk-usage-alert` — file: `/usr/local/bin/forjar-disk-alert.sh`
  ```bash
  #!/bin/bash
  USAGE=$(df /var/log --output=pcent | tail -1 | tr -d ' %')
  [ "$USAGE" -gt 85 ] && logger -p user.warning -t forjar "Log partition at ${USAGE}%"
  ```
  + cron: `*/30 * * * *` → disk usage check

**Idempotency**: Strong — config files content-hashed, service restart only on config change

**Convergence budget**: < 10s first apply, < 1s idempotent

---

### 44. Time Synchronization (Chrony/NTP)

**Tier**: 1 + 2

Reliable time sync — critical for TLS, logging, distributed systems. Chrony as primary (modern, handles VM migration), with NTP pool configuration.

**Testable in CI**:
- Tier 2: container apply — chrony installed, config deployed (service won't start in container but config is verified)

**Recipe inputs**:

| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `ntp_servers` | list | `[0.pool.ntp.org, 1.pool.ntp.org, 2.pool.ntp.org, 3.pool.ntp.org]` | NTP server list |
| `timezone` | string | `UTC` | System timezone |
| `makestep_threshold` | float | `1.0` | Seconds threshold for step correction |
| `makestep_limit` | int | `3` | Number of step corrections allowed at startup |

**Resources** (5):

- `chrony-pkg` — package (apt): chrony
- `remove-systemd-timesyncd` — package (apt): systemd-timesyncd, state: absent (conflicts with chrony)
- `chrony-config` — file: `/etc/chrony/chrony.conf`
  ```
  # Managed by forjar
  {{#each inputs.ntp_servers}}
  server {{this}} iburst
  {{/each}}
  driftfile /var/lib/chrony/chrony.drift
  makestep {{inputs.makestep_threshold}} {{inputs.makestep_limit}}
  rtcsync
  logdir /var/log/chrony
  ```
- `chrony-service` — service: chrony (running, enabled, restart_on: chrony-config)
- `timezone-config` — file: `/etc/timezone`, content: `{{inputs.timezone}}`
  + post_apply: `timedatectl set-timezone {{inputs.timezone}} 2>/dev/null || ln -sf /usr/share/zoneinfo/{{inputs.timezone}} /etc/localtime`

**Idempotency**: Strong

**Convergence budget**: < 10s first apply, < 1s idempotent

---

### 45. Custom Systemd Units

**Tier**: 1 + 2

Deploy and manage custom systemd service, timer, and oneshot units. The pattern for any "run my binary as a daemon" task.

**Testable in CI**:
- Tier 2: container apply — unit files deployed, symlinks created (systemd won't run in container but file content is verified)

**Recipe inputs**:

| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `unit_name` | string | required | Systemd unit name (e.g., `myapp`) |
| `exec_start` | string | required | ExecStart command |
| `user` | string | `root` | Run-as user |
| `group` | string | `root` | Run-as group |
| `working_dir` | path | `/` | WorkingDirectory |
| `restart_policy` | enum | `on-failure` | Restart, RestartSec |
| `restart_sec` | int | `5` | RestartSec in seconds |
| `env_vars` | map | `{}` | Environment variables |
| `wants` | list | `[]` | Unit wants (After=, Wants=) |
| `timer_schedule` | string | `""` | OnCalendar for timer unit (empty = no timer, just service) |
| `memory_limit` | string | `""` | MemoryMax (e.g., `512M`) |
| `cpu_quota` | string | `""` | CPUQuota (e.g., `50%`) |
| `sandbox` | bool | `false` | Enable security hardening (NoNewPrivileges, ProtectSystem, etc.) |

**Resources** (4–6 depending on inputs):

- `unit-service` — file: `/etc/systemd/system/{{inputs.unit_name}}.service`
  ```ini
  [Unit]
  Description={{inputs.unit_name}} managed by forjar
  {{#each inputs.wants}}
  After={{this}}
  Wants={{this}}
  {{/each}}

  [Service]
  Type=simple
  ExecStart={{inputs.exec_start}}
  User={{inputs.user}}
  Group={{inputs.group}}
  WorkingDirectory={{inputs.working_dir}}
  Restart={{inputs.restart_policy}}
  RestartSec={{inputs.restart_sec}}
  {{#each inputs.env_vars}}
  Environment="{{@key}}={{this}}"
  {{/each}}
  {{#if inputs.memory_limit}}
  MemoryMax={{inputs.memory_limit}}
  {{/if}}
  {{#if inputs.cpu_quota}}
  CPUQuota={{inputs.cpu_quota}}
  {{/if}}
  {{#if inputs.sandbox}}
  NoNewPrivileges=yes
  ProtectSystem=strict
  ProtectHome=yes
  PrivateTmp=yes
  ReadWritePaths={{inputs.working_dir}}
  {{/if}}

  [Install]
  WantedBy=multi-user.target
  ```
- `unit-timer` (when `timer_schedule` is set) — file: `/etc/systemd/system/{{inputs.unit_name}}.timer`
  ```ini
  [Unit]
  Description={{inputs.unit_name}} timer managed by forjar

  [Timer]
  OnCalendar={{inputs.timer_schedule}}
  Persistent=true

  [Install]
  WantedBy=timers.target
  ```
- `daemon-reload` — service: oneshot `systemctl daemon-reload`, restart_on: [unit-service, unit-timer]
- `unit-enable` — service: `{{inputs.unit_name}}` (running, enabled, depends_on: daemon-reload)
- `timer-enable` (when `timer_schedule` is set) — service: `{{inputs.unit_name}}.timer` (running, enabled, depends_on: daemon-reload)
- `unit-status-cron` — cron: `*/5 * * * *` → `systemctl is-active {{inputs.unit_name}} || logger -t forjar "{{inputs.unit_name}} is not running"`

**Idempotency**: Strong — unit files content-hashed, daemon-reload only on change

**Convergence budget**: < 5s first apply, < 1s idempotent

---

### 46. Resource Limits (ulimits + cgroups)

**Tier**: 1 + 2

System-wide and per-user resource limits — file descriptors, process counts, memory locks, core dumps.

**Testable in CI**:
- Tier 2: container apply — limits.d files deployed with correct content

**Recipe inputs**:

| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `nofile_soft` | int | `65535` | Open file soft limit |
| `nofile_hard` | int | `131072` | Open file hard limit |
| `nproc_soft` | int | `65535` | Process count soft limit |
| `nproc_hard` | int | `131072` | Process count hard limit |
| `memlock` | string | `unlimited` | Memory lock limit (for GPU/RDMA) |
| `core_dumps` | bool | `false` | Enable core dumps |
| `service_users` | list | `[]` | Users to get elevated limits |

**Resources** (4):

- `limits-global` — file: `/etc/security/limits.d/99-forjar.conf`
  ```
  # Managed by forjar — global resource limits
  * soft nofile {{inputs.nofile_soft}}
  * hard nofile {{inputs.nofile_hard}}
  * soft nproc  {{inputs.nproc_soft}}
  * hard nproc  {{inputs.nproc_hard}}
  {{#unless inputs.core_dumps}}
  * hard core   0
  {{/unless}}
  ```
- `limits-service-users` — file: `/etc/security/limits.d/98-forjar-services.conf` (when `service_users` non-empty)
  ```
  {{#each inputs.service_users}}
  {{this}} soft memlock {{inputs.memlock}}
  {{this}} hard memlock {{inputs.memlock}}
  {{this}} soft nofile  {{inputs.nofile_hard}}
  {{this}} hard nofile  {{inputs.nofile_hard}}
  {{/each}}
  ```
- `pam-limits` — file: `/etc/pam.d/common-session` (append line)
  ```
  session required pam_limits.so
  ```
- `sysctl-pid-max` — file: `/etc/sysctl.d/60-forjar-pidmax.conf`
  ```
  kernel.pid_max = 4194304
  ```

**Idempotency**: Strong

**Convergence budget**: < 3s first apply, < 1s idempotent

---

### 47. Automated Patching & Reboot Schedule

**Tier**: 1 + 2

Unattended security patching with configurable reboot windows. The "set it and forget it" recipe for fleet machines.

**Testable in CI**:
- Tier 2: container apply — unattended-upgrades config deployed, reboot schedule cron set

**Recipe inputs**:

| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `auto_reboot` | bool | `true` | Auto-reboot after kernel updates |
| `reboot_window` | string | `02:00` | Reboot time (24h format) |
| `reboot_day` | string | `Sun` | Reboot day of week |
| `security_only` | bool | `true` | Only install security updates |
| `blacklist_packages` | list | `[]` | Packages to never auto-update |
| `mail_to` | string | `""` | Email for update notifications |
| `pre_reboot_script` | string | `""` | Script to run before reboot (drain, etc.) |

**Resources** (6):

- `apt-unattended` — package (apt): unattended-upgrades, apt-listchanges
- `auto-upgrades-config` — file: `/etc/apt/apt.conf.d/20auto-upgrades`
  ```
  APT::Periodic::Update-Package-Lists "1";
  APT::Periodic::Unattended-Upgrade "1";
  APT::Periodic::AutocleanInterval "7";
  ```
- `unattended-config` — file: `/etc/apt/apt.conf.d/50unattended-upgrades`
  ```
  Unattended-Upgrade::Allowed-Origins {
      "${distro_id}:${distro_codename}-security";
  {{#unless inputs.security_only}}
      "${distro_id}:${distro_codename}-updates";
  {{/unless}}
  };
  {{#if inputs.blacklist_packages}}
  Unattended-Upgrade::Package-Blacklist {
  {{#each inputs.blacklist_packages}}
      "{{this}}";
  {{/each}}
  };
  {{/if}}
  Unattended-Upgrade::Automatic-Reboot "{{inputs.auto_reboot}}";
  Unattended-Upgrade::Automatic-Reboot-Time "{{inputs.reboot_window}}";
  {{#if inputs.mail_to}}
  Unattended-Upgrade::Mail "{{inputs.mail_to}}";
  {{/if}}
  ```
- `reboot-check-script` — file: `/usr/local/bin/forjar-reboot-check.sh`
  ```bash
  #!/bin/bash
  if [ -f /var/run/reboot-required ]; then
    logger -t forjar "Reboot required: $(cat /var/run/reboot-required.pkgs 2>/dev/null)"
    {{#if inputs.pre_reboot_script}}
    {{inputs.pre_reboot_script}}
    {{/if}}
  fi
  ```
- `reboot-check-cron` — cron: `0 {{inputs.reboot_window}} * * {{inputs.reboot_day}}` → reboot check script
- `update-status-cron` — cron: `0 8 * * *` → `apt list --upgradable 2>/dev/null | wc -l | xargs -I{} logger -t forjar "Pending updates: {}"`

**Idempotency**: Strong

**Convergence budget**: < 15s first apply (apt install), < 1s idempotent

---

### 48. Hostname, Locale & DNS Resolution

**Tier**: 1 + 2

Machine identity and locale — the first things configured on a new machine.

**Testable in CI**:
- Tier 2: container apply — files deployed with correct content

**Recipe inputs**:

| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `hostname` | string | required | Machine hostname (short) |
| `domain` | string | `""` | Domain name (FQDN = hostname.domain) |
| `locale` | string | `en_US.UTF-8` | System locale |
| `timezone` | string | `UTC` | System timezone (defers to recipe #44 if composed) |
| `dns_servers` | list | `[1.1.1.1, 8.8.8.8]` | DNS resolver IPs |
| `search_domains` | list | `[]` | DNS search domains |
| `use_resolved` | bool | `true` | Use systemd-resolved (false = /etc/resolv.conf) |

**Resources** (6):

- `hostname-file` — file: `/etc/hostname`, content: `{{inputs.hostname}}`
- `hosts-file` — file: `/etc/hosts`
  ```
  127.0.0.1 localhost
  127.0.1.1 {{inputs.hostname}}{{#if inputs.domain}}.{{inputs.domain}} {{inputs.hostname}}{{/if}}

  ::1 localhost ip6-localhost ip6-loopback
  ff02::1 ip6-allnodes
  ff02::2 ip6-allrouters
  ```
- `locale-gen` — file: `/etc/locale.gen`, content includes `{{inputs.locale}} UTF-8`
  + post_apply: `locale-gen && update-locale LANG={{inputs.locale}}`
- `resolved-config` (when `use_resolved`) — file: `/etc/systemd/resolved.conf.d/forjar.conf`
  ```ini
  [Resolve]
  DNS={{inputs.dns_servers | join " "}}
  {{#if inputs.search_domains}}
  Domains={{inputs.search_domains | join " "}}
  {{/if}}
  DNSOverTLS=opportunistic
  DNSSEC=allow-downgrade
  ```
- `resolved-service` (when `use_resolved`) — service: systemd-resolved (running, enabled, restart_on: resolved-config)
- `resolv-conf` (when NOT `use_resolved`) — file: `/etc/resolv.conf`
  ```
  {{#each inputs.dns_servers}}
  nameserver {{this}}
  {{/each}}
  {{#if inputs.search_domains}}
  search {{inputs.search_domains | join " "}}
  {{/if}}
  ```

**Idempotency**: Strong

**Convergence budget**: < 5s first apply, < 1s idempotent

---

### 49. Swap & Memory Management

**Tier**: 1 + 2 (file creation testable) + 3 (actual swap activation needs bare-metal)

Create and manage swapfiles, configure memory pressure behavior.

**Testable in CI**:
- Tier 2: container apply — swap setup script deployed, sysctl files written
- Tier 3: bare-metal — actual swap activation and verification

**Recipe inputs**:

| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `swap_size_mb` | int | `2048` | Swapfile size in MB |
| `swap_path` | path | `/swapfile` | Swapfile location |
| `swappiness` | int | `10` | vm.swappiness (0–100) |
| `vfs_cache_pressure` | int | `50` | vm.vfs_cache_pressure |
| `oom_kill_allocating_task` | bool | `true` | OOM killer targets the allocating process |

**Resources** (4):

- `swap-setup-script` — file: `/usr/local/bin/forjar-swap-setup.sh`
  ```bash
  #!/bin/bash
  set -euo pipefail
  SWAP="{{inputs.swap_path}}"
  SIZE="{{inputs.swap_size_mb}}"
  if [ -f "$SWAP" ]; then
    CURRENT=$(stat -c%s "$SWAP" 2>/dev/null || echo 0)
    DESIRED=$((SIZE * 1024 * 1024))
    [ "$CURRENT" -eq "$DESIRED" ] && exit 0
    swapoff "$SWAP" 2>/dev/null || true
  fi
  dd if=/dev/zero of="$SWAP" bs=1M count="$SIZE" status=none
  chmod 600 "$SWAP"
  mkswap "$SWAP" >/dev/null
  swapon "$SWAP"
  grep -q "$SWAP" /etc/fstab || echo "$SWAP none swap sw 0 0" >> /etc/fstab
  ```
- `swap-setup` — cron: `@reboot` → swap setup script (ensures swap survives reboot)
  + also runs as post_apply one-shot
- `sysctl-swap` — file: `/etc/sysctl.d/60-forjar-swap.conf`
  ```ini
  vm.swappiness = {{inputs.swappiness}}
  vm.vfs_cache_pressure = {{inputs.vfs_cache_pressure}}
  vm.oom_kill_allocating_task = {{#if inputs.oom_kill_allocating_task}}1{{else}}0{{/if}}
  ```
- `sysctl-swap-reload` — service: oneshot `sysctl --system`, restart_on: sysctl-swap

**Idempotency**: Weak (swap activation is stateful, but script checks before acting)

**Convergence budget**: < 10s first apply (dd), < 1s idempotent

---

### Linux Admin Recipe Testability Summary

| # | Recipe | Tier | Resources | Key Resource Types |
|---|--------|------|-----------|-------------------|
| 40 | Scheduled Tasks (Cron) | 1+2 | 8 | cron, file |
| 41 | User & Group Provisioning | 1+2 | 6+ | user, file |
| 42 | Kernel Tuning (sysctl) | 1+2 | 5 | file, service |
| 43 | Log Management | 1+2 | 6 | package, file, service, cron |
| 44 | Time Sync (Chrony) | 1+2 | 5 | package, file, service |
| 45 | Custom Systemd Units | 1+2 | 4–6 | file, service, cron |
| 46 | Resource Limits (ulimits) | 1+2 | 4 | file |
| 47 | Automated Patching | 1+2 | 6 | package, file, cron |
| 48 | Hostname, Locale & DNS | 1+2 | 6 | file, service |
| 49 | Swap & Memory | 1+2+3 | 4 | file, cron, service |

### Implementation Phase 9: Linux System Administration

| # | Recipe | Tier | Why |
|---|--------|------|-----|
| 40 | Scheduled Tasks | 1+2 | Exercises cron resource with real-world patterns |
| 41 | User & Group | 1+2 | Most common sysadmin task; exercises for_each |
| 42 | Kernel Tuning | 1+2 | Every production server needs this |
| 43 | Log Management | 1+2 | Prevents disk exhaustion; exercises package+file+service+cron |
| 44 | Time Sync | 1+2 | Critical for TLS, logs, distributed systems |
| 45 | Systemd Units | 1+2 | The pattern for "deploy my daemon"; most reusable recipe |
| 46 | Resource Limits | 1+2 | Prevents cascading failures from resource exhaustion |
| 47 | Automated Patching | 1+2 | Fleet hygiene; exercises conditional templating |
| 48 | Hostname/Locale/DNS | 1+2 | Day-zero machine identity setup |
| 49 | Swap & Memory | 1+2+3 | Memory management; demonstrates Tier 2 vs Tier 3 split |
