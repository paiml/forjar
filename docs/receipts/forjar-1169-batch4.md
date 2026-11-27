# forjar #1169 batch 4: non-author review receipts (v1.34.2 post-merge)

Each PR below was authored by a Sonnet 5.5 session. Each was reviewed by a
non-author model (agy `gemini-4-neon-high`, one lane) bound to its merge commit,
reading `git diff <sha>^1 <sha>`. The courier ran the changed tests and did not
edit any verdict.

| PR | merge sha | reviewer | verdict | receipt |
|----|-----------|----------|---------|---------|
| #1101 | d665a9a92dc3cb4fa45aaf228d96f7068ae0add9 | gemini-4-neon-high (agy) | PASS | /mnt/nvme-raid0/cop-inbox/handoff/comply-cop/receipts-forjar-1169/PR-1101-d665a9a92d.md |
| #1102 | 56cd6f543e4130291580957adb8006eaff9a58c9 | gemini-4-neon-high (agy) | PASS | /mnt/nvme-raid0/cop-inbox/handoff/comply-cop/receipts-forjar-1169/PR-1102-56cd6f543e.md |
| #1103 | 213b8d74d4a4447cebf6ad7ce2eaf62b9e665f68 | gemini-4-neon-high (agy) | FAIL | /mnt/nvme-raid0/cop-inbox/handoff/comply-cop/receipts-forjar-1169/PR-1103-213b8d74d4.md |
| #1104 | d196fa7a8946c888218e70e95aed32fca20fd45e | gemini-4-neon-high (agy) | FAIL | /mnt/nvme-raid0/cop-inbox/handoff/comply-cop/receipts-forjar-1169/PR-1104-d196fa7a89.md |
| #1105 | 9d64276253298708483703f671d765328dd0726a | gemini-4-neon-high (agy) | PASS | /mnt/nvme-raid0/cop-inbox/handoff/comply-cop/receipts-forjar-1169/PR-1105-9d64276253.md |
| #1106 | ff530c6dd03eb8c5892b47b92879df742ce1f367 | gemini-4-neon-high (agy) | PASS | /mnt/nvme-raid0/cop-inbox/handoff/comply-cop/receipts-forjar-1169/PR-1106-ff530c6dd0.md |
| #1107 | 4e8e7554d01dc8bc0f878befc962cab3d511cef4 | gemini-4-neon-high (agy) | FAIL | /mnt/nvme-raid0/cop-inbox/handoff/comply-cop/receipts-forjar-1169/PR-1107-4e8e7554d0.md |
| #1108 | 639386ba1fe0aea4a32a07fd4d8463da65b24e38 | gemini-4-neon-high (agy) | FAIL | /mnt/nvme-raid0/cop-inbox/handoff/comply-cop/receipts-forjar-1169/PR-1108-639386ba1f.md |

## Gaps (FAIL, blocking findings)

- **#1103**: `src/core/store/registry_push.rs:56` (and 15 more sites) builds errors with
  `ForjarError::from_untyped`, so classes still come from `legacy_prose_class`
  while 6 functions leave the `untyped_error_fns` ratchet count.
  `src/core/store/tests_registry_push.rs:329` and `:168` assert classes that depend on
  the host (`127.0.0.1:1` reply text, `$TMPDIR` holding the word "partial").
- **#1104**: `src/cli/image_cmd.rs:273` (and 6 more sites) uses `from_untyped` the same way
  (`storage_layout_yaml("connection")` gives exit 4). `src/cli/tests_image_b.rs:439` asserts
  nothing when `xorriso` is on PATH.
- **#1107**: `tests/falsification_replace_running_binary.rs:53` uses `LIVE_BIN = "/bin/sh"`.
  Where `/bin/sh` is bash, `--version` gives 5.x and `downgrade_guard` refuses the 1.0.0 pin,
  so the test depends on the host.
- **#1108**: `scripts/hooks/pre-push-test.sh` sits in `scripts/hooks/`, so
  `scripts/install-hooks.sh` (which copies every file there) installs it into `.git/hooks/`.
  Each later change to it then makes `make install-hooks` exit 1.

Follow-up fixes belong in their own PRs, linked to #1169.
