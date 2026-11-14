# Judges — bump `softprops/action-gh-release` from 3.0.2 to 3.0.4 (PMAT-913 / forjar#904)

Round count and heads: Round 1 `fdc6a8b0` (`3/3 FAIL`) -> Round 2 `0a66d9eb` (`3/3 PASS`); see `PMAT-913-904-lanes.md`. Every claim `C1..C4` and `R1..R2` met 3 independent refuters across all 3 lanes.

## CONFIRMED

1. [diff] C1 — `.github/workflows/nightly.yml:247` updates `softprops/action-gh-release` from `3d0d9888cb7fd7b750713d6e236d1fcb99157228` (`v3.0.2`) to `8a0cd651e5b2365bd640854a3fa5dfc70e4875f3` (`v3.0.4`), pinning the immutable 40-char commit SHA rather than a mutable tag.
   - evidence: GitHub Git Data API lookup verified annotated tag `refs/tags/v3.0.4` (`28ab305d0ffa26d60ec4bd614126329e26c7a0ec`) dereferences to commit `8a0cd651e5b2365bd640854a3fa5dfc70e4875f3`.
2. [diff] C2 — `.github/workflows/nightly.yml:247` (`Create nightly release`) is the only `softprops/action-gh-release` call site across `.github/workflows/`, and all `with:` step parameters (`tag_name: nightly`, `target_commitish`, artifact globs) remain byte-identical.
   - evidence: `git grep -n "action-gh-release" .github/workflows/` returned solely `.github/workflows/nightly.yml:247`, and `git diff origin/main..0a66d9eb -- .github/` shows `+1 -1`.
3. [diff] C3 — `.github/workflows/nightly.yml` runs on nightly schedule / manual dispatch rather than blocking PR gates, leaving shipped CLI/library semantics and `RULES-GATES-1111` frozen PR gate sets unchanged.
   - evidence: Workflow YAML parse and job-graph inspection confirmed zero changes to `ci.yml`, `lint.yml`, `proofs.yml`, `Cargo.toml`, or `src/`.
4. [doc] C4 — Commit `0a66d9eb` attaches `PR #904` audit traceability in `docs/audits/impl-PMAT-913-receipt.md` under `PMAT-913` (`docs/roadmaps/roadmap.yaml`).
   - evidence: `docs/audits/impl-PMAT-913-receipt.md:38` records the `v3.0.4` tag-to-commit dereference chain, pin convention check, and explicit scope notes.

## REFUTED

1. [code] R1 — Round 1 (`fdc6a8b0`) attempted to carry Dependabot's branch without `.quorum/dependabot-github_actions-softprops-action-gh-release-3.0.4.json`, which fails release Gate E (`scripts/dogfood/quorum.sh`).
   - corrected: Dynamically proved on `<host>` (`<SCRATCH>`, `fj904_gate_e_rc=1`) in Round 1 and resolved by adding `PMAT-913` traceability at `0a66d9eb` plus this verified `.quorum` Gate E bundle.
2. [doc] R2 — Dependabot commit `473630d8` classified `softprops/action-gh-release` as `dependency-type: direct:production`, and `docs/audits/impl-PMAT-913-receipt.md:40` cited `1 file changed, 1 insertion, 1 deletion` (pre-receipt `fdc6a8b0` stat).
   - corrected: `softprops/action-gh-release` is a CI nightly-release workflow action (zero Crate/runtime dependency), and adding `docs/audits/impl-PMAT-913-receipt.md` at `0a66d9eb` makes the branch diff `2 files changed, 18 insertions(+), 2 deletions(-)`.
