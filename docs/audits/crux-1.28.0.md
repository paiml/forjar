# CRUX Audit — forjar 1.28.0

## Method

One `agy` teamwork lane (`conv-ba0f8bcf`, dispatched through the
`paiml-agy-delegate` for PMAT-226 phase 3) surveyed the systems in
`scripts/dogfood/crux-reconcile.sh`'s roster against each of the nine
behaviour changes in this release, from documentation memory only — no network
access and no live invocation of any reference system.

Every claim about a third-party system is marked `[X]`: asserted from
documentation memory, not measured against a running instance. Every claim
about forjar is marked `[V]` and carries a `path:line` citation the
orchestrator re-resolved against commit 97e72ce6 of branch
`PMAT-226-release-1.28.0` after the lane returned; where the lane's
description of forjar disagreed with the code it cited, the row below says
what the code does, not what the lane said. Three rows were corrected that
way: the lane called the quorum receipt "cryptographically signed" (it is
hash-bound — `diff_sha256` and blob hashes, no signature), said the #495 fix
"excludes namespaces from `machine_is_local`" (that predicate is unchanged;
the three sites ask a different one), and described the rail as
"cryptographically verified" (it is verified from the diff). The measurements
behind each `[V]` are in `docs/audits/impl-PMAT-215-receipt.md` through
`impl-PMAT-225-receipt.md` and, for row 9, `impl-PMAT-226-receipt.md`.

