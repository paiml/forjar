# Quorum evidence — #406 (CRUX audit E04) — lane summaries

## probe lane
Applied a `file` resource whose `content:` interpolates `{{secrets.api-token}}`
and read `state/local/runs/<run>/` on unfixed main: the base64 of the whole
content line sat in `.log`, `.json` and `.script`. Ran the ticket's own
success criterion (grep for the plaintext) and watched it PASS on the broken
tree — the plaintext is encoded and misaligned, so substitution finds nothing.
Then seeded a tracked transcript and ran a real `--auto-commit`: nine
transcripts went into the commit despite `:(exclude)state/*/runs/`.

## crux lane
Ansible `no_log: true`, Chef `sensitive true` and Salt `file.managed
show_changes: False` are each a decade old and each SUPPRESS rather than
redact, because a redactor that has to recognise every encoding of a secret
is a denylist over an open set. forjar now has both: redaction for the two
encodings its own codegen produces (literal splice, base64 file content) and
suppression (`sensitive: true`, or automatically for ciphertext it cannot
name) for everything else. Terraform's `sensitive = true` marks values in
plan output the same way. At the industry default; below it before.

## design lane
Three layers, each for a case the others cannot cover; the value list is
re-derived from the UNRESOLVED resource by scanning its serialised form, so a
field added later is redacted without anyone remembering to list it. The
`--auto-commit` pathspec was measured against a repository with a tracked
transcript, which is the only shape that mattered, and rewritten to the
per-path form git actually honours.

## judges
Two options scored for the ciphertext case and two for the git exclusion;
see the judges file.

## agy /teamwork
Independent stack review; see the agy file.
