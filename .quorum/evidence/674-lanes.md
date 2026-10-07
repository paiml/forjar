# Lanes — forjar#674

Lanes run through quorum-review.sh with the ticket and the full diff against
main (ce9e4906 for r1 to r3, c6c8591c for r4). None used the author's model (claude-opus-5-5). Each lane was
read-only.

- r1 at a842da60 (before the age guard): sonnet-5 FAIL with one finding, the
  age-literal decrypt (R2); a second sonnet-5 seat (the configured fallback
  for the gemini lane) and haiku-4-5 PASS. R2 was fixed by 281515fc.
- r2 recorded 281515fc but was launched at the commit before it, so it is not
  counted. haiku-4-5 raised R1; it is answered in the judges file.
- r3 at 281515fc: AGREED, 3/3 PASS. sonnet-5 PASS, no
  findings; gemini-3.1-pro-high PASS, citing the selection in plan.rs, the
  redacted provider in print_helpers.rs and both arms in template.rs;
  haiku-4-5 PASS, no findings.

- r4 at e23e1011 (281515fc with main c6c8591c merged in; the merge brought
  in only #680, so the source diff against main is unchanged): AGREED, 3/3 PASS. sonnet-5
  and haiku-4-5 PASS with no findings; gemini-3.1-pro-high PASS, citing the
  same five lines as r3.

Three model families in r4, each measured from its lane log.