Six of the nine behaviours are defects — three found by an operator running
the tool against a real fleet, three found by the reviews of those fixes —
and three are gates the repository turned on itself. The comparison asks the
1.27.0 question of the defects ("did the surveyed systems ever have this
defect available to them, and why not") and a different one of the gates
("what do the surveyed projects do at this point in their own process").

## The comparison

| Behaviour | Systems surveyed | What the comparison found |
|---|---|---|
| (1) `Every ticket names the tag that` shipped it, a PR is tagged when it merges, and the two-day cadence is a gate | Kubernetes, Ansible, Chef, Terraform, SLSA | Every one of the four projects records a PR's release impact at merge time and joins it to the release at cut time, by construction rather than by a gate [X]. Kubernetes requires a `release-note` block on every PR and a milestone label the release team applies, and `krel` generates the release notes from the PRs between two tags [X]; Ansible merges one changelog fragment per PR and `antsibull-changelog release` collects them at the cut [X]; Chef's Expeditor reads labels on the merged PR and bumps the version and changelog itself [X]; Terraform requires a `.changelog/<PR>.txt` entry per PR and builds the changelog from them at release [X]. SLSA provenance attests which source revision produced an artifact, and nothing about which PRs or tickets that revision contains [X]. None of the five makes the cadence itself red: Kubernetes' code freeze refuses a merge that lacks the milestone, which gates the merge, not the cut [X]. forjar's join is the same shape as Kubernetes' — label at merge, reconcile at the cut — with the reconciliation run continuously and red by name: every ticket merged since the newest tag must carry `release:<next.tag>` [V `scripts/dogfood/tagged.sh:284-288`], and a cut past `next.due` with PRs merged and Cargo.toml unbumped fails the gate and says by how much [V `scripts/dogfood/tagged.sh:299-300`]. accept(the join the others compute once at the cut is computed every day and can go red; the cost is a label per ticket, which `release-goal.sh sync` applies) |
| (2) `The committed-quorum gate has a shape` a `kind: triage` branch can satisfy | Kubernetes, Chef, Ansible | All three lighten CI for a change that touches no code, and none of the three lighten the REVIEW: the skip is decided by path or by label, and review approval is never waived [X]. Kubernetes' prow runs a test job only when its `run_if_changed` path predicate matches and skips it under `skip_if_only_changed`, while OWNERS `lgtm` and `approved` apply to every PR [X]; Chef's Expeditor honours `Expeditor: Skip All` and its narrower siblings as labels on the PR [X]; ansible-test's `--changed` mode selects the tests the changed files reach [X]. The label form is the weak one — a label is applied by a person and can be applied to anything — and the path form is the strong one. forjar took the path form: a `kind: triage` receipt is admitted only when the diff, read from git, lies within the rail [V `scripts/quorum-gate.sh:384-386`], and the gate prints what it did not verify instead of printing nothing [V `scripts/quorum-gate.sh:512-517`]. accept(the rail is verified from the diff, so declaring the kind is not a way around the falsification; that is the property the label form lacks) |
| (3) `An I/O hash is recorded and` read only for a machine this host answers for | Bazel, Nix, Docker | All three key a build cache on the inputs as the BUILDER sees them, never on the requester's tree [X]. Bazel's action cache key digests the inputs and the execution platform, so a local and a remote hit do not cross [X]; Nix folds the `system` and every input derivation into the store path, so a hit is valid on any builder of that system and on no other [X]; Docker's layer cache keys `COPY`/`ADD` on the checksums of the context as the daemon received it, not the client's working directory [X]. forjar had the requester's tree in the key: the controller hashed its own files into every machine's lock, and the reader read them back for a `cache: true` task anywhere [V `src/core/executor/resource_meta.rs:43-45` — the reader now returns nothing for a machine this host does not answer for]. accept(a hash of files the builder never saw is not a cache key; refusing to answer is the only correct answer a controller can give for a remote tree) |
| (4) `A build-I/O probe answers only for` the tree it was taken on | Terraform, Ansible, Pulumi | All three address planned state per target instance and never let a measurement taken for one instance stand in for another [X]. Terraform's state keys every resource instance by module path, type, name and index, and a plan refreshes each instance from its own provider read [X]; Ansible runs a module — check mode included — on the target through the connection, so a fact about one host is never a fact about another [X]; Pulumi keys state by URN, which folds in the stack, so one stack's state cannot answer for another's [X]. forjar's probe map was keyed by resource id alone and answered for every machine the resource was declared on [V `src/core/task/probe.rs:79-81` — one map per machine now]. accept(keying by (machine, resource) is the per-instance addressing the others have by construction; the remote row keeps config-hash planning and is named as unprobed) |
| (5) `The plan names what it did` not probe | Terraform, Chef, Nix | Faced with state it cannot read, all three refuse rather than assume: Terraform's plan fails when a provider read fails, so an unreadable resource is never planned as unchanged [X]; Nix aborts evaluation when an input cannot be evaluated [X]; Chef's why-run mode executes on the node itself and therefore never plans a resource from a controller that cannot see it [X]. forjar plans a `NoOp` for a remote task it could not probe, deliberately — rebuilding every unprobed resource would break f(f(x)) = f(x) at the plan level — and carries the census of what it did not measure on every surface [V `src/core/planner/unprobed.rs:37`], with one definition of what a probe covers [V `src/core/task/probe.rs:357`]. accept(a disclosed blind spot beats failing the plan over an unreachable host, if and only if the disclosure is unconditional; it is rendered on every surface and round-trips through the sealed plan file) |
| (6) `A file resource on a remote` machine records that machine's baseline, not the controller's | Ansible, Chef, Puppet | All three compute a managed file's checksum on the target: Ansible ships its module to the host and digests there [X]; the Chef client runs on the node and hashes the file it manages [X]; the Puppet agent computes the checksum locally and compares it to the catalog [X]. In none of the three can a controller's file of the same name enter the comparison, because no code path on the controller ever opens it [X]. forjar's lock-baseline writer hashed the controller's file for every transport but a container [V `src/core/executor/helpers.rs:155-161` — the shared predicate decides, and a remote baseline goes through the reader drift uses]. accept(one predicate for apply and drift, one reader for both sides; the asymmetry was the defect, not the mechanism) |
| (7) `drift measures a guard the lock believes is broken` | Terraform, Ansible, Puppet | None of the three stop measuring a resource because a previous run failed on it [X]. Ansible and Puppet keep no failure verdict across runs and evaluate every declared resource's live state each time [X]; Terraform keeps one — a tainted resource — and its refresh still reads that resource from the provider rather than skipping it [X]. forjar skipped a `Failed` task as "not converged in the lock", treating a `completion_check` as a baseline comparison when it is a live assertion [V `src/tripwire/drift/task_check.rs:155-166` — no status stops the check; only the operator declining it does, and that stays named in the census]. accept(an assertion needs nothing from the lock to be answerable; a failed apply makes the question urgent, not unanswerable) |
| (8) `A pepita namespace is not the` controller at the three sites that read one | Ansible, Kubernetes, Podman | All three treat an execution environment on the same kernel as a different filesystem and reach it only through its transport [X]: Ansible addresses a namespace or container through a connection plugin, never by reading local paths [X]; `kubectl cp` and `kubectl exec` go through the API server and never assume a host path maps to a pod path [X]; Podman isolates the container's filesystem and requires `podman exec` or `podman cp` to read it from the host [X]. forjar's build-I/O probe, pre-plan probe and output verification read the controller's tree for a namespaced machine because they asked `machine_is_local`, which admits a namespace [V `src/core/task/probe.rs:353-358` — the shared predicate, not `machine_is_local`, at all three sites; `machine_is_local` itself is unchanged because the dispatcher returns for pepita before consulting it]. accept(one predicate, and a rule over all of `src/` so a fourth site cannot reappear) |
| (9) `The committed-quorum gate's triage rail admits` the release ledger | Kubernetes, Chef, Nix, SLSA | The bookkeeping that follows a cut is, in all four, either a bot's commit or an attestation — never a reviewed change judged as code [X]. Kubernetes' `CHANGELOG/` files are generated by `krel` and land through PRs that trigger no test job under `run_if_changed` but still need OWNERS approval [X]; Chef's Expeditor commits the version bump and changelog itself as a bot, outside review [X]; nixpkgs' release managers bump `.version` directly on the release branch [X]; SLSA records the release as a signed attestation from the builder, not as a change in the tree [X]. forjar's ledger row is a reviewed change in the tree, and before this release it had no honest shape: a `kind: triage` receipt was refused by name because the ledger was off the rail, and a `kind: code` one could anchor no citation. The rail now names the ledger by file [V `scripts/quorum-gate.sh:384-386`]; any other path outside it is still refused by name. accept(the Kubernetes shape — no test job, review still required — expressed as a rail verified from the diff; the ledger is named by file so `docs/roadmaps/**` does not become a door) |

## Gate H keys

`scripts/dogfood/crux-reconcile.sh` reads `CHANGELOG.md` for paragraphs that
open with a bold span at column 0 after a blank line, takes the first six words
of that span (backticks removed) as the key, and requires one row above
containing the key verbatim and naming at least three systems from its roster.

| Key, verbatim as the gate derives it | Systems the mapped row names |
|---|---|
| Every ticket names the tag that | Kubernetes, Ansible, Chef, Terraform, SLSA |
| The committed-quorum gate has a shape | Kubernetes, Chef, Ansible |
| An I/O hash is recorded and | Bazel, Nix, Docker |
| A build-I/O probe answers only for | Terraform, Ansible, Pulumi |
| The plan names what it did | Terraform, Chef, Nix |
| A file resource on a remote | Ansible, Chef, Puppet |
| drift measures a guard the lock | Terraform, Ansible, Puppet |
| A pepita namespace is not the | Ansible, Kubernetes, Podman |
| The committed-quorum gate's triage rail admits | Kubernetes, Chef, Nix, SLSA |

## The limit of this audit

Thirty-two system claims, all `[X]`. The lane read no vendor documentation
during the run and contacted no running instance, so each is documentation
memory and could be out of date — the exact prow field names and Expeditor
label spellings in rows 2 and 9 are the claims most likely to have moved. What
the audit is for is the SHAPE of the comparison, and the shape is not
delicate: that every surveyed build cache keys on the builder's view of the
inputs (3), that per-instance state is addressed per instance by construction
(4, 6, 8), that no surveyed system stops measuring a resource because it once
failed (7), and that every surveyed project records release impact at merge
and reconciles at the cut (1, 9), would each survive a correction to any
single row. The one claim that is forjar's alone is row 1's: that the
reconciliation runs every day and goes red, which no surveyed project does
and which this release is the first cut under.

CRUX-1.28.0-END
