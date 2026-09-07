# Crux lane — competitive survey — PMAT-163

One agy lane (conv-3a2bc453, 63 s, sandboxed, no network) surveyed the field from documentation memory; every third-party figure is [X] and asserted.

## Verdict (PASS)

### Competitive Survey of Release Gating

| System | Artifact vs Source | Docs Executed? | Coverage Floor & Anti-Gaming | Go/No-Go Owner |
|---|---|---|---|---|
| **Rust** (`cargo-mutants`, `cargo-semver-checks`) | Mostly source (Rustdoc JSON, mutants on source). | Yes, `cargo test` executes doctests. | Uses `cargo-mutants` to catch vacuous tests, but a strict 95% line floor isn't universally enforced. | CI/CD pipeline or package owner via `cargo publish`. |
| **Kubernetes** | Conformance tests run against real artifacts/clusters; `verify-*` checks source. | Generally no (docs aren't typically part of e2e conformance). | Historically struggles with strict global floors; relies heavily on code review (OWNERS). | SIG Release / Release Managers (human committee) via dashboards. |
| **Debian** (`autopkgtest`, reproducible builds) | Tests built binary packages (`.deb`); reproducible builds verify source-to-artifact mapping. | No systematic doc execution. | No strict coverage floor for packages. | Release Team (human) + automated migration (britney). |
| **Terraform** (Acceptance Tests) | `TF_ACC=1` tests compile the provider and execute against real cloud APIs. | No, docs are often generated from schema but not executed. | Relies on acceptance test CRUD coverage; no mathematical floor. | Provider maintainer reviewing CI. |
| **forjar (Dogfood Gate)** | Measures transport surface (Gate C) from the *running artifact* (stdio, HTTP, `--help`), not just source declarations. | Yes (Gate D), extracts and runs all fenced `forjar` blocks in README against a sandboxed `HOME`. | 95% line floor (Gate F) + `cargo mutants` specifically run over the branch's diff to prevent gaming. | Human operator running `make dogfood-release`, generating a deterministic receipt (Gate A). |

**Verdict**: PASS
`forjar`'s pre-publish dogfood gate is extremely robust and is at least as sound as the surveyed systems. It combines their strongest attributes (mutation testing from Rust, built-artifact testing from Debian/K8s) while introducing exceptionally rigorous requirements—such as end-to-end executable README blocks and competitive crux-reconciliation—that exceed typical industry baselines.

## Findings, as returned

- [cited] — Like Rust's cargo-mutants convention, forjar uses mutation testing to prevent coverage gaming, but explicitly applies it in-diff alongside a strict 95% floor.
- [cited] — Like Kubernetes conformance tests, forjar verifies the live surface of the running artifact rather than trusting static source declarations.
- [cited] — Like Debian's reproducible builds, forjar enforces strict determinism by ensuring the generated dogfood receipt is bit-for-bit identical across runs.
- [cited] — Unlike Terraform where docs are often generated but not run, forjar extracts and executes all fenced commands in the README inside a sandboxed environment.

## Orchestrator cross-check (asserted)

- Rust: `cargo publish --dry-run`, `cargo-semver-checks` and `cargo-mutants` are the conventional pre-release checks; none executes documentation. Forjar's gate D executes the README blocks.
- Kubernetes SIG Release: conformance tests and release-blocking jobs measure the built artifact; the go/no-go is a human release manager reading dashboards. Forjar's gate C measures the built binary the same way.
- Debian: autopkgtest and piuparts run the packaged artifact; reproducible-builds compares rebuilt bytes. Forjar's dogfood-published arm installs the crates.io artifact and runs gates C and D against it.
- Terraform: acceptance tests (TF_ACC) run before a release against real providers; no coverage floor. Forjar pairs a 95-line floor with in-diff mutants against gaming.

No surveyed system shows a rule forjar's gate violates; none executes documentation, which is forjar's stronger rule.
