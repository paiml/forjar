# PMAT-989 (`feat/989-image-base-and-exec`) — Judges Claims Digest

### CONFIRMED

1. [security] Strict base-reference allowlist prevents path traversal, option injection, and shell hint metacharacters (`C1`)
   - evidence: `src/cli/build_image_base.rs:24` restricts `base_ref` to `[A-Za-z0-9._-/:@]` while explicitly rejecting empty strings, leading `-`, whitespace/control bytes, `.`, and `..` before `base_layout_dir` (`src/cli/build_image_base.rs:15`) or runtime commands run, pinned by `src/cli/tests_build_image_base.rs:220` across `.`, `..`, `;`, `$(id)`, quotes, backticks, `&`, and `|`.

2. [code] Declared OCI base layers are verified, prepended to the manifest, and bound into the cache key (`C2`)
   - evidence: `src/cli/build_image_base.rs:52`, `src/cli/build_image.rs:95`, and `src/cli/build_image_cache.rs:15` verify base layer blobs on disk, prepend base layers ahead of built layers in the OCI manifest, fold base digests into cache keys, and return a non-zero error naming the directory and `skopeo copy` hint when absent (`CHANGELOG.md:10`).

3. [code] Parser validation and exec step extraction enforce trimmed-command and `(image)` error parity (`C3`)
   - evidence: `src/core/parser/resource_types.rs:85` trims `resource.command` identically to `src/cli/build_image_exec.rs:35`, emits `resource '<id>' (image) has no path, packages or command` on empty or base-only image resources, and is covered by `src/core/parser/tests_validation_image.rs:10`.

4. [code] Container execution captures one normalized, sorted, epoch-mtime OCI layer with base diff_id verification (`C4`)
   - evidence: `src/core/store/exec_layer.rs:25`, `src/core/store/exec_capture.rs:40`, and `src/core/store/build_runtime.rs:30` verify runtime `diff_ids` against the OCI base layout, run `packages` and `command` steps, scrub volatile `/var/cache` and `/var/log` paths, emit `.wh.` whiteouts, and sort tar headers at mtime `0`, tested in `src/core/store/tests_exec_layer.rs:20`.

5. [test] Docker falsifiers F1 and F2 prove end-to-end digest determinism and runnable package/command layers (`C5`)
   - evidence: `tests/falsification_image_exec.rs:105` and `tests/falsification_image_exec.rs:162` passed `2/2` on `kfocus` (`47.26s`), proving two independent `forjar build` runs produce identical two-layer manifest digests (`da == db`) and `--load` yields a runnable container with `jq --version` (`rc=0`) and `/etc/forjar-exec-marker`.

### REFUTED

1. [scope] `--sandbox` path, non-root UID/GID preservation, and package version locking are not implemented in this slice (`R1`)
   - evidence: `src/cli/build_image.rs:145` leaves `--sandbox` unchanged, `src/core/store/exec_capture.rs:40` normalizes captured tar entries to UID/GID `0:0`, and `packages:` delegates to distribution repositories without lockfile pinning.
   - corrected: `CHANGELOG.md:19` and `CHANGELOG.md:30` explicitly disclose all three boundaries (`--sandbox` unchanged, `0:0` ownership, unpinned package versions) rather than overclaiming full hermetic sandbox parity.

2. [ci] Live Docker falsifiers F1 and F2 are feature-gated outside default daemonless CI (`R2`)
   - evidence: `tests/falsification_image_exec.rs:1` requires `--features container-test` and a live Docker daemon, and containerd-image-store daemons do not resolve `--load`ed image tags by name (`tests/falsification_image_exec.rs:167`).
   - corrected: Fake-runtime unit suites (`src/core/store/fake_build_runtime.rs:15`, `src/cli/tests_build_image_exec.rs:15`) run on every default `cargo test`, `CHANGELOG.md:32` documents both the feature flag and image-ID invocation, and `agy` executed live Docker F1/F2 on `kfocus` (`2/2` green).
