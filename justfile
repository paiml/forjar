# Forjar development tasks

# Run all tests
test:
    cargo test --lib

# Run clippy lints
lint:
    cargo clippy --all-targets -- -D warnings

# Build release binary
build:
    cargo build --release

# Run benchmarks
bench:
    cargo bench

# Check formatting
fmt:
    cargo fmt --all -- --check

# Run coverage
coverage:
    cargo llvm-cov --summary-only

# Run audit
audit:
    cargo audit
    cargo deny check
