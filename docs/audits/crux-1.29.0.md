# CRUX Audit — forjar 1.29.0

## Method

One `agy` quorum lane surveyed world-class release-engineering, CI and
configuration-management systems against ten of the twelve behaviour changes in
this release, from documentation memory only — no network access and no live
invocation of any reference system. The lane read the release's CHANGELOG
section and the row keys `scripts/dogfood/crux-reconcile.sh` asked for, and a
read-only copy of `docs/audits/crux-1.28.0.md` as a worked example. It wrote
nothing in the repository; this file is the orchestrator's.

Every claim about a third-party system is marked `[X]`: asserted from
documentation memory, not measured against a running instance. Every claim
about forjar is measured in this session against the real tree, and the
measurements are in the receipts each row's ticket names.

## The comparison

| Behaviour | Systems surveyed | What the comparison found |
|---|---|---|
| (1) `Every tagged release names the cookbook` it was qualified against | Kubernetes, Nix, Ansible | All three verify releases against specific configurations: Kubernetes tests against specific cluster versions and configs [X]; Nix evaluates and tests against a specific commit of nixpkgs [X]; Ansible tests against exact versions of collections in the community package [X]. forjar names the exact commit hash of paiml/forjar-cookbook in the release ledger, and gate T refuses that ROW — not the tag, which already exists by then — when the cookbook at that commit cannot use the version that shipped. accept(explicitly naming the exact consumer configuration commit tested against prevents silent incompatibilities better than assuming semver adherence) |
| (2) `A shipped ticket says it shipped` | Chef, Terraform, Kubernetes | None of the three strictly block a release if an issue tracker drifts: Chef's Expeditor syncs statuses automatically [X]; Terraform relies on manual or PR-driven issue closing but doesn't fail the build on drift [X]; Kubernetes tracks issues via milestones but doesn't abort the release if a ticket state lags [X]. forjar's gate checks every ticket named in a tag and fails if any drifted. accept(failing the release on ticket drift forces the roadmap to reflect reality, even though coupling the build to external tracker latency makes the pipeline more brittle) |
| (3) `Gate R's verdict says what is` pending, and why | SLSA, in-toto, Bazel | All three provide specific, verifiable reasons for states: SLSA attestations require verifiable measurements rather than assertions [X]; in-toto checks that specific required evidence links are present [X]; Bazel reports exactly which targets failed or cached [X]. forjar now accurately reports if the release is pending a tag or cut, and measures the existence of the crux document rather than asserting it is missing. accept(verifying file existence as a measurement rather than asserting it ensures the audit trail is actually present, aligning with provenance best practices) |
| (4) `A PR runs what its change` can break; the release still runs everything | Bazel, Nix, Kubernetes | All three support selective execution: Bazel incrementally tests only targets affected by changed sources [X]; Nix builds only changed derivations [X]; Kubernetes uses prow to run tests selectively based on changed paths, while release jobs run the full suite [X]. forjar PRs skip heavy jobs for unclassified paths via an allow-list, while the release runs everything unconditionally. accept(skipping tests on known-harmless paths saves resources, and falling back to a full run at release ensures no edge cases escape the pipeline) |
| (5) `The release workflow stops assuming it` owns /tmp | systemd, Docker, Podman | All three use namespaces or specific configurations to avoid shared /tmp collisions: systemd services use PrivateTmp to isolate scratch space [X]; Docker provides an isolated filesystem per container [X]; Podman uses mount namespaces for a fresh /tmp per execution [X]. forjar's workflow now clobbers downloaded files and clears /tmp paths before use. accept(explicitly clearing shared paths on non-ephemeral runners is a necessary defense, though forjar is worse than the industry default here because it relies on manual cleanup rather than true filesystem isolation) |
| (6) `publish-release publishes a draft whoever created` it | Kubernetes, Chef, Terraform | All three decouple artifact promotion from the creator's identity: Kubernetes krel promotes artifacts across stages automatically [X]; Chef's Expeditor promotes channels based on pipeline progression [X]; Terraform release pipelines publish based on tags, regardless of who pushed the tag [X]. forjar's publisher job now asserts draft=false unconditionally, fixing a bug where it only published releases it created itself. accept(decoupling the publication step from the draft creator's identity prevents automation from silently leaving releases invisible, a common pitfall in distributed CI) |
| (7) `The status line renders what it` can, and a census note is a diagnostic | systemd, Kubernetes, Vault | All three strictly separate data from diagnostics: systemd logs diagnostics to journald, keeping state output clean [X]; Kubernetes kubectl outputs structured data to stdout and warnings to stderr [X]; Vault emits requested secrets on stdout and connection warnings on stderr [X]. forjar moved a census note to stderr and explicitly renders merged=UNMEASURED on failure rather than printing nothing. accept(strictly separating data on stdout from diagnostics on stderr and explicitly representing unmeasured states is the standard for robust machine-readable tools) |
| (8) `Three pipelines could take SIGPIPE and` call a readable registry UNMEASURED | Nix, Docker, Kubernetes | All three gracefully handle broken pipes or disconnected clients: the Nix daemon survives a broken pipe without crashing [X]; Docker daemon multiplexes streams and handles disconnects cleanly [X]; Kubernetes API server manages stream disconnects without system instability [X]. forjar pipelines using set -o pipefail exited 141 on SIGPIPE in grep -q, so they were refactored to single processes. accept(refactoring shell pipelines to avoid SIGPIPE under pipefail prevents flaky telemetry, though forjar is worse than the industry default here because its reliance on shell scripts for critical telemetry is inherently brittle) |
| (9) `The verdict line counts tickets and` PRs separately | Kubernetes, Chef, Ansible | All three distinguish between integration units and tracked work: Kubernetes release notes explicitly list PRs merged alongside the issues they fix [X]; Chef changelogs separate PRs from Jira issues [X]; Ansible generates release notes that independently list PRs and fixed issues [X]. forjar's verdict line now explicitly counts tickets and PRs instead of conflating them when one PR carries multiple tickets. accept(distinguishing between the unit of work and the unit of integration provides accurate accounting, as a single PR routinely resolves multiple tickets) |
| (10) `The 1.28.0 release record (PMAT-231, PMAT-227,` PMAT-235) | SLSA, Nix, Kubernetes | All three emphasize durable records of release events: SLSA attestations create a permanent record of the exact build materials [X]; Nix channels record the exact git commit evaluated [X]; Kubernetes conducts release retrospectives to document defects and roll over pending issues [X]. forjar booked the ledger row, moved the open goal, backfilled drifted statuses, and explicitly ticketed defects found during the release. accept(systematically recording defects found during the release and backfilling drift ensures the roadmap is an accurate historical record rather than relying on maintainer memory) |
| (11) `Gate B records what pmat 3.40's` six new checks measure, and refuses growth | Bazel, Kubernetes, SLSA | All three face the same problem — a rule that arrives over a corpus predating it — and each answers with a RECORDED, DATED, REVIEWABLE allowance rather than a switch. Bazel introduces a breaking change behind a named `--incompatible_*` flag that flips on a stated release, so the debt is visible and scheduled rather than hidden [X]. Kubernetes gives a deprecated API a named removal release instead of breaking clusters at upgrade [X]. SLSA's levels are an explicit ladder: a project records the level it MEETS and moves up, rather than claiming the top one or disabling the question [X]. None answers it by turning the check off, and none by failing the release the upgrade did not break. forjar records a per-check ceiling in a committed file, enforced by the gate, lowerable only, with the exemption list and the ceiling map refused when they disagree. accept(a dated ceiling that may only shrink is the answer all three give, and it keeps the backlog visible where a disabled check would hide it — though forjar is worse than Bazel's default here in one respect: Bazel's flag names the release at which the debt STOPS being tolerated, and this ceiling names no such date)
| (12) `A ratchet measurement cannot eat the` machine | Bazel, Nix, systemd | All three treat unbounded recursion in a build or unit graph as a structural error to be REFUSED rather than survived. Bazel refuses a cyclic dependency at load time and names the cycle [X]. Nix refuses an infinite recursion during evaluation against a stack-depth limit rather than exhausting the machine [X]. systemd refuses an ordering cycle between units, breaks it at a named edge, and logs which edge it broke [X]. None relies on an operator noticing the load average. forjar's ratchet had neither a cycle check nor a resource bound and now has both: a sentinel that names the re-entry, and a cap in the unit the kernel actually compares. accept(refusing the re-entry and bounding the blast radius is the right pair — though forjar remains WORSE than all three in one respect: they refuse the cycle at DECLARATION time from the graph itself, while this refuses it at run time plus a test over the committed config, and the tool's own side still has no guard at all)

