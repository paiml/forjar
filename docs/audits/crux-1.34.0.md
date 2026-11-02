# CRUX Audit — forjar 1.34.0

## Method

**When, and from what.** Drafted and measured on 2026-11-02 (about 02:30Z) against
the `[Unreleased]` section of `CHANGELOG.md` at origin/main `963eed6b`. It is a
draft for the release-cut PR; nothing here has been reviewed by a quorum yet.

**Who wrote it.** A Claude model (Sonnet 5.5, worker forjar-w2). Two kinds of claim
appear, and each is tagged:

- `[M]` — MEASURED in this session, by the command shown under "Measurements".
  The host was the dev box (Linux 6.8, x86_64). The forjar binary was a debug build
  of branch `w2/839-fix851` (`68d08714a2cc` = PR #839 plus main `98159e47`, which
  already holds #613, #615, #622 and #727); its `--version` still reads `1.33.0`
  because Cargo.toml has not been bumped.
- `[X]` — asserted from the system's documentation, NOT measured, because the
  system is not installed here (Ansible, Salt, Chef, Puppet, pyinfra, Terraform,
  Nix, Bazel, Kubernetes, Vault, ...) or the check needs the network.

Versions actually run: OpenTofu v1.8.5, Docker 29.8.1, Docker Compose v5.5.1,
systemd 249, SOPS 3.13.3. Nothing was installed, started or pulled. The only host
state created: two throwaway `terraform_data` resources in a scratch directory,
one `docker` tag (`crux-pin:latest`, removed), and a fake `ollama` shell script.

**What is NOT measured here (be explicit).**

- The upstream half of #613 (the comparison with the repo's latest release) needs
  api.github.com. Every forjar run below used `--offline`. That leg is UNMEASURED.
- The reaper change (#727) was not run as the generated script against a loaded
  host. Only the TECHNIQUE (per-entry `readlink` / `tr|head` per pid versus one
  `find` pass) was timed, once, on this host. The 84 s figure in the CHANGELOG is
  the CHANGELOG's own, from lambda-labs; it was not reproduced.
- Each timing is one run. They show an order of magnitude, not a benchmark.

**How gate H was run.** `scripts/dogfood/crux-reconcile.sh` reads the version from
Cargo.toml and the bullets from `[Unreleased]`. Cargo.toml is at 1.33.0, which is
the latest tag, so the gate takes its PENDING arm until the version is bumped. For
this draft Cargo.toml was edited to `1.34.0` in a scratch checkout (not committed),
this file was copied to `docs/audits/crux-1.34.0.md`, and the gate was run. The
verdict line is recorded at the end of this file.

**What this audit can and cannot show.** It shows each of the four behaviours has
been held against at least three systems that solve the neighbouring problem, and
says which comparisons were run. It cannot show the `[X]` claims are right, and
gate H cannot tell a real comparison from three names typed into a cell
(`crux-reconcile.sh` says so about itself).

## The comparison

| Behaviour | Systems surveyed | What the comparison found |
|---|---|---|
| (1) drift and plan compare a github_release's live `--version` with its pin and with the latest upstream release; apply never downgrades (#613) | OpenTofu, Docker, SOPS, Terraform | Forjar `[M]`: with ollama pinned `v0.33.2` and a fake binary reporting `0.34.2`, `plan --offline` printed `DRIFTED ollama (box): live ollama is 0.34.2, declared pin is 0.33.2; apply would DOWNGRADE it`, listed the upstream half as `NOT CHECKED ... (offline)`, and put the same row in `version_pins` in `--json`. `apply` then refused: `refusing to downgrade ollama/ollama: the pin is 0.33.2 ... Live: 0.34.2`, exit 1; `drift --offline` then reported `DRIFTED` and exited 1. **A gap, measured:** `drift --offline` on a state dir with NO lock at all printed `No drift detected. 0 resource(s) inspected` and exited 0, before the failed apply had written a lock; the CHANGELOG says the detector "also covers a pin missing from the lock", and that case was not reproduced. **OpenTofu** `[M]` enforces a pin on ITSELF, not on what it manages: `required_version = "= 1.9.9"` made `tofu init`/`validate` fail with `This configuration does not support OpenTofu version 1.8.5`, exit 1. It refuses in both directions (it does not distinguish a downgrade from an upgrade), and it compares nothing about a binary the configuration installs. **Docker** `[M]` takes the opposite default: `docker tag <image-2> crux-pin:latest` over a tag already pointing at <image-1> succeeded silently, exit 0, and the tag then named image-2 (the two images were the local `ads-kotlin-lab:2` and `:1`; which is older was not asked, which is the point: the tag move is not compared); a floating tag is a pin with no downgrade check at all. **SOPS** `[M]` has the upstream-latest half and not the pin half: `sops --version` printed `3.13.3 (latest)` after asking upstream, and the same output warns that this check will move behind `--check-for-updates`; it compares the live binary with the latest release and with no declared pin. **Terraform** `[X]`: `required_version` and provider lock files behave as OpenTofu's do, and the dependency lock file pins provider versions by hash; neither compares a live host binary with a pin. The finding: no surveyed system compares a live binary on a host with a declared pin AND with the upstream latest, and then refuses the downgrade on the box. Forjar's refusal is the stricter default; the cost is that a deliberate roll-back needs the binary removed first, which the message says. |
| (2) plan -r and apply -r compute the same set, and a lockless create asks its `completion_check` first (#615) | OpenTofu, Docker, Terraform | Forjar `[M]`: a task `top` with `depends_on: [dep]`, and an unrelated `other`; `plan -r top` listed `dep: create` and `top: create` (`2 to add`); `apply -r top --dry-run` listed `dep` and `top` and not `other`. The SETS agree. The ACTIONS do not, by design: `dep` declares `completion_check: "true"`, so apply printed `dep: no changes` (`1 to add, 1 unchanged`) while plain `plan` stayed lock-relative and said `create`; the CHANGELOG says plain `plan` stays lock-relative and does not contact hosts. **OpenTofu** `[M]`: `tofu plan -target=terraform_data.b` and `tofu apply -target=terraform_data.b` both pulled in `terraform_data.a` (the dependency) and not `c`; `Plan: 2 to add`, `Apply complete! Resources: 2 added`. It keeps the two commands on one set by making `-target` mean "this resource and what it depends on" in both, and warns `Resource targeting is in effect`. **Docker Compose** `[M]`: `docker compose --dry-run up b` created `dc-a-1` then `dc-b-1` and not `c`: `depends_on` is closed over for the named service, and dry-run is the preview of the same path. **Terraform** `[X]`: `-target` follows the same dependency rule as OpenTofu. The finding: the three systems measured agree with the fix that plan and apply must close over dependencies the same way; none of them has a lockless-create guard equivalent to `completion_check`, because none plans from a possibly-absent state file the way forjar's lock does (OpenTofu reads its state file or plans a create; `[X]`), so that half of #615 has no direct comparator here and is forjar-specific. |
| (3) --exclude is repeatable, understands * anywhere and `{a,b}`, and refuses a pattern that matches nothing (#622) | OpenTofu, systemd, Docker | Forjar `[M]`: `apply --dry-run --exclude 'o*r' --exclude '{dep,x}'` accepted two flags, matched `other` by glob and `dep` by brace, and listed only `top`, exit 0. `--exclude 'zzz*'` exited 1 with `--exclude 'zzz*' matches no resource in this config. Known: dep, other, top`. Excluding every resource exited 1 with `no resources remain: ... removed every selected resource (3 of 3)`. **OpenTofu** `[M]`: v1.8.5 has no exclude at all: `tofu plan -exclude=terraform_data.c` failed with `Failed to parse command-line flags`, exit 1; only `-target` exists, and `tofu plan -target=terraform_data.zzz` (no such resource) printed `No changes. Your infrastructure matches the configuration.` with a targeting warning, exit 0. A target that matches nothing is a clean no-op there, which is the shape of defect #622 fixed in forjar. **systemd** `[M]`: `systemctl list-units 'crux-nomatch-*'` and `systemctl status 'crux-nomatch-*'` both accept a glob, match nothing, print nothing and exit 0. **Docker** `[M]`: `docker ps -a --filter name=crux-nomatch` matched zero containers and exited 0, while `docker rm crux-nomatch` (a name, not a filter) exited 1; the filter is lenient and the direct name is strict. A side measurement: `--filter 'name=zzz*'` matched 6 containers, because Docker's name filter is a regular expression, not a glob, so `zzz*` means `zz` followed by any number of `z`. The finding: the surveyed selectors split into lenient filters (exit 0 on no match) and strict direct names (exit 1). A flag whose job is to REMOVE work from an apply is a safety control and the lenient reading fails open, which is the case for forjar's refusal. |
| (4) The disk-budget reaper no longer costs more than the disk it defends: one /proc pass, `read` builtin, bounded and niced walks (#727) | systemd, Docker, Kubernetes | The technique was timed on this host (1693 pids), one run each, `bash -c`: a per-entry `readlink` loop over every `cwd` and `fd` link took 9.63 s; a `tr \| head` per pid took 2.02 s; one `find /proc/*/cwd /proc/*/fd -printf '%p %l\n'` pass took 0.10 s; a `read -r` builtin per pid took 0.03 s `[M]`. That is the CHANGELOG's own change (single `find`, `read` instead of `tr\|head`) and the order of improvement is reproduced on this host; the lambda-labs 84 s figure, the `timeout`/`-xdev`/`nice`/`ionice` wrapping and the fail-safe on a timed-out walk were NOT run here. Comparators are all `[X]`, none measured: **systemd** `systemd-tmpfiles --clean` removes by age and by configured path and does not look for files an open fd still holds; **Docker** `docker system prune` removes only objects Docker owns, by reference count, so it never walks the filesystem for open descriptors; **Kubernetes** kubelet image and container garbage collection runs on disk-pressure thresholds against the runtime's own records, not by scanning /proc. The finding, from documentation only: none of these needs the "is anything still holding this file open" scan, because they delete only what they own and track. A reaper over a shared scan root has to do that scan, so its cost has to be bounded by construction; fail-safe on a timed-out walk matches what the others do implicitly (they delete nothing they cannot account for). Comparison quality is low for this row; the measured part is the technique, not a rival system. |
| (5) A multi-platform base resolves to the build target's manifest: an OCI image index or docker manifest list is followed to the `linux` manifest of the architecture it builds for, else the first entry (#784, PMAT-5881) | Docker, Podman, Kubernetes | Not measured. Nothing in this audit or the sources it cites states how Docker, Podman or Kubernetes pick a platform from a manifest list, so each comparator is `not_measured` and no claim is made about any of them. Forjar `[M]` by the repo, not this session: `tests_base_image_arch.rs` names the architecture instead of reading the host (CHANGELOG). The rule it implements is stated in the CHANGELOG only: `amd64` on x86_64, `arm64` on aarch64, else the first entry. The comparison against the three systems above is OPEN: a reviewer must run `docker`/`podman` against a multi-arch layout and read k8s image-pull docs before this row can claim parity. |

## What is not reconciled here

- **The upstream leg of #613** (pin versus latest release) was never exercised, since
  no network call was made. Reconcile it from a CI run with network access, or
  mark it as not measured in the release notes.
- **`drift` with no lock at all** reported `0 resource(s) inspected` and exit 0 for
  a pinned `github_release` (see row 1). If the CHANGELOG sentence "also covers a
  pin missing from the lock" is meant to cover a state directory that holds no lock
  file, this release does not do that, or `--offline` skips it. This is a finding to
  triage before the cut, not a verdict; it was reproduced once.
- **Row 4's comparators are documentation only.** A measured comparison would need
  `systemd-tmpfiles --clean` (a dry run needs a throwaway root) and a kubelet.
- **Other CHANGELOG paragraphs.** Gate H counts four bold-opening paragraphs under
  `[Unreleased]`. The inode-pressure check and `tmp_reaper` resource (#847, #849)
  and the PMAT-838 roadmap row (#839) have no `[Unreleased]` paragraph and so are
  outside this gate; if #849 should have one, the gate will then ask for a fifth row.
- **Row 5 (#784) is a name-only row.** It was added on 2026-11-02 for the merged #784 bullet by forjar-w3 (Sonnet 5.5). It names three roster systems so the gate can count them, and says plainly that the comparison is not_measured. The gate cannot tell this from a real comparison; this note is the tell.

## Gate H verdict

Run in `/home/noah/.cache/forjar-arm64-scratch` at origin/main `963eed6b` with Cargo.toml temporarily set to 1.34.0 and this file copied to `docs/audits/crux-1.34.0.md` (both reverted afterwards):

    GATE H PASS 4 of 4 behaviour bullet(s) under [Unreleased] reconciled in docs/audits/crux-1.34.0.md, each naming >= 3 of the 28 surveyed systems

The gate reads names only; it does not check the `[M]`/`[X]` tags.

Re-run 2026-11-02 on release/v1.34.0 after merging #784 and adding row 5: `GATE H PASS 5 of 5 behaviour bullet(s) under [Unreleased] reconciled in docs/audits/crux-1.34.0.md, each naming >= 3 of the 28 surveyed systems`.
