# Judges — forjar 1.33.0 release PR, rc.1 promoted + #611 (PMAT-652)

Round count and heads: see `PMAT-652-lanes.md`.

## CONFIRMED

1. [workflow] C1 — release.yml gains one line, in the aarch64 branch of the Linux prerequisites step, after the cross install.
   - evidence: `.github/workflows/release.yml:275`; `git diff 2fd44833..a5374809 -- .github` is 1 insertion, 0 deletions.
2. [test] C2 — The test runs the step's own `run:` from release.yml with a fake cargo, replays GITHUB_PATH and asks the next step's shell for cross.
   - evidence: `tests/falsification_611_cross_on_path.rs:24` reads the step, `:59` installs cross where cargo does, `:89` replays GITHUB_PATH, `:93` resolves cross; the two legs are `:98` and `:107`.
3. [falsify] C3 — RED without the line, GREEN with it.
   - evidence: with line 275 removed both tests FAILED at "cross installed but off PATH"; restored 2/2, and falsification_version_matches_manifest 4/4.
4. [scope] C4 — The diff is 6 files: the workflow line, the test, the bump in Cargo.toml and Cargo.lock, the CHANGELOG section and the roadmap row; no src/ change.
   - evidence: `git diff --stat 2fd44833..a5374809`.

## REFUTED

1. [scope] R1 — The first head claimed the workflow change was the one #611 line. It also added a comment line, which the ticket's "exactly" excludes.
   - corrected: a5374809 removes the comment; round 2 3/3 PASS.

## NOTED, OUT OF SCOPE

- `rule2_release_yml_is_the_only_v_star_tag_push_producer` fails on the rc.1 tree too (bench.yml, ci.yml, coverage.yml); it is not touched by this diff.
