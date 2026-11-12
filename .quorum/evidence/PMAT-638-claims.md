# Claims — Per-run PID file staging `.forjar-new.$$` & I8 script corpus coverage (PMAT-638 / forjar#906)

Extracted from PR `#906` (`s-forjar-1/638`, head `1e5c9be8`) and attacked across all 3 review lanes:

- **C1 (Per-PID staging `<path>.forjar-new.$$` with `EXIT` trap & pre-rename permissions)**: `src/resources/file.rs:182` stages regular-file writes at `'<path>.forjar-new.'$$` under `trap 'rm -f "$s"' EXIT`, applies `chown`/`chmod` at `src/resources/file.rs:216` before `mv -f "$s" "$p"`, and updates `decode_written_file` in `src/core/shell_escape.rs:200` (`src/resources/tests_file_staging.rs:10`).
- **C2 (I8 generated-script lint gate covers `state: file` `content` and `source` rows)**: `src/transport/tests_generated_scripts_lint.rs:80` adds `file/state-file-content` and `file/state-file-source` to the I8 shell lint corpus.
- **C3 (Package `completion_check` appended across providers)**: `src/resources/package_check.rs:199` and `src/resources/tests_package_d.rs:90` verify `completion_check` execution across `apt`, `cargo`, `uv`, and `brew`.
- **C4 (Gate A receipt, `CHANGELOG.md` & revert kill)**: `CHANGELOG.md:8`, `docs/roadmaps/roadmap.yaml:7110`, and `docs/audits/impl-PMAT-638-receipt.md:17` record `PMAT-638`, and reverting `.$$` in `src/resources/file.rs:182` fails `test_638_staging_name_is_per_run` (`FJ_906_REVERT_RC=101`).
- **R1 (Earlier docstring overclaimed that `.$$` closed all staging races)**: Commit `5b3bccad` refuted that claim: `.$$` prevents concurrent `forjar apply` collisions on fixed `.forjar-new`, whereas PID predictability (`mktemp`) and `SIGKILL` orphans remain documented residue.
- **R2 (Stale-branch CI `dogfood-guards` failure & 10 host-specific integration failures misread as PR defects)**: `docs/audits/impl-PMAT-638-receipt.md:40` proved `dogfood-guards` failed on a pre-merge workflow target list (`905fd5af` fixed it) and the 10 host failures match plain `origin/main` (`dbd1ec3e`).
