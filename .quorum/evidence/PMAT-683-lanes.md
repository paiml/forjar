# Lanes — forjar#683

One counted round at head d2210701, 14m budget. Three lanes from two
families, none the author's model (claude-opus-5-5). Each lane was
read-only and had the full diff and the ticket.

- claude-sonnet-5: PASS, two non-blocking findings. The test's WORK_ROOT
  constant is one runner's layout; the run id and failure history in the
  doc comment are not checkable from the diff.
- agy gemini-3.1-pro-high, sandboxed: PASS. Confirmed the RUSTUP_HOME
  placement and the discovery of every cross job.
- claude-haiku-4-5: PASS, no findings.
