# 18: Benchmark Framework & Colorized Output UX

> Performance regression gates, semantic color system, OutputWriter abstraction, and Criterion.rs integration.

**Spec IDs**: FJ-2900 (benchmarks), FJ-2910 (color system), FJ-2920 (OutputWriter) | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## Motivation

Forjar's CLI output uses 5 ad-hoc ANSI helpers (`green`, `red`, `yellow`, `dim`, `bold`) scattered across command handlers. pmat's CLI (`paiml-mcp-agent-toolkit`) demonstrates a mature pattern: a `colors.rs` module with semantic helpers (`pass`, `fail`, `warn`, `grade`, `pct`, `delta`), an `OutputWriter` trait for testability, and Criterion.rs benchmarks with quality gates. Forjar should adopt these patterns for consistency, testability, and performance accountability.

---

## Part 1: Semantic Color System (FJ-2910)

### Current State

`src/cli/helpers.rs` provides 5 raw ANSI wrappers:

```rust
pub(crate) fn green(s: &str) -> String { ... }
pub(crate) fn red(s: &str) -> String { ... }
pub(crate) fn yellow(s: &str) -> String { ... }
pub(crate) fn dim(s: &str) -> String { ... }
pub(crate) fn bold(s: &str) -> String { ... }
```

Problems:
1. **No semantic meaning** — callers must know "green = pass, red = fail"
2. **No combined styles** — no `bold_red`, `dim_cyan`, `bold_green`
3. **No grade coloring** — `forjar score` hardcodes ANSI codes inline
4. **No threshold coloring** — percentage/delta values aren't color-coded
5. **No structural elements** — no rule/separator helpers
6. **Inconsistent icons** — `[pass]` vs `✓` vs `✅` across commands

### Target: `src/cli/colors.rs`

Mirror pmat's `colors.rs` pattern — ANSI constants + semantic helpers, all respecting `NO_COLOR`:

