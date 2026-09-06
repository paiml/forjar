# Quorum evidence — PMAT-166 / PMAT-170 — the claims as put to the refuters

Three claim lanes (lenses: release.yml; binary-release.yml and every other tag trigger; the path end to end), one agy /teamwork-preview lane and one crux lane read `git diff origin/main...HEAD` at 95322bb5. Their findings forced a restructure, ca5e305d: the release is created as a DRAFT prerelease, the assert step requires exactly that when this run created it, a new `publish-release` job un-drafts only after every asset job succeeded and asserts a published prerelease, and the dispatch-only backfill path creates a prerelease too. HEAD is ca5e305d; a refuter must re-locate a cited line. Ids: L1..L3 (claim lanes, findings L<i>-F<j>), T (teamwork), X (crux), D1..D6 (dispositions), F1..F3 (measured).

## L1 — lane-1.json (verdict FAIL)

C1: At .github/workflows/release.yml:18, release.yml still triggers on a tag push and creates the release, uploads binaries, sidecars, SHA256SUMS, dist artifacts (install.sh), and publishes the Homebrew formula.
C2: At .github/workflows/release.yml:137, the create-release step exits early if the release exists, leaving an existing FULL release completely alone without modification.
C3: At .github/workflows/release.yml:159, the assert step fails on a draft release, fails on a missing release (via set -e), and successfully passes on a promoted release.
C4: At .github/workflows/binary-release.yml:32, the exact `on:` block is `workflow_dispatch` with a `tag` input, meaning backfill is invoked manually by a user.
C5: At .github/workflows/release.yml:157, the comment "A release promoted by the operator earlier ... is reported, not failed" is true because the code does not exit when isPrerelease is false.
C6: At .github/workflows/release.yml:29, the jobs, runners ([self-hosted, clean-room]), secrets, and permissions (contents: write) did not change, preserving the clean-room boundaries.
C7: the diff has defect missing --prerelease flag at .github/workflows/binary-release.yml:88.

Findings:
- L1-F1 [cited] .github/workflows/release.yml:18 — release.yml still triggers on a tag push
- L1-F2 [cited] .github/workflows/release.yml:137 — create-release leaves existing full release alone
- L1-F3 [asserted] .github/workflows/release.yml:159 — assert step fails on draft or missing, passes on promoted
- L1-F4 [cited] .github/workflows/binary-release.yml:32 — binary-release.yml triggers exactly on workflow_dispatch
- L1-F5 [asserted] .github/workflows/release.yml:157 — comment accurately reflects passing on promoted release
- L1-F6 [cited] .github/workflows/release.yml:29 — permissions and runners are unchanged
- L1-F7 [asserted] .github/workflows/binary-release.yml:88 — the diff has defect missing --prerelease flag at .github/workflows/binary-release.yml:88

## L2 — lane-2.json (verdict FAIL)

C1: .github/workflows/release.yml:17 is the only workflow that still triggers on a tag push, creating the GitHub Release as a prerelease and uploading cross-compiled binary archives, checksums, and the install script.
C2: .github/workflows/release.yml:137 checks if the release exists, and if so skips creation, leaving an existing FULL release unmodified and its notes intact.
C3: .github/workflows/release.yml:162 asserts the release state, failing if it is a draft or missing, while passing if it is a prerelease or has been promoted.
C4: .github/workflows/binary-release.yml:31 triggers strictly on workflow_dispatch with a tag input, meaning a manual backfill is invoked by dispatching the workflow for a specific tag.
C5: .github/workflows/release.yml:135 contains the true comment "Idempotent: create if missing, otherwise leave existing release (and its hand-written notes) intact so re-runs don't clobber them".
C6: .github/workflows/release.yml:1 shows no jobs, runners, secrets, or permissions changed, but the diff has defect X at .github/workflows/binary-release.yml:88 where the fallback gh release create omits --prerelease and creates a full release.

Findings:
- L2-F1 [asserted] .github/workflows/binary-release.yml:88 — The fallback `gh release create` omits `--prerelease`, meaning it creates a FULL release if it is missing, which violates the constraint that promotion is never the workflow's. (proposed fix: Add `--prerelease \` before `--repo`.)

## L3 — lane-3.json (verdict do-not-implement-as-written)

