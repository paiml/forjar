# PMAT-547 — the claims put to the round

The branch moves every Linux runner declaration in `.github/workflows/` from a
GitHub-hosted label to `[self-hosted, clean-room]`, adds a falsification test
that parses the workflows to keep the count from growing back, and declares the
fleet's labels to actionlint.

Three lanes, read-only, against a full standalone clone at `4707bf24`.

LANE 1 — WILL THESE JOBS ACTUALLY RUN ON THE FLEET? The lane that mattered most,
because a label change that makes 33 jobs fail is worse than the problem it
fixes. Which steps assume a hosted environment (`sudo`, `apt-get`, preinstalled
tools, `$RUNNER_TEMP`, `runner.os`, a hardcoded hosted-runner home path, services)? Is the new
`musl-tools` guard correct or does it proceed silently to an obscure failure? Is
`cross` installed before it is used on the right legs, and what changes for the
Windows leg now that the build step gained `shell: bash`? Does the new glibc
step measure the right machine on a leg built inside a cross container? Is any
converted job one that cannot work on clean-room at all?

LANE 2 — THE TEST, AND WHAT IT LETS THROUGH. What shapes of `runs-on` does the
parser MISS — the object form with `group`/`labels`, an expression nested in a
list, a matrix key not named `runner`/`os`, a reusable-workflow job with no
`runs-on` at all? Is `hosted_label`'s prefix check case-correct? Is the expected
leg map right, counted from the YAML? Is any case satisfiable without the
behaviour it names — attack the vacuity threshold specifically. Do any of the
repository's existing workflow-reading suites now assert something this branch
contradicts?

LANE 3 — THE DOCUMENTS, THE NUMBERS, AND WHAT WAS LEFT OUT. Re-derive every
number: the 32 declarations, the 17 files, the base-tree counts, the actionlint
22→15, and the receipt's "six legs" against its own table which sums to seven.
The receipt's contract. Whether the log's kill matrix shows what the mutations
would actually do, and whether any "single kill" is really collateral. The
label counts in `.github/actionlint.yaml`. The roadmap row. And what the branch
omitted that a reader of the receipt would expect.

Every lane ended with the standing instruction: quote any sentence in the
receipt, the log, the commit messages, the workflow comments or the test's own
doc comments that a reader could check and find false; quote it exactly, say
what is actually true, and cite where you measured it.
