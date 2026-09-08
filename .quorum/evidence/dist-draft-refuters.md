# Quorum evidence — PMAT-208 — refuter rulings

Five lanes at 7bcc2ad1, each in its own standalone clone: three blind claim lanes, one `/teamwork-preview`, one CRUX. Verdicts 1 FAIL, 4 PASS.

## Lane 1 (FAIL)

The fix implemented in release.yml correctly fetches the draft assets over the authenticated API and passes `--checksums-file` to `forjar dist`, resolving PMAT-208. The falsification test (Rule 8) correctly isolates the job and is non-vacuous. However, the release receipt `docs/audits/release-1.26.0-receipt.md` contains a direct contradiction regarding what shipped: it claims there are "Two user-visible behaviour changes", but immediately lists three distinct changes in the bullet points below it (which correctly correspond to the 3 behaviour paragraphs in CHANGELOG.md). This ships a false claim.

## Lane 2 (PASS)

After a thorough review, all validation criteria pass. 
1. The fix effectively downloads checksums from the draft release over the API and passes them using the `--checksums-file` flag (which is supported by `forjar dist`), ensuring that the fetch occurs before the file is used. 
2. RULE 8 in the tests is non-vacuous: its job-slicing logic properly isolates `dist-artifacts` and the assertions fail appropriately on the `main` branch. 
3. The receipts contain exact counts, accurate SHAs and tag IDs, exactly one verdict line, and their respective END markers. The asset count (14) correctly matches the assets published on the v1.26.0 release. 
4. The release receipt provides an honest explanation of the hand-finishing step. 
5. All diff additions strictly correspond to the ticket (PMAT-208).
Verdict is PASS.

## Lane 3 (PASS)

Review of PMAT-208 completed successfully with no defects found. The fix correctly resolves the checksum fetch via the GitHub API (which functions on draft releases), the falsification test strongly guards the workflow shape and would fail on main, the receipts reflect the repository and release state correctly with accurate END markers, and all changes are strictly within scope.

## Lane 4 (PASS)

The implemented repair is the pragmatic and correct choice. It technically papers over a deeper architectural coupling—using the GitHub Release itself as an intermediate artifact store rather than using native CI artifacts—but a fully decoupled design is not worth the cost here. A decoupled pipeline would pass the `SHA256SUMS` file between jobs using `actions/upload-artifact` and `actions/download-artifact`, keeping the build independent of the release platform until a final publishing job. However, PMAT-166 explicitly designed the workflow to stage assets in a draft release to easily save partial progress. The current fix gracefully bridges the gap by using the authenticated `gh` CLI to read from the draft, introducing minimal complexity while being fully verified by a new falsification test.

## Lane 5 (PASS)

Successfully created the Implementation Plan artifact detailing how release automations sequence draft creation, upload, and publication, and recommended the cargo-dist [X] posture for forjar.


## Merge-review round at bffc3e15 — three lanes, all FAIL, and what reproduced

The pre-merge round above ran on the diff. A second round ran on the merge candidate and refuted two of its own conclusions. Lanes 1, 2 and 3 all said RULE 8 was reading the flag out of the step's comment, not out of the command; lanes 1 and 3 said the receipt's `recorded_at` predated the failure it describes. Both were measured here, not accepted:

- The `--checksums-file` argument was deleted from the command with the comments left in place. The first version of RULE 8 reported `8 passed`. The rule was vacuous exactly as claimed, and lanes 2 and 3 of the round above were wrong to call it non-vacuous. The job text now drops comment lines before any search, and the flag is looked for after the `dist` command rather than anywhere in the job; the same mutation now reports `7 passed; 1 failed`.
- The `gh release download` command was deleted with its comment left in place. The old rule failed, but on the wrong assertion and with the comment quoted back as the offending text. It now fails on `dist-artifacts never fetches the checksums over the API`.
- Lane 2 also found an unbounded slice, `job[fetch..fetch + 400]`, which would panic instead of naming the rule on a shorter job. Every window is now clamped to the job length.
- `recorded_at` said 2026-09-07. PMAT-208 was opened on 2026-09-08, after the release run failed that morning. Corrected to 2026-09-08.

Lane 2 of this round is the sharpest finding of the release: a shape gate that reads its own documentation is worse than no gate, because it reports green for the thing it was written to prevent.
