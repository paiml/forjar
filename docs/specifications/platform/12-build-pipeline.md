# 12: Build Pipeline

> Shell purification, model compilation, WASM deployment, and the forjar self-build.

**Spec ID**: FJ-2400-FJ-2403 | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## Overview

Forjar's build pipeline has four subsystems:

1. **bashrs purification** (FJ-2400) — Every shell script is parsed, purified, and validated before execution
2. **apr model compilation** (FJ-2401) — ML models are pulled, converted, compiled into standalone executables, and served
3. **WASM deployment** (FJ-2402) — Browser-targeted compilation and CDN deployment via presentar
4. **forjar self-build** (FJ-2403) — Cargo workspace, release profile, CI matrix

```
                    ┌─────────────────────────────────────┐
                    │          Source Inputs                │
                    │  YAML configs, Rust code, ML models  │
                    └──────┬──────────┬──────────┬────────┘
                           │          │          │
              ┌────────────▼──┐  ┌────▼────┐  ┌──▼──────────┐
              │  bashrs       │  │  apr    │  │  presentar  │
              │  purifier     │  │  CLI    │  │  CLI        │
              │               │  │         │  │             │
              │  parse → AST  │  │  pull   │  │  bundle     │
              │  purify → AST │  │  convert│  │  wasm-opt   │
              │  format → sh  │  │  compile│  │  deploy     │
              │  validate     │  │  serve  │  │             │
              └──────┬────────┘  └────┬────┘  └──────┬──────┘
                     │                │              │
              ┌──────▼────────────────▼──────────────▼──────┐
              │              forjar runtime                  │
              │                                              │
              │  transport layer dispatches purified scripts  │
              │  model resources managed via apr provider     │
              │  WASM dashboards deployed via presentar       │
              └──────────────────────────────────────────────┘
```

---

## FJ-2400: bashrs Shell Purification

### The Problem

Forjar generates shell scripts for every resource operation — install packages, write files, enable services, mount filesystems. These scripts execute on remote machines via SSH, in containers, or in kernel namespaces. A single quoting error, unhandled variable, or non-idempotent command can:

- Corrupt target machines
- Leak secrets via `/proc/*/cmdline`
- Fail silently (exit 0 with wrong state)
- Behave differently across sh/bash/dash

### Invariant I8

**No raw shell execution — all shell is bashrs-purified.**

This is enforced at the transport layer, not at codegen. Even if a resource handler generates broken shell, the transport gate blocks execution.

### Three Safety Levels

```
fn validate_script(script: &str) -> Result<(), String>
fn lint_script(script: &str) -> LintResult
fn purify_script(script: &str) -> Result<String, String>
```

| Level | Function | Guarantee | Cost |
|-------|----------|-----------|------|
| **Validate** | `validate_script()` | Fails on Error-severity diagnostics; warnings acceptable | ~1ms |
| **Lint** | `lint_script()` | Returns all diagnostics (errors + warnings + notes) | ~1ms |
| **Purify** | `purify_script()` | Parse to AST, transform, reformat, validate output | ~5ms |

### Purification Transforms

bashrs does not just warn — it **fixes**:

| Input | Output | Why |
|-------|--------|-----|
| `mkdir /opt/app` | `mkdir -p /opt/app` | Idempotent: second run doesn't fail |
| `$UNQUOTED` | `"$UNQUOTED"` | Prevents word splitting and globbing |
| `echo $secret` | `echo "$secret"` | Prevents field splitting |
| Non-deterministic constructs | Deterministic rewrites | Reproducible across runs |
| Non-POSIX syntax | POSIX-compliant equivalent | Works on sh, dash, bash, ash |

All output passes strict ShellCheck validation.

### Transport Gate (I8 Enforcement)

```
fn validate_before_exec(script: &str) -> Result<(), String>:
    let sanitised = strip_data_payloads(script)
    purifier::validate_script(&sanitised)
        .map_err("I8 violation — script failed bashrs validation: {e}")
```

Called before every execution path:

| Entry Point | Gate |
|-------------|------|
| `exec_script(machine, script)` | `validate_before_exec(script)?` at line 1 |
| `query(machine, cmd)` | `validate_before_exec(cmd)?` defense-in-depth |
| `exec_script_timeout()` | Via `exec_script()` |
| `exec_script_retry()` | Per-attempt via `exec_script()` |

