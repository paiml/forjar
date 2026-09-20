# Claims — lint toolchain override (PMAT-567)

Six claims about the diff, put to three independent lanes. Counts live in
`PMAT-567-lanes.md`; numbers here quote it.

- **C1** the toolchain step runs `rustup override unset` before clippy
- **C2** it refuses BY NAME, before the lint, when clippy cannot run
- **C3** no `|| true` on any measurement
- **C4** the test carries an anti-vacuity arm
- **C5** the RED proof is real and scoped to one test
- **C6** no `src/**.rs` changes

## What came back

**The lanes rewrote half the fix.** C2 as first implemented compared the active
toolchain's version against the pinned `channel`, and all three lanes refuted it
independently with three different counterexamples. C1 as worded overstated what
unsetting can reach. And the falsification test was found to be VACUOUS under its
own stated mutation.

Every one of those was a real defect in work that had already passed the
pre-commit hooks, `cargo fmt`, clippy and three green test runs.
