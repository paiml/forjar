# Quorum evidence — PMAT-217 — agy lanes

One dispatch: three review-only lanes through `paiml-agy-delegate`, `--mode plan`, sandboxed, each in its own `git clone --shared` with `CARGO_HOME` pointed at an empty directory. `agy` 1.1.28, exit 0 on all three, no truncated emissions, `partial=false`.

Isolation was verified by the delegate rather than assumed: after the run all three clones were byte-clean, the working repository carried only the dirt it started with, and free disk was unchanged.

The delegate forbade cargo inside the lanes, having measured 213G of build output against 220G free and citing the recorded incident where a lane filled the disk. It said so in its receipt rather than letting the absence pass unremarked, which is why every count in this ticket's receipt is an orchestrator rerun.

It also reported brief item 5 as UNCOVERED — no lane addressed scope — instead of letting an unanswered question read as a pass.
