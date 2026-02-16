<p align="center">
  <img src="docs/hero.svg" alt="forjar — Rust-native Infrastructure as Code" width="900" />
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> &middot;
  <a href="docs/book/">Book</a> &middot;
  <a href="docs/specifications/forjar-spec.md">Specification</a> &middot;
  <a href="#benchmarks">Benchmarks</a>
</p>

---

Forjar is a single-binary IaC tool written in Rust. It manages bare-metal machines over SSH using YAML configs, BLAKE3 content-addressed state, and deterministic DAG execution. No cloud APIs, no runtime dependencies, no remote state backends.

```
forjar.yaml  →  parse  →  resolve DAG  →  plan  →  codegen  →  execute  →  BLAKE3 lock
```

## Why Forjar

| | Terraform | Ansible | **Forjar** |
|---|---|---|---|
| Runtime | Go + providers | Python + SSH | **Single Rust binary** |
| State | S3 / Consul / JSON | None | **Git (BLAKE3 YAML)** |
| Drift detection | API calls | None | **Local hash compare** |
| Bare metal | Weak | Strong | **First-class** |
| Dependencies | ~200 Go modules | ~50 Python pkgs | **6 crates** |
| Apply speed | Seconds–minutes | Minutes | **Milliseconds–seconds** |

## Quick Start

```bash
# Install from source
cargo install --path .

# Initialize a project
forjar init my-infra && cd my-infra

# Edit forjar.yaml (see Configuration below)

# Preview changes
forjar plan -f forjar.yaml

# Apply
forjar apply -f forjar.yaml

# Check for unauthorized changes
forjar drift --state-dir state

# View current state
forjar status --state-dir state
```

## Configuration

A `forjar.yaml` declares machines, resources, and policy:

```yaml
version: "1.0"
name: home-lab
description: "Sovereign AI stack provisioning"

params:
  data_dir: /mnt/data

machines:
  gpu-box:
    hostname: lambda
    addr: 192.168.50.100
    user: noah
    ssh_key: ~/.ssh/id_ed25519
    arch: x86_64
    roles: [gpu-compute]

resources:
  base-packages:
    type: package
    machine: gpu-box
    provider: apt
    packages: [curl, htop, git, tmux, ripgrep]

  data-dir:
    type: file
    machine: gpu-box
    state: directory
    path: "{{params.data_dir}}"
    owner: noah
    mode: "0755"
    depends_on: [base-packages]

  app-config:
    type: file
    machine: gpu-box
    path: /etc/app/config.yaml
    content: |
      data_dir: {{params.data_dir}}
      log_level: info
    owner: noah
    mode: "0644"
    depends_on: [data-dir]

policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
```

### Resource Types

| Type | States | Key Fields |
|------|--------|------------|
| `package` | present, absent | `provider` (apt/cargo/pip), `packages` |
| `file` | file, directory, symlink, absent | `path`, `content`, `owner`, `group`, `mode` |
| `service` | running, stopped, enabled, disabled | `name`, `enabled`, `restart_on` |
| `mount` | mounted, unmounted, absent | `path`, `target`, `fstype`, `options` |

### Templates

Use `{{params.key}}` to reference global parameters in any string field. Templates are resolved before codegen.

### Recipes

Reusable, parameterized resource patterns (like Homebrew formulae):

```yaml
# recipes/dev-tools.yaml
name: dev-tools
version: "1.0"
inputs:
  user:
    type: string
    required: true
  shell:
    type: enum
    values: [bash, zsh, fish]
    default: zsh
resources:
  packages:
    type: package
    provider: apt
    packages: [build-essential, cmake, pkg-config]
  dotfiles:
    type: file
    state: directory
    path: "/home/{{inputs.user}}/.config"
    owner: "{{inputs.user}}"
    mode: "0755"
```

## How It Works

1. **Parse** — Read `forjar.yaml`, validate schema and references
2. **Resolve** — Expand templates, build dependency DAG (Kahn's toposort, alphabetical tie-break)
3. **Plan** — Diff desired state against BLAKE3 lock file (hash comparison, no API calls)
4. **Codegen** — Generate shell scripts per resource type
5. **Execute** — Run scripts locally or via SSH (stdin pipe, not argument passing)
6. **State** — Atomic lock file write (temp + rename), append to JSONL event log

### Failure Policy (Jidoka)

On first failure, execution stops immediately. Partial state is preserved in the lock file. No cascading damage. Re-run to continue from where it stopped.

### Transport

- **Local**: `bash` via stdin pipe (for `127.0.0.1` / `localhost`)
- **SSH**: `ssh -o BatchMode=yes` with stdin pipe (no argument length limits)

## Benchmarks

```bash
cargo bench
```

| Operation | Input | Mean | 95% CI |
|---|---|---|---|
| BLAKE3 hash | 64 B string | 27 ns | +/- 0.5 ns |
| BLAKE3 hash | 1 KB string | 92 ns | +/- 1.2 ns |
| BLAKE3 hash | 1 MB file | 172 us | +/- 0.4 us |
| YAML parse | 500 B config | 20.7 us | +/- 0.2 us |
| Topo sort | 100 nodes | 34.6 us | +/- 0.4 us |

Criterion.rs, 100 samples, 3s warm-up. Run locally to reproduce.

## Falsifiable Claims

<details>
<summary>10 testable claims with linked tests (click to expand)</summary>

### C1: Deterministic hashing
BLAKE3 of identical inputs always produces identical outputs.
Tests: `test_fj014_hash_file_deterministic`, `test_fj014_hash_string`

### C2: Deterministic DAG order
Same dependency graph always produces the same execution order.
Tests: `test_fj003_topo_sort_deterministic`, `test_fj003_alphabetical_tiebreak`

### C3: Idempotent apply
Second apply on unchanged config produces zero changes.
Tests: `test_fj012_idempotent_apply`, `test_fj004_plan_all_unchanged`

### C4: Cycle detection
Circular dependencies are rejected at parse time.
Tests: `test_fj003_cycle_detection`

### C5: Content-addressed state
Lock hashes are derived from desired state, not timestamps.
Tests: `test_fj004_hash_deterministic`, `test_fj004_plan_all_unchanged`

### C6: Atomic state persistence
Lock writes use temp file + rename. No corruption on crash.
Tests: `test_fj013_atomic_write`, `test_fj013_save_and_load`

### C7: Recipe input validation
Invalid typed inputs are rejected before expansion.
Tests: `test_fj019_validate_inputs_type_mismatch`, `test_fj019_validate_inputs_enum_invalid`

### C8: Heredoc injection safety
Single-quoted heredoc prevents shell expansion in file content.
Tests: `test_fj007_heredoc_safe`

### C9: Minimal dependencies
Fewer than 10 direct crate dependencies. Single binary output.
Verify: `cargo metadata --no-deps --format-version 1 | jq '.packages[0].dependencies | length'`

### C10: Jidoka failure isolation
First failure stops execution. Previously converged state is preserved.
Tests: `test_fj012_apply_local_file`

</details>

## Testing

```bash
cargo test                    # 126+ unit tests
cargo test -- --nocapture     # with output
cargo test planner            # specific module
cargo bench                   # Criterion benchmarks
cargo clippy -- -D warnings   # lint
```

## License

MIT OR Apache-2.0
