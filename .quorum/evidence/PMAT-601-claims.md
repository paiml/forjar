# Claims — impl receipts for the v1.32.0 window (PMAT-601)

Counts live in `PMAT-601-lanes.md`.

- **C1** every PR merged since v1.31.0 carries a well-formed impl receipt; gate A reports 6 of 6
- **C2** each new receipt states the work was done directly with a quorum, not through the paiml-implement harness, and invents no harness artefact
- **C3** every factual claim in the receipts matches the record it cites
- **C4** each receipt names its own gaps
- **C5** the diff touches only paths on the kind: triage rail

The receipts are the audit trail for a release, so the lanes were asked to treat
an overstatement as a durable falsehood. Every structural claim held. What did not
hold was, again, arithmetic and checkability in the prose.
