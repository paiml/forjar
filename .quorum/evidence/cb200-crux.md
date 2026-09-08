# Crux lane — PMAT-206 — how the field handles a stale analysis cache and a small ceiling overrun

Analyzed third-party behaviors for stale analysis indices and debt ceiling margins from documentation memory. Recommended that forjar refuse as unmeasured for stale indices and compare against the merge base for debt ceilings.

## What forjar took from it

SonarQube refreshes its analysis in CI and evaluates new code against a merge base; CodeClimate analyses the PR diff without a local index; a Ratchet-style gate fails when the recorded number does not match the tree; clippy with a stale baseline misfires on shifted line numbers. Every figure is documentation memory and marked [X]. forjar takes the first posture for the cache (refresh, and say so) and keeps its absolute ceiling for the number, which PMAT-203 already tracks bringing down; comparing against the merge base instead is recorded as the panel's recommendation for 1.27.
