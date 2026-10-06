# Lanes — forjar#590

One counted round at head e18cd9ec, 14m budget. Three lanes from two
families, none the author's model (claude-opus-5-5). Each lane was
read-only and had the full diff and the ticket.

- claude-sonnet-5: PASS, no findings. Confirmed the seeding and unlatching
  paths now hash and check the resolved resource, resolved with the
  config's secrets, which is how the planner reads the entry back.
- agy gemini-3.1-pro-high, plan mode, sandboxed: PASS, no findings.
  Confirmed the planner's hash now matches the lock entry's hash.
- claude-haiku-4-5: PASS, no findings. Confirmed no regression for
  untemplated resources and that the marker file proves execution.