## The limit of this audit

**Thirty-six system claims, all `[X]`.** Thirty are the lane's; rows 11 and 12
are the orchestrator's, written after the lane ran because neither behaviour
existed yet — pmat 3.40 turned gate B red the same day, and the fork storm
happened while fixing it. The lane read no vendor documentation
during the run and contacted no running instance, so each is documentation
memory and could be out of date. The strongest reason to distrust the rows is
that for fast-moving platforms — Kubernetes, Bazel — the exact mechanism
described may have shifted since the lane's memory was formed, even where the
shape of the comparison holds.

What the audit is for is that SHAPE, and it is not delicate: that a release is
qualified against a NAMED consumer rather than whatever HEAD happened to be;
that a status summary is load-bearing rather than decorative; that selective
CI on a pull request is paired with an unconditional full run at the release;
and that diagnostics belong on stderr and data on stdout. Each would survive a
correction to any single row.

**Four rows conclude forjar is WORSE than the industry default**, and they are
the four worth reading first. Row 5: clearing shared `/tmp` paths by hand is a
defence, but systemd's `PrivateTmp`, a container filesystem and a mount
namespace are isolation, and forjar has none. Row 8: refactoring pipelines to
survive SIGPIPE removes the flake, but a release gate whose telemetry depends
on shell pipelines is brittle by construction, and eighteen further sites of
the same shape are still open as PMAT-240. Row 11: a dated ceiling records the
debt and refuses growth, but Bazel's `--incompatible_*` flag names the release
at which the debt stops being tolerated and this ceiling names no such date.
Row 12: Bazel, Nix and systemd all refuse a cycle at DECLARATION time from the
graph itself; this refuses it at run time, with a test over the committed
config, and the tool's own side has no guard at all.

CRUX-1.29.0-END
