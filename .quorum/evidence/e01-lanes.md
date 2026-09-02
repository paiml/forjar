# Quorum evidence — #403 (CRUX audit E01) — lane summaries

## probe lane
Re-ran the audit's measurement on the branch rather than trusting the changelog:
two resources declared twice through `parse_config`, differing in exactly one
field the old allowlist omitted. Eight such fields (`uid`, `ssh_authorized_keys`,
`timeout`, `working_dir`, `sudo`, `tag`, `driver_version`, `checksum`) now hash
differently; with main's `hashing.rs` restored all eight hash IDENTICALLY. That
is the defect the ticket measured (`state.lock.yaml` byte-identical across an
eleven-field config change) reproduced at the unit the gate can bind.

## crux lane
Terraform hashes the ENTIRE planned attribute set and lets a provider mark
specific attributes `Computed`/ignore — a denylist, never an allowlist.
Ansible has no desired-state hash at all: every module re-checks every argument
on every run, so a changed argument is always seen. Nix derives the store path
from every input of the derivation; an input that does not move the hash is a
bug called "impurity". Puppet compares every declared property against the
provider's `instances` each run. None of the four asks a hand-maintained list
which fields count. forjar's allowlist was BELOW the industry default; the
denylist brings it to parity.

## design lane
Two designs were live in the ticket: canonicalise the resolved `Resource` minus
a denylist, or hash the generated apply/check/destroy scripts. The scripts
option was rejected by the judges (see judges file): a script embeds resolved
secrets and host-specific paths, so the hash would leak and would differ across
hosts for one declaration. The denylist has the failure mode the ticket wanted —
it fails loudly — and `tests_hash_completeness` reflects over the serialised
`Resource` so a field added later is hashed until someone argues it out.

## judges
Three options scored; see the judges file. The denylist won on honesty and on
being the only option with a mechanical completeness guard.

## agy /teamwork
Independent stack review in plan mode; see the agy file. Ran against the diff
after the two quorum commits (the integration test and the tag-encoding fix).
