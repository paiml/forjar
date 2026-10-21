# Implementation receipt — PMAT-756 — `type: image` builds a real image (IMG-001)

verdict: PASS for the unit-tested surface; the docker/podman runtime export (`save` of a real base, real `docker diff` against a live container) was NOT run in this worktree.

## Identity

- ticket PMAT-756 (forjar#756), PR paiml/forjar#757, branch `fix/image-build-base-packages`; kind: code

## Change

- the declared base is exported from the runtime and layered under the build; a `name@sha256:` base must match exactly
- `packages` then `command` run in a container; their filesystem diff is the top layer, then the resource's `path:` Files layers
- review round 1 fixes: the diff layer now carries Added, Changed and Deleted (new `src/core/store/container_diff.rs`; D becomes `.wh.<name>`, a deleted-and-rebuilt dir becomes `.wh..wh..opq`; changed directories are never copied whole); `--sandbox --json` is rejected again; an OCI image index at `manifests[0]` resolves to linux/amd64 (else first); `podman save` uses `--format oci-archive` (docker has no such flag); CHANGELOG updated

## Test evidence

- `container_diff`: 3 tests, including a fake-runtime run of `extract_changes` whose scan holds the changed file and a whiteout for the deleted one. Reverting the C/D parsing: 3 of 3 FAILED.
- `sandbox_with_json_is_rejected_before_any_work`, `podman_save_asks_for_an_oci_archive_and_docker_does_not`, `extract_base_layers_follows_image_index_to_amd64_manifest`: all three FAILED with their fix reverted, pass with it.
- not run: a real `docker diff`/`docker cp`/`podman save` round trip; the OCI opaque-dir case depends on how the runtime reports a deleted-then-recreated dir (modelled as D plus A/C at or under the same path).
