# Claims — the MSRV job left a toolchain override behind (PMAT-566)

Counts and heads live in `PMAT-566-lanes.md`.

- **C1** `rustup override set` writes a persistent directory override that outlives the job
- **C2** a directory override outranks rust-toolchain.toml, and RUSTUP_TOOLCHAIN outranks a directory override
- **C3** the msrv job still tests exactly the MSRV Cargo.toml declares, and a test asserts it
- **C4** no workflow writes an override, mutation clears a stale one, and nothing is skipped, waived or continue-on-error'd
- **C5** lint and mutation failed from one cause, and the commit's quoted measurement shows it
- **C6** each declared mutation reddens exactly its own test

The fix rests entirely on C2, so the lanes were asked to attack it hardest. The
Claude lane did not argue it from documentation — it built a scratch directory
with an override and a conflicting toolchain file and measured the precedence.