C1: The release.yml workflow is the only one that still triggers on a tag push, creating a GitHub prerelease and uploading binary archives, checksum sidecars, and distribution artifacts to it (file: .github/workflows/release.yml:17). C2: When a release already exists, the create-release step skips the `gh release create` command entirely, leaving an existing full release intact without demoting it (file: .github/workflows/release.yml:137). C3: The assert step fails if the release is missing (triggering set -e) or marked as a draft, but intentionally passes if it has already been promoted to a full release (file: .github/workflows/release.yml:153). C4: The binary-release.yml workflow now triggers solely on `workflow_dispatch` with a required tag input, meaning backfills must be manually invoked by an operator (file: .github/workflows/binary-release.yml:31). C5: The comment 'release.yml (clean-room) is the one producer of a release and its assets' is true since the tag push trigger was removed from binary-release.yml (file: .github/workflows/binary-release.yml:27). C6: The diff did not change the jobs, clean-room self-hosted runners, permissions, or GitHub token secrets, preserving the overall workflow topology (file: .github/workflows/release.yml:129). Claim: the diff has defect missing --prerelease flag when creating the release in backfill at .github/workflows/binary-release.yml:88.

Findings:
- L3-F1 [asserted] .github/workflows/binary-release.yml:88 — The diff has defect missing --prerelease flag when creating the release in backfill (proposed fix: Add the --prerelease flag to the gh release create command so it does not silently create a full release.)

## T — lane-4.json (verdict FAIL)

The branch delivers the structural changes (dispatch-only `binary-release.yml`, moving binaries to `release.yml`) and avoids regressions with prereleases and events. However, it fails to deliver the core promise of its PMAT-166 ticket: the assertion that the workflow actually created a PRERELEASE. The `jq/case` logic only asserts the release is not a draft, completely ignoring the `prerelease` variable. A silently promoted (full) release is permitted as a "NO-GO" violation. What I contributed that a claim lane would not: the realization that `gh release create --verify-tag` queries the remote API (obviating the need for a local checkout), the validation that `release-audit.yml` gracefully handles prereleases via its 0-asset fast path, and the observation that `jq` without `-r` outputs literal quotes (e.g. `"prerelease=true draft=false"`), which substring matching happens to survive but masks the missing logic.