```rust
//! ANSI color constants and semantic formatting helpers.
//!
//! All CLI handlers use these for consistent colorized output.
//! The global NO_COLOR flag disables all ANSI sequences.

use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) static NO_COLOR: AtomicBool = AtomicBool::new(false);

fn enabled() -> bool { !NO_COLOR.load(Ordering::Relaxed) }
fn wrap(code: &str, text: &str) -> String {
    if enabled() { format!("{code}{text}\x1b[0m") } else { text.to_string() }
}

// ── ANSI constants ──────────────────────────────────────────

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const ITALIC: &str = "\x1b[3m";
pub const UNDERLINE: &str = "\x1b[4m";

pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
pub const CYAN: &str = "\x1b[36m";

pub const BOLD_RED: &str = "\x1b[1;31m";
pub const BOLD_GREEN: &str = "\x1b[1;32m";
pub const BOLD_YELLOW: &str = "\x1b[1;33m";
pub const BOLD_CYAN: &str = "\x1b[1;36m";
pub const BOLD_WHITE: &str = "\x1b[1;37m";
pub const DIM_WHITE: &str = "\x1b[2;37m";
pub const DIM_CYAN: &str = "\x1b[2;36m";

// ── Raw wrappers ────────────────────────────────────────────

pub fn green(s: &str) -> String { wrap(GREEN, s) }
pub fn red(s: &str) -> String { wrap(RED, s) }
pub fn yellow(s: &str) -> String { wrap(YELLOW, s) }
pub fn blue(s: &str) -> String { wrap(BLUE, s) }
pub fn cyan(s: &str) -> String { wrap(CYAN, s) }
pub fn dim(s: &str) -> String { wrap(DIM, s) }
pub fn bold(s: &str) -> String { wrap(BOLD, s) }

// ── Semantic helpers ────────────────────────────────────────

/// Section header: bold + underline
pub fn header(text: &str) -> String {
    if enabled() { format!("{BOLD}{UNDERLINE}{text}{RESET}") } else { text.to_string() }
}

/// Pass indicator: ✓ green
pub fn pass(text: &str) -> String {
    if enabled() { format!("{GREEN}✓{RESET} {text}") } else { format!("✓ {text}") }
}

/// Warning indicator: ⚠ yellow
pub fn warn(text: &str) -> String {
    if enabled() { format!("{YELLOW}⚠{RESET} {text}") } else { format!("⚠ {text}") }
}

/// Failure indicator: ✗ red
pub fn fail(text: &str) -> String {
    if enabled() { format!("{RED}✗{RESET} {text}") } else { format!("✗ {text}") }
}

/// Skipped indicator: ⏭ dim
pub fn skip(text: &str) -> String {
    if enabled() { format!("{DIM}⏭{RESET} {DIM}{text}{RESET}") }
    else { format!("⏭ {text}") }
}

/// Grade coloring: A/B=green, C=yellow, D/F=red
pub fn grade(g: &str) -> String {
    let color = match g.chars().next() {
        Some('A') => GREEN,
        Some('B') => GREEN,
        Some('C') => YELLOW,
        Some('D') => RED,
        Some('F') => BOLD_RED,
        _ => "",
    };
    wrap(color, g)
}

/// Threshold-colored percentage (higher is better)
pub fn pct(value: f64, good: f64, warn_at: f64) -> String {
    let s = format!("{value:.1}%");
    let color = if value >= good { GREEN }
        else if value >= warn_at { YELLOW }
        else { RED };
    wrap(color, &s)
}

/// Delta coloring: positive=green, negative=red, zero=dim
pub fn delta(value: f64) -> String {
    let s = format!("{value:+.1}");
    let color = if value > 0.0 { GREEN }
        else if value < 0.0 { RED }
        else { DIM };
    wrap(color, &s)
}

/// Score fraction: "earned/max" with threshold coloring
pub fn score_frac(earned: f64, max: f64, good_pct: f64, warn_pct: f64) -> String {
    let pct = if max > 0.0 { earned / max * 100.0 } else { 0.0 };
    let color = if pct >= good_pct { GREEN }
        else if pct >= warn_pct { YELLOW }
        else { RED };
    if enabled() {
        format!("{color}{earned:.1}{RESET}/{DIM}{max:.1}{RESET}")
    } else {
        format!("{earned:.1}/{max:.1}")
    }
}

/// Heavy horizontal rule (━━━)
pub fn rule() -> String {
    dim(&"━".repeat(60))
}

/// Light separator (───)
pub fn separator() -> String {
    dim(&"─".repeat(60))
}

/// File path (cyan, matching rg/fd convention)
pub fn path(text: &str) -> String { wrap(CYAN, text) }

/// Duration coloring: fast=green, slow=yellow, very slow=red
pub fn duration(secs: f64, target_secs: f64) -> String {
    let s = if secs >= 1.0 { format!("{secs:.2}s") }
        else if secs >= 0.001 { format!("{:.1}ms", secs * 1_000.0) }
        else { format!("{:.1}µs", secs * 1_000_000.0) };
    let color = if secs <= target_secs { GREEN }
        else if secs <= target_secs * 2.0 { YELLOW }
        else { RED };
    wrap(color, &s)
}
```

### Migration Plan

1. Create `src/cli/colors.rs` with all constants and helpers
2. Move `NO_COLOR` and `color_enabled()` from `helpers.rs` to `colors.rs`
3. Re-export from `helpers.rs` for backward compatibility: `pub(crate) use colors::*;`
4. Grep for inline ANSI codes (`\x1b[`) and replace with semantic helpers
5. Standardize icons: `✓` (pass), `✗` (fail), `⚠` (warn), `⏭` (skip)

### Icon Standardization

| Meaning | Before (inconsistent) | After |
|---------|----------------------|-------|
| Pass | `[pass]`, `✅`, `[PASS]` | `✓` (green) |
| Fail | `[fail]`, `[31m✗`, `❌` | `✗` (red) |
| Warning | `[warn]`, `⚠️` | `⚠` (yellow) |
| Skip | `⏭️`, `(skipped)` | `⏭` (dim) |
| Info | `🔍`, `ℹ` | `·` (dim) |
| Error | `error:`, `[31m` | `✗` (bold red) |

---

