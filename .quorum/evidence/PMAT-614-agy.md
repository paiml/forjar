# agy lanes — the nightly's Windows leg builds OpenSSL with Strawberry perl (PMAT-614)

Round count and heads: see `PMAT-614-lanes.md`.

Three agy lanes per round, two rounds: gemini-3.1-pro-high, gemini-3.1-pro-low
and gemini-3.8-flash-high, one conversation each, JSON-schema output, and the
brief pasted inline with no build. The lanes read the workspace and wrote
nothing.

Round 1 at f78aef43: 0/3. All three failed the branch on the collateral
roadmap round-trip (R1), and flash-high also named the missing `release:`
binding (R2). Both are corrected in 215f2e8a.

Round 2 at 215f2e8a: 3/3 PASS, with no defect findings. The summaries agree
that the step exports the Strawberry perl on Windows only and fails loudly when
that perl cannot load `Params::Check`. They also agree that the falsifier
extracts and executes the step with stub cargo/cross, so it discriminates on
behaviour rather than text, and that ci.yml runs it.
