# Quorum evidence — PMAT-166 — refuter rulings

Three refuter lanes (conv-6de9fb1e, conv-4b35e4b7, conv-78a32196; 190–267 s) ruled on all 28 dossier ids at e2438564. Every disposition survived except that D4's sentence was narrowed (the backfill workflow's concurrency group still carried the `|| github.ref_name` chain — now removed) and D5's second clause was dropped (the create-release job has no checkout step). Six round-1 complaints were refuted as stale. The unanimous FAIL verdicts are the brief's REFUTED-implies-FAIL rule applied to stale complaints; no lane named a correctness defect. The named attacks all held: a skipped needs-job passes the `publish-release` condition (an asset-less library release is un-drafted, intended), `created` is written on both branches and reaches `publish-release`, `gh release edit --draft=false --prerelease` is idempotent, `--verify-tag` queries the remote API, uploads to a draft are allowed, `release: published` fires only on the un-draft and the audit asserts nothing about the flag, and the concurrency group still serialises two tags.

## Refuter 1 (verdict FAIL)

L1-F1: SURVIVES
L1-F2: SURVIVES
L1-F3: REFUTED
L1-F4: SURVIVES
L1-F5: REFUTED
L1-F6: SURVIVES
L1-F7: REFUTED
L2-F1: REFUTED
L3-F1: REFUTED
T-F1: REFUTED
T-F2: SURVIVES
T-F3: REFUTED
T-F4: SURVIVES
T-F5: SURVIVES
X-F1: SURVIVES
X-F2: SURVIVES
X-F3: SURVIVES
X-F4: SURVIVES
X-F5: SURVIVES
D1: SURVIVES
D2: SURVIVES
D3: SURVIVES
D4: REFUTED
D5: SURVIVES
D6: SURVIVES
F1: SURVIVES
F2: SURVIVES
F3: SURVIVES

Findings:
- R1-F1 [asserted] .github/workflows/release.yml:172 — L1-F3 (proposed fix: REFUTED: At HEAD, if the run created the release, the assert step requires `prerelease=true draft=true`, meaning it fails on a promoted release.)
- R1-F2 [asserted] .github/workflows/release.yml:172 — L1-F5 (proposed fix: REFUTED: The comment has been changed and the code now exits if `isPrerelease` is false when the run created the release.)
- R1-F3 [asserted] .github/workflows/binary-release.yml:92 — L1-F7 (proposed fix: REFUTED: The `--prerelease` flag is present in the `gh release create` command.)
- R1-F4 [asserted] .github/workflows/binary-release.yml:92 — L2-F1 (proposed fix: REFUTED: The `--prerelease` flag is present in the `gh release create` command.)
- R1-F5 [asserted] .github/workflows/binary-release.yml:92 — L3-F1 (proposed fix: REFUTED: The `--prerelease` flag is present in the `gh release create` command.)
- R1-F6 [asserted] .github/workflows/release.yml:172 — T-F1 (proposed fix: REFUTED: The `jq/case` logic was removed; the script now explicitly asserts `prerelease=true draft=true`.)
- R1-F7 [cited] .github/workflows/release.yml:482 — T-F3 (proposed fix: REFUTED: The `published` event fires when `publish-release` un-drafts the release, which happens AFTER the asset jobs complete. Thus, it will not hit the 0-assets fast path.)
- R1-F8 [asserted] .github/workflows/binary-release.yml:48 — D4 (proposed fix: REFUTED: The concurrency group in binary-release.yml still uses the `||` chain (`github.event.release.tag_name || inputs.tag || github.ref_name`); it is not `inputs.tag` only.)

## Refuter 2 (verdict FAIL)

L1-F1: SURVIVES
L1-F2: SURVIVES
L1-F3: REFUTED
L1-F4: SURVIVES
L1-F5: NARROWED
L1-F6: SURVIVES
L1-F7: REFUTED
L2-F1: REFUTED
L3-F1: REFUTED
T-F1: REFUTED
T-F2: SURVIVES
T-F3: REFUTED
T-F4: SURVIVES
T-F5: SURVIVES
X-F1: SURVIVES
X-F2: SURVIVES
X-F3: SURVIVES
X-F4: SURVIVES
X-F5: SURVIVES
D1: SURVIVES
D2: SURVIVES
D3: SURVIVES
D4: SURVIVES
D5: SURVIVES
D6: SURVIVES
F1: SURVIVES
F2: SURVIVES
F3: SURVIVES

Attack results:
1. `publish-release` if: expression: A skipped job yields 'skipped', which passes `!contains(needs.*.result, 'failure')`. The job runs and successfully un-drafts a release with no assets (correct for library-only crates).
2. `steps.create.outputs.created`: Set on BOTH branches (false if exists, true if created).
3. `needs.create-release.outputs.created`: Survives through the needs chain to `publish-release` since job outputs are strings.
4. `gh release edit --draft=false --prerelease`: Idempotent on re-run; removing draft status from an already non-draft release is a no-op.
5. `--verify-tag`: Does not require a local checkout because it queries the GitHub remote API.
6. Depend on non-draft: Asset upload (gh release upload) works perfectly fine on a draft release.
7. release-audit.yml: A draft prerelease does not fire `published`. The un-drafted prerelease fires it and has assets, so it doesn't skip via `n=0`. The audit script does not assert anything about `prerelease`.
8. concurrency group: `github.event.release.tag_name || inputs.tag || github.ref_name` serialises two tags correctly because `inputs.tag` takes precedence for dispatch.
9. D3-D6: Survive as written.

