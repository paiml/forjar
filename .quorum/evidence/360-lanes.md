# Quorum evidence — #360 / #362 — lane summaries

## probe lane
Applied the branch's own diff onto the integrated tree (every open PR)
and built it there; ran the two mask falsifiers, the cron exec suite, the
observation_mask and cron unit tests, clippy and fmt. On the merge-base:
`ignore_drift: ["mode"]` switched the whole comparison off; a cron
schedule edit left the old job scheduled forever; `backup` orphaned
`backup-db`. On the branch: one field ignored, the rest watched; one
block per job, exact-line.

## crux lane
Terraform's `lifecycle.ignore_changes` scopes a structured attribute;
Puppet's `audit` metaparameter tracks per-attribute state; Ansible's and
Puppet's cron providers parse and replace a named job. forjar observes a
shell transcript; the token mask and the exact-line block are the honest
forms of the same two ideas on that substrate. At the default for
`ignore` semantics; the crontab parser is the recorded gap (#445).

## design lane
Mask at every writer; record the mask with the baseline; census what
cannot be compared. Delete only what forjar wrote, as a block.

## judges
Two decisions scored: per-field lock schema vs stdout mask; delete the
block only when intact vs replace whatever sits under the marker. See
the judges file.

## agy /teamwork
Independent plan-mode review in a scrubbed HOME against the preview tree
— twelve attacks refuted, one confirmed (#445), one unique finding
(double-reported files) fixed in the rebuild. See the agy file.
