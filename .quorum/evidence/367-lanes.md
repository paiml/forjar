# Quorum evidence — #367 / #371 / #375 — lane summaries

## probe lane
Built the branch; ran the five falsifier binaries, `cargo test --lib`,
clippy `-D warnings`, `fmt --check`. On 1.24.0 with a selected workspace:
verb answers described an empty project while the CLI saw the applied
state; over stdio no tool carried annotations; `docs/mcp-schema.json` was
stale against `--schema`. On the branch: the verb surface follows the
selection, every tool sends `readOnlyHint`, the checked-in copy is gone,
and `outputSchema` is not promised.

## crux lane
Terraform's `-chdir`/workspace selection binds every command to one
state, CLI and automation alike (Terraform Cloud's API reads the same
workspace the CLI does). Pulumi's stack selection is a single source of
truth for `pulumi` and the Automation API. The MCP specification
(2025-06-18) makes `annotations.readOnlyHint` advisory and `outputSchema`
binding; reference servers publish the hint and omit the schema unless
they fill `structuredContent`. This branch puts forjar at that default.

## design lane
One joined resolver for readers, one unjoined for the enumerator; an
in-tree pmcp server so the wire carries what `--schema` says; the docs
describe the surface instead of copying it.

## judges
Three decisions scored: where the join lives; how to get annotations on
the wire; whether to publish `outputSchema`. See the judges file.

## agy /teamwork
Independent plan-mode review in a scrubbed HOME (no publish/push
credentials reachable) — see the agy file.