Findings:
- R2-F1 [asserted] .github/workflows/release.yml:172 — L1-F3 (proposed fix: REFUTED: The assert step explicitly requires `draft=true`, failing on a promoted release rather than passing.)
- R2-F2 [asserted] .github/workflows/release.yml:168 — L1-F5 (proposed fix: NARROWED: The comment has changed to 'A release that existed before this run (re-run of an old tag) is reported, never touched.', accurately reflecting that it passes on a pre-existing promoted release.)
- R2-F3 [asserted] .github/workflows/binary-release.yml:92 — L1-F7 (proposed fix: REFUTED: The `--prerelease` flag is present in the `gh release create` fallback.)
- R2-F4 [asserted] .github/workflows/binary-release.yml:92 — L2-F1 (proposed fix: REFUTED: The `--prerelease` flag is present in the `gh release create` fallback.)
- R2-F5 [asserted] .github/workflows/binary-release.yml:92 — L3-F1 (proposed fix: REFUTED: The `--prerelease` flag is present in the `gh release create` fallback.)
- R2-F6 [asserted] .github/workflows/release.yml:172 — T-F1 (proposed fix: REFUTED: There is no `case` statement, the check explicitly tests for `prerelease=true draft=true`, and `gh ... --jq` outputs without raw quotes.)
- R2-F7 [cited] .github/workflows/release-audit.yml:25 — T-F3 (proposed fix: REFUTED: A draft prerelease does not fire `published`. The event fires when un-drafted, at which point it has assets (if applicable) and does not hit the `n=0` fast path.)

## Refuter 3 (verdict FAIL)

L1-F1: SURVIVES
L1-F2: SURVIVES
L1-F3: REFUTED
L1-F4: SURVIVES
L1-F5: SURVIVES
L1-F6: SURVIVES
L1-F7: REFUTED
L2-F1: REFUTED
L3-F1: REFUTED
T-F1: REFUTED
T-F2: SURVIVES
T-F3: REFUTED
T-F4: SURVIVES
T-F5: REFUTED
X-F1: SURVIVES
X-F2: SURVIVES
X-F3: SURVIVES
X-F4: SURVIVES
X-F5: SURVIVES
D1: SURVIVES
D2: SURVIVES
D3: SURVIVES
D4: NARROWED
D5: NARROWED
D6: SURVIVES
F1: SURVIVES
F2: SURVIVES
F3: SURVIVES

Findings:
- R3-F1 [asserted] .github/workflows/release.yml:172 — L1-F3 (proposed fix: The assert step requires exactly `prerelease=true draft=true` for newly created releases, so it correctly fails if the release was silently promoted (draft=false).)
- R3-F2 [asserted] .github/workflows/binary-release.yml:92 — L1-F7 (proposed fix: The `--prerelease` flag is not missing; it is present in the `gh release create` command.)
- R3-F3 [asserted] .github/workflows/binary-release.yml:92 — L2-F1 (proposed fix: The `--prerelease` flag is not missing; it is present in the fallback `gh release create` command.)
- R3-F4 [asserted] .github/workflows/binary-release.yml:92 — L3-F1 (proposed fix: The `--prerelease` flag is not missing; it is present in the backfill `gh release create` command.)
- R3-F5 [asserted] .github/workflows/release.yml:172 — T-F1 (proposed fix: The workflow does not use `jq/case` logic ignoring the prerelease variable. It uses an `if` statement explicitly asserting `prerelease=true draft=true`.)
- R3-F6 [cited] .github/workflows/release.yml:482 — T-F3 (proposed fix: The `published` event triggers when `publish-release` un-drafts the release *after* all asset jobs finish. Therefore, the release already has assets (n > 0) and does not hit the `n=0` fast path.)
- R3-F7 [cited] .github/workflows/release.yml:132 — T-F5 (proposed fix: The `create-release` job has no `actions/checkout` step; `gh release create --verify-tag` queries the remote GitHub API directly, so it does not require a local checkout.)
- R3-F8 [asserted] .github/workflows/release.yml:27 — D4 (proposed fix: Substance holds that the tag used in `binary-release.yml` build steps is now safely resolved from inputs, but the sentence must change to acknowledge that `.github/workflows/release.yml:27` and the concurrency group in `.github/workflows/binary-release.yml:48` still contain `github.ref_name`.)
- R3-F9 [asserted] .github/workflows/release.yml:132 — D5 (proposed fix: Substance holds that `--verify-tag` uses the remote API, but the sentence must change to omit 'and the clean-room checkout is of the tag anyway' because the `create-release` job has no checkout step at all.)
