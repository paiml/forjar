# Quorum evidence — #366 / #369 — lane summaries

## probe lane
Built the branch; ran the four falsifier binaries, `cargo test --lib`,
clippy `-D warnings`, `fmt --check`. On 1.24.0: a rule scoped to
`systemd_unit` matched nothing; `query --type systemd_unit` found nothing
while printing `SystemdUnit`; two un-id'd rules sharing a message counted
as one and `remediate --policy-id` applied both. On the branch: enforced,
found, counted, selected one.

## crux lane
Terraform/OPA (Sentinel, conftest): a policy's identity is its declared
name or its file path plus rule name — never its message; a scope
mismatch is a validation error, not silence. Ansible-lint and Puppet's
lint use stable rule ids (`yaml[trailing-spaces]`, `E0001`) and refuse an
unknown id in a skip list. Kubernetes admission (Kyverno, Gatekeeper)
identifies a constraint by kind+name and reports violations under it, one
per constraint. This branch moves forjar to that default: an index-stable
generated id, the document's own spelling for scopes.

## design lane
One identity (`display_id_at`) on both surfaces; a structural tally; the
serde spelling on the surfaces #366 names (the rest is #433).

## judges
Two decisions scored: how to generate a missing id; whether to re-publish
`policy-coverage`. See the judges file.

## agy /teamwork
Independent plan-mode review in a scrubbed HOME (no publish/push
credentials reachable) — see the agy file.
