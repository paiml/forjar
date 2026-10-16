# CRUX — #624 nightly artifacts vs release artifacts

- ripgrep (BurntSushi): release CI ships x86_64-unknown-linux-musl statically
  linked; there is no nightly channel to keep in parity.
- rustup (Rust dist): nightly is built for every target stable ships (a superset);
  parity comes from one build pipeline, not from a test comparing two.
- Deno (canary builds): canary and release come from one workflow matrix, so a
  target cannot exist in one channel only.

Verdict: at the industry default for artifact parity (the others get it from a
single matrix; forjar has two workflows, so it gets it from a test that
compares them), above it in that the test fails loudly on a vacuous set.
