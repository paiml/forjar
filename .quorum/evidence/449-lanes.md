# Quorum evidence — #449 — lane summaries

## probe lane
The S4 dogfood's sandbox (local, /tmp-only, `snapshot_generations: 3`):
apply → generation 0; destroy → no new generation; undo → exit 1 naming
"0 earlier generations". Control: apply, apply, undo restores the first
content. On the branch: destroy → generation 1; undo → the managed file's
bytes are back. Second round (after the agy refutation): the fourth case
died at its second apply on a stale `.b3` sidecar (claim 3), then on an
undo that announced "will be destroyed" and destroyed nothing (claim 4).
6/6 falsifier cases, 13380 lib tests, six undo suites green; clippy 0.

## crux lane
Terraform backs the state up (`terraform.tfstate.backup`) and bumps the
serial on EVERY state-changing command, destroy included; Nix records a
profile generation for every switch and `nixos-rebuild --rollback` rewinds
one regardless of what the switch did; Puppet has no undo, only `--noop`.
Recording the destroy's generation puts forjar at the Terraform/Nix
default: one rule for every state mutation.

## design lane
One call, the same helper apply uses, at the same point (after the lock is
rewritten), with the config pruned to what survived; destroy removes or
re-seals the sidecar; undo destroys what the target lacks before the
rollback (`src/cli/undo_prune.rs`).

## judges
One decision scored: record before or after the mutation. See the judges file.

## agy /teamwork
Independent plan-mode review in a scrubbed HOME — see the agy file.
