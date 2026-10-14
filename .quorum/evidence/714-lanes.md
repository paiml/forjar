# Lanes — forjar#711 / #714

All rounds ran through quorum-review.sh with ticket PMAT-714 and the full
diff against main 9a37a2ff, read-only. The author model is claude-opus-5-5,
and no lane used it. No round was run with --pr, so no round touched arming.

- Decision round 1 (fix design, a vs b): void. The agy lane did not launch.
  sonnet-5-5 PASS and caught the plant; haiku-4-5 FAIL on brief errors.
- Decision round 2: sonnet-5-5 PASS, gemini-4-argon-high PASS (served model
  measured from its lane log), a second sonnet-5-5 sample PASS; all three
  caught the plant (R2). haiku-4-5 PASS twice but missed the plant, so it was
  not counted. Verdict: (b), a scoped deny.toml ignore with a reason, and not
  a waiver.
- Control round at the planted commit (`unmaintained = "none"`): NOT AGREED,
  sonnet-5-5 FAIL, gemini-4-argon-high FAIL, haiku-4-5 FAIL, each naming the
  planted line (R1, R4).
- Final round at 03bb633b: AGREED, 3/3 PASS. sonnet-5-5 PASS with two notes
  (PMAT-710 rides the branch; R3); gemini-4-argon-high PASS, measuring the
  Cargo.lock sets; haiku-4-5 PASS, no findings.

Three model families in the final round, each read from its lane log.
