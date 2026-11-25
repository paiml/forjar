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

- **Tests**: `cargo test` (12000+ tests, 95%+ line coverage)
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

## Filing Tickets

The tracker has a cap of 50 open tickets. File complete or not at all:

1. **Look for the group ticket first.** If an open group, master or train ticket covers the work, add a checklist line there instead of opening a new ticket.
2. **File with the helper:** `scripts/file-ticket.sh --parent <N> --title "..." --body-file body.md`. It creates the ticket, links it as a sub-issue of its parent, sets the parent's milestone and adds `triaged`. It refuses when no parent is given.
3. **At 50 open, fold or close another ticket** before filing a new one.
4. **Review follow-ups, CI reds and flake reports** go into the existing master as a checklist line, unless a PR will close them on its own.

`scripts/ticket-triage-count.sh` prints the nightly count (open, no_parent, no_milestone, no_triaged, over_cap). It is a nightly report, not a PR check; `not_measured` (rc 3) means the read failed.
