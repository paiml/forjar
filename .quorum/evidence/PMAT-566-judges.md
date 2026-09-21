# Judges — the MSRV job left a toolchain override behind (PMAT-566)

Round count and heads: see `PMAT-566-lanes.md`.

## CONFIRMED

1. [fleet] C1 — Running rustup's override-set command records a mapping against the absolute workspace path that persists until it is explicitly removed, so on a runner whose workspace directory is reused across jobs it affects every job that follows there.
   - evidence: the Claude lane observed an override surviving separate invocations until unset, and the test at `tests/falsification_no_workflow_leaves_a_toolchain_override.rs:89` refuses any workflow line that is that command.
2. [fleet] C2 — A directory override outranks the repository's toolchain file while the toolchain environment variable outranks a directory override, which the Claude lane measured in a scratch directory rather than reciting, and on which the whole fix rests.
   - evidence: 1.91.0 reported as a directory override over a toolchain file naming 1.95.0, then 1.97.1 reported as overridden by the environment variable once it was set.
3. [msrv] C3 — The msrv job still tests exactly the minimum supported version the crate declares, because its job-level toolchain variable is 1.89.0 and the manifest's rust-version is 1.89.0, and a test fails if the two ever disagree.
   - evidence: the equality assertion at `tests/falsification_no_workflow_leaves_a_toolchain_override.rs:134` reads both files independently, from the parsed workflow and from the manifest.
4. [test] C6 — Each of the three declared mutations reddens exactly its own test, because the three assertions read disjoint files and fields and so cannot trip one another, which was traced through the test code and measured against restored copies.
   - evidence: the ordering assertion at `tests/falsification_no_workflow_leaves_a_toolchain_override.rs:161` and the scan-coverage floor at `tests/falsification_no_workflow_leaves_a_toolchain_override.rs:101` are the two arms most likely to cross, and they read different workflows.

## REFUTED

1. [claim] C4 — The claim that after this change nothing in the workflows is skipped, waived or continue-on-error'd is false as written, because the mutation workflow's existing test step still carries continue-on-error, a pre-existing setting this branch neither added nor touched.
   - corrected: recorded as a wording refutation raised independently by all three lanes; the commit message never made the claim, and the lines this branch adds carry no such setting.
2. [quote] C5 — The claim that the commit message's quoted measurement shows the shared cause overstated it, since the quote contained only rustup's two lines reporting the override as active and none of the compile failure it caused.
   - corrected: the commit message now quotes the cargo-mutants compile error beside the override lines from the same job's log, so cause and symptom appear together.
