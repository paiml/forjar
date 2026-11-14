# Claims — bump `softprops/action-gh-release` from 3.0.2 to 3.0.4 (PMAT-913 / forjar#904)

Extracted from PR `#904` (`dependabot/github_actions/softprops/action-gh-release-3.0.4`, Round 1 `fdc6a8b0` -> Round 2 `0a66d9eb`) and attacked across all 3 review lanes:

- **C1 (Upstream tag dereference & 40-char commit SHA pin)**: `.github/workflows/nightly.yml:247` pins `softprops/action-gh-release@8a0cd651e5b2365bd640854a3fa5dfc70e4875f3 # v3.0.4`, which is the exact commit object annotated tag `refs/tags/v3.0.4` (`28ab305d0ffa26d60ec4bd614126329e26c7a0ec`) dereferences to.
- **C2 (Single call site & unchanged release step contract)**: `.github/workflows/nightly.yml:247` (`Create nightly release`) is the sole `softprops/action-gh-release` call site across `.github/workflows/`, preserving `tag_name: nightly`, `target_commitish`, and artifact glob inputs.
- **C3 (Non-blocking workflow surface & `RULES-GATES-1111` compliance)**: `.github/workflows/nightly.yml` is a scheduled/manual release workflow rather than a blocking PR gate, leaving shipped Rust CLI/library code untouched.
- **C4 (`PMAT-913` Gate A receipt traceability)**: Commit `0a66d9eb` adds the dedicated `PR #904` section to `docs/audits/impl-PMAT-913-receipt.md` matching `PMAT-913` in `docs/roadmaps/roadmap.yaml`.
- **R1 (Round 1 missing `.quorum` Gate E receipt at `fdc6a8b0`)**: Merging `fdc6a8b0` without `.quorum/dependabot-github_actions-softprops-action-gh-release-3.0.4.json` failed `scripts/dogfood/quorum.sh` (`rc=1`) because Gate E checks every merged PR without a `dependabot[bot]` exemption.
- **R2 (Dependabot commit `473630d8` `direct:production` classification & `impl-PMAT-913-receipt.md` pre-receipt `--shortstat` figure)**: Commit `473630d8` labelled the GitHub Action pin `dependency-type: direct:production` and `docs/audits/impl-PMAT-913-receipt.md:40` recorded `1 file changed, 1 insertion, 1 deletion` (the pre-receipt `fdc6a8b0` stat before adding `impl-PMAT-913-receipt.md` itself).
