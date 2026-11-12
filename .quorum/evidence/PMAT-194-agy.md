# Independent Review (`agy`) — `build.rs` runtime `CARGO_MANIFEST_DIR` & tool-job guard (PMAT-194 / forjar#909)

- **Reviewer**: `agy` — Model: `Gemini 4 Carbon (thinking=high)` (non-author model)
- **Round 1 (`@6f655eac9d62adedb4f54f161ea6d217dddb329a`)**: `3/3 FAIL` — caught multi-`-p` false negative and `--package=<pkg>` blind spot in `tests/falsification_tool_jobs_run_where_the_tools_are.rs:289`.
- **Round 2 (`@8905e5a29801bcb1ba7775d37d3f99bdfbcffe88`)**: `3/3 PASS` — verified both defects fixed plus `build.rs` runtime `CARGO_MANIFEST_DIR` lookup (`FJ_909_REVERT_RC=101` on `build.rs` revert on `<host>`).
