# Contributing to Forjar

## Development Setup

```bash
# Clone and build
git clone https://github.com/paiml/forjar.git
cd forjar
cargo build

# Run tests
cargo test

# Run lints
cargo clippy -- -D warnings
cargo fmt --check
```

## Quality Gates

All contributions must pass:

- **Tests**: `cargo test` (8000+ tests, 95%+ line coverage)
- **Clippy**: `cargo clippy -- -D warnings` (zero warnings)
- **Format**: `cargo fmt --check`
- **Deny**: `cargo deny check` (supply chain security)
- **File size**: No source file over 500 lines
- **Complexity**: All functions TDG grade A (cyclomatic <= 10)

## Pull Request Process

1. Fork the repository and create your branch from `main`
2. Add tests for any new functionality
3. Ensure all quality gates pass
4. Update CHANGELOG.md with your changes
5. Submit a pull request with a clear description
