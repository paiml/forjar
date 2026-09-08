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

