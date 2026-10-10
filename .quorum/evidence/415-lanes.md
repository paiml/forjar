# Lanes — forjar#415

No lane ran the author's model (claude-opus-5-5). Each lane was read-only and
had the full diff and the ticket.

- r9 at head 4b3f22c2, the merged head being pushed: claude-sonnet-5 PASS
  twice and claude-haiku-4-5 PASS, AGREED, no findings; the agy
  gemini-3.1-pro-high plan lane PASS at the same head.
- r8 at 378578f9 (identical hunks; only the roadmap blob ids differ after
  the merge): claude-sonnet-5 PASS twice, claude-haiku-4-5 PASS, AGREED.
- r7 at 378578f9: sonnet PASS twice, haiku NO-VERDICT (prose only, no
  structured verdict; not counted), agy gemini-3.1-pro-high PASS.

Counted lanes: agy gemini-3.1-pro-high, claude-sonnet-5 and
claude-haiku-4-5, all from r9: three model families.

Rounds r3 to r6 at earlier heads each had one or two sonnet FAILs. Every
finding is a refuted claim in the judges digest and was fixed before r7.
