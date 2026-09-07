# Quorum evidence — PMAT-166 — adjudicated claims (majority of three judges)

28 claim ids (three claim lanes' findings, the teamwork review, the crux rows, six dispositions and three measured claims) were put to three refuters at e2438564 and then to three judges at 58ef4b0c (conv-f58eff92, conv-bd04775b, conv-96bd48c6; 160–212 s). The judges agreed on every directed question and on 22 of 28 rows; two verdict fields said FAIL while the tables named no defect unfixed at HEAD. Majority rule; CONFIRMED-AS-NARROWED counts as survived; D5 is ruled CONFIRMED (the disposition's sentence, narrowed) with T-F5 REFUTED. After the judges, one commit added the shape test tests/falsification_release_workflow_shape.rs and its contract; the workflow files are unchanged since 58ef4b0c. Every item names the shape-test rule in this diff that pins the property it describes.

## REFUTED — 8 claims killed

1. [binary-release.yml] L2-F1 — .github/workflows/binary-release.yml:88 — The fallback `gh release create` omits `--prerelease`, meaning it creates a FULL release if it is missing, which violates the constraint that promotion is never the workflow's. (proposed fix: Add `--prerelease \` before `--repo`.)
   - evidence: REFUTED — stale: fixed in ca5e305d; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_release_workflow_shape.rs:177.

2. [end-to-end] L3-F1 — .github/workflows/binary-release.yml:88 — The diff has defect missing --prerelease flag when creating the release in backfill (proposed fix: Add the --prerelease flag to the gh release create command so it does not silently create a full release.)
   - evidence: REFUTED — stale: fixed in ca5e305d; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_release_workflow_shape.rs:332.

3. [release.yml] L1-F3 — .github/workflows/release.yml:159 — assert step fails on draft or missing, passes on promoted
   - evidence: REFUTED — stale: fixed in ca5e305d (the assert step now requires prerelease=true draft=true when this run created the release); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_release_workflow_shape.rs:215.

4. [release.yml] L1-F7 — .github/workflows/binary-release.yml:88 — the diff has defect missing --prerelease flag at .github/workflows/binary-release.yml:88
   - evidence: REFUTED — stale: fixed in ca5e305d (--prerelease on the backfill create); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_release_workflow_shape.rs:292.

