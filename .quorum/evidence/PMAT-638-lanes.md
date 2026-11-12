# Lanes — Per-run PID file staging `.forjar-new.$$` & I8 script corpus coverage (PMAT-638 / forjar#906)

Three-lane `LANES=3` review by `agy` (`Gemini 4 Carbon (thinking=high)`), distinct from the Claude author model, at head `1e5c9be8eeb873fe206eaeccd1de8a169c25658f`:

- **Lane 1 (Per-PID Staging & Concurrent Writer Isolation)**: `PASS` — verified `src/resources/file.rs:182` (`'<path>.forjar-new.'$$`), `src/core/shell_escape.rs:200`, `test_638_staging_name_is_per_run` (`1/1`), and `FJ_906_REVERT_RC=101` on `<host>`.
- **Lane 2 (Shell Quoting, `EXIT` Trap Hygiene & I8 Lint Gate)**: `PASS` — verified `sh_squote` single-quote escaping before unquoted `$$`, `trap 'rm -f "$s"' EXIT`, and `transport::tests_generated_scripts_lint` (`4/4` green).
- **Lane 3 (Delivery Modes, Pre-Rename Permissions & Package Suite)**: `PASS` — verified all 4 payload modes write to `"$s"` with `append_owner_mode` (`src/resources/file.rs:216`) before `mv -f`, plus `resources::tests_package_d` (`12/12` green).
