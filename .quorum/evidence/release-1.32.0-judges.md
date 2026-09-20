# Judges — v1.32.0 release cut (PMAT-592, PR #593)

Round count and heads: see `release-1.32.0-lanes.md`. Numbers here quote it.

## CONFIRMED — claims that survived three lanes

1. [diff] C2 — The CHANGELOG heading rename is the only edit this branch makes to that file: `CHANGELOG.md:8` becomes the version heading and the behaviour paragraph opening at `CHANGELOG.md:10` appears in the diff as unchanged context on every one of its lines, so no bullet was added, reworded or dropped while the heading moved.
   - evidence: all three lanes confirmed independently; the hunk contains exactly one `-`/`+` pair and the file length is unchanged at 4347 lines at base and at head.
2. [gate] C3 — The crux audit carries exactly one behaviour row and that row contains the first six words of the bold span at `CHANGELOG.md:10` with no backticks inside the span, which is the fixed string gate H greps for rather than a regex it might otherwise satisfy loosely.
   - evidence: GATE H PASS 1 of 1 behaviour bullet(s) under [1.32.0] reconciled, naming 5 of the 28 surveyed systems; re-run after every edit to the crux file in this branch.
3. [diff] C7 — The twelve roadmap title syncs are lossless, because every row whose `title:` changed now carries its previous title verbatim inside its `notes:` field, so the definition-of-done wording that the sync overwrote is still readable at head.
   - evidence: the claude lane spot-checked PMAT-191, PMAT-196 (whose notes were null before), PMAT-198, PMAT-581 and PMAT-582 and found the pre-diff title reproduced exactly in each.
4. [diff] C8 — This branch changes no Rust at all, so v1.32.0 ships exactly the code that merged in the window and nothing this cut authored, which is what makes the release reviewable as bookkeeping rather than as behaviour.
   - evidence: GATE F PASS line coverage 96.44% >= 95%; no .rs differs from origin/main, so there is nothing to mutate — and the diff touches nine files, none with an .rs extension.

## REFUTED — claims that did not survive

1. [diff] C1 — The claim said no other version string in the diff asserts a different number, which elides that three roadmap rows carry `release: 1.33.0`; those are ledger fields naming a future release rather than this crate's version, but the claim as written did not say so and a lane was right to read it literally.
   - corrected: C1 now distinguishes the crate version at `Cargo.toml:3` and `Cargo.lock:1171` from the ledger field, and names the three rows.
2. [tree] C4 — The claim that only the `why` array grew in the ratchet baseline was false when made: rewriting that JSON with ensure_ascii=False had converted twenty pre-existing unicode escapes to literal em-dashes on lines this branch has no business touching.
   - corrected: rewritten with ensure_ascii=True; the removed-line count for that file is now 1, and the parsed ceiling object is verified identical at base and at head.
3. [diff] C5 — The claim listed four minted roadmap rows and the diff mints five, because PMAT-592 is its own row and the sentence omitted it while purporting to enumerate the whole repair.
   - corrected: C5 now names PMAT-592, 594, 590, 591 and 595, and the count in the sentence matches the list beside it.
4. [diff] C6 — The claim said each receipt gains exactly two lines, a verdict and an end marker, which is not what the diff shows: each also gains the blank lines separating those from the surrounding prose, so the word exactly was wrong.
   - corrected: C6 now says the receipts gain those two lines plus their separating blanks.
5. [prose] R5 — The crux audit called this behaviour the fourth signature of an upstream investigation while the CHANGELOG paragraph for the very same behaviour at `CHANGELOG.md:11` calls it the third, so the document written to reconcile the release contradicted the release notes it reconciles.
   - corrected: the crux audit now says THIRD and records that an earlier draft said fourth and that a lane caught it against the changelog's own sentence.
6. [prose] R6 — The crux audit described this as a cadence release cut two days after 1.31.0, which two dates in this same diff refute: `CHANGELOG.md:8` dates it 2026-09-20 and the section below dates 1.31.0 at 2026-09-16, four days, against a two-day cadence.
   - corrected: the paragraph now says the cut is four days on from 1.31.0 and fifty hours overdue, and that the release is evidence of the cadence being missed rather than kept.
7. [prose] R7 — The ratchet baseline's new note quoted CB-2115 at 55 and then at 53 in consecutive sentences as though both described the same measurement, and the arithmetic offered to explain the repair only works from one of them.
   - corrected: every number in that entry now names its instrument, and the entry states that the residual one-finding gap comes from the check reading live GitHub between two runs.
8. [prose] R8 — A single reason was given for holding both #590 and #591 out of this release, that neither has reached a root cause, which the diff itself disproves for #591 whose roadmap notes name the cause to the file and line.
   - corrected: the two are now split, with #590 held for having a reproduction and no root cause and #591 held for being two halves neither of which is safe to ship alone.
9. [prose] R9 — The issue this cut filed about a missing tool on CI runners had the wrong denominator: it concluded the clean-room hosts disagreed with each other when the job in fact runs in a container and the tool is absent from every runner image on the fleet.
   - corrected: closed against the upstream infra ticket that owns the runner-image inventory, with the roadmap row cancelled rather than deleted so the wrong reasoning stays on the record.
