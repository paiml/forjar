# Lanes — forjar#410

Two rounds at the same head, 01f5e86b; the diff did not change between them.
No lane ran the author's model (claude-opus-5-5). Each lane was read-only and
had the full diff and the ticket.

- Round 3: claude-sonnet-5 PASS (twice) and claude-haiku-4-5 PASS. The
  dispatcher withheld the agy lane by policy for this diff.
- Round 4, canary shape: agy gemini-3.1-pro-high PASS; claude-haiku-4-5 PASS;
  the agy claude-opus-4-6-thinking lane answered NO-VERDICT because agy no
  longer offers that model, and it is not counted.

Counted lanes: agy gemini-3.1-pro-high (round 4), claude-sonnet-5 (round 3)
and claude-haiku-4-5 (round 4), three model families.

Earlier rounds 1 and 2 at prior heads each had one sonnet FAIL; both findings
are the refuted claims in the judges digest and were fixed before round 3.
