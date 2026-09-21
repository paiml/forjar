# Claims — the v1.32.0 booking, and Coverage and Benchmarks on tagged releases only (PMAT-604)

Counts and heads live in `PMAT-604-lanes.md`.

- **C1** the v1.32.0 row is what `release-goal.sh cut` measured, and cookbook a8e758ec admits and pins 1.32.0
- **C2** the nine gates on 2a39ed91 transfer to the tag by tree identity with b12a8392
- **C3** coverage.yml and bench.yml run only on a v* tag push and dispatch, and their heavy jobs still run on the tag
- **C4** ci.yml's two edits are both needed and together sufficient for coverage on a release and NOT MEASURED elsewhere
- **C5** the falsification test reads parsed YAML, refuses the old shapes and half opt-ins, and each mutation reddens one test
- **C6** the roadmap edits are correct and weaken no gate

The lanes were pointed at the writing as well as the diff, and each brief ended
with a hostile-reader instruction to quote any sentence a reader could check and
find false. That instruction produced the C2 refutation from all three lanes.