## Part 2: OutputWriter Abstraction (FJ-2920)

### Problem

All 138 commands use `println!`/`eprintln!` directly. This makes:
- **Testing impossible** — can't assert on output without capturing stdout
- **Benchmarking noisy** — benchmarks include I/O overhead
- **Redirection fragile** — no separation of data (stdout) vs status (stderr)

### Target: `src/cli/output.rs`

```rust
//! Output abstraction for CLI handlers.
//!
//! Enables testable handlers by injecting an OutputWriter.
//! Production code uses StdoutWriter, tests use TestWriter,
//! benchmarks use NullWriter.

pub trait OutputWriter {
    fn status(&mut self, msg: &str);    // Progress → stderr
    fn result(&mut self, msg: &str);    // Data → stdout
    fn warning(&mut self, msg: &str);   // ⚠ → stderr
    fn error(&mut self, msg: &str);     // ✗ → stderr
    fn success(&mut self, msg: &str);   // ✓ → stderr
    fn flush(&mut self);
}

pub struct StdoutWriter;
pub struct TestWriter { ... }  // Captures all output for assertions
pub struct NullWriter;          // Discards (for benchmarks)
```

### Adoption Strategy

This is **incremental** — not a big-bang rewrite. New commands and commands being refactored should accept `&mut dyn OutputWriter`. Existing commands continue using `println!` until touched.

Priority order for OutputWriter adoption:
1. `cmd_bench` — immediate benefit (NullWriter for benchmark overhead)
2. `cmd_score` — grade coloring benefits from semantic helpers
3. `cmd_doctor` — pass/fail/warn icons
4. `apply` output — resource table with status coloring
5. `state-query` output — table formatting

---

## Part 3: Benchmark Framework (FJ-2900)

### Current State

Two separate benchmark systems:
1. **`forjar bench` CLI** — 4 targets (validate, plan, drift, blake3), simple loop timing
2. **`benches/store_bench.rs`** — 10 Criterion.rs targets for store operations

Problems:
1. CLI bench uses wall-clock loop — no statistical rigor (no warmup, no CI, no outlier detection)
2. No regression gates — benchmarks run but nothing prevents regressions
3. No Criterion.rs for core operations (parse, plan, codegen, apply output)
4. No comparison against baselines

### Target Architecture

```
benches/
├── core_bench.rs       # Existing: BLAKE3, YAML parse, topo sort
├── store_bench.rs      # Existing: store operations (Phase J)
├── cli_bench.rs        # NEW: parse → plan → codegen pipeline
└── output_bench.rs     # NEW: table formatting, color rendering
```

### New Criterion.rs Benchmarks (`benches/cli_bench.rs`)

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_validate(c: &mut Criterion) {
    let mut group = c.benchmark_group("validate");
    for (label, n_resources) in [("small-5", 5), ("medium-20", 20), ("large-100", 100)] {
        let yaml = generate_config(n_resources);
        group.bench_with_input(
            BenchmarkId::new("parse_and_validate", label),
            &yaml,
            |b, yaml| b.iter(|| parse_and_validate_from_str(yaml)),
        );
    }
    group.finish();
}

fn bench_plan(c: &mut Criterion) {
    let mut group = c.benchmark_group("plan");
    for (label, n) in [("5r", 5), ("20r", 20), ("100r", 100)] {
        let config = build_config(n);
        let order = build_execution_order(&config).unwrap();
        group.bench_with_input(
            BenchmarkId::new("plan", label),
            &(&config, &order),
            |b, (config, order)| {
                let locks = HashMap::new();
                b.iter(|| plan(config, order, &locks, None));
            },
        );
    }
    group.finish();
}

fn bench_codegen(c: &mut Criterion) {
    // Benchmark script generation for each resource type
    let mut group = c.benchmark_group("codegen");
    for rt in ["file", "package", "service", "cron", "mount", "user"] {
        let resource = build_resource(rt);
        group.bench_function(
            BenchmarkId::new("generate_script", rt),
            |b| b.iter(|| generate_apply_script(&resource)),
        );
    }
    group.finish();
}