### Payload Stripping

Data payloads (file content being written to disk) are **not shell** — they're opaque bytes piped through redirection. bashrs would misinterpret them as syntax errors. Two patterns are stripped before validation:

**Pattern 1: Base64 binary deployment**
```bash
# Before stripping:
echo 'SGVsbG8gV29ybGQ=...' | base64 -d > '/opt/app/binary'

# After stripping (for lint only):
echo 'FORJAR_BASE64_STRIPPED' > '/opt/app/binary'
```

**Pattern 2: Heredoc text deployment**
```bash
# Before stripping:
cat > '/etc/app/config.yaml' <<'FORJAR_EOF'
host: db.internal
password: {{ secrets.db_password }}
FORJAR_EOF

# After stripping (for lint only):
cat > '/etc/app/config.yaml' <<'FORJAR_EOF'
# payload stripped for lint
FORJAR_EOF
```

The original payloads are preserved for execution — only the lint input is sanitized.

### bashrs Integration Points

| Module | Usage |
|--------|-------|
| `src/core/purifier.rs` | Core pipeline: validate, lint, purify |
| `src/transport/mod.rs` | I8 gate: `validate_before_exec()` |
| `src/cli/lint.rs` | `forjar lint` command: reports per-resource diagnostics |
| `src/core/codegen/dispatch.rs` | Script generation with post-generation validation |
| `src/core/store/derivation.rs` | Store derivation script validation |

### `forjar lint` Command

```bash
forjar lint config.yaml              # Lint all resource scripts
forjar lint config.yaml --json       # Structured output
forjar lint config.yaml --strict     # Additional policy checks
forjar lint config.yaml --fix        # Auto-fix (sort keys, etc.)
```

**Strict mode** adds policy checks beyond bashrs:
- Root-owned files must be tagged `system`
- All resources must have tags
- Privileged containers flagged
- SSH key checks

### Performance Targets

| Operation | Target | Mechanism |
|-----------|--------|-----------|
| `validate_script()` | <1ms | Regex-based linting, no AST |
| `purify_script()` | <5ms | Full parse → transform → format |
| `forjar lint` (50 resources) | <500ms | 3 scripts per resource (check, apply, state_query) |

---

## FJ-2401: apr Model Compilation

### Pipeline

```
apr pull <repo>  →  apr convert  →  apr compile  →  apr serve
   ↓                    ↓               ↓               ↓
 Download           Format/           Standalone      HTTP API
 + cache           quantize           binary          (OpenAI-compat)
```

### Stage 1: Pull (Download + Cache)

```bash
apr pull 'TheBloke/Llama-2-7B-GGUF'
```

- Downloads model from HuggingFace (or URL, or local path)
- Caches to `~/.cache/apr/` (configurable via `cache_dir`)
- Output includes `Path: /cached/file/path` — forjar parses this line
- ANSI escape codes stripped: `sed 's/\x1b\[[0-9;]*m//g'`
- Symlinks cached file to target: `ln -sf "$CACHED" '/models/llama.gguf'`

**Forjar integration** (`src/resources/model.rs`):
```
fn apply_script(resource):
    APR_OUT=$(apr pull '{source}' 2>&1)
    CACHED=$(echo "$APR_OUT" | sed 's/\x1b\[[0-9;]*m//g' | grep 'Path:' | head -1 | sed 's/.*Path: *//')
    ln -sf "$CACHED" '{path}'
```

Fallback: if `apr` is not installed, tries `huggingface-cli download`.

### Stage 2: Convert (Format + Quantize)

```bash
apr convert model.gguf --format apr --quantization q4_k_m --compression zstd
```

| Option | Values | Effect |
|--------|--------|--------|
| `--format` | `gguf`, `safetensors`, `apr` | Output format |
| `--quantization` | `int8`, `int4`, `fp16`, `q4k`, `q4_k_m`, `q5_k_m`, `q8_0` | Precision reduction |
| `--compression` | `zstd`, `zstd-max`, `lz4` | On-disk compression |

### Stage 3: Compile (Bundle into Executable)

```bash
apr compile model.apr --target x86_64-linux-gnu --release --strip --lto
```

Generates an ephemeral Cargo project with `include_bytes!` embedding the model, then runs `cargo build --release`.

