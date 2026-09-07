# Crux lane — PMAT-164 is itself the crux audit

The audit under review IS the release's competitive survey; its three source lanes (the PMAT-161 trcx run's CRUX panel, from documentation memory, no network, every third-party figure [X]) are reproduced here as the crux evidence for this ticket.

## Panel lane 1 (verdict FAIL)

| Behaviour | Reference System | How they do it [X] | forjar today [V] | Delta | Disposition |
|-----------|------------------|---------------------|------------------|-------|-------------|
| (1) Selectors close depends_on | Terraform `-target` | Selects target and its dependencies (closure) [X] | Resolves target and closes downward over `depends_on` [V] | Matches Terraform targeting logic | reject(forjar's downward closure ensures safe convergence) |
| (1) Selectors close depends_on | Ansible `--tags` | Selects only explicitly tagged tasks, ignores deps [X] | Closes downward over `depends_on` [V] | forjar is safer by honoring the dependency graph | reject(forjar's downward closure ensures safe convergence) |
| (1) Selectors close depends_on | Make goals | Builds target and all its prerequisites recursively [X] | Closes downward over `depends_on` [V] | Matches Make | reject(forjar's downward closure ensures safe convergence) |
| (2) apply --check honours selectors | Terraform `plan -target` | Restricts plan to target, exits 0 if clean [X] | Scopes check to selectors, exits 0 if clean [V] | Matches Terraform | reject(scoping check to selection prevents spurious failures) |
| (2) apply --check honours selectors | Ansible `--check --tags` | Checks only matching tags, exits 0 if clean [X] | Scopes check to selectors, exits 0 if clean [V] | Matches Ansible | reject(scoping check to selection prevents spurious failures) |
| (2) apply --check honours selectors | Kubernetes `diff` | Diff ignores unselected external resources [X] | Scopes check to selectors, exits 0 if clean [V] | Matches Kubernetes | reject(scoping check to selection prevents spurious failures) |
| (3) check verb refuses empty selector | Terraform `plan -target` | Errors with "Resource target not found" [X] | Refuses selector naming nothing up front [V] | Matches Terraform | reject(failing fast on typos prevents silent false-positives) |
| (3) check verb refuses empty selector | Make goals | Errors with "No rule to make target" [X] | Refuses selector naming nothing up front [V] | Matches Make | reject(failing fast on typos prevents silent false-positives) |
| (3) check verb refuses empty selector | Ansible `--check --tags` | Warns but exits 0 if nothing matches [X] | Refuses selector naming nothing up front [V] | forjar is stricter | reject(failing fast on typos prevents silent false-positives) |
| (4) empty negative selection refused | Terraform `-exclude` | May exit 0 saying "No changes" [X] | Refuses negative selector that empties selection [V] | forjar is stricter | reject(refusal prevents operators mistakenly thinking it succeeded) |
| (4) empty negative selection refused | Ansible `--skip-tags` | Exits 0 doing nothing [X] | Refuses negative selector that empties selection [V] | forjar is stricter | reject(refusal prevents operators mistakenly thinking it succeeded) |
| (4) empty negative selection refused | SaltStack targeting | Empty target returns an error [X] | Refuses negative selector that empties selection [V] | Matches SaltStack | reject(refusal prevents operators mistakenly thinking it succeeded) |
| (5) phony stripping contracts edges | Make | Executes phony targets, doesn't strip them [X] | Contracts `depends_on` edges through unrequested phony [V] | forjar improves on execution graph preservation | reject(contracting edges preserves transitive dependencies safely) |
| (5) phony stripping contracts edges | Terraform | No direct equivalent to phonies [X] | Contr

## Panel lane 2 (verdict FAIL)

### CRUX Analysis: forjar 1.26.0 Behaviour vs. Reference Systems

| Behaviour | System | How they do it [X] | forjar today [V from the code you read] | Delta | Disposition |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **(1) selectors select closure** | Terraform | `-target` selects the resource and its dependencies [X] | `resolver::goal_closure` closes the positive set over `depends_on` [V] | Matches Terraform's DAG closure | reject(DAG closure guarantees preconditions are met) |
| | Ansible | `--tags` filters tasks without strict DAG closure [X] | | Differs from Ansible's linear filtering | |
| | Make | Goal evaluates target and all prerequisites [X] | | Matches Make's closure | |
| **(2) apply --check honours scope** | Terraform | `plan -target` ignores out-of-scope drift, exits 0 if target clean [X] | check modes honour `ApplyScope` and exit 0 if out-of-scope resources drift [V] | Exact alignment | reject(Filtering scope before evaluation is industry standard) |
| | Ansible | `--check --limit` only evaluates in-scope hosts [X] | | Exact alignment | |
| | Kubernetes | `apply --dry-run` ignores cluster-wide drift [X] | | Exact alignment | |
| **(3) check verb refuses unknown** | Terraform | `plan -target=unknown` refuses immediately [X] | `check_existence` refuses unknown selectors upfront, follows closure [V] | Matches Terraform strictness | reject(Upfront refusal prevents silent false-positives) |
| | Ansible | `--check -t unknown` often exits 0 silently [X] | | Stricter than Ansible | |
| | Make | `make unknown` fails immediately [X] | | Matches Make strictness | |
| **(4) empty negative selection refused** | Terraform | `-exclude` emptying state results in "No changes" [X] | `empty_after_narrowing` refuses with error [V] | Stricter than Terraform | reject(Refusing an emptied selection catches typos immediately) |
| | Ansible | `--limit` matching no hosts exits with warning [X] | | Stricter/clearer error | |
| | Puppet | `--tags` with no matches exits 0 [X] | | Stricter than Puppet | |
| **(5) phony stripping contracts edges** | Make | Phony targets evaluate normally; skipped targets don't contract edges [X] | `contract_edges` connects dependents to the removed phony's dependencies [V] | Novel safe pruning approach | reject(Contracting edges preserves intended ordering) |
| | Terraform | Removing a node breaks dependent edges [X] | | Safer than dangling edges | |
| | SaltStack | Excluded requisites cause failures [X] | | Prevents failure cascades | |
| **(6) stamp keyed per config name** | Nix | Profiles are independent state histories [X] | `GlobalLock::stacks` map keyed by config name [V] | Enables shared state dirs securely | reject(Keying by stack elegantly supports multi-manifest repos) |
| | Terraform | Workspaces isolate state physically [X] | | Allows shared lock file | |
| | Pulumi | Stacks isolate state physically [X] | | Allows shared lock file | |
| **(7) undo refused if >1 stack** | Nix | `--rollback` scopes exactly to one profile [X] | `multi_stack_restore_refusal` blocks whole-dir restore [V] | Lacks Nix's scoped restore | adopt(The GO spec must implement stack-scoped restore by recording and replaying generations per stack name, matching Nix profile generations, rather than whole-dir snapshots.) |
| | Kubernetes | `rollout undo` targets a specific Deployment [X] | | Lacks K8s's scoped restore | |
| | Pulumi | Operations are scoped to the specific stack [X] | | Temporary safe refusal | |
| **(8) outputs merge per st

## Panel lane 3 (verdict FAIL)

| Behaviour | System | How they do it [X] | Forjar today [V] | Delta | Disposition |
|-----------|--------|--------------------|------------------|-------|-------------|
| (1) Selectors use depends_on closure | Terraform | `-target` includes downstream dependencies [X] | Selects positive closure down `depends_on` edges [V from CHANGELOG.md] | Aligns with Terraform's DAG execution | reject(Downward closure safely prevents unconverged prerequisites) |
| (1) Selectors use depends_on closure | Ansible | `--tags` evaluates linearly, no DAG auto-inclusion [X] | (as above) | Stricter than Ansible | (as above) |
| (1) Selectors use depends_on closure | Make | Target inherently builds prerequisites [X] | (as above) | Aligns with Make | (as above) |
| (2) apply --check honours selectors | Terraform | `plan -target` ignores outside drift [X] | Consumes resolved selection, exits 0 if only unselected drift [V from CHANGELOG.md] | Aligns with Terraform | reject(Correctly scopes assessment to requested subset) |
| (2) apply --check honours selectors | Ansible | `--check --tags` skips untagged [X] | (as above) | Aligns with Ansible | (as above) |
| (2) apply --check honours selectors | Puppet | `--tags --noop` limits scope [X] | (as above) | Aligns with Puppet | (as above) |
| (3) standalone check refuses empty | Terraform | `plan -target=typo` errors out [X] | Resolves through same resolver, refuses typo up-front [V from CHANGELOG.md] | Aligns with Terraform | reject(Up-front refusal prevents silent false positives) |
| (3) standalone check refuses empty | Ansible | Exits cleanly but warns if no tags match [X] | (as above) | Stricter than Ansible | (as above) |
| (3) standalone check refuses empty | Make | Fails with 'No rule to make target' [X] | (as above) | Aligns with Make | (as above) |
| (4) empty negative is refused | Terraform | Excluding all targets leaves empty plan [X] | Refused with 'no resources remain' [V from src/cli/apply_selection/narrow.rs] | Stricter than Terraform | reject(Explicit refusal prevents silent no-ops when action intended) |
| (4) empty negative is refused | Ansible | `--limit` matching no hosts fails [X] | (as above) | Aligns with Ansible | (as above) |
| (4) empty negative is refused | SaltStack | Target matching zero minions fails [X] | (as above) | Aligns with SaltStack | (as above) |
| (5) make phony stripping contracts | Make | Phony targets evaluate prerequisites [X] | Contracts edges through phony target [V from CHANGELOG.md] | Preserves DAG flow | reject(Maintains graph integrity for transitive ordering) |
| (5) make phony stripping contracts | Terraform | Removing node drops it, no edge auto-contract [X] | (as above) | Safer than naive edge drop | (as above) |
| (5) make phony stripping contracts | Nix | Derivations natively handle transitive chains [X] | (as above) | Analogous to Nix graphs | (as above) |
| (6) stamp keyed per config | Pulumi | Stack state centrally isolated [X] | Stamp `stacks:` map keyed by config name [V from src/core/state/stamp.rs] | Forjar shares state dir; Pulumi isolates | reject(Keying by name safely isolates identity within a shared dir) |
| (6) stamp keyed per config | Terraform | Workspaces isolate state files [X] | (as above) | Forjar allows multi-tenant files | (as above) |
| (6) stamp keyed per config | Kubernetes | OwnerReferences track controller ownership [X] | (as above) | Analogous to OwnerReferences | (as above) |
| (7) undo refused on >1 stack | Pulumi | Independent stack

## Systems named

Terraform (-target, -exclude, workspaces), Ansible (--tags, --limit, --check), SaltStack (targeting, requisites), Puppet (--tags, --noop), Make, Nix profile generations, Kubernetes rollout history and undo, Pulumi per-stack state — every behaviour has at least three; the single adopt row is behaviour 7 (stack-scoped restore, PMAT-162), the rest reject with rationale.
