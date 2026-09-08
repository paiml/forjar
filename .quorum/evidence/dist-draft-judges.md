# Quorum digest — PMAT-208 — adjudicated claims

Eight ids: five claims the lanes and the measurements confirm, and three rulings. Each citation supports its sentence; where a claim is design rather than test, the item says so.

## CONFIRMED

1. [measured] C1 — The repair fetches SHA256SUMS from the draft with an authenticated gh call before the dist step and passes --checksums-file, which is the interface forjar's own error message names; three lanes read the job and agreed on all three halves.
- evidence: tests/falsification_release_workflow_shape.rs:369 asserts the fetch, the flag and their order, and was observed 8 passed here against 7 passed / 1 failed on main.

2. [design] C2 — The draft-until-the-end design PMAT-166 shipped is untouched: the release is still created as a draft prerelease and un-drafted by the final job.
- evidence: tests/falsification_release_workflow_shape.rs:177 and tests/falsification_release_workflow_shape.rs:241 still pass unchanged on this branch; the diff adds a step to one job and changes no job's condition.

3. [measured] C3 — Both receipts carry exactly one verdict line and their literal END marker, and their shas, tag, gate results and asset count match the repository and the published release.
- evidence: two lanes checked them against `gh release view v1.26.0` and reported 14 assets, draft=false, prerelease=false; the count is now reconciled in the receipt as 13 from the workflow plus the hand-added installer.

4. [measured] C4 — The release receipt says what was done by hand and why, in its own section, rather than reading as though the workflow completed.
- evidence: two lanes cited that section approvingly; the workflow defect it describes is the ticket this branch closes, pinned at tests/falsification_release_workflow_shape.rs:369.

5. [scope] C5 — Nothing in the diff is outside the ticket: one workflow job, one rule, one roadmap row, two receipts.
- evidence: one lane checked the scope explicitly and agreed; the receipts are required by the release protocol and the rule is required by the repair.

## REFUTED

6. [q1] R1 — One lane found the release receipt saying 'Two user-visible behaviour changes' and then listing three — the CHANGELOG carries three and gate H counts three.
- corrected: the receipt says three and names how gate H counts them; the count is what tests/falsification_release_workflow_shape.rs:369's sibling rules keep honest at release time.

7. [q1] R2 — The teamwork lane found the deeper coupling: the workflow uses the GitHub release as a staging area, so a job that produces assets consumes assets.
- evidence: recorded rather than acted on — it judged a decoupled design not worth the cost against PMAT-166's deliberate draft-as-staging, and the repair fixes the one broken edge; the shape is pinned at tests/falsification_release_workflow_shape.rs:177.

8. [q1] R3 — The CRUX lane found that every surveyed tool checksums locally before touching the releases API, and none reads an unpublished draft.
- evidence: carried as the 1.27 candidate posture in the crux evidence file; nothing changed here on its account, and every figure in it is [X].