fn bench_blake3(c: &mut Criterion) {
    let mut group = c.benchmark_group("blake3");
    for (label, size) in [("1KB", 1024), ("4KB", 4096), ("1MB", 1_048_576)] {
        let data = "x".repeat(size);
        group.bench_with_input(
            BenchmarkId::new("hash_string", label),
            &data,
            |b, data| b.iter(|| hash_string(data)),
        );
    }
    group.finish();
}

criterion_group!(benches, bench_validate, bench_plan, bench_codegen, bench_blake3);
criterion_main!(benches);
```

### Performance Targets

| Operation | Size | Target | Notes |
|-----------|------|--------|-------|
| `parse_and_validate` | 5 resources | < 500µs | YAML parse + validation |
| `parse_and_validate` | 20 resources | < 2ms | Current bench target |
| `parse_and_validate` | 100 resources | < 10ms | Realistic large config |
| `plan` | 5 resources | < 100µs | Hash compare + ordering |
| `plan` | 20 resources | < 500µs | |
| `plan` | 100 resources | < 5ms | DAG resolution + diff |
| `codegen` (per type) | 1 resource | < 50µs | Script string building |
| `blake3 hash` | 1KB | < 1µs | SIMD-accelerated |
| `blake3 hash` | 4KB | < 2µs | |
| `blake3 hash` | 1MB | < 500µs | |
| `drift detect` | 100 resources | < 1ms | Lock file comparison |
| `topo sort` | 100 nodes | < 100µs | DAG ordering |
| `FTS5 query` | 1000 resources | < 10ms | SQLite in-process |
| `score` | 20 resources | < 5ms | 8-dimension calculation |

### Quality Gates

Benchmark regressions block CI:

```yaml
# .github/workflows/bench.yml
bench-gate:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - run: cargo bench --bench cli_bench -- --save-baseline main
    - run: |
        # Compare against baseline
        cargo bench --bench cli_bench -- --baseline main --output-format bencher \
          | python3 scripts/bench-gate.py --max-regression 20
```

| Metric | Regression Threshold | Action |
|--------|---------------------|--------|
| Validate (20r) | +20% | Warn |
| Plan (20r) | +20% | Warn |
| Blake3 (4KB) | +50% | Block |
| Any operation | +100% | Block |

### Enhanced `forjar bench` CLI Output

The existing `forjar bench` command gets colorized output:

```
$ forjar bench --iterations 1000

Forjar Performance Benchmarks (1000 iterations)

  Operation                     Average       Target       Status
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  validate (3m, 20r)           1.2ms         < 10ms       ✓ pass
  plan (3m, 20r)               0.4ms         < 2s         ✓ pass
  drift (100 resources)        0.8ms         < 1s         ✓ pass
  blake3 hash (4KB)            0.3µs         < 1µs        ✓ pass
  codegen (file)               12.3µs        < 50µs       ✓ pass
  codegen (package)            18.7µs        < 50µs       ✓ pass
  score (20r)                  2.1ms         < 5ms        ✓ pass
  topo sort (100 nodes)        45.2µs        < 100µs      ✓ pass
  ───────────────────────────────────────────────────────────────
  8/8 targets met

