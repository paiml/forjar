# CRUX Audit — forjar 1.33.0

## Method

**When, and after what.** This audit was written and measured on 2026-10-07,
nine days AFTER v1.33.0 was tagged (2026-09-28T13:45:50Z) and after 1.33.0 was
published. It is not evidence that the release was reconciled before it was cut.
It was not. No `crux-1.33.0.md` existed until this file, and gate H did not run
for 1.33.0 before the tag. This file records the comparison the release should
have carried, and it says when it was made.

**Who wrote it, and from what.** Like the 1.30.0 to 1.32.0 audits, it was
written by a Claude model (Opus 5.5) from documentation memory. It had no
network access to any reference system and invoked none of them. Every claim
about a third-party system is marked `[X]`: asserted from documentation, not
measured. The claim about forjar is the CHANGELOG paragraph at the tag and the
workflow change it names.

**How it was measured.** Gate H as it stood at the tag
(`git show v1.33.0:scripts/dogfood/crux-reconcile.sh`) was run in a detached
worktree of `v1.33.0` with this file added. Its verdict line:

    GATE H PASS 1 of 1 behaviour bullet(s) under [1.33.0] reconciled in docs/audits/crux-1.33.0.md, each naming >= 3 of the 28 surveyed systems

That gate reconciled at all only because of the defect PMAT-607 fixes. At the
tag, its newest tag was `v1.33.0-rc.1`, which `git tag --sort=-v:refname` ranks
above `v1.33.0`, so it did not take the post-tag PENDING arm. Gate H with the
PMAT-607 tag rule, run on the same tree, takes it:

    GATE H PENDING Cargo.toml is still at v1.33.0's version (1.33.0); no release is being cut, so there is nothing to reconcile yet

**What this audit can and cannot show.** It shows that the one behaviour gate H
asks about for 1.33.0 has been held against at least three systems that solve
the same problem. It cannot show that the comparison is right in detail.
`scripts/dogfood/crux-reconcile.sh` says the same about itself.

## The comparison

| Behaviour | Systems surveyed | What the comparison found |
|---|---|---|
| (1) An aarch64 Linux release leg can run the `cross` it installs (#611) | Nix, Bazel, Docker, systemd, Ansible | The defect is a tool installed into a prefix the next step does not search: `build-binaries` set a private `CARGO_HOME`, `cargo install cross` wrote to `$CARGO_HOME/bin`, and the build step looked for `cross` on a PATH that did not contain it. Every system surveyed treats "where an installed tool is found" as declared state, not as ambient state. **Nix** constructs a build's PATH from its declared inputs: a derivation sees exactly the `bin/` of each input in its closure, and a tool that is not an input is not found, whatever is installed on the host `[X]`. **Bazel** does not resolve tools by PATH at all. A toolchain is a label resolved for the execution platform, and an action runs with the tool's path in its command line `[X]`. **Docker** carries PATH between build steps only through `ENV`. A `RUN` that installs into a new prefix leaves the next `RUN` unable to see it unless the Dockerfile declares the prefix `[X]`. **systemd** gives a unit a fixed default PATH, not the login shell's, and a unit that needs another directory must say so with `Environment=` `[X]`. **Ansible** runs each task in a fresh process with the remote user's non-interactive environment. A tool installed by one task into a user prefix is not on the next task's PATH unless `environment:` adds it `[X]`. **forjar's fix takes the declared route.** The step now appends `$CARGO_HOME/bin` to `$GITHUB_PATH` (`git show v1.33.0:.github/workflows/release.yml`, line 275, added by 49b33fbd, #654), the runner's own declaration of a PATH entry for the steps after it. It does not reinstall `cross` per step, and it does not fall back to a global `~/.cargo/bin`, which would make the build depend on what a runner happened to have. What the fix does NOT add, and every system above has in some form, is a check that the tool the next step runs is the one the step installed. Bazel and Nix get this by construction. forjar's release legs still trust that the first `cross` on PATH is the right one. |

## What is not reconciled here

**The rc.1 behaviours were never reconciled by gate H.** The 1.33.0 CHANGELOG
has two sections. `[1.33.0]` holds one paragraph: the release-pipeline fix
above. `[1.33.0-rc.1]` holds the behaviours that 1.33.0 "promotes unchanged":
as the CHANGELOG lists them: the mount ownership check (PMAT-642, #642), the automount stack (PMAT-648,
#648), `query_latency_under_50ms` (#647), the `state: file` content check
(PMAT-600, #600) and the others under that heading. Gate H reads
`[Unreleased]`, or failing that `[<version from Cargo.toml>]`. It never reads a
pre-release section. So every behaviour that shipped in 1.33.0 through rc.1 went
out with no CRUX row, and the gate had no way to say so. This audit does not
write those rows. It records that the gate let them through and leaves the
decision to the gate's owner: either gate H also reads the pre-release sections
a release promotes, or a release that promotes an rc must repeat its paragraphs.
