# Quorum evidence — #400 / #401 / #386 — lane summaries

## probe lane
Built the branch; ran the three falsifier binaries, `cargo test --lib`,
clippy `-D warnings`, `fmt --check`. Measured on the merge-base's script in
a synthetic repo: the cross-branch push refused for a receipt that exists;
the hash computed against the wrong tree; an untracked waived receipt
passed silently; a branch deletion hit `git merge-base` with an all-zero
sha. On the branch: resolved from the pushed sha, read from the object
database, deletion short-circuited, the skip loud.

## crux lane
Kubernetes (Prow) validates the PR head remotely and has no local
pre-push binding at all; Terraform's `pre-commit` hooks look at the
working tree, not the pushed commit; Ansible's CI clears exhausted caches
reactively. Binding a local hook to the pushed sha via `git cat-file`,
asserting the tracked/ignored invariant structurally, and refusing a
hosted job that caches an instrumented tree are above those defaults.

## design lane
One subject per verdict: the pushed sha. Nothing tracked under a
directory another tool owns and ignores wholesale. A guard that names the
cache actions it knows and the reduction that makes the tree fit.

## judges
Two decisions scored: re-include the baseline vs untrack everything under
`.pmat/`; build at the pushed sha vs skip loudly. See the judges file.

## agy /teamwork
Independent plan-mode review in a scrubbed HOME (no publish/push
credentials reachable) — see the agy file. No charge survived.
