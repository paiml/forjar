# Crux lane — competitive survey — PMAT-161

One agy lane (conv-68c7fbbb, 63 s, sandboxed, no network) surveyed the field from documentation memory; every third-party figure is marked [X] and every finding is self-labelled asserted. The orchestrator's cross-check below is likewise from documentation memory and marked asserted.

## Verdict (PASS)

### Competitive Survey: State Management and Ownership

| System | Stack Identity | Wrong Stack Detection | Cross-Stack Machine/Resource Ownership | Rename Operation |
|--------|----------------|-----------------------|----------------------------------------|------------------|
| **forjar** | Name-keyed `stacks:` map in shared `forjar.lock.yaml` | Warns if SAME name applies from DIFFERENT `-f` config file | Prevents cross-stack fighting: warns if a machine is recorded under ANOTHER stack's entry | Natively supported; applying same config under new `name:` retires old name and moves lineage |
| **Terraform** | One backend, many workspaces; `terraform state` lineage ID [X] | State locking prevents concurrency; workspaces isolate state [X] | Two workspaces can import and fight over the same resource (no cross-workspace locks) [X] | `terraform workspace select/new/delete` or `state mv`; no native CLI workspace rename [X] |
| **Pulumi** | Project + Stack name in backend; URNs contain stack name [X] | Backend enforces stack identity; URN mismatch on wrong config [X] | Like TF, multiple stacks can import the same resource and conflict [X] | Natively supported via `pulumi stack rename` (preserves lineage/URNs) [X] |
| **Nix** | Profile symlink to generation (e.g. `nix-env --switch-profile`) [X] | Profile path explicitly selected [X] | Multiple profiles can exist on one machine; conflicts happen at path level [X] | No native rename; simply point a new symlink to a generation [X] |
| **Kubernetes** | Namespaces + ownerReferences [X] | Namespaces isolate resources [X] | `ownerReferences` prevent multiple controllers from adopting the same child resource [X] | Resources/namespaces cannot be natively renamed; must recreate [X] |
| **Ansible** | Stateless; playbooks and inventory define targets [X] | N/A (no state) [X] | Playbooks blindly overwrite each other (last apply wins) [X] | N/A (just rename playbook file) [X] |

**Verdict Paragraph:**
PASS. forjar's rules are at least as sound as the field's. By explicitly checking if a machine is already owned by another stack in the shared state directory, forjar prevents cross-stack resource fighting, providing stronger cross-stack safety than Terraform [X] and Pulumi [X] which allow users to import and fight over the same physical resources. It matches the robustness of Kubernetes [X] `ownerReferences` and improves upon the lack of ownership tracking in Nix [X] and Ansible [X]. Furthermore, forjar natively supports rename-as-lineage (matching Pulumi [X] and exceeding Terraform CLI [X]), and safely refuses restore operations when multiple stacks share a directory to prevent cross-stack corruption.

## Findings, as returned

- [asserted] — Terraform identifies stacks via workspaces with isolated state files and lineage IDs. It relies on operators not to import the same resource into multiple workspaces, as it lacks cross-workspace locks on physical resources. CLI workspace renames require manual state moves [X].
- [asserted] — Pulumi identifies deployments by project and stack name. It natively supports stack renaming (preserving lineage) but, like Terraform, does not inherently prevent two stacks from importing and fighting over the same physical resource [X].
- [asserted] — Nix identifies stacks using profile symlinks pointing to specific generations. Multiple profiles can exist on a machine, but renaming is not natively modeled beyond updating symlinks [X].
- [asserted] — Kubernetes relies on namespaces for isolation and ownerReferences to explicitly prevent controllers from adopting and fighting over the same child resources. Resources cannot be natively renamed [X].
- [asserted] — Ansible is stateless, operating without a central state file or stack identity. It does not detect wrong stacks or track resource ownership, allowing playbooks to overwrite each other [X].

## Orchestrator cross-check (asserted)

- Terraform: one backend, many workspaces; state locking is per workspace; two workspaces may import the same physical resource and fight over it; there is no native workspace rename in the CLI. Forjar's machine-ownership guard is stronger.
- Pulumi: per-stack state; `pulumi stack rename` keeps history; two stacks may manage one resource unless protected. Forjar's rename-as-lineage matches.
- Nix profiles: one profile dir, many generations; `--switch-profile` changes the pointer; no notion of two owners. Forjar's refusal of a multi-stack restore is the honest analogue until PMAT-162.
- Kubernetes: ownerReferences give a single owner per object; a second controller adopting it is a conflict. Forjar's machine-owned-by-another-stack refusal is comparable.
- Ansible: no state, so no wrong-stack guard at all; forjar's stamp is a superset.

No surveyed system has a rule forjar violates; the one deviation (refusing a multi-stack restore rather than scoping it) is the deferred PMAT-162.
