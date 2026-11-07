# Implementation receipt — PMAT-756 — `type: image` builds a real image (IMG-001)

verdict: PASS (unit scope and a real docker round trip, both measured; see "Real runtime round trip")

## Identity

- ticket PMAT-756 (forjar#756), PR paiml/forjar#757, branch `fix/image-build-base-packages`; kind: code

## Change

- the declared base is exported from the runtime and layered under the build; a `name@sha256:` base must match exactly
- `packages` then `command` run in a container; their filesystem diff is the top layer, then the resource's `path:` Files layers
- review round 1 fixes: the diff layer now carries Added, Changed and Deleted (new `src/core/store/container_diff.rs`; D becomes `.wh.<name>`, a deleted-and-rebuilt dir becomes `.wh..wh..opq`; changed directories are never copied whole); `--sandbox --json` is rejected again; an OCI image index at `manifests[0]` resolves to linux/<host arch> (round 1 said amd64; round 2 maps ARCH; else first); `podman save` uses `--format oci-archive` (docker has no such flag); CHANGELOG updated

## Test evidence

- `container_diff`: 3 tests, including a fake-runtime run of `extract_changes` whose scan holds the changed file and a whiteout for the deleted one. Reverting the C/D parsing: 3 of 3 FAILED.
- `sandbox_with_json_is_rejected_before_any_work`, `podman_save_asks_for_an_oci_archive_and_docker_does_not`, `extract_base_layers_follows_image_index_to_amd64_manifest`: all three FAILED with their fix reverted, pass with it.
- review round 2 (dc97e686 rejected 2/3): the implicit container path now runs only the image resource's own `image_build_scripts` (`container_build_scripts`; only an explicit `--sandbox` keeps the legacy all-resources scripts); the base index pick maps `ARCH` (x86_64->amd64, aarch64->arm64) instead of hard-coding amd64; an image-index digest pin against a single-platform `docker save` layout is accepted only if the layout holds the index blob, it hashes to the pin and lists the layout's manifest, else refused with a clear error.
- round 2 RED-on-revert: implicit-path test (unrelated task script leaked in, 3 scripts vs 2) FAILED with the filter disabled; `oci_arch_maps_rust_arch_names` FAILED with aarch64->arm64 reverted; `index_digest_pin_is_accepted_when_the_layout_holds_the_hashing_index_blob` FAILED with the index-pin check disabled. Caveat: this host is x86_64, so the host-arch index selection test cannot fail against the old hard-coded amd64; the pure mapping test is the discriminating one.

## Real runtime round trip

- docker 29.8.1 on this host (podman absent). Base `debian:bookworm-slim` (1 layer, local); spec: `packages: [coreutils]`, `command: echo MODIFIED > /etc/issue && rm /etc/issue.net && echo ADDED > /opt-added.txt`, `path:` a local file, plus an unrelated `type: task` resource that writes /tmp/should-not-exist.
- `forjar build -f forjar.yaml --resource img --load`: "Scripts: 2" (unrelated task excluded), "3 files changed, 2 layers", "Loaded image: forjar-rt:1".
- Inspected with `docker run --entrypoint sh`: /etc/issue = MODIFIED (base: Debian GNU/Linux 12); /etc/issue.net absent (present in base); /opt-added.txt = ADDED; the `path:` file present (path shown by `find`; content not read back); /etc/debian_version 12.15 (base preserved); /tmp/should-not-exist absent. `docker history`: base layer + `forjar: files (container diff)` + `forjar: files` layers; image layers=3 vs base 1; arch amd64.
- Review of #757 at 9e04d702 (13 findings), fixed on epic #768 (PMAT-756): REAL package install on docker 29.8.1, base debian:bookworm-slim, `packages: [vim-tiny]` + `command: mkdir -p /opt/emptydir` + a `path:` file, `forjar build --load`: "115 files changed, 3 layers"; in the loaded image /usr/bin/vi -> /etc/alternatives/vi -> /usr/bin/vim.tiny are symlinks (not host file content), /opt/emptydir exists, /var/cache/apt/archives/partial keeps mode 700, `vi --version` runs. RED-on-revert (each fix reverted, its test FAILED): symlink-following walk (read host /etc), no dir entries, dirs skipped in extract, 300 s lifetime, default-base applied, total_size built-only. The `--sandbox --json` findings do not apply at c781b7db: `reject_json_stdout_conflicts` already names `--sandbox` and `sandbox_with_json_is_rejected_before_any_work` passes; the implicit packages path with `--json` is intended and prints JSON only.
- Not covered: podman, an index-digest-pinned base via a real pull, non-amd64 hosts.

IMPL-PMAT-756-RECEIPT-END
