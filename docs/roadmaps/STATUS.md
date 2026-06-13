# Forjar Roadmap Status

Human-readable companion to the machine-readable backlog in
[`roadmap.yaml`](./roadmap.yaml). Last updated **2026-06-13**.

## Current state

- **Latest release:** v1.6.1 — published to [crates.io](https://crates.io/crates/forjar)
  and [GitHub Releases](https://github.com/paiml/forjar/releases) (4 Linux
  targets × gnu/musl + `SHA256SUMS`; `curl | sh` installer verified end-to-end).
- **Backlog:** 93 / 93 roadmap items **completed**. 0 open issues, 0 open PRs.
- **CI:** green on `main` across `ci`, `coverage`, `audit`, and `nightly`
  (the nightly also exercises the release action versions on the self-hosted
  clean-room runners).

## How we got here (2026-06 sprint)

The 10-day roadmap was executed as a compressed release train, each tag a
hosted-runner GitHub release with full binaries + checksums:

| Tag | Theme |
|-----|-------|
| v1.4.3 | Release-pipeline repair (committed `Cargo.lock`, CI lockfile-preflight, hosted-runner release path, Security-Audit green), 3 user-input panic fixes, `GIT_DIR` scrub |
| v1.4.4 | `install.sh` works end-to-end (SHA256SUMS, glibc-2.35 baseline, naming parity), stale-issue close-out + regression tests, F-grade burn-down |
| v1.5.0 | `forjar dist` real checksum resolution + Tier-1 static `--verify`, Coverage de-flake, +69 CLI coverage tests |
| v1.6.0 | **L3–L5 test-coverage persistence** + **Tier-2 container `dist --verify`**; remediation of 28 defects from an 84-agent adversarial bug-hunt; provable convergence contracts; mdBook in CI |
| v1.6.1 | Remediation of 9 regressions/gaps found by a focused audit of the v1.6.0 fix/feature code (lock-acquire livelock, two missed shell-escape sites, moved post-expansion checks, non-retryable `pre_apply`, recency-aware coverage demotion, Tier-2 custom checksum filename) |

### Quality work

- **Two adversarial bug-hunts**, every finding double-refuted (both verifiers
  had to confirm *real* + *reachable* + *unguarded*): 84 agents over the
  existing code (28 defects → v1.5.x/v1.6.0) and 30 over the newly-merged
  fix/feature code (9 defects → v1.6.1). **37 confirmed defects, all fixed.**
- All 18 original open issues triaged and closed (most already-fixed, closed
  with added regression tests).
- 24 stale superseded PRs closed; CI actions modernized (checkout v6,
  upload-artifact v7, etc.).

## Distribution feature family (FJ-3600 / spec 25) — complete

`generators → real checksum resolution (--version / --checksums-file) →
Tier-1 static --verify → Tier-2 container --verify (--verify-containers)`.
All phases IMPLEMENTED.

## What's next

No committed next-cycle items. The product is feature-complete for the
v1.6.x line with a clean tracker. Candidate future work (not yet scheduled):

- Refresh the `.pmat` TDG baseline so the pre-commit quality gate stops
  flagging an in-grade `package.rs` delta (currently worked around).
- De-flake the two environment-dependent tests (`build-image` needs Docker,
  `secret-from-env` needs a spawnable subprocess) so they skip cleanly in
  constrained environments.
- Pages deploy for the mdBook (build is gated in CI; deploy is not).

New work is tracked as `PMAT-###` entries in `roadmap.yaml` and as GitHub
issues.
