# Quorum evidence — PMAT-231 — the claims as put to the lanes

Two rounds; the second set differs because three facts in the first were refuted and corrected.

## round 1

# PMAT-231 — claims for the quorum lanes (the 1.28.0 release record)

Branch PMAT-231-record-1.28.0, one commit (fcbc5b7e) on main (aff6be71).
Judge the diff `main...fcbc5b7e`. This is a kind: triage branch — classify
and link, no code.

1. Every identity fact in docs/audits/release-1.28.0-receipt.md is
   measured, not remembered: the tag `v1.28.0` is annotated, object
   931ee2eb, tagger date 2026-09-10T16:07:14Z, on commit cdcc0e80; the
   GitHub release is published (draft=false, prerelease=false) at
   2026-09-10T23:56:51Z with exactly 14 assets; crates.io reports
   created_at 2026-09-10T16:12:45Z, crate_size 3822649, yanked false;
   docs.rs reports doc_status true for 1.28.0.
2. The 14 assets are the six target tarballs, their six `.sha256`
   sidecars, `SHA256SUMS` and `install.sh` — no more, no fewer.
3. `scripts/dogfood/release-check.sh` (gate R) exits 0 on this HEAD, and
   docs/audits/logs/PMAT-231-release-check.log records its output verbatim
   together with the two `GATE C`/`GATE D` lines from
   `make dogfood-published VERSION=1.28.0`, which also exited 0.
4. The window census the receipt gives — 10 PRs, 10 tickets
   (PMAT-215, 217, 219, 220, 221, 222, 223, 224, 225, 226) — is exactly
   the v1.28.0 row in docs/roadmaps/releases.yaml, and every one of those
   roadmap rows carries `release:v1.28.0`.
5. PMAT-232's row describes the defect accurately: release.yml's
   `publish-release` runs `gh release edit --draft=false` only inside
   `if [ "${{ needs.create-release.outputs.created }}" = "true" ]`, the
   assertion that the release is `prerelease=true draft=false` is inside
   the same guard, and run 34530301814's publish-release job reported
   success while the release stayed a draft.
6. PMAT-233's row describes the defect accurately: in run 34530301814 the
   `homebrew` job's "Publish to Homebrew tap" step logs `GH_TOKEN:` with
   an empty value while other steps in the same job log `GH_TOKEN: ***`,
   the step reached `Cloning into '/tmp/tap'`, and it died on
   `remote: Invalid username or token`.
7. PMAT-234's row describes the defect accurately: gate R printed
   `GATE R PASS pre-tag: … PENDING until the tag is cut: no
   docs/audits/crux-1.28.0.md: Cargo.toml is still at v1.28.0's version`
   while origin serves the tag and docs/audits/crux-1.28.0.md exists at
   HEAD; the "pre-tag" wording is produced by a single branch that fires
   on any non-empty pending list.
8. Every new roadmap row (PMAT-231, 232, 233, 234) carries
   `release:v1.29.0` and a `kind:` field, and PMAT-231's notes carry an
   `orch-basis:` token.
9. The diff touches only docs/audits/** and docs/roadmaps/roadmap.yaml —
   the kind: triage rail — and no file under src/, tests/ or .github/.
10. The receipt's "What is not claimed" section is accurate: the homebrew
    job has published no formula for this or any release, cargo mutants
    is unmeasured on this host, and the first tag run had four jobs
    cancelled at 16:53:40Z.

## round 2

# PMAT-231 — round 2 claims (three facts were refuted and corrected)

Branch PMAT-231-record-1.28.0, one commit (f2c5cb58) on main (aff6be71).
Judge the diff `main...f2c5cb58`. This is a kind: triage branch — classify
and link, no code.

1. Every identity fact in docs/audits/release-1.28.0-receipt.md is measured and EXACT: the tag `v1.28.0` is annotated, object 931ee2eb, tagger date 2026-09-10T16:07:14Z, on cdcc0e80; the release is published (draft=false, prerelease=false) at 2026-09-10T23:56:51Z with 14 assets; crates.io reports created_at 2026-09-10T16:12:45.938977Z, crate_size 3822649, yanked false; docs.rs doc_status true. No number is rounded or truncated.
   measured, not remembered: the tag `v1.28.0` is annotated, object
   931ee2eb, tagger date 2026-09-10T16:07:14Z, on commit cdcc0e80; the
   GitHub release is published (draft=false, prerelease=false) at
   2026-09-10T23:56:51Z with exactly 14 assets; crates.io reports
   created_at 2026-09-10T16:12:45Z, crate_size 3822649, yanked false;
   docs.rs reports doc_status true for 1.28.0.
2. The 14 assets are the six target tarballs, their six `.sha256`
   sidecars, `SHA256SUMS` and `install.sh` — no more, no fewer.
3. `scripts/dogfood/release-check.sh` (gate R) exits 0 on this HEAD, and
   docs/audits/logs/PMAT-231-release-check.log records its output verbatim
   together with the two `GATE C`/`GATE D` lines from
   `make dogfood-published VERSION=1.28.0`, which also exited 0.
4. The window census the receipt gives — 10 PRs, 10 tickets
   (PMAT-215, 217, 219, 220, 221, 222, 223, 224, 225, 226) — is exactly
   the v1.28.0 row in docs/roadmaps/releases.yaml, and every one of those
   roadmap rows carries `release:v1.28.0`.
5. PMAT-232's row describes the defect accurately: release.yml's
   `publish-release` runs `gh release edit --draft=false` only inside
   `if [ "${{ needs.create-release.outputs.created }}" = "true" ]`, the
   assertion that the release is `prerelease=true draft=false` is inside
   the same guard, and run 34530301814's publish-release job reported
   success while the release stayed a draft.
6. PMAT-233's row describes the defect accurately: in run 34530301814 the
   `homebrew` job's "Publish to Homebrew tap" step logs `GH_TOKEN:` with
   an empty value while other steps in the same job log `GH_TOKEN: ***`,
   the step reached `Cloning into '/tmp/tap'`, and it died on
   `remote: Invalid username or token`.
7. PMAT-234's row describes the defect accurately: gate R printed
   `GATE R PASS pre-tag: … PENDING until the tag is cut: no
   docs/audits/crux-1.28.0.md: Cargo.toml is still at v1.28.0's version`
   while origin serves the tag and docs/audits/crux-1.28.0.md exists at
   HEAD; the "pre-tag" wording is produced by a single branch that fires
   on any non-empty pending list.
8. Every new roadmap row (PMAT-231, 232, 233, 234) carries `release:v1.29.0`, a `kind:` field and a non-null `notes:` value, and PMAT-231 carries an `orch-basis:` token.
   `release:v1.29.0` and a `kind:` field, and PMAT-231's notes carry an
   `orch-basis:` token.
9. The diff touches only docs/audits/** and docs/roadmaps/roadmap.yaml —
   the kind: triage rail — and no file under src/, tests/ or .github/.
10. The receipt's "What is not claimed" section is accurate: the homebrew job has published no formula for this or any release; cargo mutants is unmeasured on this host; and the runner-side event at 16:53:40-42Z cancelled SEVEN jobs across three runs (3 in the tag run, 3 in the CI run, 1 in the Proofs run), visible only in each run's attempt 1.
    job has published no formula for this or any release, cargo mutants
    is unmeasured on this host, and the first tag run had four jobs
    cancelled at 16:53:40Z.
