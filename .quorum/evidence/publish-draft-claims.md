# Quorum evidence — PMAT-232 — the claims as put to the lanes

# PMAT-232 — claims for the quorum lanes

Branch PMAT-232-publish-a-draft-it-did-not-create, one commit (efa49822)
on main (a138c809). Judge the diff `main...efa49822`.

1. The defect is observed, not reasoned about: in run 34530301814 every
   asset job succeeded, the `publish-release` job reported `success`, and
   the v1.28.0 GitHub release stayed `draft=true` — because both the
   `gh release edit --draft=false` and the assertion were guarded by
   `needs.create-release.outputs.created`, and that run did not create the
   release.
2. The fix decides from the release's own state, not from who created it:
   it reads `isDraft`/`isPrerelease`, edits ONLY when the state ends
   `draft=true`, re-reads, and fails with `::error::` and `exit 1` if the
   release is still a draft.
3. The assertion is unconditional and asserts only `draft=false`.
   `prerelease=true` is deliberately not asserted, because a re-dispatch
   over a release an operator has promoted to a full release would then
   fail on a correct state; rule 4 still asserts `prerelease=true
   draft=true` for a run that creates a release.
4. Because the edit is conditional on `draft=true`, a re-dispatch cannot
   demote an already-published full release back to a prerelease.
5. Rule 5 of tests/falsification_release_workflow_shape.rs is RED against
   main's release.yml (7 passed, 1 failed) and GREEN at HEAD (8 passed).
6. Three mutations each turn rule 5 red, measured one at a time
   (docs/audits/logs/PMAT-232-rule5-mutations.log): the `created` guard
   restored, the end-state assertion removed, and the edit made
   unconditional.
7. The first version of rule 5 was too weak and is recorded as such: it
   asserted only that `draft=false` appears in the run block, which the
   `gh release edit --draft=false` command satisfies by itself, so two of
   those three mutations stayed green until the rule was tightened.
8. The normal tag-triggered path is unaffected: a run that creates the
   release leaves it `draft=true`, so the edit fires and the assertion
   passes exactly as before.
9. `.github/workflows/release.yml` still parses as YAML and actionlint
   reports the same finding set as main (22 findings both sides, differing
   only in line numbers).
10. Triage in the same commit: PMAT-233 is `cancelled` as a duplicate of
    PMAT-200, which already carries the missing HOMEBREW_TAP_TOKEN as an
    operator decision, and PMAT-200's notes now carry PMAT-233's
    measurement. No other ticket's status changed.
11. No file under src/ changes; the diff is one workflow, one test file
    and the roadmap.
