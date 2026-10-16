# Claims — forjar#674: plan --output-dir exports the plan's selection, never a secret

Briefed to every lane with the ticket and the full diff against main
c6c8591c (which carries #680); head e23e1011, which is 281515fc with that
main merged in; 281515fc added the age-literal guard.

- C1: `plan -r a --output-dir D` succeeds while out-of-scope `b` references an unset secret, and D holds `a`'s scripts only.
- C2: an unscoped export writes each secret as `FORJAR_REDACTED_SECRET_<key>` and never the value, even when the provider could resolve it.
- C3: an export writes an `ENC[age,...]` literal as it stands, in both a default and an `encryption` build.
- C4: reverting the age guard turns the age and unscoped tests red; reverting the whole fix turns all three red.
- R1 (haiku-4-5, r2): the selection read from `plan.changes` drops a selected resource whose action is NoOp.
- R2 (sonnet-5, r1): the export still decrypts an `ENC[age,...]` literal after template substitution.
