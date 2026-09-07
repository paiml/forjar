# Independent review — agy /teamwork-preview — PMAT-166

One /teamwork-preview lane (35-minute budget, sandboxed) reviewed the two workflow files at 95322bb5 without seeing the claim lanes.

## Verdict (FAIL)

The branch delivers the structural changes (dispatch-only `binary-release.yml`, moving binaries to `release.yml`) and avoids regressions with prereleases and events. However, it fails to deliver the core promise of its PMAT-166 ticket: the assertion that the workflow actually created a PRERELEASE. The `jq/case` logic only asserts the release is not a draft, completely ignoring the `prerelease` variable. A silently promoted (full) release is permitted as a "NO-GO" violation. What I contributed that a claim lane would not: the realization that `gh release create --verify-tag` queries the remote API (obviating the need for a local checkout), the validation that `release-audit.yml` gracefully handles prereleases via its 0-asset fast path, and the observation that `jq` without `-r` outputs literal quotes (e.g. `"prerelease=true draft=false"`), which substring matching happens to survive but masks the missing logic.

## Findings, as returned

- T-F1 [asserted] .github/workflows/release.yml:162 — The assert step validates the release is a prerelease.
- T-F2 [asserted] .github/workflows/release.yml:90 — No job in `release.yml` is broken by the release being a prerelease.
- T-F3 [cited] .github/workflows/release-audit.yml:25 — `release: published` in `release-audit.yml` will break on prereleases.
- T-F4 [asserted] .github/workflows/binary-release.yml:48 — `github.ref_name` is broken when dispatched from a branch.
- T-F5 [cited] .github/workflows/release.yml:148 — `--verify-tag` requires the tag to exist in the local checkout.

## What became of it

Its central finding — the assert step ignored the prerelease flag — forced the restructure in ca5e305d (draft prerelease at create, asserted when this run created it, published only after every asset job); its dispatch-tag finding was fixed in 95322bb5 and 58ef4b0c; its release-audit and --verify-tag findings were rejected with the evidence in the dossier (D3, D5).