| Target | Triple | Notes |
|--------|--------|-------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | Primary production target |
| Linux aarch64 | `aarch64-unknown-linux-gnu` | Jetson, Graviton |
| macOS x86_64 | `x86_64-apple-darwin` | Intel Macs |
| macOS aarch64 | `aarch64-apple-darwin` | Apple Silicon |
| WASM (browser) | `wasm32-unknown-unknown` | Via presentar |
| WASM (server) | `wasm32-wasi` | WASI runtime |

Output: self-contained binary at `/opt/apr/bin/model-server` (or specified path).

### Stage 4: Serve (HTTP Inference)

```bash
apr serve --model /models/llama.gguf --port 8080 --workers 4 --device 0
```

- OpenAI-compatible REST API (`/predict`, `/generate`, `/health`, `/metrics`)
- CUDA device selection via `CUDA_VISIBLE_DEVICES`
- Streaming output support
- Systemd service integration via forjar `type: service` resource

### Forjar Model Resource

```yaml
resources:
  llama-7b:
    type: model
    source: "TheBloke/Llama-2-7B-GGUF"
    path: /models/llama-7b.gguf
    format: gguf
    quantization: q4_k_m
    checksum: abc123def456...   # BLAKE3 hex digest
    cache_dir: /opt/cache
    owner: apr
    state: present
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `source` | Yes | — | HuggingFace repo ID, HTTP URL, or local path |
| `path` | Yes | — | Destination path on target machine |
| `format` | No | `gguf` | Model format: `gguf`, `safetensors`, `apr` |
| `quantization` | No | — | Quantization level |
| `checksum` | No | — | Expected BLAKE3 digest for verification |
| `cache_dir` | No | `~/.cache/apr/` | Local cache directory |
| `owner` | No | — | File ownership |

### Drift Detection for Models

```bash
# state_query_script output:
model=llama-7b:size=4294967296:hash=abc123def456...

# Missing model:
model=MISSING:llama-7b
```

- File size via `stat -c%s` (Linux) or `stat -f%z` (macOS)
- BLAKE3 hash via `b3sum <path> | cut -d' ' -f1`
- Hash comparison detects corruption, truncation, or unauthorized replacement

### Import Provider Integration

```rust
pub enum ImportProvider {
    Apt, Cargo, Uv, Nix, Docker, Tofu, Terraform, Apr,
}
```

| Provider | Command | Provenance | Capture |
|----------|---------|------------|---------|
| `Apr` | `apr pull <reference>` | `apr:model-name@version` | Model artifacts (gguf, safetensors) |

Apr imports flow through the universal store — after import, all artifacts are BLAKE3-hashed and provider-agnostic.

### Full Inference Stack Recipe

A complete GPU inference deployment (`apr-inference-server.yaml`) composes 8 resources:

```
gpu-driver (type: package)
    └─→ model-download (type: model, apr pull)
            └─→ apr-data-dir (type: file, directory)
                    └─→ apr-service-user (type: user)
                            └─→ apr-systemd-unit (type: file)
                                    └─→ apr-serve (type: service)
                                            └─→ apr-firewall (type: network)
                                                    └─→ apr-health-check (type: cron)
```

Template variables: `{{inputs.model_source}}`, `{{inputs.port}}`, `{{inputs.gpu_device}}`, `{{inputs.quantization}}`, `{{inputs.workers}}`.

---

## FJ-2402: WASM Deployment via Presentar

### What Presentar Is

Presentar is a WASM-first visualization framework for the Sovereign AI Stack. It compiles Rust to WebAssembly and deploys to CDN-backed static hosting. Forjar does not depend on presentar directly, but they operate in the same ecosystem:

- **Forjar** provisions machines, manages models, converges infrastructure
- **Presentar** deploys browser UIs — dashboards, monitoring, model interaction

### Build Pipeline

```
Rust source  →  wasm-pack  →  wasm-opt  →  HTML bundle  →  deploy
                (compile)     (optimize)    (package)       (CDN)
