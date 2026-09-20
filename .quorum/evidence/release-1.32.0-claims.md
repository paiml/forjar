# Claims — v1.32.0 release cut (PMAT-592, PR #593)

Eight claims about the diff, put to three independent lanes. Counts live in
`release-1.32.0-lanes.md`; numbers here are quotations of that table.

## The eight

- **C1** `Cargo.toml:3` moves 1.31.0 to 1.32.0 and `Cargo.lock:1171` moves with it. No other CRATE version is asserted anywhere in the diff.
- **C2** `CHANGELOG.md:8` renames `## [Unreleased]` to `## [1.32.0] - 2026-09-20`, and the behaviour paragraph beneath it at `CHANGELOG.md:10` is unchanged by this branch.
- **C3** `docs/audits/crux-1.32.0.md` carries exactly one behaviour row containing the first six words of the bold span at `CHANGELOG.md:10`, with no backticks inside that span, which is what gate H greps for as a fixed string.
- **C4** No ratchet ceiling in `scripts/ratchets/cb21xx-baseline.json` changes; only the `why` array grows.
- **C5** The gate B repair is real work: title syncs, a status correction, release bindings and minted rows. No check skipped, no floor lowered.
- **C6** The three `docs/audits/impl-PMAT-579|581|582-receipt.md` each gain a `verdict:` line and a final `IMPL-PMAT-<n>-RECEIPT-END` marker.
- **C7** The title syncs are lossless: every row whose `title:` changed carries its original title verbatim in `notes:`.
- **C8** No `.rs` file changes, so v1.32.0 ships exactly the code that merged in the window.

## What the lanes were asked to do

To REFUTE, and specifically to quote any sentence a reader could check and find
false. A release cut's prose is a durable public claim about how the software
behaves, so overstatement is a finding and not a style note. The lanes were
read-only: no repository, no shell, no network, no credentials, and no build.
Everything they could rely on was pasted into the brief — the eight claims, the
recorded `make dogfood-release` result, and the complete base..head diff.

## What came back

Every STRUCTURAL claim about the diff held under three lanes. What did not hold
was the PROSE: four sentences in this cut's own documentation were checkable and
false, and one tree defect was found that no gate would have caught. That split
is the finding worth keeping — a green nine-gate run said nothing about whether
the release described itself truthfully.
