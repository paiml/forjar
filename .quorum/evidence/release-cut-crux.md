# Crux lane — PMAT-165 — how comparable Rust projects cut a release

Generated a plan analyzing standard Rust release patterns against the forjar release cut.

## What forjar took from it

The panel's posture — a release cut is a metadata commit, and product fixes belong to their own PRs — is the rule forjar follows: every behaviour change in this release landed in its own PR under its own quorum receipt (#472, #473, #476, #477, #478, #479, #480, #481, #482), and the diff this lane read only appears to carry source changes because its base predates the last two merges. What the cut itself adds beyond metadata is three gate repairs, each because the act of cutting was what exposed the gate's blind spot, each with a falsification test. Every third-party figure in the lane is [X]: documentation memory, no network.
