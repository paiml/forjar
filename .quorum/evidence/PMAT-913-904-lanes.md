# Lanes — bump `softprops/action-gh-release` from 3.0.2 to 3.0.4 (PMAT-913 / forjar#904)

Two-round `LANES=3` review by `agy` (`Gemini 4 Carbon (thinking=high)`):

- **Round 1 (`@fdc6a8b0`, 3/3 FAIL)**:
  - **Lane 1 (Release Gate E Simulation & Pin Dereference)**: `FAIL` — proved `scripts/dogfood/quorum.sh` fails (`rc=1`) if `#904` merges without `.quorum/dependabot-github_actions-softprops-action-gh-release-3.0.4.json`, while confirming `v3.0.4` dereferences to `8a0cd651e5b2365bd640854a3fa5dfc70e4875f3`.
  - **Lane 2 (Workflow Failure Modes & Pin Style)**: `FAIL` — missing `PMAT-913` audit/quorum traceability at `fdc6a8b0`; verified `nightly.yml:247` step inputs and pin conventions.
  - **Lane 3 (Governance & Gate A Traceability)**: `FAIL` — missing `docs/audits/impl-PMAT-913-receipt.md` coverage for `#904` and `.quorum` bundle prerequisite.
- **Round 2 (`@0a66d9eb`, 3/3 PASS)**:
  - **Lane 1**: `PASS` — verified `.github/workflows/nightly.yml:247` commit SHA pin (`8a0cd651e5b2365bd640854a3fa5dfc70e4875f3`) and unchanged `nightly` release inputs.
  - **Lane 2**: `PASS` — verified scheduled/manual workflow isolation and absence of blocking gate changes (`RULES-GATES-1111` clean).
  - **Lane 3**: `PASS` — verified `docs/audits/impl-PMAT-913-receipt.md` (`0a66d9eb`) documents `#904` under `PMAT-913`, enabling Gate E bundle generation.
