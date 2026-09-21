# Judges — impl receipts for the v1.32.0 window (PMAT-601)

Round count and heads: see `PMAT-601-lanes.md`.

## CONFIRMED

1. [ledger] C1 — Every one of the six PRs merged since v1.31.0 carries an impl receipt that ends with its own marker and has exactly one verdict line, which the Claude lane verified file by file and which gate A reports on this branch as six of six.
   - evidence: the measured gate output is quoted at `docs/audits/impl-PMAT-601-receipt.md:15`.
2. [honesty] C2 — Each new receipt says in a section of its own that the work was implemented directly with a quorum rather than through the paiml-implement harness, and none invents a discovery file, a route verdict or a dispatch ledger for work that never ran through it.
   - evidence: the section opens at `docs/audits/impl-PMAT-592-receipt.md:5` and its counterparts; the Claude lane searched the tree and found no harness artefact for any of the four tickets.
3. [honesty] C4 — Every receipt carries a gaps section naming what is unresolved, from the stale runner overrides and the unmeasured mutation workflow to the detector refinements no lane reviewed, instead of presenting its work as finished.
   - evidence: `docs/audits/impl-PMAT-598-receipt.md:27` and `docs/audits/impl-PMAT-567-receipt.md:29`.
4. [rail] C5 — The diff touches only the four new receipts under the audits directory, the roadmap, and files under the quorum directory, which is exactly the set of paths the triage rail admits.
   - evidence: the refutation that a code-labelled row was edited conflated a row's label with the path rail, and is recorded as not a finding.

## REFUTED

1. [cite] C3 — The claim that every factual claim matches its record failed on one citation: the PMAT-601 receipt named its own quorum receipt as if it already existed, when that file is produced by the very quorum that was reading the sentence.
   - corrected: `docs/audits/impl-PMAT-601-receipt.md:10` now says the file is produced by this quorum and committed in the same PR.
2. [arith] R2 — The PMAT-592 receipt described gate T's failures as four findings while listing two missing labels and three stale statuses, which is five findings across three tickets, the same miscount the cut's own merged commit message made.
   - corrected: `docs/audits/impl-PMAT-592-receipt.md:16` now counts five and names each ticket.
3. [check] R3 — Both agy lanes could not verify that gate A reports six of six, because the number came from a local gate run and appeared nowhere in the diff or the commit messages they were given, so the claim was true and uncheckable.
   - corrected: the gate's output is quoted verbatim in the receipt, so any reader can check it against a rerun.
4. [record] R4 — The merged quorum receipt for the cut said four confirmed and nine refuted in its prose while its own structured counts said three and ten, a sentence written before the eighth claim moved to refuted and never updated.
   - corrected: the prose now matches the structured counts and says why it changed; the file is excluded from the diff hash, so the binding is unaffected.
