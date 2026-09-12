# PMAT-534 — the claims put to the round

The branch adds arm 2b to `scripts/dogfood/release-check.sh`: after the tag and
the release exist, `repos/:owner/:repo/releases/latest` must RESOLVE TO THIS
RELEASE. A prerelease is reported with the command that promotes it; a full
release the URL does not point at is refused.

Three lanes, read-only, against a full standalone clone at `545a095f`.

LANE 1 — THE ARM. What `gh api … --jq .tag_name` does on 404, on a missing
field and unauthenticated; whether `2>&1` lets an error message be compared
against the tag; whether `latest_rel` collides with any existing variable or is
read by a later arm; whether `rc=0 … || rc=$?` is correct under
`set -euo pipefail`; WHERE arm 2b sits in the block structure; what
`note_pending` does to the verdict; and whether the `# mutation:` comment names
a change that is specific to this arm.

LANE 2 — THE FOUR CASES AND THE STUB. Whether the stub's `api` branch matches
the argv the gate passes and comes first; whether the new stubs break the two
other suites that share the fixture; whether any assertion is satisfiable by a
different arm; whether any case is vacuous because the gate exits before
reaching arm 2b; and the single most valuable missing case.

LANE 3 — THE DOCUMENTS AND THE NUMBERS. Re-derive every number in the log;
whether M4 (`elif true`) is disclosed as the collateral kill it is; the
receipt's END marker and single `^verdict:` line; `${{ github.repository }}`
quoting in the new `release.yml` echoes; whether CLAUDE.md and the
`forjar-dogfood` skill promise more than the arm delivers; the roadmap row.

Every lane ended with the standing instruction: quote any sentence in the
receipt, the log, the commit messages or the script's own comments that a reader
could check and find false; quote it exactly, say what is actually true, and
cite where you measured it.
