# Changelog fragments

One file per PR: `changelog.d/<PMAT-id>[-slug].md`, holding the entry exactly as it
would appear under `## [Unreleased]` in `CHANGELOG.md` (bold headline paragraph,
then body). Do not edit `CHANGELOG.md` in a PR. The release cut runs
`make assemble-fragments`, which appends the fragments to `[Unreleased]` in file
name order and deletes them. Two fragments with the same ticket id fail
`make check-fragments`.

Roadmap rows work the same way: `docs/roadmaps/fragments/<PMAT-id>.yaml` holds one
row, starting `- id: <PMAT-id>` at column 0. An id already in `roadmap.yaml`
replaces that row at assembly; a new id is appended.