Findings:
- T-F1 [asserted] .github/workflows/release.yml:162 — The assert step validates the release is a prerelease. (proposed fix: Add a check for `*prerelease=false*` in the case statement if you genuinely want to forbid silent promotion by the workflow, though distinguishing this from an operator's past promotion is impossible using just `gh release view`. Also add `-r` to jq to remove raw quotes.)
- T-F2 [asserted] .github/workflows/release.yml:90 — No job in `release.yml` is broken by the release being a prerelease. (proposed fix: None needed. `gh release download "$RELEASE_TAG"` and `gh release upload "$RELEASE_TAG"` operate directly against the named tag and are indifferent to the prerelease bit. `--latest` is not used.)
- T-F3 [cited] .github/workflows/release-audit.yml:25 — `release: published` in `release-audit.yml` will break on prereleases. (proposed fix: None needed. Creating a prerelease *does* trigger the `published` event, but the audit script loops over all tags and gracefully skips tags with 0 assets (`if [ "$n" = "0" ]; then continue`). Because the prerelease is created before binaries exist, it hits this `n=0` fast path and gracefully skips.)
- T-F4 [asserted] .github/workflows/binary-release.yml:48 — `github.ref_name` is broken when dispatched from a branch. (proposed fix: None needed. While `github.ref_name` is the branch name (e.g. `main`) on dispatch, `inputs.tag` takes precedence in the `||` chain and is marked `required: true`, so it always supplies the concurrency group.)
- T-F5 [cited] .github/workflows/release.yml:148 — `--verify-tag` requires the tag to exist in the local checkout. (proposed fix: None needed. The `create-release` job has no checkout step at all. `gh release create --verify-tag` verifies the tag's existence in the *remote repository* via the GitHub API.)

## X — lane-5.json (verdict FAIL)

### Competitive Survey: Release Paths

| System [X] | When Visible | Who Promotes | Racing Producer Prevention | Package Publish |
| :--- | :--- | :--- | :--- | :--- |
| **cargo-dist** [1] | Draft during build; public at end. | Workflow automatically (`announce` flow). | Draft status prevents webhook/API visibility during build; CI concurrency. | Workflow-owned (OIDC/Tokens). |
| **GoReleaser** [2] | Draft during build; public at end. | Workflow automatically. | Draft releases hide partial assets from consumers. | Workflow-owned. |
| **semantic-release / release-please** [3] | Upon PR merge and CI completion. | Workflow automatically. | Strict branch protection and CI concurrency. | Workflow-owned. |
| **GitHub Events** [4] | `prerelease` is public but excluded from `--latest`. | Manual or workflow. | Draft prevents webhook races; prerelease does not. | N/A |
| **Homebrew/Nix** [5] | Immediate upon tap/flake Git push. | N/A | Release is dictated entirely by Git commits, not GitHub Release UI state. | N/A |

### Verdict

**FAIL**. Forjar's release path is fundamentally broken because it violates the behavior of its distribution consumers (Homebrew/Nix). Forjar creates a GitHub `prerelease` intending to hold the release for an operator's "dogfood" test before manual promotion. However, the workflow immediately pushes the updated formula to `paiml/homebrew-tap.git`. Because Homebrew users pull directly from the tap repository via Git, the GitHub "prerelease" status is completely ignored by Homebrew clients, and the untested release is instantly shipped to all Homebrew users, completely bypassing the dogfood phase. 

Additionally, forjar requires manual crates.io publication from a detached worktree. This violates the industry standard established by systems like cargo-dist, GoReleaser, and semantic-release, where package publishing is strictly workflow-owned to prevent race conditions, overlapping producers, and human error. To achieve its dogfooding goals and match the soundness of the field, forjar must transition to workflow-owned publishing and separate its distribution artifact pushing into a workflow triggered by the `release: published` event (which occurs when the operator removes the prerelease flag).

Findings:
- X-F1 [asserted] .github/workflows/release.yml:? — cargo-dist [1]: Uses a draft release to hide artifacts and prevent races during the build, then automatically promotes to published via an 'announce' flow that strictly handles workflow-owned package publishing.
- X-F2 [asserted] .github/workflows/release.yml:? — GoReleaser [2]: Automatically detects prereleases and builds as draft releases, automating promotion and guaranteeing workflow-owned package publication without detached manual steps.
- X-F3 [asserted] .github/workflows/release.yml:? — semantic-release / release-please [3]: Automates the transition from draft PRs to published releases, ensuring that package publication is strictly workflow-owned and atomic.
- X-F4 [asserted] .github/workflows/release.yml:? — GitHub Release Events [4]: GitHub's prerelease flag only hides the release from the `--latest` API; it does not prevent assets from being publicly downloadable or consumed, unlike `draft` status.
- X-F5 [asserted] .github/workflows/release.yml:450 — Homebrew / Nix Consumers [5]: Homebrew and Nix clients pull directly from the Git tap/flake repository. Pushing to the tap immediately exposes the release, blatantly violating the manual dogfood phase. (proposed fix: Trigger Homebrew tap updates on `release: types: [published]` instead of during the prerelease build.)

## Orchestrator dispositions (each is itself a claim to refute)

- D1 — the assert step checked only `draft`, so a silently promoted release passed (T): CONFIRMED. FIXED in ca5e305d: the create step records `created`; when this run created the release the assert requires `prerelease=true draft=true`, and `publish-release` requires `prerelease=true draft=false` after un-drafting; a pre-existing release is reported, never touched.
- D2 — the backfill path in binary-release.yml created a FULL release (L1-F7): CONFIRMED. FIXED in ca5e305d: `--prerelease` added.
- D3 — release-audit.yml listens to `release: published` and 'will break on prereleases' (T): read at HEAD — the audit reads the published objects back and asserts each describes itself; a draft never fires `published`, an un-drafted prerelease does, and nothing in the audit asserts `prerelease=false` (see F3). REJECTED as a defect unless the refuters find the assertion.
- D4 — `github.ref_name` is wrong when binary-release is dispatched from a branch (T): CONFIRMED for the old expression. FIXED in 95322bb5: the tag is `inputs.tag` only; release.yml's dispatch input is `required: true`.
- D5 — `--verify-tag` requires the tag in the local checkout (T-F5): REJECTED — `gh release create --verify-tag` asks the remote API whether the tag exists (the same lane's own contribution states this), and the clean-room checkout is of the tag anyway.
- D6 — consumers could see a half-uploaded asset set because the release was public before the asset jobs finished (X): CONFIRMED, the field's pattern is draft-then-publish. ADOPTED in ca5e305d.

## Orchestrator's own measured claims

- F1 — Both files parse as YAML at HEAD (`python3 -c 'import yaml; yaml.safe_load(...)'`), and `grep -n 'continue-on-error\|cargo publish\|CARGO_REGISTRY_TOKEN'` over the two files matches only comments.
- F2 — At HEAD the only workflow with `push: tags:` is release.yml; binary-release.yml's `on:` is `workflow_dispatch` alone (verified with `grep -ln 'tags:' .github/workflows/*.yml`).
- F3 — release-audit.yml triggers on `schedule` and `release: [published]` and delegates to scripts/release-object-audit.sh; the grep for `prerelease` / `draft` / `latest` over both is recorded in the receipt with the lines it found.
