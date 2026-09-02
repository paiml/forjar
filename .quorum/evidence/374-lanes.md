# Quorum evidence — #374 — lane summaries

## probe lane
Built the branch and ran the eleven falsifiers plus `cargo test --lib`,
clippy `-D warnings` and `fmt --check`. On 1.24.0 with
`allowed_operators: [alice]` on two machines: `--canary-machine` and
`--refresh-only` converged/rewrote for `mallory` with exit 0; `--pre-script`
ran before the refusal; the canary's fleet leg rolled without a prompt for
`alice`. On the branch: refused with the ordinary path's own text, nothing
written, the prompt restored.

## crux lane
Terraform Cloud/Enterprise: run tasks and Sentinel policies evaluate WHO
may apply before any plan is applied, and a targeted/partial apply is still
an apply. Ansible Tower/AWX: job templates bind credentials and
execute-permission per template; a limit (`--limit canary`) does not change
the permission check. Pulumi: stack-level permissions gate `up`, and
`--target` is still `up`. Salt: `publisher_acl` gates who may publish which
functions to which minions; a batch of one is still a publish. None of them
let the SHAPE of the invocation (canary, refresh, pre-hook) route around the
operator check; forjar did.

## design lane
Position over copies: the gate joins the two cross-cutting checks at the top
of the dispatcher. Reads stay open, named, and fail-safe. A hook makes an
invocation an execution.

## judges
Two decisions scored: gate position vs per-exit copies; gate the reads or
not. See the judges file.

## agy /teamwork
Independent plan-mode review in a scrubbed HOME (no publish/push
credentials reachable) — see the agy file.
