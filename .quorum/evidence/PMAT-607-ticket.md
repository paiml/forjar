# PMAT-607: the ticket, verbatim

The review brief carries only this ticket's title. Every claim file in
.quorum/evidence/PMAT-607-*.md is judged against the full row below, copied
byte for byte from docs/roadmaps/roadmap.yaml lines 4701-4723 at this head, by
`sed -n '4701,4723p' docs/roadmaps/roadmap.yaml`.

This PR carries only what the criteria below name. The last criterion makes
the T3 github_issue resolution a separate ticket, and no criterion names an
amend path, so neither is in this diff (5748bbc9 took both off). Gate T is not
green on this head, and the last criterion says so.

```yaml
- id: PMAT-607
  github_issue: 607
  item_type: task
  title: 'release goal: book v1.33.0 with the tool gate T names, and run the cut path without python3; a pre-release is not a release'
  status: inprogress
  priority: high
  assigned_to: null
  created: 2026-10-07T19:00:00Z
  updated: 2026-10-07T19:00:00Z
  spec: null
  acceptance_criteria:
  - 'scripts/release-goal.sh (label_row, unlabel_row, cut) and scripts/dogfood/lib/releases.sh (the ledger loader) run no python3: the loader is scripts/dogfood/lib/releases.awk, which reads the ledger''s subset of YAML and fails by line number on any other line shape, and the row and ledger edits are awk over the file text with the same messages and exit codes as before'
  - 'Every script that picks a release tag (lib/window.sh dogfood_prev_tag, tagged.sh reachable_tags_from_floor and lower_tag_of, release-goal.sh lower_tag_of, crux-reconcile.sh, release-check.sh) takes it through lib/tags.sh dogfood_release_tags, which keeps vX.Y.Z only, the one shape the ledger admits for a tag; a fixture with an rc tag above its release goes red under the old rule'
  - 'One fixture per form variant the port reads or writes (labels: [] and a labels list, an unlabel that empties the list, releases: [], a flow list, a comment line in a row), each shown red with the port reverted'
  - 'docs/roadmaps/releases.yaml has a v1.33.0 row written by `scripts/release-goal.sh cut v1.33.0 --next v1.34.0`, and next is v1.34.0'
  - 'Gate T (scripts/dogfood/tagged.sh) is not green on this head, and this ticket does not claim it is. It runs on main daily and before a tag, not in PR CI. Its first red is T3 on #646 and #643, which name only a GitHub issue; resolving one through a row''s github_issue changes what the gate accepts and is a separate ticket. After that, T5 asks for docs/audits/dogfood-1.33.0-receipt.md, the path the cut writes into the row. This change does not write that receipt because it was not measured, and a receipt that was not measured is not written'
  phases: []
  subtasks: []
  estimated_effort: null
  labels:
  - kind:code
  - release:v1.34.0
  notes: null
```
