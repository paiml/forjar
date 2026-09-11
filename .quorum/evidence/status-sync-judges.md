# Quorum evidence — PMAT-235 — adjudicated claims

One round of three sandboxed lanes, base pinned at d7028f5b, the diff at 691c1faa: 0/3 PASS. Nine claims confirmed by all three; one refuted by all three. Every lane parsed both versions of `docs/roadmaps/roadmap.yaml` into structured rows and diffed them field by field — the brief refused to let a lane judge a data change by reading a summary of it — and every lane ran the refused transition itself.

Citations resolve at the merge base: `docs/roadmaps/roadmap.yaml:3292` is `- id: PMAT-226`, the cut whose eight sibling tickets are the bulk of what this branch marks completed.

## CONFIRMED

1. [the-drift] SIXTEEN SHIPPED TICKETS READ `planned` OR `inprogress` — PMAT-137 and 159 (v1.25.2), 162 and 204 (v1.26.0), 208 (v1.27.0), 215, 217, 219, 220, 221, 222, 223 and 224 (v1.28.0), and 227, 230 and 231 (merged since v1.28.0). All sixteen now read `completed`.
- evidence: three independent parses of both versions of `docs/roadmaps/roadmap.yaml`, cross-checked against the ticket lists in `docs/roadmaps/releases.yaml` and against `gh pr list --state merged`. The v1.28.0 eight are the window of the cut at `docs/roadmaps/roadmap.yaml:3292`.

2. [no-sweep] NOTHING WAS MARKED COMPLETED THAT DID NOT SHIP, AND NOTHING SHIPPED WAS LEFT BEHIND — every ticket moved is named by a ledger row or by a PR merged since v1.28.0, and a sweep of the whole file found no other ticket that should have moved.
- evidence: all three lanes swept every row rather than the sixteen named, which is what the brief asked and the only way this claim can be judged; the sixteen are listed in `docs/audits/impl-PMAT-235-receipt.md:20` and each was tied to a ledger row or a merged PR.

3. [no-collateral] FIELD BY FIELD, NOTHING ELSE CHANGED MEANING — every title, description, acceptance criterion, note, label and creation timestamp on every row is unchanged outside `status`, `updated` and the top-level `kind:` field.
- evidence: two lanes diffed the parsed structures directly; the third confirmed the same by field census. `docs/roadmaps/roadmap.yaml:3292`'s own row is one of the fourteen the tool touched and is unchanged but for those three fields.

4. [kind-safe] THE FOURTEEN DROPPED `kind:` FIELDS COST NOTHING — every one of those rows carries the matching `kind:*` label, and `kind-gate.sh` reads the label before the field.
- evidence: confirmed 3/3 against the labels, and stated in `docs/audits/impl-PMAT-235-receipt.md:34`. The drop is a known behaviour of `pmat work edit` with any field it does not recognise, and it is recorded rather than worked around.

5. [labels-intact] EVERY `release:<tag>` LABEL SURVIVES — v1.25.0=1, v1.25.2=2, v1.26.0=9, v1.27.0=4, v1.28.0=10, v1.29.0=10.
- evidence: counted independently by all three lanes. The labels are what gate T reconciles, and they were correct before this branch and after it; only the status field had drifted.

6. [tool-refuses] THE TOOL ENFORCES THE LIFECYCLE — `pmat work edit <id> -s completed` on a `planned` ticket returns `Invalid transition: Planned → Completed. See work-dbc-v1.yaml §work_lifecycle`, and the legal path `planned -> inprogress -> completed` is what every ticket took.
- evidence: each lane ran the refused command in its own clone and quoted the error, which `docs/audits/impl-PMAT-235-receipt.md:32` quotes too. No status in this diff was written by editing YAML.

7. [gates] `pmat work validate` PASSES AND GATE T IS GREEN ON THE BRANCH — and the new rows PMAT-235 and PMAT-236 carry `release:v1.29.0`, a kind, acceptance criteria and notes.
- evidence: run by all three lanes in their clones. PMAT-236 is the arm gate T is missing: it reconciles the `release:<tag>` label in both directions and never looks at `status`, which is why this drifted across five releases with nothing going red.

8. [rail] THE DIFF IS ON THE TRIAGE RAIL AND NOTHING ELSE — `docs/roadmaps/roadmap.yaml`, this branch's receipt and its one audit log. No script, no test, no workflow and no source file is touched, which is what makes a `kind: triage` receipt the honest shape for it rather than a shape chosen to avoid writing a falsification test.
- evidence: `git diff --name-only main...HEAD`, run by all three lanes. The rail that admits `docs/roadmaps/roadmap.yaml` has always admitted it; the ledger beside it was added to the rail by PMAT-226, the ticket at `docs/roadmaps/roadmap.yaml:3292`.

9. [receipt-otherwise-true] APART FROM THE COUNT BELOW, EVERY OTHER SENTENCE OF THE RECEIPT IS SUPPORTED BY THE DATA — including the claim that `pmat work complete` was deliberately not used.
- evidence: each lane was asked to read the receipt as a hostile reader and quote anything checkable and false; all three quoted the same single sentence and nothing else.

## REFUTED

10. [row-count] AS WORDED: "183 rows before, 183 after, none lost or added" — refuted by all three lanes.
- evidence: the file holds 184 rows at the merge base and 186 at HEAD; the two added are PMAT-235 and PMAT-236, the tickets this branch files. The claim was true of the measurement I actually took and false of the commit, because I took it before minting those two rows; the count also used a `PMAT-` regex and so missed the one row whose id does not begin with `PMAT-`.
- corrected: the receipt and the log now say 184 in, 186 out, name the two added rows, and record why the first figure was wrong — a measurement taken at one moment and reported as a description of another is not a measurement of the commit. `docs/roadmaps/roadmap.yaml:3292` and every other pre-existing row is still present and unchanged in meaning, which is the part of the claim that survives.
