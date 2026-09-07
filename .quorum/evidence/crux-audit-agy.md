# Independent review — agy /teamwork-preview — PMAT-164

One /teamwork-preview lane reviewed the audit at 240fb93f without seeing the claim lanes; its clone lost its object store mid-run, so its code-facing claims were asserted from the document and the lane files, which stayed readable.

## Verdict (FAIL)

I have completed the independent review of the CRUX audit document `docs/audits/crux-1.26.0.md` as requested. 

1. **Behaviour row mismatch**: Behaviour (4) claims that an emptied selection is refused entirely, which describes what the ticket wanted, rather than reflecting the code's exemption for machine narrowing.
2. **Contradictory rationale**: In behaviour (4), the document cites `Terraform -exclude` and rationalizes rejecting it because it can "report false success". This contradicts Terraform's documented behaviour because Terraform does not have an `-exclude` flag.
3. **Name-drop without mechanism**: Behaviour (3) cites Make and simply drops the error message (`fails with "No rule to make target"`) without explaining the resolution mechanism.
4. **Lane attributions**: All three lane citations are accurately attributed. Lane 1 calls it a blocked safe operation, Lane 2 calls it a "Temporary safe refusal", and Lane 3 uses the exact words "conflates history".
5. **Reconciliation check**: The document passes the structure check. There are exactly 9 behaviour bullets in the reconciliation list, and each behaviour has exactly 3 rows in the comparison table.

*(Note: Direct `git show` against `PMAT-161-state-stamp-per-name` failed due to missing Git objects in the isolated test environment, but the analysis was completed using the document and lane files.)*

## Findings, as returned

- T-F1 [asserted] docs/audits/crux-1.26.0.md:40 — The 'forjar today' cell for behaviour (4) describes what the ticket wanted rather than the code. It claims ALL emptied selections are refused ('An --exclude/--skip that removes every selected resource is refused'), but the actual code exempts machine narrowing.
- T-F2 [asserted] docs/audits/crux-1.26.0.md:41 — The reject rationale for Terraform in behaviour (4) ('refusing an emptied selection catches an operator's typo instead of reporting false success') contradicts Terraform's documented behaviour because Terraform does not even have an '-exclude' flag to report false success on.
- T-F3 [asserted] docs/audits/crux-1.26.0.md:36 — Behaviour (3) Make row is a name-drop without a mechanism. It simply states the error message 'fails with "No rule to make target"' without explaining the mechanism Make uses.
- T-F4 [measured] docs/audits/crux-1.26.0.md:68 — The three reference lanes DO say what the document attributes to them. Lane 1 states 'forjar blocks a safe, isolated operation', Lane 2 states 'Temporary safe refusal', and Lane 3 states 'Forjar conflates history; Pulumi scopes it' and 'Forjar lacks K8s's resource-scoped history'.
- T-F5 [measured] docs/audits/crux-1.26.0.md:79 — A dogfood/crux-reconcile check would PASS. Every one of the 9 changelog bullets maps to exactly 3 rows in the table. There are no bullets without a row and no behaviours with fewer than 3 systems.

## What became of it

Its three asserted defects were each true of the document and fixed in 17bdb497 (the behaviour-4 rule, the Terraform hedge, the Make mechanism); its two measured findings — the three cited lanes say what the audit attributes to them, and a crux-reconcile check passes — stand.
