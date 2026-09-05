# Crux lane — competitive survey — PMAT-160

One agy lane (conversation conv-c8d71ac2, sandboxed, no network, 68 s) surveyed four systems from documentation memory; every external finding is self-labelled `asserted`. The orchestrator cross-checked the four load-bearing facts against documentation memory as well — none was fetched over the network during this quorum, and the receipt says so.

## Verdict (PASS)

### Competitive Survey: Targeted Run Semantics

| System | 1. Closure Included? | 2. Dry-run Scope Identical? | 3. Excluded Dependency | 4. Empty Selection |
|--------|----------------------|-----------------------------|------------------------|--------------------|
| **Terraform** | Yes (target + dependencies) | Yes (by construction) | Dependent runs anyway (assumed satisfied in state) | Warning / No-op |
| **Ansible** | No (runs target alone) | Yes (by construction) | Silently skipped, dependent runs anyway | No-op |
| **SaltStack** | No (runs target alone) / Refuses if missing | Yes (by construction) | Refused (Missing Requisite Error) | Error |
| **Puppet** | No (runs target alone) | Yes (by construction) | Silently skipped (relationships dropped), dependent runs anyway | No-op |
| **Forjar** (current) | Yes (positive selection closed over `depends_on`) | Yes (by construction) | Contracted edge (dependency assumed satisfied), dependent runs anyway | Refused (Error) |

### VERDICT

**PASS**. Forjar's chosen semantics (closure expansion, edge contraction with the dependent still running, refusal of an empty selection, dry-run/check/apply agreeing by construction, and `-m` left as an executor-level filter) are at least as sound as the field's. 

Forjar deviates from Ansible and Puppet's "run target alone" by expanding the closure to include dependencies (matching Terraform); this is a defensible choice that prevents runtime failures in declarative DAGs. Dry-run and apply scopes agree by construction, identically matching all surveyed systems. For explicitly excluded dependencies, forjar contracts the edge so the dependent runs anyway, deviating from SaltStack's strict refusal but matching Puppet's relationship-dropping and Terraform's behavior; this is a defensible choice that allows safe partial runs without stalling downstream dependents. Finally, refusing an empty targeted selection deviates from the silent no-op of Terraform, Ansible, and Puppet, but matches SaltStack. This is a deliberate, defensible choice to avoid vacuous successes when an operator mistypes a selector. No surveyed system shows a clearly better rule that forjar violates.

## Findings, as returned

- [cited] src/cli/apply_selection/closure.rs — Forjar validates the full graph, expands positive selection to its depends_on closure, contracts edges for excluded dependencies (allowing dependents to run), and explicitly errors on empty negative selections.
- [asserted] Terraform CLI docs, Command: plan — Resource Targeting — I assert from memory that Terraform targeted runs include dependencies; dry-run (plan) and apply share identical scope by construction; excluded dependencies cause the dependent to run anyway (read from state); and empty selection yields a warning or no-op.
- [asserted] Ansible Documentation, Tags; Ansible Documentation, ansible-playbook — I assert from memory that Ansible --tags runs the target alone without expanding closure; --check identically scopes by construction; --skip-tags explicitly skips the dependency while the dependent runs anyway; and an empty selection is a no-op.
- [asserted] SaltStack Documentation, States; SaltStack Documentation, State Requisites — I assert from memory that SaltStack targeted runs execute the target alone but refuse if a required dependency is missing; test=True identically scopes by construction; explicitly excluded dependencies cause dependents to be refused; and an empty selection errors.
- [asserted] Puppet Documentation, Tags; Puppet Documentation, Run Puppet in noop mode — I assert from memory that Puppet --tags runs the target alone; --noop identically scopes by construction; excluded/untagged dependencies are silently skipped by dropping relationship edges so the dependent runs anyway; and an empty selection is a no-op.

## Orchestrator cross-check (documentation memory, asserted)

- Terraform, *Resource Targeting* (`terraform plan -target`): a targeted plan includes the resources the target depends on, and `terraform apply` of that plan executes exactly the reviewed set — closure plus plan/apply agreement by construction. Forjar matches on both.
- Ansible, *Tags* (`--tags`, `--skip-tags`) and *Check mode* (`--check`): only tagged tasks run, dependencies are not pulled in, a skipped prerequisite is silently dropped, and check mode scopes exactly as the run does. Forjar deviates on closure (deliberately, the declarative-DAG argument) and matches on check/apply agreement.
- SaltStack, *Requisites* (`require`, `watch`) and `state.apply ... test=True`: a missing requisite is an error (`The following requisites were not found`), `test=True` scopes exactly as the apply does, and an empty target errors. Forjar matches on refusing an emptied selection and on dry-run agreement; it deviates on a cut prerequisite (contraction, the dependent still runs) — the Puppet/Terraform behaviour, chosen so `--skip` of a converged prerequisite remains usable.
- Puppet, *Tags* (`--tags`) and `--noop`: only tagged resources are managed, relationship edges to unmanaged resources are dropped so the dependent still applies, and `--noop` scopes identically. Forjar matches on contraction and on noop agreement.

Deviations, each defensible and named in CHANGELOG.md: closure expansion (matches Terraform, not Ansible/Puppet), edge contraction on an explicit negative (matches Puppet/Terraform, not Salt), refusal of an emptied selection (matches Salt, not Terraform/Ansible/Puppet). No surveyed system has a rule forjar violates.