5. [teamwork] T-F1 — .github/workflows/release.yml:162 — The assert step validates the release is a prerelease. (proposed fix: Add a check for `*prerelease=false*` in the case statement if you genuinely want to forbid silent promotion by the workflow, though distinguishing this from an operator's past promotion is impossible using just `gh release view`. Also add `-r` to jq to remove raw quotes.)
   - evidence: REFUTED — stale: fixed in ca5e305d (the assert checks the prerelease flag); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_release_workflow_shape.rs:215.

6. [teamwork] T-F3 — .github/workflows/release-audit.yml:25 — `release: published` in `release-audit.yml` will break on prereleases. (proposed fix: None needed. Creating a prerelease *does* trigger the `published` event, but the audit script loops over all tags and gracefully skips tags with 0 assets (`if [ "$n" = "0" ]; then continue`). Because the prerelease is created before binaries exist, it hits this `n=0` fast path and gracefully skips.)
   - evidence: REFUTED — refuted on the merits: release-audit.yml and scripts/release-object-audit.sh assert nothing about the prerelease flag, and a draft never fires the published event; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_release_workflow_shape.rs:177.

7. [teamwork] T-F4 — .github/workflows/binary-release.yml:48 — `github.ref_name` is broken when dispatched from a branch. (proposed fix: None needed. While `github.ref_name` is the branch name (e.g. `main`) on dispatch, `inputs.tag` takes precedence in the `||` chain and is marked `required: true`, so it always supplies the concurrency group.)
   - evidence: REFUTED — stale: fixed in 95322bb5 and 58ef4b0c (inputs.tag alone, concurrency group included); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_release_workflow_shape.rs:115.

8. [teamwork] T-F5 — .github/workflows/release.yml:148 — `--verify-tag` requires the tag to exist in the local checkout. (proposed fix: None needed. The `create-release` job has no checkout step at all. `gh release create --verify-tag` verifies the tag's existence in the *remote repository* via the GitHub API.)
   - evidence: REFUTED — refuted on the merits: gh release create --verify-tag asks the remote API; the create-release job has no checkout step; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_release_workflow_shape.rs:177.

## CONFIRMED — 20 claims survived refutation (17 as written, 3 as narrowed)

1. [crux] X-F1 — .github/workflows/release.yml:? — cargo-dist [1]: Uses a draft release to hide artifacts and prevent races during the build, then automatically promotes to published via an 'announce' flow that strictly handles workflow-owned package publishing.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:241.

2. [crux] X-F2 — .github/workflows/release.yml:? — GoReleaser [2]: Automatically detects prereleases and builds as draft releases, automating promotion and guaranteeing workflow-owned package publication without detached manual steps.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:241.

3. [crux] X-F3 — .github/workflows/release.yml:? — semantic-release / release-please [3]: Automates the transition from draft PRs to published releases, ensuring that package publication is strictly workflow-owned and atomic.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:292.

4. [crux] X-F4 — .github/workflows/release.yml:? — GitHub Release Events [4]: GitHub's prerelease flag only hides the release from the `--latest` API; it does not prevent assets from being publicly downloadable or consumed, unlike `draft` status.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:292.

5. [crux] X-F5 — .github/workflows/release.yml:450 — Homebrew / Nix Consumers [5]: Homebrew and Nix clients pull directly from the Git tap/flake repository. Pushing to the tap immediately exposes the release, blatantly violating the manual dogfood phase. (proposed fix: Trigger Homebrew tap updates on `release: types: [published]` instead of during the prerelease build.)
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:292.

6. [disposition] D1 — — the assert step checked only `draft`, so a silently promoted release passed (T): CONFIRMED. FIXED in ca5e305d: the create step records `created`; when this run created the release the assert requires `prerelease=true draft=true`, and `publish-release` requires `prerelease=true draft=false` after un-drafting; a pre-existing release is reported, never touched.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:177.

7. [disposition] D2 — — the backfill path in binary-release.yml created a FULL release (L1-F7): CONFIRMED. FIXED in ca5e305d: `--prerelease` added.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:332.

8. [disposition] D3 — — release-audit.yml listens to `release: published` and 'will break on prereleases' (T): read at HEAD — the audit reads the published objects back and asserts each describes itself; a draft never fires `published`, an un-drafted prerelease does, and nothing in the audit asserts `prerelease=false` (see F3). REJECTED as a defect unless the refuters find the assertion.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:215.

9. [disposition] D4 — — `github.ref_name` is wrong when binary-release is dispatched from a branch (T): CONFIRMED for the old expression. FIXED in 95322bb5 for the tag resolution (`inputs.tag`) and, after the refuters found the concurrency group still carried the `|| github.ref_name` chain, in the follow-up commit that makes the group `binary-release-${{ inputs.tag }}`; release.yml's `RELEASE_TAG: github.event.inputs.tag || github.ref_name` is the tag-push path, where `github.ref_name` IS the tag, and is unchanged.
   - corrected: the tag resolution and the concurrency group are inputs.tag alone (58ef4b0c); release.yml keeps github.ref_name for the tag-push path, where it is the tag
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_release_workflow_shape.rs:115.

10. [disposition] D5 — — `--verify-tag` requires the tag in the local checkout (T-F5): REJECTED — `gh release create --verify-tag` asks the remote API whether the tag exists (the same lane's own contribution states this); the create-release job has no checkout step at all, so no local tag is involved.
   - corrected: --verify-tag asks the remote API and the create-release job has no checkout step at all (the clause about a checkout of the tag is dropped)
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_release_workflow_shape.rs:177.

11. [disposition] D6 — — consumers could see a half-uploaded asset set because the release was public before the asset jobs finished (X): CONFIRMED, the field's pattern is draft-then-publish. ADOPTED in ca5e305d.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:292.

12. [measured] F1 — — Both files parse as YAML at HEAD (`python3 -c 'import yaml; yaml.safe_load(...)'`), and `grep -n 'continue-on-error\|cargo publish\|CARGO_REGISTRY_TOKEN'` over the two files matches only comments.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:292.

13. [measured] F2 — — At HEAD the only workflow with `push: tags:` is release.yml; binary-release.yml's `on:` is `workflow_dispatch` alone (verified with `grep -ln 'tags:' .github/workflows/*.yml`).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:115.

14. [measured] F3 — — release-audit.yml triggers on `schedule` and `release: [published]` and delegates to scripts/release-object-audit.sh; `grep -n -i 'prerelease\|draft\|latest'` over both files matches nothing but a `runs-on` line, so the audit asserts nothing about the prerelease flag; a draft never fires `published`, the un-draft does, by which point every asset job has finished.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:215.

15. [release.yml] L1-F1 — .github/workflows/release.yml:18 — release.yml still triggers on a tag push
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:140.

16. [release.yml] L1-F2 — .github/workflows/release.yml:137 — create-release leaves existing full release alone
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:177.

17. [release.yml] L1-F4 — .github/workflows/binary-release.yml:32 — binary-release.yml triggers exactly on workflow_dispatch
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:115.

18. [release.yml] L1-F5 — .github/workflows/release.yml:157 — comment accurately reflects passing on promoted release
   - corrected: the comment now reads that a release which existed before this run is reported, never touched, and the code fails when the release this run created is not a draft prerelease
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_release_workflow_shape.rs:241.

19. [release.yml] L1-F6 — .github/workflows/release.yml:29 — permissions and runners are unchanged
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:292.

20. [teamwork] T-F2 — .github/workflows/release.yml:90 — No job in `release.yml` is broken by the release being a prerelease. (proposed fix: None needed. `gh release download "$RELEASE_TAG"` and `gh release upload "$RELEASE_TAG"` operate directly against the named tag and are indifferent to the prerelease bit. `--latest` is not used.)
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_release_workflow_shape.rs:292.

## Tables as returned

### Judge 1 (verdict FAIL)

### Pipeline Analysis
- **publish-release**: When `build-binaries`, `checksums` and `dist-artifacts` are skipped, their result is `skipped`, satisfying the condition `!contains(needs.*.result, 'failure')`. Thus `publish-release` runs. If one fails, the condition fails and `publish-release` skips.
- **created propagation**: Set on both branches in the `create` step and passed correctly as a string to `publish-release`.
- **assert step**: Fails if `created=true` and `$state != "prerelease=true draft=true"`.
- **Triggers at HEAD**: `release.yml` fires on `push: tags: ['v*']` and `workflow_dispatch`. `release-audit.yml` fires on `schedule`, `workflow_dispatch`, and `release: [published]`. `binary-release.yml` fires on `workflow_dispatch`. No other workflows fire on tag push.
- **release-object-audit.sh**: Asserts nothing about `prerelease` or `draft`.

### Adjudication Table
| Claim | Status |
| :--- | :--- |
| L1-F1 | CONFIRMED |
| L1-F2 | CONFIRMED |
| L1-F3 | REFUTED |
| L1-F4 | CONFIRMED |
| L1-F5 | CONFIRMED-AS-NARROWED |
| L1-F6 | CONFIRMED |
| L1-F7 | REFUTED |
| L2-F1 | REFUTED |
| L3-F1 | REFUTED |
| T-F1 | REFUTED |
| T-F2 | CONFIRMED |
| T-F3 | REFUTED |
| T-F4 | REFUTED |
| T-F5 | REFUTED |
| X-F1 | CONFIRMED |
| X-F2 | CONFIRMED |
| X-F3 | CONFIRMED |
| X-F4 | CONFIRMED |
| X-F5 | CONFIRMED |
| D1 | CONFIRMED |
| D2 | CONFIRMED |
| D3 | CONFIRMED |
| D4 | CONFIRMED |
| D5 | CONFIRMED |
| D6 | CONFIRMED |
| F1 | CONFIRMED |
| F2 | CONFIRMED |
| F3 | CONFIRMED |

Totals:
CONFIRMED: 19
CONFIRMED-AS-NARROWED: 1
REFUTED: 8

### Judge 2 (verdict PASS)

### Answers from HEAD Verification
- **publish-release `if:` expression**: When `build-binaries`, `checksums`, and `dist-artifacts` are skipped (e.g. `has_binaries` is false), their result is `skipped`. Since `skipped` is not `failure` or `cancelled`, `!contains(needs.*.result, 'failure')` evaluates to true, and the `publish-release` job runs. If one fails, its result is `failure`, so `!contains(needs.*.result, 'failure')` evaluates to false, and the `publish-release` job is skipped.
- **`created` output**: The `steps.create.outputs.created` is explicitly set to `false` or `true` on both branches of the `if/else` statement in the `create-release` job. It successfully reaches the `publish-release` job via `needs.create-release.outputs.created`.
- **Assert step exact pass/fail conditions**: If the run created the release (`created=true`), it checks that `$state` is exactly `"prerelease=true draft=true"`, failing otherwise. If it did not create the release (`created=false`), the check is completely skipped (passes automatically).
- **`on:` blocks**: At HEAD, `release.yml` is the ONLY workflow that still fires on a tag push (`push: tags: ['v*']`). `release-audit.yml` is the ONLY workflow that fires on a `release:` event.
- **`scripts/release-object-audit.sh`**: The script checks assets, sums, and sidecars using the `gh api` and `gh release download`. A `grep` confirms it asserts absolutely nothing about the `prerelease` or `draft` flags.
- **Rule on D3-D6**: Ruled as written. D4 was fixed in 58ef4b0c2154fd87e482b55eac4b75c6a509d79a by changing the concurrency group to exactly `binary-release-${{ inputs.tag }}`. D5's claim that `--verify-tag` requires a local checkout is false (it uses the remote API). D6 was adopted in ca5e305d. D3 is confirmed since the refuters found the assertion does not exist.

### Adjudication Table
| ID | Claim | Verdict |
| :--- | :--- | :--- |
| L1-F1 | release.yml still triggers on a tag push | CONFIRMED |
| L1-F2 | create-release leaves existing full release alone | CONFIRMED |
| L1-F3 | assert step fails on draft or missing, passes on promoted | REFUTED |
| L1-F4 | binary-release.yml triggers exactly on workflow_dispatch | CONFIRMED |
| L1-F5 | comment accurately reflects passing on promoted release | REFUTED |
| L1-F6 | permissions and runners are unchanged | CONFIRMED |
| L1-F7 | the diff has defect missing --prerelease flag at .github/workflows/binary-release.yml:88 | REFUTED |
| L2-F1 | The fallback `gh release create` omits `--prerelease` | REFUTED |
| L3-F1 | The diff has defect missing --prerelease flag when creating the release in backfill | REFUTED |
| T-F1 | The `jq/case` logic only asserts the release is not a draft, completely ignoring the `prerelease` variable. | REFUTED |
| T-F2 | No job in `release.yml` is broken by the release being a prerelease. | CONFIRMED |
| T-F3 | `release: published` in `release-audit.yml` will break on prereleases. | REFUTED |
| T-F4 | `inputs.tag` takes precedence in the `||` chain and is marked `required: true`, so it always supplies the concurrency group. | REFUTED |
| T-F5 | `--verify-tag` requires the tag to exist in the local checkout. | REFUTED |
| X-F1 | cargo-dist [1]: Uses a draft release to hide artifacts and prevent races during the build... | CONFIRMED |
| X-F2 | GoReleaser [2]: Automatically detects prereleases and builds as draft releases... | CONFIRMED |
| X-F3 | semantic-release / release-please [3]: Automates the transition from draft PRs... | CONFIRMED |
| X-F4 | GitHub Release Events [4]: GitHub's prerelease flag only hides the release from the `--latest` API... | CONFIRMED |
| X-F5 | Homebrew / Nix Consumers [5]: Homebrew and Nix clients pull directly from the Git tap/flake repository... | CONFIRMED |
| D1 | the assert step checked only `draft`, so a silently promoted release passed... | CONFIRMED |
| D2 | the backfill path in binary-release.yml created a FULL release... | CONFIRMED |
| D3 | release-audit.yml listens to `release: published` and 'will break on prereleases'... | CONFIRMED |
| D4 | `github.ref_name` is wrong when binary-release is dispatched from a branch... | CONFIRMED |
| D5 | `--verify-tag` requires the tag in the local checkout... | CONFIRMED |
| D6 | consumers could see a half-uploaded asset set because the release was public before the asset jobs finished... | CONFIRMED |
| F1 | Both files parse as YAML at HEAD... | CONFIRMED |
| F2 | At HEAD the only workflow with `push: tags:` is release.yml... | CONFIRMED |
| F3 | release-audit.yml triggers on `schedule` and `release: [published]` and delegates to scripts/release-object-audit.sh... | CONFIRMED |

Totals:
CONFIRMED: 19
CONFIRMED-AS-NARROWED: 0
REFUTED: 9

### Judge 3 (verdict FAIL)

- publish-release if: expression: When build-binaries, checksums and dist-artifacts are skipped (has_binaries false), their result is 'skipped', which passes the !contains('failure') and !contains('cancelled') check, so the job runs. When one fails, contains finds 'failure' and the job is skipped.
- created is set on both branches: confirmed. The create-release job outputs created=false if the release exists, and created=true if it creates it. Because outputs are strings, this survives the needs chain and reaches publish-release.
- assert step's exact pass/fail conditions: confirmed. If created is "true", it fails if state is not exactly "prerelease=true draft=true". It passes if state is exactly "prerelease=true draft=true", or if created is not "true" (i.e. existing release).
- workflow on: blocks: confirmed. grep over .github/workflows/*.yml shows release.yml is the only workflow that still fires on a tag push (push: tags: ['v*']). release-audit.yml is the only workflow that fires on release: events (release: types: [published]).
- scripts/release-object-audit.sh asserts nothing about prerelease: confirmed. The script checks for stray files, denominator, sums, and sidecars, but does not check or assert anything about the prerelease status.

| Claim ID | Verdict |
|---|---|
| L1-F1 | CONFIRMED |
| L1-F2 | CONFIRMED |
| L1-F3 | REFUTED |
| L1-F4 | CONFIRMED |
| L1-F5 | CONFIRMED-AS-NARROWED |
| L1-F6 | CONFIRMED |
| L1-F7 | REFUTED |
| L2-F1 | REFUTED |
| L3-F1 | REFUTED |
| T-F1 | REFUTED |
| T-F2 | CONFIRMED |
| T-F3 | REFUTED |
| T-F4 | REFUTED |
| T-F5 | REFUTED |
| X-F1 | CONFIRMED |
| X-F2 | CONFIRMED |
| X-F3 | CONFIRMED |
| X-F4 | CONFIRMED |
| X-F5 | CONFIRMED |
| D1 | REFUTED |
| D2 | REFUTED |
| D3 | REFUTED |
| D4 | REFUTED |
| D5 | REFUTED |
| D6 | REFUTED |
| F1 | CONFIRMED |
| F2 | CONFIRMED |
| F3 | CONFIRMED |

Totals: 13 CONFIRMED / 1 CONFIRMED-AS-NARROWED / 14 REFUTED

