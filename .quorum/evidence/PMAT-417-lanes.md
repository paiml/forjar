# Lanes — forjar#417

One round at head 83c5b130. Three lanes from three model families. None is the
author's model (claude-opus-5-5). Each lane was read-only and had the full
diff and the ticket.

- agy gemini-3.1-pro-high, plan mode, sandboxed: PASS. C1-C5 confirmed.
- claude-sonnet-5-5: PASS. C1-C5 confirmed. Findings: the fn predicate would
  over-count a signature such as `Result<HashMap<String, String>, E>` (none
  live today), and a trailing `//` or `/* */` comment after a lock literal
  is still counted.
- claude-haiku-4-5: PASS. C1-C5 confirmed. Findings: a mid-file
  `#[cfg(test)]` stops the scan for the rest of the file, which lowers the
  count, and the ceiling is `<=`, not `==`, so lowering it relies on review.

Every finding makes the count a floor or a looser bound. None lets a new site
through under a ceiling it should break. They are recorded as follow-ups on
the issue, not fixed here.
