# Claims — forjar#680: apply -r a runs with b's secret unset and never renders b

Briefed to every lane with the ticket and the full diff against main
ce9e4906; head 345c3e30 (1abaa323 with main merged in). Test-only: no source
change.

- C1: `apply -r a` succeeds while an out-of-scope resource `b` references an unset `{{secrets.unset-key}}`, writes `a` and its dependency `dep`, and never renders `b`.
- C2: the fixture is live: an unscoped `apply` of the same config fails, naming `unset-key`.
- C3: resolving every resource's templates before `resolve_selection` turns C1's test red.
- C4: dropping the `resolve_selection` call turns C1's test red.
- C5: a whole-file resolve placed after `resolve_selection`, where the executor runs, leaves both tests green, as the file header says.
