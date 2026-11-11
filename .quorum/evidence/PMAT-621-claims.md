# PMAT-621 (`forjar#933`) — Extracted Claim Manifest (`C1..C3`, `R1..R2`)

Base commit: `5b8f8123be40731340e2a74a6e4ae365648cab45`
Head commit: `21a730e853584d929e4c8b3375f15ef35e350bab`
Branch: `ct3/621-docker-import-rm`

1. **C1 (`[code]`)**: `import_command` for `ImportProvider::Docker` (`src/core/store/provider.rs:125`) captures `cid=$(docker create {}{tag})`, exports `"$cid"`, and removes `"$cid"` via `docker rm "$cid" >/dev/null` on success or failure.
2. **C2 (`[code]`)**: Wrapping `docker export "$cid" | tar -x -C "$STAGING"` in `(set -o pipefail; ...)` (`src/core/store/provider.rs:126`) ensures `rc=$?` reflects `docker export` failure even when `tar -x` exits `0` on a block-aligned truncated stream.
3. **C3 (`[test]`)**: `docker_import_removes_created_container` and `docker_import_fails_when_export_fails_and_still_removes_container` (`src/core/store/tests_provider.rs:255`) falsify both container-leak regressions and missing-`pipefail` regressions hermetic to `PATH` tempdirs.
4. **R1 (`[diff]`, Round 1 refutation)**: Round 1 (`91a0e86`) omitted `pipefail`, allowing a truncated `docker export` to return `rc=0` and poison `/var/lib/forjar/store/<blake3>`; fixed at `21a730e8`.
5. **R2 (`[doc]`, `docker rm` exit-code boundary)**: `exit $rc` (`src/core/store/provider.rs:126`) returns the export+tar status even if `docker rm "$cid"` fails after a valid export, surfacing `docker rm` stderr without failing an extracted artifact.
