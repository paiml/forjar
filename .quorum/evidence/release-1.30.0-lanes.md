# PMAT-555 — the lanes

One round, three sandboxed agy lanes on head 81b2aae4 against `main`,
review-only (`writes=false`), each in its own clone with push removed, each
handed the full diff, the ticket and `docs/audits/impl-PMAT-555-receipt.md`.

| lane | model (declared = measured) | verdict | findings |
|---|---|---|---|
| 1 | `gemini-3.1-pro-high` | PASS | 0 |
| 2 | `gemini-3.6-flash-high` | PASS | 0 |
| 3 | `gemini-3.8-flash-medium` | PASS | 0 |

`agree = three lanes, three PASS`, so the artifact records `agreed: true` and
`pmat-merge` arms on it and on nothing else.

## Why a clean round here is worth less than it looks

A release cut is the easiest diff in the repository to pass: it changes no
Rust, so there is no logic to refute, and its claims are checkable against
files in the same commit. Three PASS verdicts over such a diff say that the
record matches the change — which is the whole job — and they say nothing about
whether the release is a good idea. The gates say that, and three of them said
no first.

The lanes were also given a diff that had already survived those gates. A round
run before them would have had four real defects to find and would have found
them; this one had none left, which is a fact about the ORDER of the work
rather than about the diff's quality.