$ forjar bench --json | jq '.[0]'
{
  "name": "validate (3m, 20r)",
  "target_us": 10000,
  "avg_us": 1234.5,
  "min_us": 1100.0,
  "max_us": 1500.0,
  "p50_us": 1200.0,
  "p95_us": 1450.0,
  "iterations": 1000,
  "status": "pass"
}
```

### Benchmark Result Tracking

Store benchmark results in `benchmarks/RESULTS.md` (auto-updated by `make bench-update`):

```markdown
<!-- BENCH-TABLE-START -->
| Operation | Target | Last Run | Status |
|-----------|--------|----------|--------|
| validate (20r) | < 10ms | 1.2ms | ✓ |
| plan (20r) | < 2s | 0.4ms | ✓ |
| drift (100r) | < 1s | 0.8ms | ✓ |
| blake3 (4KB) | < 1µs | 0.3µs | ✓ |
<!-- BENCH-TABLE-END -->
```

---

## Part 4: UX Polish Catalog

### Table Formatting Conventions

All CLI tables follow this pattern:

```
COLUMN1              COLUMN2    COLUMN3       COLUMN4
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
value1               value2     value3        value4
value5               value6     value7        value8
───────────────────────────────────────────────────────
TOTAL                count      aggregate     summary
```

Rules:
- Header row: **bold**
- Heavy rule (`━`) after header
- Light rule (`─`) before totals
- Status values: colored by meaning (converged=green, failed=red, drifted=yellow)
- Numbers: right-aligned
- Strings: left-aligned
- Durations: right-aligned, colored by target (green=fast, yellow=slow, red=very slow)

### Box Drawing Characters

| Character | Unicode | Usage |
|-----------|---------|-------|
| `━` | U+2501 | Heavy rule (after headers) |
| `─` | U+2500 | Light separator (before totals) |
| `│` | U+2502 | Vertical separator (between columns) |
| `┌┐└┘` | U+250C etc. | Box corners (detail panels) |

### Command Output Consistency Matrix

| Command | Header | Status Icons | Grade Colors | Duration Colors | Table Rules |
|---------|--------|-------------|-------------|-----------------|-------------|
| `bench` | ✓ | ✓ pass/fail | — | ✓ | ✓ heavy+light |
| `score` | ✓ | — | ✓ A-F | — | ✓ bar chart |
| `doctor` | ✓ | ✓ pass/warn/fail | — | — | — |
| `validate` | — | ✓ pass/fail | — | — | — |
| `status` | ✓ | — | — | ✓ | ✓ heavy+light |
| `state-query --health` | ✓ | — | — | — | ✓ heavy+light |
| `apply` | ✓ | ✓ converged/failed | — | ✓ | ✓ heavy+light |
| `security-scan` | ✓ | ✓ severity | — | — | ✓ heavy |
| `lock-verify` | — | ✓ pass/fail | — | — | — |
| `plan` | ✓ | ✓ +/~/- | — | — | — |

---

## Implementation

### Phase 18a: Color System (FJ-2910)
- [x] Create `src/cli/colors.rs` with ANSI constants and semantic helpers
- [x] Move `NO_COLOR` from `helpers.rs` to `colors.rs`
- [x] Add `grade()`, `pct()`, `delta()`, `score_frac()`, `duration()` helpers
- [x] Add `pass()`, `fail()`, `warn()`, `skip()` icon helpers
- [x] Add `header()`, `rule()`, `separator()`, `path()` structural helpers
- [x] Re-export from `helpers.rs` for backward compatibility
- [x] Replace inline ANSI codes across codebase with semantic helpers
- [x] Standardize icons across all commands
- **Deliverable**: All CLI output uses semantic color helpers; `--no-color` disables everything

### Phase 18b: OutputWriter (FJ-2920)
- [x] Create `src/cli/output.rs` with `OutputWriter` trait
- [x] Implement `StdoutWriter`, `TestWriter`, `NullWriter`
- [x] Adopt in `cmd_bench` (NullWriter eliminates I/O overhead)
- [x] Adopt in `cmd_score` (TestWriter enables output assertions)
- [x] Adopt in `cmd_lint` (TestWriter enables output assertions)
- [ ] Adopt in `cmd_doctor` (TestWriter enables output assertions)
- **Deliverable**: Test coverage for command output content; benchmark accuracy improved

### Phase 18c: Benchmark Framework (FJ-2900)
- [x] Create `benches/cli_bench.rs` with Criterion.rs groups: codegen, score, pipeline
- [x] Parameterize by config size (5/20/100 resources)
- [x] Expand `forjar bench` to 6 targets (validate, plan, drift, blake3 4KB, topo sort, blake3 1MB)
- [x] Colorize bench output (pass/fail against targets, duration coloring)
- [ ] Add `--compare` flag to compare against stored baseline
- [ ] Add percentile stats (p50, p95) to JSON output
- [ ] Create `benchmarks/RESULTS.md` with auto-update markers
- [ ] Create `make bench-update` target
- **Deliverable**: `cargo bench` with quality gates; `forjar bench` with rich colorized output
