# Quorum evidence — PMAT-210 — CRUX survey

Lane: `agy-lane.sh --mode plan`, sandboxed. Conversation `239fd011-83b4-4c8d-a7dd-c2b992c992d3`. Verdict PASS.

Systems surveyed: Ansible, Puppet/Chef, Terraform, nixpkgs.

Question: when an S1 is reported against a release that is already published and pinned by its consumers, where does the report land, is the published release ever amended or re-tagged for it, and is there an artifact a reader can point at that says "this was classified, and here is the release it is going to"?

Verdict: none of the surveyed systems amends or re-tags a published release for a defect of this class; the fix is always a later version, and the report is tracked in an issue with a milestone. forjar's practice on this branch is equivalent on the deferral and stronger on the artifact — the disposition, its rationale, the ticket id and the target release sit in one row in the release's own triage document, which is a thing none of the four produces per release.

Its one finding, and the answer to it, are in `triage485-lanes.md`: amending a cut release's audit document is a difference from the surveyed practice, and the difference is defended by what was amended (an internal audit artifact, under a heading that dates the addition) rather than waved away.
