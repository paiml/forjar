# Lanes — forjar#671

One counted round at head af12be69 (round 4; round 3 at the same head had
the agy lane time out at 9m with no verdict, so it is not counted). Three
lanes from two families, none the author's model (claude-opus-5-5). Each lane
was read-only and had the full diff and the ticket.

- claude-sonnet-5: PASS. C1-C4 confirmed. Checked that only nightly.yml:137
  and release.yml:273 contain `cargo install cross`, so discovery finds
  exactly the two real sites, and that CARGO_HOME is job-level env in both.
- agy gemini-3.1-pro-high, plan mode, sandboxed: PASS. C1-C4 confirmed; no
  claim refuted.
- claude-haiku-4-5: PASS. C1-C4 confirmed; no scope violations.
