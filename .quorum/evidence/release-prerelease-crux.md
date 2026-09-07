# Crux lane — competitive survey — PMAT-166

One agy lane surveyed the field from documentation memory (sandboxed, no network); every third-party figure is [X] and asserted.

## Verdict (FAIL)

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

## Findings, as returned

- [asserted] — cargo-dist [1]: Uses a draft release to hide artifacts and prevent races during the build, then automatically promotes to published via an 'announce' flow that strictly handles workflow-owned package publishing.
- [asserted] — GoReleaser [2]: Automatically detects prereleases and builds as draft releases, automating promotion and guaranteeing workflow-owned package publication without detached manual steps.
- [asserted] — semantic-release / release-please [3]: Automates the transition from draft PRs to published releases, ensuring that package publication is strictly workflow-owned and atomic.
- [asserted] — GitHub Release Events [4]: GitHub's prerelease flag only hides the release from the `--latest` API; it does not prevent assets from being publicly downloadable or consumed, unlike `draft` status.
- [asserted] — Homebrew / Nix Consumers [5]: Homebrew and Nix clients pull directly from the Git tap/flake repository. Pushing to the tap immediately exposes the release, blatantly violating the manual dogfood phase.

## Orchestrator cross-check (asserted)

- cargo-dist: builds into a draft release and announces at the end; the workflow owns the publish. Forjar adopts the draft-then-publish shape (ca5e305d) and keeps the crates.io publish out of the workflow by the operator directive.
- GoReleaser: `release.draft` and `release.prerelease: auto`; one producer per tag. Forjar's binary-release.yml is dispatch-only so one producer remains (PMAT-170).
- release-please / semantic-release: a release PR, then a tag and release on merge; the workflow publishes packages. Forjar promotes by hand after the published crate passes the dogfood arm.
- GitHub events: `published` fires when a release or prerelease is un-drafted; `prereleased` fires for prereleases; `--latest` is never set on a prerelease. Forjar's release-audit listens to `published`, which fires on the un-draft with every asset in place.
- Homebrew and Nix consumers fetch tagged assets by URL; the prerelease flag does not change asset URLs.

The crux lane's FAIL was the draft-first pattern forjar then adopted; the remaining deviation (manual crates.io publish with local credentials) is the operator's sanctioned path.
