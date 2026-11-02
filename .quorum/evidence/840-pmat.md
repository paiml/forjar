# pmat lane — #835

`pmat analyze vacuous-tests -p tests -f json` over the tests directory reported zero items in tests/falsification_840_advisory_ignores_are_bounded.rs. The test parses deny.toml, .cargo/audit.toml and Cargo.lock, and was shown green on the branch and red with one audit.toml ignore line deleted.
