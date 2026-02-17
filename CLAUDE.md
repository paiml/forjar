# Forjar Development Guidelines

## Code Search

NEVER use grep/glob for code search. ALWAYS prefer `pmat query`.

```bash
# Find functions by intent
pmat query "error handling" --limit 10

# Find with fault patterns (--faults)
pmat query "unwrap" --faults --exclude-tests

# Find coverage gaps
pmat query --coverage-gaps --limit 20 --exclude-tests
```

## Quality Gates

- All functions must be TDG grade A (complexity <= 10)
- Minimum 95% line coverage (`cargo llvm-cov`)
- Zero clippy warnings (`cargo clippy -- -D warnings`)
- Never use `cargo tarpaulin` — use `cargo llvm-cov` instead

## Testing

```bash
cargo test                              # Run all tests
cargo llvm-cov --summary-only           # Check coverage
cargo clippy -- -D warnings             # Lint check
```

## Architecture

- `src/core/` — Config parsing, planning, execution, state management
- `src/resources/` — Resource handlers (package, file, service, mount)
- `src/transport/` — Local and SSH execution
- `src/tripwire/` — Drift detection, hashing, event logging
- `src/cli/` — CLI commands and argument parsing