```

**Step 1: Compile to WASM**
```bash
wasm-pack build --target web --release crates/presentar
```
- Target: `wasm32-unknown-unknown`
- Output: `www/pkg/` with `.wasm` binary + JavaScript glue code

**Step 2: Optimize**
```bash
wasm-opt -Oz pkg/presentar_bg.wasm -o pkg/presentar_bg.wasm
```

| Level | Flag | Use Case |
|-------|------|----------|
| Fast | `-O1` | Development |
| Balanced | `-O2` | CI |
| Max speed | `-O3` | Compute-heavy apps |
| Min size | `-Oz` | Production (default) |

**Step 3: Package**
Copy `www/index.html` + `www/pkg/` to `dist/` directory.

**Step 4: Deploy**
```bash
presentar deploy --source dist --target s3 --bucket my-bucket --distribution E1234567
```

### Size Budget

| Stage | Size | Method |
|-------|------|--------|
| Debug build | ~5 MB | Unoptimized |
| Release build | ~800 KB | `opt-level=3`, `lto=true` |
| wasm-opt -Oz | ~500 KB | WASM-specific dead code elimination |
| gzip (transfer) | ~150 KB | HTTP compression |

**Target**: Core <100 KB, widgets <150 KB, full app <500 KB.

### Deploy Targets

```bash
presentar deploy --source dist --target <target> [options]
```

| Target | Command | Options |
|--------|---------|---------|
| **S3** | `aws s3 cp` | `--bucket`, `--region`, `--distribution` (CloudFront) |
| **CloudFlare** | `npx wrangler pages deploy` | `--bucket` (project name) |
| **Vercel** | `vercel deploy` | (auto-detected) |
| **Netlify** | `netlify deploy` | (auto-detected) |
| **Local** | `cp -r` | `--bucket` (directory path) |

S3 deployment includes automatic CloudFront cache invalidation on `/*`.

### Cache Control Headers

| Extension | Cache-Control | TTL |
|-----------|--------------|-----|
| `.wasm`, `.js` (hashed) | `public, max-age=31536000, immutable` | 1 year |
| `.css`, fonts | `public, max-age=604800` | 7 days |
| Images | `public, max-age=86400` | 1 day |
| `.html` | `no-cache, must-revalidate` | 0 (always fresh) |

WASM binaries include content hashes in filenames — cache forever, invalidate via new filename.

### Integration Points with Forjar

| Use Case | How |
|----------|-----|
| Infrastructure dashboard | Presentar WASM app queries `forjar query --json` output |
| Drift visualization | Presentar renders drift findings from `state.db` |
| Model monitoring | Presentar displays `apr serve --metrics` endpoint |
| Deploy via forjar | `type: file` resource writes WASM bundle to web server path |

---

## FJ-2403: Forjar Self-Build

### Cargo Configuration

```toml
[package]
name = "forjar"
version = "1.1.1"
edition = "2021"
rust-version = "1.88.0"

[dependencies]
bashrs = "6.64.0"        # Shell purification
blake3 = "1.8"           # Content-addressed hashing
serde_yaml_ng = "0.10"   # YAML parsing
tokio = { version = "1.35", features = ["full"] }
clap = { version = "4", features = ["derive"] }

[features]
encryption = []           # Optional age encryption
container-test = []       # Container test harness
gpu-container-test = []   # GPU test harness
```

### Release Profile

```toml
[profile.release]
opt-level = 3       # Maximum optimization
lto = true           # Link-time optimization (cross-crate inlining)
codegen-units = 1    # Single codegen unit (better optimization, slower compile)
strip = true         # Strip debug symbols
panic = "abort"      # No unwinding (smaller binary)
```

### Workspace Lints

```toml
[workspace.lints.rust]
unsafe_code = "forbid"                    # Zero unsafe
unexpected_cfgs = { level = "allow",
  check-cfg = ['cfg(kani)', 'cfg(verus)'] }  # Gated formal verification

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
checked_conversions = "warn"
```

### Binary Size Targets

| Build | Target Size | Mechanism |
|-------|------------|-----------|
| Debug | <50 MB | Default Cargo settings |
| Release | <10 MB | LTO + strip + abort |
| Release (musl) | <15 MB | Static linking for portability |

### CI Matrix

| Axis | Values |
|------|--------|
| Platform | ubuntu-latest, macos-latest, windows-latest |
| Rust | stable, MSRV (1.88.0) |
| Features | default, `encryption`, `container-test` |
| Checks | fmt, clippy, test, doc, coverage, audit |

### Quality Gates (Pre-Merge)

| Gate | Command | Threshold |
|------|---------|-----------|
| Format | `cargo fmt --check` | Zero diff |
| Lint | `cargo clippy -- -D warnings` | Zero warnings |
| Test | `cargo test` | All pass |
| Coverage | `cargo llvm-cov --summary-only` | >= 95% lines |
| Audit | `cargo audit` | Zero advisories |
| File size | Pre-commit hook | <= 500 lines per source file |
| Complexity | Pre-commit hook | Cyclomatic <= 30, cognitive <= 25 |

---

## Cross-Cutting: Compilation Dependency Graph

```
                bashrs 6.64.0 (crates.io)
                    │
    ┌───────────────┤
    │               │
    ▼               ▼
forjar          apr-cli (aprender)
    │               │
    │               ├── apr pull (download)
    │               ├── apr convert (quantize)
    │               ├── apr compile (bundle)
    │               └── apr serve (inference)
    │
    ├── forjar apply (converge resources)
    │       └── transport gate: bashrs validate
    │
    ├── forjar lint (validate scripts)
    │       └── bashrs lint_script()
    │
    └── forjar build (OCI images)
            └── layer scripts: bashrs purified

presentar (separate binary)
    ├── presentar bundle (WASM compile)
    ├── presentar deploy (CDN push)
    └── presentar serve (dev server)
```

### Version Compatibility

| Component | Minimum Version | Reason |
|-----------|----------------|--------|
| bashrs | 6.64.0 | Purification API stability |
| apr-cli | 0.4.10 | `Path:` output format |
| presentar | 0.3.5 | Deploy command API |
| Rust | 1.88.0 | Edition 2021 + feature requirements |
| wasm-pack | 0.13+ | `--target web` support |
| wasm-opt | 116+ | `-Oz` optimization level |

---

## Implementation

### Phase 19: bashrs Purification Spec (FJ-2400)
- [x] `PurificationBenchmark` with validate_us, purify_us, overhead_ratio()
- [x] Add `forjar lint --bashrs-version` to report bashrs version
- [x] Benchmark purification: `PurificationBenchmark` with per-resource-type timing
- [x] Add bashrs version to generation metadata for reproducibility
- **Deliverable**: I8 enforcement documented and measurable

### Phase 20: apr Model Pipeline (FJ-2401)
- [x] `apr compile` integration in forjar recipes — `examples/apr-compile-integration.yaml` (pull → compile → checksum → serve)
- [x] Cross-compilation matrix in `apr-crosscompile-matrix.yaml` — x86_64 + aarch64 parallel builds with verification
- [x] Model checksum verification: `ModelIntegrityCheck` with BLAKE3 hash comparison
- [ ] `forjar query --type model --drift` for model integrity monitoring
- **Deliverable**: Full pull-convert-compile-serve pipeline in forjar recipes

### Phase 21: WASM Deployment (FJ-2402) -- PARTIAL
- [x] WASM types: `WasmOptLevel`, `WasmBuildConfig`, `WasmSizeBudget`, `WasmBuildResult`
- [x] CDN deploy targets: `CdnTarget` (S3/Cloudflare/Local) with Display
- [x] Cache policies: `CachePolicy::defaults()` per extension
- [x] Size budget checks: `check_core()`, `check_full_app()`
- [ ] `type: wasm_bundle` resource for deploying presentar apps via forjar
- [x] S3 deployment with CloudFront invalidation via forjar resources — `examples/s3-cloudfront-deploy.yaml` (build → size-check → s3-sync → invalidation)
- [x] Bundle size drift detection: `BundleSizeDrift::check()` with budget + 20% growth limit alerting
- **Deliverable**: Presentar WASM apps deployable via `forjar apply`

### Phase 22: Self-Build Hardening (FJ-2403)
- [x] Reproducible builds: `ReproBuildConfig` with locked, LTO, codegen_units, `cargo_args()`, `env_vars()`
- [x] Binary size tracking per release (`BuildMetrics`, `SizeThreshold` types)
- [x] `BuildMetrics::current()` compile-time metric collection
- [x] `SizeThreshold::check()` regression detection with absolute + growth limits
- [x] `BuildMetrics::size_change_pct()` for release-over-release comparison
- [x] `BuildMetrics::format_summary()` human-readable build report
- [x] MSRV CI enforcement: `MsrvCheck` with `satisfies()` version comparison
- [x] Feature flag matrix: `FeatureMatrix` with `combinations()` and `cargo_commands()`
- **Deliverable**: Every forjar release is reproducible and size-tracked
