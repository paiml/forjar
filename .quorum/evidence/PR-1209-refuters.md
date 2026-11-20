=== REFUTER 1 ===
[
  {"claim":"1.1","verdict":"SURVIVED","evidence":"src/resources/package/cargo.rs:34,325,336 `c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+' | '*' | '~' | '^')`; `let pin_arg = version.map_or(\" ${_FJ_PIN:+--version \\\"=$_FJ_PIN\\\"}\", |_| \"\");`"},
  {"claim":"1.2","verdict":"SURVIVED","evidence":"PR_BODY.md:8 `Re-cut of pull request #1206 (closed): same code and tests, new commit messages, and the comment-only edits listed below.`; no `.git` directory in tree"},
  {"claim":"1.3","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:55-77 `pub enum SweepError` (no `UnknownRunnerHost`); src/core/pool_orphans_tests.rs:137-138 `assert_eq!(s.terminate_hosts, [\"o1\", \"o2\"]);` / `assert_eq!(s.unknown_runners.len(), 2);`"},
  {"claim":"1.4","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests_b4.rs:300-318 `for bad in [\"sir-0a1b2c3d4e5f6a7b8\", ... \"xx-0a1b2c3d\"]`; `assert_eq!(count(&log, \"terminate-instances\"), 1`; `assert_eq!(count(&log, \"cancel-spot\"), 1`"},
  {"claim":"1.5","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:138-151 `.filter(|r| !hosts.iter().any(|h| h.id == r.host_id))` and `&& !unknown_runners.iter().any(|(_, u)| u.eq_ignore_ascii_case(&h.id))`"},
  {"claim":"1.6","verdict":"SURVIVED","evidence":"src/resources/package/cargo.rs:276-281,336,359 `_FJ_PIN=\"$_FJ_LATEST\"`, `_FJ_NC=1`, `let pin_arg = version.map_or(\" ${_FJ_PIN:+--version \\\"=$_FJ_PIN\\\"}\", |_| \"\");`"},
  {"claim":"1.7","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:11,52,55,59 `sirs=\"$({cmd})\" || { ... exit 1; }`; `if [ \"${{#h}}\" -ne 8 ]; then bad=1; continue; fi`; `[ -z \"$bad\" ] || exit 1`"},
  {"claim":"1.8","verdict":"SURVIVED","evidence":"src/core/parser/mod.rs:30-31 `#[cfg(test)] mod tests_cloud_instance_validate;` + src/core/parser/tests_cloud_instance_validate.rs:30-154 (6 `#[test]`) + src/resources/cloud_instance/*.rs (133 `#[test]`) = 139 tests, not 133"},
  {"claim":"1.9","verdict":"SURVIVED","evidence":"src/core/pool_orphans_tests.rs:28-249 (21 `#[test]`) and src/resources/mod.rs:86-94 (`tests_package` 37 + `_b` 30 + `_c` 15 + `_d` 13 + `_e` 5 = 100 `#[test]`)"},
  {"claim":"1.10","verdict":"SURVIVED","evidence":"PR_BODY.md:18,20-22 `no log attached, and no CI check at 56ecdf0 shows these results`; `cargo test` and `cargo clippy` at `56ecdf0` are unverified without logs or execution"},
  {"claim":"1.11","verdict":"SURVIVED","evidence":"docs/roadmaps/roadmap.yaml:8238-8291 (`- id: PMAT-1186` ... `- id: PMAT-1185`); `yq . docs/roadmaps/roadmap.yaml` exits 0"},
  {"claim":"1.12","verdict":"SURVIVED","evidence":"ANGLE.diff:80-86 `+ pub unknown_runners: Vec<(String, String)>,` and src/core/pool_orphans_tests.rs:138 `assert_eq!(s.unknown_runners.len(), 2);`"},
  {"claim":"1.13","verdict":"SURVIVED","evidence":"ANGLE.diff:313-314 `- cargo install --force --locked --root \"$_STAGING\"{features_arg} {install_arg}` and src/resources/tests_package_c.rs:280 `o.contains(\"--version =0.24.0 \")`"},
  {"claim":"1.14","verdict":"SURVIVED","evidence":"ANGLE.diff:220 `- if [ \"${{#h}}\" -ne 8 ]; then ... exit 1; fi` before terminate, and src/resources/cloud_instance/tests_b4.rs:317 `assert_eq!(count(&log, \"terminate-instances\"), 1, \"{bad}: {log}\");`"},
  {"claim":"1.15","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:149-151 + pool_orphans_tests.rs:137; src/resources/package/cargo.rs:276,336 + tests_package_c.rs:280,284; src/resources/cloud_instance/destroy.rs:52,59 + tests_b4.rs:314,318 — all 5 named mutants fail their tests"},
  {"claim":"1.16","verdict":"SURVIVED","evidence":"PR_BODY.md:26 `Versus the reviewed #1206 head (4eb1d3f)`; no `.git` history exists in tree"},
  {"claim":"1.17","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:148-151 `u.eq_ignore_ascii_case(&h.id)` and ANGLE.diff:178 `- for bad in [\"ip-10-0-0-5\", \"zzz\", \"I-0ABC\", \"i-0ab\"]`"},
  {"claim":"1.18","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:52,58-59 `if [ \"${{#h}}\" -ne 8 ]; then bad=1; continue; fi`, `echo \"destroyed $ids\"`, `[ -z \"$bad\" ] || exit 1` and tests_b4.rs:301 `\"sir-0a1b2c3d4e5f6a7b8\"`"},
  {"claim":"1.19","verdict":"SURVIVED","evidence":"src/resources/package/cargo.rs:34,325,348 `Some(v) => (v.to_string(), \"FORJAR_NO_CARGO_CACHE\", String::new())` and `_CACHE_KEY=\"{crate_name}-{ver_tag}{cache_suffix}-$(uname -m)\"`"},
  {"claim":"1.20","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests.rs:50 `const STUB: &str`, src/resources/tests_package_c.rs:265 `cargo() {{ ... }}`, docs/roadmaps/roadmap.yaml:8220-8237 (`PMAT-1187` untouched in ANGLE.diff)"},
  {"claim":"1.21","verdict":"SURVIVED","evidence":"PR.stat:1,8 `docs/roadmaps/roadmap.yaml | 54 +++...` and `7 files changed, 135 insertions(+), 66 deletions(-)`"},
  {"claim":"1.22","verdict":"SURVIVED","evidence":"PR_BODY.md:38 `4 commits (c33c01b8, 31be7b82, 23f79e35, 56ecdf0)`; no `.git` directory in tree"},
  {"claim":"1.23","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:8 `//! It returns hosts to terminate only; runner deregistration is not part of it.` and :43 `/// What one sweep removes. The list keeps input order, without duplicates.` vs :50 `pub unknown_runners: Vec<(String, String)>,`"},
  {"claim":"1.24","verdict":"REFUTED","evidence":"src/core/pool_orphans.rs:141,149-151 `&& !unknown_runners.iter().any(|(_, u)| u.eq_ignore_ascii_case(&h.id))` and src/core/pool_orphans_tests.rs:135-137 `r(\"b\", \"I-0ABC\", true)` / `assert_eq!(s.terminate_hosts, [\"o1\", \"o2\"]);` — swapping `(r.name, r.host_id)` puts `\"b\"` in `.1` so `\"i-0abc\"` is no longer protected and is added to `terminate_hosts`, failing line 137 rather than passing all tests"},
  {"claim":"1.25","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:11,43-45 `sirs=\"$({cmd})\" || { ... exit 1; }` runs at `{spot}` before `aws ec2 terminate-instances`, and tests_b4.rs:323 `a_failed_spot_request_lookup_is_an_error_before_any_terminate`, vs docs/roadmaps/roadmap.yaml:8277 `title: 'destroy blocks terminate on a cancel-only lookup'`"},
  {"claim":"1.26","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:58-59 `echo \"destroyed $ids\"` followed by `[ -z \"$bad\" ] || exit 1` and ANGLE.diff:280 `- assert!(!out.contains(\"destroyed\"), \"{bad}: {out}\");`"},
  {"claim":"1.27","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests_b4.rs:312-318 `assert_eq!(count(&log, \"cancel-spot\"), 1, \"{bad}: {log}\");` — never checks whether `sir-0a1b2c3d` or `{bad}` was cancelled, so inverting the check passes this test"},
  {"claim":"1.28","verdict":"SURVIVED","evidence":"src/resources/tests_package_c.rs:265-273,283-286 `Command::new(\"bash\").args([\"-c\", &sh]).output().unwrap()` ignores `o.status`, omits `set -euo pipefail`, and only asserts `!unpinned_cargo(\"\").contains(\"--version\")`"},
  {"claim":"2.1","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:55-77 (`UnknownRunnerHost` removed) and src/core/pool_orphans_tests.rs:137-138 `assert_eq!(s.terminate_hosts, [\"o1\", \"o2\"]);` / `assert_eq!(s.unknown_runners.len(), 2);`"},
  {"claim":"2.2","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests_b4.rs:300-318 (8 malformed spellings in `for bad in [...]`, 2-instance fixture, `terminate-instances` == 1, `cancel-spot` == 1, no id check)"},
  {"claim":"2.3","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:138-151 `.filter(|r| !hosts.iter().any(|h| h.id == r.host_id))` and `!unknown_runners.iter().any(|(_, u)| u.eq_ignore_ascii_case(&h.id))`"},
  {"claim":"2.4","verdict":"SURVIVED","evidence":"src/resources/package/cargo.rs:276-281,336,359 `_FJ_PIN=\"$_FJ_LATEST\"`, `_FJ_NC=1`, `let pin_arg = version.map_or(\" ${_FJ_PIN:+--version \\\"=$_FJ_PIN\\\"}\", |_| \"\");`"},
  {"claim":"2.5","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:11,43-59 `sirs=\"$({cmd})\" || { ... exit 1; }`, `if [ \"${{#h}}\" -ne 8 ]; then bad=1; continue; fi`, `[ -z \"$bad\" ] || exit 1`"},
  {"claim":"2.6","verdict":"SURVIVED","evidence":"src/lib.rs:38 `pub mod core;` -> src/core/parser/mod.rs:30-31 `#[cfg(test)] mod tests_cloud_instance_validate;` (6 tests at lines 30,108,119,136,145,154) + 133 tests across 12 files in `src/resources/cloud_instance/` = 139 tests"},
  {"claim":"2.7","verdict":"SURVIVED","evidence":"src/core/pool_orphans_tests.rs:28-249 (21 tests) and src/resources/tests_package*.rs (37+30+15+13+5 = 100 tests); no test log attached in PR_BODY.md:18"},
  {"claim":"2.8","verdict":"SURVIVED","evidence":"Cargo.toml:1-282 (`cargo fmt --manifest-path tree/Cargo.toml --all -- --check` exits 0) and docs/roadmaps/roadmap.yaml:8238-8291 (`yq . tree/docs/roadmaps/roadmap.yaml` exits 0)"},
  {"claim":"2.9","verdict":"SURVIVED","evidence":"PR_BODY.md:18,22 `no log attached, and no CI check at 56ecdf0 shows these results`; `cargo clippy` was not executed"},
  {"claim":"2.10","verdict":"SURVIVED","evidence":"PR_BODY.md:23-24 `from the earlier run on pull request #1206... not rerun on this cut`; no #1206 log attached"},
  {"claim":"2.11","verdict":"SURVIVED","evidence":"PR_BODY.md:8,26 `Versus the reviewed #1206 head (4eb1d3f)`; no `.git` directory in tree"},
  {"claim":"2.12","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:151 `u.eq_ignore_ascii_case(&h.id)`, src/resources/cloud_instance/destroy.rs:52,58-59 `if [ \"${{#h}}\" -ne 8 ]` / `echo \"destroyed $ids\"`, src/resources/package/cargo.rs:325,333 `Some(v) => (v.to_string(), ...)`"},
  {"claim":"2.13","verdict":"SURVIVED","evidence":"PR.stat:1-8 and ANGLE.diff:1-359 — none of the 7 changed files touch `#1187`, `src/cli/check_marker.rs`, or `PMAT-1187`"},
  {"claim":"2.14","verdict":"SURVIVED","evidence":"PR.stat:1,8 `docs/roadmaps/roadmap.yaml | 54 +++...` and `7 files changed, 135 insertions(+), 66 deletions(-)`"},
  {"claim":"2.15","verdict":"SURVIVED","evidence":"PR_BODY.md:38 `4 commits (c33c01b8, 31be7b82, 23f79e35, 56ecdf0)`; no `.git` directory in tree"},
  {"claim":"2.16","verdict":"SURVIVED","evidence":"src/resources/tests_package_c.rs:269-273,283-286 (`o.status` and `o.stderr` discarded, no `set -euo pipefail` unlike `src/resources/package/cargo.rs:155`, unresolved check only asserts `!unpinned_cargo(\"\").contains(\"--version\")`)"},
  {"claim":"2.17","verdict":"SURVIVED","evidence":"src/resources/tests_package_c.rs:265,280 `echo \"ARGS $*\"` / `o.contains(\"--version =0.24.0 \")` — outer-quoting `\"${_FJ_PIN:+--version =$_FJ_PIN}\"` passes single arg `--version =0.24.0` (or `\"\"` when empty) yet still prints matching space-joined `ARGS $*`"},
  {"claim":"2.18","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests_b4.rs:312,318 `sir-0a1b2c3d\\ni-00b2c3d4 ... {bad}` always puts valid id first, and no other test feeds a malformed spot id, so `break` at `src/resources/cloud_instance/destroy.rs:52` survives all `cloud_instance` tests"},
  {"claim":"2.19","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:11,43-45 `{spot}` exits 1 on `lookup_sirs` failure before `aws ec2 terminate-instances`, contradicting `PR_BODY.md:3` `Closes #1185` and `docs/roadmaps/roadmap.yaml:8277` `title: 'destroy blocks terminate on a cancel-only lookup'`"},
  {"claim":"2.20","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:40,53-56 `set -eu` with unguarded `aws ec2 cancel-spot-instance-requests ... >/dev/null` aborts before remaining tokens, line 55 `spot-request-malformed`, and line 56 `{relookup}`"},
  {"claim":"2.21","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:58-59 `echo \"destroyed $ids\"` before `[ -z \"$bad\" ] || exit 1`, ANGLE.diff:280 `- assert!(!out.contains(\"destroyed\"), \"{bad}: {out}\");`, omitted from PR_BODY.md:10 GATE CHANGE"},
  {"claim":"2.22","verdict":"SURVIVED","evidence":"src/core/pool_orphans_tests.rs:135-138 `assert_eq!(s.unknown_runners.len(), 2);` (only test with non-empty `unknown_runners`, no exact-match known runner mixed in) — replacing `r.name.clone()` at `src/core/pool_orphans.rs:141` with `String::new()` or `r.host_id.clone()` leaves `.1` intact and passes all 21 tests"},
  {"claim":"2.23","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:8 `//! It returns hosts to terminate only; runner deregistration is not part of it.` and :138-154 (only `eq_ignore_ascii_case` protects unlisted hosts; `Runner.busy` is unread, so non-case mismatches schedule busy hosts for termination)"},
  {"claim":"3.1","verdict":"SURVIVED","evidence":"src/resources/package/cargo.rs:34,276,325,333,336,348 `matches!(c, '.' | '_' | '-' | '+' | '*' | '~' | '^')`, `let pin_arg = version.map_or(\" ${_FJ_PIN:+--version \\\"=$_FJ_PIN\\\"}\", |_| \"\");`, `_CACHE_KEY=\"{crate_name}-{ver_tag}{cache_suffix}-$(uname -m)\"`"},
  {"claim":"3.2","verdict":"SURVIVED","evidence":"PR_BODY.md:8,26 `git diff 4eb1d3f HEAD --stat`; no `.git` directory in tree"},
  {"claim":"3.3","verdict":"SURVIVED","evidence":"src/core/pool_orphans_tests.rs:137-138 `assert_eq!(s.terminate_hosts, [\"o1\", \"o2\"]);` / `assert_eq!(s.unknown_runners.len(), 2);` — never checks any runner name or host id in `unknown_runners`"},
  {"claim":"3.4","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests_b4.rs:300-318 (8 malformed spellings, 2-instance fixture, `rc != 0`, `\"not a whole spot request id\"`, `terminate-instances` == 1, `cancel-spot` == 1)"},
  {"claim":"3.5","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:138-158 `.filter(|r| !hosts.iter().any(|h| h.id == r.host_id))` and `&& !unknown_runners.iter().any(|(_, u)| u.eq_ignore_ascii_case(&h.id))`"},
  {"claim":"3.6","verdict":"SURVIVED","evidence":"src/resources/package/cargo.rs:276-281,336,359 `_FJ_PIN=\"$_FJ_LATEST\"` and `pin_arg` expands to two args `--version` `=0.24.0` (and zero args when empty) in both `bash` and `dash`"},
  {"claim":"3.7","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:11,43-59 `sirs=\"$({cmd})\" || { ... exit 1; }`, `if [ \"${{#h}}\" -ne 8 ]; then bad=1; continue; fi`, `[ -z \"$bad\" ] || exit 1`"},
  {"claim":"3.8","verdict":"REFUTED","evidence":"src/core/parser/mod.rs:30-31 `#[cfg(test)] mod tests_cloud_instance_validate;` and src/core/parser/tests_cloud_instance_validate.rs:30,108,119,136,145,154 add 6 `#[test]` functions matching filter `cloud_instance` alongside the 133 in `src/resources/cloud_instance/*.rs` (139 total, not 133), refuting the claim that `static #[test] counts match exactly (21, 133, 100)`"},
  {"claim":"3.9","verdict":"SURVIVED","evidence":"docs/roadmaps/roadmap.yaml:1-8291 (`- id: PMAT-1186`, `- id: PMAT-1188`, `- id: PMAT-1185`); `yq . docs/roadmaps/roadmap.yaml` exits 0"},
  {"claim":"3.10","verdict":"SURVIVED","evidence":"PR_BODY.md:18,22 `cargo clippy -j 4 --locked --lib --tests -- -D warnings: finished, no warnings`; `cargo clippy` was not run and no CI log is attached"},
  {"claim":"3.11","verdict":"SURVIVED","evidence":"PR_BODY.md:23-24 `from the earlier run on pull request #1206`; no pre-PR git history or run logs exist in tree, and all 5 named mutants statically violate assertions"},
  {"claim":"3.12","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:148-151 `u.eq_ignore_ascii_case(&h.id)` and src/core/pool_orphans_tests.rs:133-139 (no test for `i-0ab` vs `i-0abc`)"},
  {"claim":"3.13","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:52,58-59 `if [ \"${{#h}}\" -ne 8 ]`, `echo \"destroyed $ids\"`, `[ -z \"$bad\" ] || exit 1` and tests_b4.rs:301 `\"sir-0a1b2c3d4e5f6a7b8\"`"},
  {"claim":"3.14","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests.rs:50-84 (`STUB`), src/resources/tests_package_c.rs:265 (`cargo() {{ ... }}`), PR.stat:1-8 and docs/roadmaps/roadmap.yaml:8220-8291 (`#1187` untouched)"},
  {"claim":"3.15","verdict":"SURVIVED","evidence":"PR.stat:1,8 `docs/roadmaps/roadmap.yaml | 54 +++...` and `7 files changed, 135 insertions(+), 66 deletions(-)`"},
  {"claim":"3.16","verdict":"SURVIVED","evidence":"PR_BODY.md:38 `4 commits (c33c01b8, 31be7b82, 23f79e35, 56ecdf0)`; no `.git` directory in tree"},
  {"claim":"3.17","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:8 `//! It returns hosts to terminate only; runner deregistration is not part of it.` contradicts :5-6,45-51,128-129,155-158 (`unknown_runners`)"},
  {"claim":"3.18","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:43 `/// What one sweep removes. The list keeps input order, without duplicates.` on `pub struct Sweep` (:45-51) which now has two `Vec` fields and does not remove `unknown_runners`"},
  {"claim":"3.19","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:140,149-151 `.filter(|r| !hosts.iter().any(|h| h.id == r.host_id))` places `r(\"b\", \"I-0ABC\", true)` into `unknown_runners` identically to `r(\"ghost\", \"gone\", false)` while `u.eq_ignore_ascii_case(&h.id)` protects `\"i-0abc\"`"},
  {"claim":"3.20","verdict":"SURVIVED","evidence":"src/core/pool_orphans_tests.rs:138 `assert_eq!(s.unknown_runners.len(), 2);` — mutating `src/core/pool_orphans.rs:141` to `(String::new(), r.host_id.clone())` or reversing `unknown_runners` leaves `.1` and `.len()` intact so all 21 tests pass"},
  {"claim":"3.21","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:120-126,143-154 (`validate()` no longer checks `UnknownRunnerHost`; `Runner.busy` is unread) and ANGLE.diff:178 `- for bad in [\"ip-10-0-0-5\", \"zzz\", \"I-0ABC\", \"i-0ab\"]`"},
  {"claim":"3.22","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:40,52-56 `set -eu` aborts immediately if `aws ec2 cancel-spot-instance-requests` on line 53 exits non-zero, skipping line 55 `spot-request-malformed` and line 56 `{relookup}`"},
  {"claim":"3.23","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:58-59 `echo \"destroyed $ids\"` before `[ -z \"$bad\" ] || exit 1`, ANGLE.diff:280 `- assert!(!out.contains(\"destroyed\"), \"{bad}: {out}\");`, omitted from PR_BODY.md:10 GATE CHANGE"},
  {"claim":"3.24","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:11,43-45,52 (`{spot}` still exits 1 before `terminate-instances` on failure; `\"${{#h}}\" -ne 8` rejects 17-char `sir-` ids, locked in at `tests_b4.rs:301`), vs `PR_BODY.md:3` `Closes #1185` and `docs/roadmaps/roadmap.yaml:8277`"},
  {"claim":"3.25","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests_b4.rs:299-320 only asserts `rc != 0`, `\"not a whole spot request id\"`, `count(&log, \"terminate-instances\") == 1`, and `count(&log, \"cancel-spot\") == 1`, without checking which spot id, both instance ids, or call order"},
  {"claim":"3.26","verdict":"SURVIVED","evidence":"src/resources/tests_package_c.rs:265-273,283-286 omits `set -euo pipefail` (present at `src/resources/package/cargo.rs:155`), ignores `o.status`, and checks only `!unpinned_cargo(\"\").contains(\"--version\")` without verifying `ARGS ` or `KEY=bat-unresolved-`"},
  {"claim":"3.27","verdict":"SURVIVED","evidence":"src/resources/tests_package_c.rs:265,280 `echo \"ARGS $*\"` / `o.contains(\"--version =0.24.0 \")` erases argv boundaries, so outer-quoting `${_FJ_PIN:+--version \"=$_FJ_PIN\"}` at `src/resources/package/cargo.rs:336` passes a single invalid arg yet still satisfies the test"}
]
VERDICT: FAIL
text_only: yes
refuter: 1
reviewer_model: gemini-4-neon-high

=== REFUTER 2 ===
[
  {"claim":"1.1","verdict":"SURVIVED","evidence":"src/resources/package/cargo.rs:34,276,325,336,359 (`let pin_arg = version.map_or(\" ${_FJ_PIN:+--version \\\"=$_FJ_PIN\\\"}\", |_| \"\");` and `matches!(c, '.' | '_' | '-' | '+' | '*' | '~' | '^')`): unpinned crates pin via `$_FJ_PIN` while `Some(v)` retains raw `v` in `_CACHE_KEY` with empty `pin_arg`."},
  {"claim":"1.2","verdict":"SURVIVED","evidence":"PR_BODY.md:8 (`Re-cut of pull request #1206 (closed): same code and tests, new commit messages, and the comment-only edits listed below.`): exported `tree/` has no `.git` directory and no PR #1206 diff, so this is genuinely unverifiable."},
  {"claim":"1.3","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:55-77 (`pub enum SweepError`) and src/core/pool_orphans_tests.rs:137-138 (`assert_eq!(s.terminate_hosts, [\"o1\", \"o2\"]);`, `assert_eq!(s.unknown_runners.len(), 2);`): matches the code and explicitly notes `(specifically length 2)`."},
  {"claim":"1.4","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests_b4.rs:300-318 (`for bad in [\"sir-0a1b2c3d4e5f6a7b8\", ... \"xx-0a1b2c3d\"]`, `assert_eq!(count(&log, \"terminate-instances\"), 1`, `assert_eq!(count(&log, \"cancel-spot\"), 1`): 8 malformed spellings paired with `sir-0a1b2c3d`, asserting non-zero exit, message, 1 terminate, and 1 cancel-spot without checking which id."},
  {"claim":"1.5","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:138-158 (`filter(|r| !hosts.iter().any(|h| h.id == r.host_id))` and `!unknown_runners.iter().any(|(_, u)| u.eq_ignore_ascii_case(&h.id))`): matches PR_BODY.md:14 exactly."},
  {"claim":"1.6","verdict":"SURVIVED","evidence":"src/resources/package/cargo.rs:276-281,324-336,359 (`_FJ_PIN=\"$_FJ_LATEST\"`, `_FJ_NC=1`, `Some(v) => (v.to_string(), \"FORJAR_NO_CARGO_CACHE\", String::new())`): unpinned passes `--version =<resolved>`, unresolved leaves `_FJ_PIN` empty with `_FJ_NC=1`, pinned script is unchanged."},
  {"claim":"1.7","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:11,43-59 (`sirs=\"$({cmd})\" || { ... exit 1; }`, `if [ \"${{#h}}\" -ne 8 ]; then bad=1; continue; fi`, `[ -z \"$bad\" ] || exit 1`): matches PR_BODY.md:16."},
  {"claim":"1.8","verdict":"SURVIVED","evidence":"src/lib.rs:38 (`pub mod core;`), src/core/parser/mod.rs:30-31 (`#[cfg(test)] mod tests_cloud_instance_validate;`), src/core/parser/tests_cloud_instance_validate.rs:30,108,119,136,145,154 (6 `#[test]` functions) plus 133 `#[test]` functions in `src/resources/cloud_instance/*.rs`: `--lib cloud_instance` matches 139 tests, not 133."},
  {"claim":"1.9","verdict":"SURVIVED","evidence":"src/core/pool_orphans_tests.rs:28-249 (21 `#[test]` functions) and src/resources/mod.rs:86-94 (`tests_package` 37 + `tests_package_b` 30 + `tests_package_c` 15 + `tests_package_d` 13 + `tests_package_e` 5 = 100): static `--lib` counts match 21 and 100."},
  {"claim":"1.10","verdict":"SURVIVED","evidence":"PR_BODY.md:18,20-22 (`no log attached, and no CI check at 56ecdf0 shows these results`): no CI logs or `.git` commit `56ecdf0` exist in `tree/`, and `rustfmt --edition 2021 --check` exits 0 on all 6 modified `.rs` files."},
  {"claim":"1.11","verdict":"SURVIVED","evidence":"docs/roadmaps/roadmap.yaml:8238-8291 (`- id: PMAT-1186`, `- id: PMAT-1188`, `- id: PMAT-1185`): `yq . docs/roadmaps/roadmap.yaml` exits 0."},
  {"claim":"1.12","verdict":"SURVIVED","evidence":"ANGLE.diff:80-86 (`pub struct Sweep` had only `pub terminate_hosts: Vec<String>`) and src/core/pool_orphans_tests.rs:138 (`assert_eq!(s.unknown_runners.len(), 2);`): pre-fix code fails compilation with E0609."},
  {"claim":"1.13","verdict":"SURVIVED","evidence":"ANGLE.diff:313 (`cargo install --force --locked --root \"$_STAGING\"{features_arg} {install_arg}`) and src/resources/tests_package_c.rs:280 (`o.contains(\"--version =0.24.0 \")`): pre-fix script omits `--version`, failing the first assert."},
  {"claim":"1.14","verdict":"SURVIVED","evidence":"ANGLE.diff:220 (`if [ \"${{#h}}\" -ne 8 ]; then echo 'spot-request-malformed' >&2; ... not a whole spot request id ... exit 1; fi`) and src/resources/cloud_instance/tests_b4.rs:314-317 (`assert_eq!(count(&log, \"terminate-instances\"), 1`): pre-fix exited 1 before terminate, failing line 317 with left 0, right 1."},
  {"claim":"1.15","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:149-151 + pool_orphans_tests.rs:137; src/resources/package/cargo.rs:276,336 + tests_package_c.rs:280,284; src/resources/cloud_instance/destroy.rs:52,59 + tests_b4.rs:314,318: each of the 5 named mutants violates its cited test assertion in this tree."},
  {"claim":"1.16","verdict":"SURVIVED","evidence":"PR_BODY.md:26 (`Versus the reviewed #1206 head (4eb1d3f)`): no `.git` metadata or commit `4eb1d3f` exists in `tree/`."},
  {"claim":"1.17","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:148-151 (`!runners.iter().any(|r| r.host_id == h.id)` and `u.eq_ignore_ascii_case(&h.id)`) and ANGLE.diff:176-182 (deletion of `\"i-0ab\"` test): matches PR_BODY.md:30."},
  {"claim":"1.18","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:52,58-59 (`if [ \"${{#h}}\" -ne 8 ]; then bad=1; continue; fi`, `echo \"destroyed $ids\"`, `[ -z \"$bad\" ] || exit 1`) and src/resources/cloud_instance/tests_b4.rs:301 (`\"sir-0a1b2c3d4e5f6a7b8\"`): matches PR_BODY.md:31."},
  {"claim":"1.19","verdict":"SURVIVED","evidence":"src/resources/package/cargo.rs:34,325,333,336 (`Some(v) => (v.to_string(), \"FORJAR_NO_CARGO_CACHE\", String::new())`): non-exact pins like `^1` still key `_CACHE_KEY` on `^1` with empty `pin_arg`."},
  {"claim":"1.20","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests.rs:50-84 (`const STUB: &str`), src/resources/tests_package_c.rs:265 (`cargo() {{ ... }}`), docs/roadmaps/roadmap.yaml:8220-8237 (`PMAT-1187` untouched in `ANGLE.diff`): matches PR_BODY.md:36."},
  {"claim":"1.21","verdict":"SURVIVED","evidence":"PR.stat:1,8 (`docs/roadmaps/roadmap.yaml | 54`, `7 files changed, 135 insertions(+), 66 deletions(-)`): 135 + 66 = 201 changed lines across 7 files, 54 roadmap lines for 3 rows."},
  {"claim":"1.22","verdict":"SURVIVED","evidence":"PR_BODY.md:38 (`4 commits (c33c01b8, 31be7b82, 23f79e35, 56ecdf0)`): exported `tree/` has no `.git` history to verify commit count or hashes."},
  {"claim":"1.23","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:8 (`//! It returns hosts to terminate only; runner deregistration is not part of it.`) and line 43 (`/// What one sweep removes. The list keeps input order, without duplicates.`): directly contradicts lines 5-6, 48-50, and 128-129."},
  {"claim":"1.24","verdict":"REFUTED","evidence":"src/core/pool_orphans.rs:141,149-151 (`.map(|r| (r.name.clone(), r.host_id.clone()))` and `!unknown_runners.iter().any(|(_, u)| u.eq_ignore_ascii_case(&h.id))`) and src/core/pool_orphans_tests.rs:135-137 (`r(\"b\", \"I-0ABC\", true)`, `assert_eq!(s.terminate_hosts, [\"o1\", \"o2\"]);`): swapping `(r.name, r.host_id)` at line 141 puts `\"b\"` instead of `\"I-0ABC\"` into `u`, so `\"i-0abc\"` is no longer protected by line 151 and is added to `s.terminate_hosts`, failing line 137 rather than passing all tests."},
  {"claim":"1.25","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:11,43-45 (`sirs=\"$({cmd})\" || {{ ... exit 1; }}`, `{spot}`, `aws ec2 terminate-instances`) and docs/roadmaps/roadmap.yaml:8277 (`title: 'destroy blocks terminate on a cancel-only lookup'`): a failed cancel-only lookup still exits 1 before terminate."},
  {"claim":"1.26","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:58-59 (`echo \"destroyed $ids\"\\n`, `[ -z \"$bad\" ] || exit 1\\n`) and ANGLE.diff:280 (`- assert!(!out.contains(\"destroyed\"), \"{bad}: {out}\");`): prints `destroyed $ids` to stdout before exiting 1 on malformed spot id."},
  {"claim":"1.27","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests_b4.rs:312-318 (`assert_eq!(count(&log, \"cancel-spot\"), 1, \"{bad}: {log}\");`) and src/resources/cloud_instance/tests.rs:50-84 (`STUB` accepts any `cancel-spot` arg with exit 0): this test never checks which id was cancelled, so inverting the check passes this test."},
  {"claim":"1.28","verdict":"SURVIVED","evidence":"src/resources/tests_package_c.rs:264-273,283-286 (`Command::new(\"bash\").args([\"-c\", &sh]).output().unwrap()`, `assert!(!unpinned_cargo(\"\").contains(\"--version\"), \"unresolved must not pin\");`): omits `set -euo pipefail`, ignores `o.status`, and never checks `cargo install` executed on the unresolved branch."},
  {"claim":"2.1","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:55-77 (`pub enum SweepError`) and src/core/pool_orphans_tests.rs:134-139 (`assert_eq!(s.terminate_hosts, [\"o1\", \"o2\"]);`, `assert_eq!(s.unknown_runners.len(), 2);`): matches the code and explicitly notes `.len() == 2`."},
  {"claim":"2.2","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests_b4.rs:300-318 (`for bad in [\"sir-0a1b2c3d4e5f6a7b8\", ... \"xx-0a1b2c3d\"]`, `assert_eq!(count(&log, \"terminate-instances\"), 1`, `assert_eq!(count(&log, \"cancel-spot\"), 1`): accurately reflects PR_BODY.md:10 and lines 313-318."},
  {"claim":"2.3","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:138-152 (`filter(|r| !hosts.iter().any(|h| h.id == r.host_id))` and `!unknown_runners.iter().any(|(_, u)| u.eq_ignore_ascii_case(&h.id))`): matches PR_BODY.md:14."},
  {"claim":"2.4","verdict":"SURVIVED","evidence":"src/resources/package/cargo.rs:276-281,336,359 (`_FJ_PIN=\"$_FJ_LATEST\"`, `version.map_or(\" ${_FJ_PIN:+--version \\\"=$_FJ_PIN\\\"}\", |_| \"\")`): matches PR_BODY.md:15."},
  {"claim":"2.5","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:11,43-59 (`sirs=\"$({cmd})\" || {{ ... exit 1; }}`, `if [ \"${{#h}}\" -ne 8 ]; then bad=1; continue; fi`, `[ -z \"$bad\" ] || exit 1`): matches PR_BODY.md:16."},
  {"claim":"2.6","verdict":"SURVIVED","evidence":"src/lib.rs:38 (`pub mod core;`), src/core/parser/mod.rs:30-31 (`#[cfg(test)] mod tests_cloud_instance_validate;`), src/core/parser/tests_cloud_instance_validate.rs:30,108,119,136,145,154 (6 `#[test]` functions) plus 133 in `src/resources/cloud_instance/` across 12 files: `--lib cloud_instance` matches 139 tests, not 133."},
  {"claim":"2.7","verdict":"SURVIVED","evidence":"src/core/pool_orphans_tests.rs:28-249 (21 `#[test]` functions) and src/resources/tests_package*.rs (37+30+15+13+5=100): static counts match 21 and 100 while no test execution log is attached."},
  {"claim":"2.8","verdict":"SURVIVED","evidence":"Cargo.toml:1-282 and docs/roadmaps/roadmap.yaml:8238-8291: both `cargo fmt --manifest-path tree/Cargo.toml --all -- --check` and `yq . tree/docs/roadmaps/roadmap.yaml` exit 0."},
  {"claim":"2.9","verdict":"SURVIVED","evidence":"PR_BODY.md:22 (`cargo clippy -j 4 --locked --lib --tests -- -D warnings: finished, no warnings`): no CI check or log is attached."},
  {"claim":"2.10","verdict":"SURVIVED","evidence":"PR_BODY.md:23-24 (`Red before fix (from the earlier run on pull request #1206...`, `Mutants (from the earlier run on pull request #1206...`): statically consistent with the diff, with no PR #1206 log attached."},
  {"claim":"2.11","verdict":"SURVIVED","evidence":"PR_BODY.md:8,26 (`Versus the reviewed #1206 head (4eb1d3f)`): commit `4eb1d3f` and `.git` history are absent from `tree/`."},
  {"claim":"2.12","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:148-151 (`u.eq_ignore_ascii_case(&h.id)`), src/resources/cloud_instance/destroy.rs:52,58-59 (`if [ \"${{#h}}\" -ne 8 ]`, `echo \"destroyed $ids\"`), src/resources/package/cargo.rs:325,333 (`Some(v) => (v.to_string(), ...)`): matches PR_BODY.md:30-32."},
  {"claim":"2.13","verdict":"SURVIVED","evidence":"PR.stat:1-8 (`7 files changed, 135 insertions(+), 66 deletions(-)`), src/cli/check_marker.rs:1 (`//! #1187: surface cloud_instance volume-tag markers from a passing check.`), docs/roadmaps/roadmap.yaml:8220-8237 (`PMAT-1187` untouched): `#1187` is not in this PR."},
  {"claim":"2.14","verdict":"SURVIVED","evidence":"PR.stat:1-8 (`docs/roadmaps/roadmap.yaml | 54`, `7 files changed, 135 insertions(+), 66 deletions(-)`): matches PR_BODY.md:38."},
  {"claim":"2.15","verdict":"SURVIVED","evidence":"PR_BODY.md:38 (`4 commits (c33c01b8, 31be7b82, 23f79e35, 56ecdf0)`): `.git` history is absent from `tree/`."},
  {"claim":"2.16","verdict":"SURVIVED","evidence":"src/resources/tests_package_c.rs:264-273,283-286 (`Command::new(\"bash\").args([\"-c\", &sh]).output().unwrap()`, `assert!(!unpinned_cargo(\"\").contains(\"--version\"), \"unresolved must not pin\");`) and src/resources/package/cargo.rs:155 (`set -euo pipefail`): discards status/stderr, omits `set -euo pipefail`, and passes vacuously on empty stdout."},
  {"claim":"2.17","verdict":"SURVIVED","evidence":"src/resources/tests_package_c.rs:265,280 (`cargo() {{ [ \"$1\" = search ] && echo '{found}' || echo \"ARGS $*\"; }}`, `o.contains(\"--version =0.24.0 \")`) and src/resources/package/cargo.rs:336: verified in `bash` that outer-quoting `\"${_FJ_PIN:+--version =$_FJ_PIN}\"` passes single arg `--version =0.24.0` (or stray empty arg `\"\"`) while `echo \"ARGS $*\"` still satisfies both asserts."},
  {"claim":"2.18","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests_b4.rs:312,318 (`sir-0a1b2c3d\\ni-00b2c3d4 {KEY_A} {OWN_A} {bad}\\n`, `assert_eq!(count(&log, \"cancel-spot\"), 1`) and src/resources/cloud_instance/destroy.rs:52 (`if [ \"${{#h}}\" -ne 8 ]; then bad=1; continue; fi`): no test puts a valid `sir-` id after a malformed id, so mutating `continue` to `break` survives the entire `cloud_instance` test suite."},
  {"claim":"2.19","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:11,43-45 (`sirs=\"$({cmd})\" || {{ ... exit 1; }}`, `{spot}`, `aws ec2 terminate-instances`), docs/roadmaps/roadmap.yaml:8275-8277 (`title: 'destroy blocks terminate on a cancel-only lookup'`), PR_BODY.md:3 (`Closes #1185`): failed `lookup_sirs` still exits 1 before terminate."},
  {"claim":"2.20","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:40,47-56 (`set -eu`, `aws ec2 cancel-spot-instance-requests --region {region} --spot-instance-request-ids \"$r\" >/dev/null`, `if [ -n \"$bad\" ]; then echo 'spot-request-malformed' >&2`): unguarded `cancel-spot-instance-requests` failure under `set -eu` aborts the loop before subsequent tokens, line 55 reporting, and `{relookup}`."},
  {"claim":"2.21","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:58-59 (`echo \"destroyed $ids\"`, `[ -z \"$bad\" ] || exit 1`), ANGLE.diff:280 (`- assert!(!out.contains(\"destroyed\"), \"{bad}: {out}\");`), and PR_BODY.md:10: `destroyed <ids>` is printed before exiting 1, and removal of `assert!(!out.contains(\"destroyed\"))` is omitted from `GATE CHANGE`."},
  {"claim":"2.22","verdict":"SURVIVED","evidence":"src/core/pool_orphans_tests.rs:134-139 (`assert_eq!(s.unknown_runners.len(), 2);`) and src/core/pool_orphans.rs:140-141 (`.map(|r| (r.name.clone(), r.host_id.clone()))`): replacing `r.name.clone()` with `String::new()` or `r.host_id.clone()` preserves `u` for line 151 and passes all 21 `pool_orphans` tests; no test mixes exact-match and unknown runners."},
  {"claim":"2.23","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:8 (`//! It returns hosts to terminate only; runner deregistration is not part of it.`) and lines 130,138-154 (`Runner.busy` is not read; only `u.eq_ignore_ascii_case(&h.id)` protects unmatched runners): removing `SweepError::UnknownRunnerHost` allows scheduling old/idle hosts for termination even with busy unmatched runners, and line 8 is stale."},
  {"claim":"3.1","verdict":"SURVIVED","evidence":"src/resources/package/cargo.rs:34,276,325,333,336,348 (`let pin_arg = version.map_or(\" ${_FJ_PIN:+--version \\\"=$_FJ_PIN\\\"}\", |_| \"\");`, `matches!(c, '.' | '_' | '-' | '+' | '*' | '~' | '^')`): matches PR_BODY.md:5,32."},
  {"claim":"3.2","verdict":"SURVIVED","evidence":"PR_BODY.md:8,26 (`git diff 4eb1d3f HEAD --stat: 4 files, 5 insertions, 10 deletions`): `.git` history and PR #1206 are absent from `tree/`."},
  {"claim":"3.3","verdict":"SURVIVED","evidence":"src/core/pool_orphans_tests.rs:137-138 (`assert_eq!(s.terminate_hosts, [\"o1\", \"o2\"]);`, `assert_eq!(s.unknown_runners.len(), 2);`) and PR_BODY.md:10 (`lists the unknown runner`): line 138 asserts only `len() == 2` and never verifies any runner name or host id in `unknown_runners`."},
  {"claim":"3.4","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests_b4.rs:300-318 (`for bad in [\"sir-0a1b2c3d4e5f6a7b8\", ... \"xx-0a1b2c3d\"]`, `assert_eq!(count(&log, \"cancel-spot\"), 1, \"{bad}: {log}\");`): matches PR_BODY.md:10."},
  {"claim":"3.5","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:138-158 (`.filter(|r| !hosts.iter().any(|h| h.id == r.host_id))`, `&& !unknown_runners.iter().any(|(_, u)| u.eq_ignore_ascii_case(&h.id))`): matches PR_BODY.md:14."},
  {"claim":"3.6","verdict":"SURVIVED","evidence":"src/resources/package/cargo.rs:275-281,324-336,359 (`_FJ_PIN=\"$_FJ_LATEST\"`, `version.map_or(\" ${_FJ_PIN:+--version \\\"=$_FJ_PIN\\\"}\", |_| \"\")`): verified in both `bash` and `dash` (`argc=9` with `[--version] [=0.24.0]` when resolved, `argc=7` when empty)."},
  {"claim":"3.7","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:11,43-59 (`if [ \"${{#h}}\" -ne 8 ]; then bad=1; continue; fi`, `[ -z \"$bad\" ] || exit 1`): matches PR_BODY.md:16 on the non-failing `cancel-spot` path."},
  {"claim":"3.8","verdict":"REFUTED","evidence":"src/lib.rs:38 (`pub mod core;`), src/core/parser/mod.rs:30-31 (`#[cfg(test)] mod tests_cloud_instance_validate;`), and src/core/parser/tests_cloud_instance_validate.rs:30,108,119,136,145,154 (6 `#[test]` functions) alongside 133 `#[test]` functions in `src/resources/cloud_instance/*.rs`: `--lib cloud_instance` matches 139 tests (133 + 6), not 133, refuting the claim that `static #[test] counts match exactly (21, 133, 100)`."},
  {"claim":"3.9","verdict":"SURVIVED","evidence":"docs/roadmaps/roadmap.yaml:8238-8291 (`- id: PMAT-1186`, `- id: PMAT-1188`, `- id: PMAT-1185`): `yq . docs/roadmaps/roadmap.yaml` exits 0."},
  {"claim":"3.10","verdict":"SURVIVED","evidence":"PR_BODY.md:22 (`cargo clippy -j 4 --locked --lib --tests -- -D warnings: finished, no warnings`): `rustfmt --edition 2021 --check` exits 0 on all 6 modified `.rs` files, while `cargo clippy` has no log and was not executed."},
  {"claim":"3.11","verdict":"SURVIVED","evidence":"PR_BODY.md:23-24 (`Red before fix (from the earlier run on pull request #1206...)`, `Mutants (from the earlier run on pull request #1206...`): no PR #1206 logs exist, and statically each of the 5 named mutations violates a current test assert."},
  {"claim":"3.12","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:145-152 (`&& !unknown_runners.iter().any(|(_, u)| u.eq_ignore_ascii_case(&h.id))`) and src/core/pool_orphans_tests.rs:133-139: matches PR_BODY.md:30."},
  {"claim":"3.13","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:52,58-59 (`echo \"destroyed $ids\"`, `[ -z \"$bad\" ] || exit 1`) and src/resources/cloud_instance/tests_b4.rs:301 (`\"sir-0a1b2c3d4e5f6a7b8\"`): matches PR_BODY.md:31."},
  {"claim":"3.14","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests.rs:50-84 (`const STUB: &str`), src/resources/tests_package_c.rs:265 (`cargo() {{ ... }}`), PR.stat:1-8, docs/roadmaps/roadmap.yaml:8220-8291: matches PR_BODY.md:36."},
  {"claim":"3.15","verdict":"SURVIVED","evidence":"PR.stat:1,8 (`docs/roadmaps/roadmap.yaml | 54`, `7 files changed, 135 insertions(+), 66 deletions(-)`): matches PR_BODY.md:38."},
  {"claim":"3.16","verdict":"SURVIVED","evidence":"PR_BODY.md:38 (`4 commits (c33c01b8, 31be7b82, 23f79e35, 56ecdf0)`): `.git` is absent from `tree/`."},
  {"claim":"3.17","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:8 (`//! It returns hosts to terminate only; runner deregistration is not part of it.`) vs lines 5-6,45-51,128-129,155-158 (`Sweep::unknown_runners`): directly contradictory module doc comment."},
  {"claim":"3.18","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:43-51 (`/// What one sweep removes. The list keeps input order, without duplicates.` above `pub struct Sweep` with `terminate_hosts` and `unknown_runners`): stale doc comment describing a single list of removals."},
  {"claim":"3.19","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:138-152 (`.filter(|r| !hosts.iter().any(|h| h.id == r.host_id))`, `&& !unknown_runners.iter().any(|(_, u)| u.eq_ignore_ascii_case(&h.id))`) and src/core/pool_orphans_tests.rs:135-138 (`r(\"b\", \"I-0ABC\", true)` on `\"i-0abc\"`): case-variant live busy runner both protects its host and is emitted in `unknown_runners` identically to stale gone-host runners."},
  {"claim":"3.20","verdict":"SURVIVED","evidence":"src/core/pool_orphans_tests.rs:138 (`assert_eq!(s.unknown_runners.len(), 2);`) and src/core/pool_orphans.rs:141 (`.map(|r| (r.name.clone(), r.host_id.clone()))`): dropping runner names to `String::new()` or reversing `unknown_runners` preserves `u` for line 151 and `.len() == 2`, passing all 21 `pool_orphans` tests."},
  {"claim":"3.21","verdict":"SURVIVED","evidence":"src/core/pool_orphans.rs:120-126,149-151 and ANGLE.diff:178 (`for bad in [\"ip-10-0-0-5\", \"zzz\", \"I-0ABC\", \"i-0ab\"]`): removing `SweepError::UnknownRunnerHost` lets `sweep` schedule old/idle hosts for termination even when every runner is busy and unmatched."},
  {"claim":"3.22","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:25-27,40,52-56 (`set -eu`, `if [ \"${{#h}}\" -ne 8 ]; then bad=1; continue; fi`, `aws ec2 cancel-spot-instance-requests ... >/dev/null`, `if [ -n \"$bad\" ]; then echo 'spot-request-malformed' >&2`): a failing `cancel-spot-instance-requests` exits immediately under `set -eu`, dropping line 55 diagnostics, remaining cancellations, and `{relookup}`."},
  {"claim":"3.23","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:58-59 (`echo \"destroyed $ids\"`, `[ -z \"$bad\" ] || exit 1`), ANGLE.diff:280 (`- assert!(!out.contains(\"destroyed\"), \"{bad}: {out}\");`), PR_BODY.md:10: emits `destroyed $ids` before exiting 1 and omits the deleted assertion from `GATE CHANGE`."},
  {"claim":"3.24","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/destroy.rs:11,43-45,52 (`sirs=\"$({cmd})\" || {{ ... exit 1; }}`, `if [ \"${{#h}}\" -ne 8 ]`), docs/roadmaps/roadmap.yaml:8274-8277 (`title: 'destroy blocks terminate on a cancel-only lookup'`), PR_BODY.md:3 (`Closes #1185`), src/resources/cloud_instance/tests_b4.rs:301 (`\"sir-0a1b2c3d4e5f6a7b8\"`): `{spot}` still blocks terminate on lookup failure and 17-char `sir-` ids remain rejected."},
  {"claim":"3.25","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests_b4.rs:299,314-318 (`assert_eq!(count(&log, \"terminate-instances\"), 1`, `assert_eq!(count(&log, \"cancel-spot\"), 1`): never asserts which spot id was cancelled, never asserts both instance ids were passed to `terminate-instances`, and never checks call ordering."},
  {"claim":"3.26","verdict":"SURVIVED","evidence":"src/resources/tests_package_c.rs:264-273,283-286 (`Command::new(\"bash\").args([\"-c\", &sh]).output().unwrap()`, `assert!(!unpinned_cargo(\"\").contains(\"--version\"), \"unresolved must not pin\");`) and src/resources/package/cargo.rs:155,279-280 (`set -euo pipefail`, `_FJ_NC=1`, `_FJ_LATEST=unresolved`): omits `set -euo pipefail`, ignores exit status, and never verifies `ARGS ` or `KEY=bat-unresolved-` on the unresolved path."},
  {"claim":"3.27","verdict":"SURVIVED","evidence":"src/resources/tests_package_c.rs:265,280 (`cargo() {{ [ \"$1\" = search ] && echo '{found}' || echo \"ARGS $*\"; }}`, `o.contains(\"--version =0.24.0 \")`) and src/resources/package/cargo.rs:336: verified in `bash` that `echo \"ARGS $*\"` erases argv boundaries so outer-quoting `${_FJ_PIN:+--version \"=$_FJ_PIN\"}` breaks real `cargo install` while still passing the test."}
]
VERDICT: FAIL
text_only: yes
refuter: 2
reviewer_model: gemini-4-neon-high

=== REFUTER 3 ===
[
  {
    "claim": "1.1",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:34,276,325,336,359: `matches!(c, '.' | '_' | '-' | '+' | '*' | '~' | '^')`, `_FJ_PIN=\"$_FJ_LATEST\"`, `let pin_arg = version.map_or(\" ${_FJ_PIN:+--version \\\"=$_FJ_PIN\\\"}\", |_| \"\");`"
  },
  {
    "claim": "1.2",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:8: `Re-cut of pull request #1206 (closed): same code and tests, new commit messages, and the comment-only edits listed below.`; exported `tree/` has no `.git` directory or PR #1206 history"
  },
  {
    "claim": "1.3",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:55-77 and src/core/pool_orphans_tests.rs:137-138: `assert_eq!(s.terminate_hosts, [\"o1\", \"o2\"]);` and `assert_eq!(s.unknown_runners.len(), 2);` (claim explicitly notes length 2)"
  },
  {
    "claim": "1.4",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_b4.rs:300-318: 8 entries in `for bad in [\"sir-0a1b2c3d4e5f6a7b8\", ..., \"xx-0a1b2c3d\"]`, `assert_ne!(rc, 0, ...)`, `out.contains(\"not a whole spot request id\")`, `count(&log, \"terminate-instances\") == 1`, `count(&log, \"cancel-spot\") == 1`"
  },
  {
    "claim": "1.5",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:138-152: `.filter(|r| !hosts.iter().any(|h| h.id == r.host_id))` and `&& !unknown_runners.iter().any(|(_, u)| u.eq_ignore_ascii_case(&h.id))`"
  },
  {
    "claim": "1.6",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:276-281,325,336,359: `_FJ_PIN=\"$_FJ_LATEST\"`, `if [ -z \"$_FJ_LATEST\" ]; then ... _FJ_NC=1`, `let pin_arg = version.map_or(\" ${_FJ_PIN:+--version \\\"=$_FJ_PIN\\\"}\", |_| \"\");`"
  },
  {
    "claim": "1.7",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/destroy.rs:11,43-59: `lookup_sirs` runs at line 43 before `terminate-instances` (line 45), `if [ \"${{#h}}\" -ne 8 ]; then bad=1; continue; fi` at line 52, stderr report at line 55, `[ -z \"$bad\" ] || exit 1` at line 59"
  },
  {
    "claim": "1.8",
    "verdict": "SURVIVED",
    "evidence": "src/lib.rs:38 (`pub mod core;`), src/core/mod.rs:23 (`pub mod parser;`), src/core/parser/mod.rs:30-31 (`#[cfg(test)] mod tests_cloud_instance_validate;`), src/core/parser/tests_cloud_instance_validate.rs:30,108,119,136,145,154 (6 `#[test]` functions) + 133 `#[test]` functions in src/resources/cloud_instance/*.rs = 139 tests, not 133"
  },
  {
    "claim": "1.9",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans_tests.rs:28-249 (21 `#[test]` functions) and src/resources/mod.rs:86-94 (`tests_package` 37 + `tests_package_b` 30 + `tests_package_c` 15 + `tests_package_d` 13 + `tests_package_e` 5 = 100 `#[test]` functions)"
  },
  {
    "claim": "1.10",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:18,20-22: `no log attached, and no CI check at 56ecdf0 shows these results`; exported `tree/` has no `.git` commit `56ecdf0` and no test/clippy execution logs"
  },
  {
    "claim": "1.11",
    "verdict": "SURVIVED",
    "evidence": "docs/roadmaps/roadmap.yaml:8238-8291: `- id: PMAT-1186`, `- id: PMAT-1188`, `- id: PMAT-1185`; `yq . tree/docs/roadmaps/roadmap.yaml` exits 0"
  },
  {
    "claim": "1.12",
    "verdict": "SURVIVED",
    "evidence": "ANGLE.diff:80-86 (`+ pub unknown_runners: Vec<(String, String)>,`) and src/core/pool_orphans_tests.rs:138 (`assert_eq!(s.unknown_runners.len(), 2);`); pre-fix `Sweep` lacked `unknown_runners`, causing E0609"
  },
  {
    "claim": "1.13",
    "verdict": "SURVIVED",
    "evidence": "ANGLE.diff:313-314 (`- cargo install --force --locked --root \"$_STAGING\"{features_arg} {install_arg}`) and src/resources/tests_package_c.rs:279-282 (`o.contains(\"--version =0.24.0 \")`); without `pin_arg`, fake cargo omits `--version`"
  },
  {
    "claim": "1.14",
    "verdict": "SURVIVED",
    "evidence": "ANGLE.diff:220 (`echo 'ERROR: spot request lookup returned something that is not a whole spot request id; nothing was terminated' >&2; exit 1;`) and src/resources/cloud_instance/tests_b4.rs:314-317 (`assert_eq!(count(&log, \"terminate-instances\"), 1, \"{bad}: {log}\");`)"
  },
  {
    "claim": "1.15",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:149-151 + pool_orphans_tests.rs:137; src/resources/package/cargo.rs:276,336 + tests_package_c.rs:280,284; src/resources/cloud_instance/destroy.rs:52,59 + tests_b4.rs:314,318 — all 5 named mutants fail their corresponding assertions"
  },
  {
    "claim": "1.16",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:26: `Versus the reviewed #1206 head (4eb1d3f)`; exported `tree/` has no `.git` history or commit `4eb1d3f`"
  },
  {
    "claim": "1.17",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:148-151 (`!runners.iter().any(|r| r.host_id == h.id)` and `u.eq_ignore_ascii_case(&h.id)`) and ANGLE.diff:176-182 (deletion of `for bad in [\"ip-10-0-0-5\", \"zzz\", \"I-0ABC\", \"i-0ab\"]` test)"
  },
  {
    "claim": "1.18",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/destroy.rs:52,58-59 (`if [ \"${{#h}}\" -ne 8 ]; then bad=1; continue; fi`, `echo \"destroyed $ids\"`, `[ -z \"$bad\" ] || exit 1`) and src/resources/cloud_instance/tests_b4.rs:301 (`\"sir-0a1b2c3d4e5f6a7b8\"`)"
  },
  {
    "claim": "1.19",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:34,325,333,336,348: `is_safe_cargo_version` allows `^`, and `Some(v) => (v.to_string(), \"FORJAR_NO_CARGO_CACHE\", String::new())` keys `_CACHE_KEY` on raw `v` with empty `pin_arg`"
  },
  {
    "claim": "1.20",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests.rs:50-84 (`const STUB: &str`), src/resources/tests_package_c.rs:265 (`cargo() {{ ... }}`), src/cli/check_marker.rs:1 (`//! #1187: ...`) untouched in PR.stat:1-8"
  },
  {
    "claim": "1.21",
    "verdict": "SURVIVED",
    "evidence": "PR.stat:1,8: `docs/roadmaps/roadmap.yaml | 54 +++...` and `7 files changed, 135 insertions(+), 66 deletions(-)` (135 + 66 = 201)"
  },
  {
    "claim": "1.22",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:38: `4 commits (c33c01b8, 31be7b82, 23f79e35, 56ecdf0)`; exported `tree/` contains no `.git` directory"
  },
  {
    "claim": "1.23",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:8 (`//! It returns hosts to terminate only; runner deregistration is not part of it.`) and src/core/pool_orphans.rs:43 (`/// What one sweep removes. The list keeps input order, without duplicates.`) contradict lines 5-6, 48-50, and 128-129"
  },
  {
    "claim": "1.24",
    "verdict": "REFUTED",
    "evidence": "src/core/pool_orphans.rs:141,149-151 (`&& !unknown_runners.iter().any(|(_, u)| u.eq_ignore_ascii_case(&h.id))`) and src/core/pool_orphans_tests.rs:135-137 (`assert_eq!(s.terminate_hosts, [\"o1\", \"o2\"]);`): swapping `(r.name, r.host_id)` or corrupting `r.host_id` at line 141 puts `\"b\"`/garbage into the second tuple slot `(_, u)`, breaking the ASCII case-insensitive protection of `\"i-0abc\"` so `terminate_hosts` becomes `[\"o1\", \"i-0abc\", \"o2\"]` and fails line 137"
  },
  {
    "claim": "1.25",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/destroy.rs:11,43-45 (`sirs=\"$({cmd})\" || {{ ... exit 1; }}` runs via `{spot}` before `aws ec2 terminate-instances`) vs docs/roadmaps/roadmap.yaml:8277 (`title: 'destroy blocks terminate on a cancel-only lookup'`)"
  },
  {
    "claim": "1.26",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/destroy.rs:58-59 (`echo \"destroyed $ids\"` followed by `[ -z \"$bad\" ] || exit 1`) and ANGLE.diff:280 (`- assert!(!out.contains(\"destroyed\"), \"{bad}: {out}\");`)"
  },
  {
    "claim": "1.27",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_b4.rs:312-318 (`assert_eq!(count(&log, \"cancel-spot\"), 1, \"{bad}: {log}\");`) and src/resources/cloud_instance/tests.rs:50-84 (`STUB` exits 0 for any `cancel-spot` arg): inverting the check to cancel `{bad}` and skip `sir-0a1b2c3d` still passes this test"
  },
  {
    "claim": "1.28",
    "verdict": "SURVIVED",
    "evidence": "src/resources/tests_package_c.rs:264-273,283-286 (`Command::new(\"bash\").args([\"-c\", &sh]).output().unwrap()` discards `o.status` and `o.stderr`, omits `set -euo pipefail`, and `assert!(!unpinned_cargo(\"\").contains(\"--version\"))` passes on empty stdout)"
  },
  {
    "claim": "2.1",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:55-77 (`pub enum SweepError {` without `UnknownRunnerHost`) and src/core/pool_orphans_tests.rs:137-138 (`assert_eq!(s.terminate_hosts, [\"o1\", \"o2\"]);`, `assert_eq!(s.unknown_runners.len(), 2);`, explicitly caveated in claim)"
  },
  {
    "claim": "2.2",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_b4.rs:300-318: 8 malformed spellings paired with `sir-0a1b2c3d`, asserting non-zero `rc`, `\"not a whole spot request id\"`, 1 `terminate-instances`, and 1 `cancel-spot`, matching PR_BODY.md:10"
  },
  {
    "claim": "2.3",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:138-152: `.filter(|r| !hosts.iter().any(|h| h.id == r.host_id))` and `!unknown_runners.iter().any(|(_, u)| u.eq_ignore_ascii_case(&h.id))`"
  },
  {
    "claim": "2.4",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:276-281,336,359: `_FJ_PIN=\"$_FJ_LATEST\"`, `_FJ_NC=1` when empty, and `let pin_arg = version.map_or(\" ${_FJ_PIN:+--version \\\"=$_FJ_PIN\\\"}\", |_| \"\");`"
  },
  {
    "claim": "2.5",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/destroy.rs:11,43-59: `lookup_sirs` still exits 1 before `terminate-instances`; malformed check `if [ \"${{#h}}\" -ne 8 ]; then bad=1; continue; fi` moved to cancel loop, followed by stderr report, `{relookup}`, and `[ -z \"$bad\" ] || exit 1`"
  },
  {
    "claim": "2.6",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/*.rs (133 `#[test]` across 12 files) + src/core/parser/mod.rs:30-31 (`#[cfg(test)] mod tests_cloud_instance_validate;`) & src/core/parser/tests_cloud_instance_validate.rs:30,108,119,136,145,154 (6 `#[test]`) via src/lib.rs:38 = 139 `--lib cloud_instance` tests, not 133"
  },
  {
    "claim": "2.7",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans_tests.rs (21 `#[test]`) and src/resources/tests_package*.rs (37+30+15+13+5=100 `#[test]`); counts match, while PR_BODY.md:18 states `no log attached, and no CI check at 56ecdf0 shows these results`"
  },
  {
    "claim": "2.8",
    "verdict": "SURVIVED",
    "evidence": "Cargo.toml:1-282 and docs/roadmaps/roadmap.yaml:8238-8291: `cargo fmt --manifest-path tree/Cargo.toml --all -- --check` exits 0 and `yq . tree/docs/roadmaps/roadmap.yaml` exits 0"
  },
  {
    "claim": "2.9",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:18,22: `cargo clippy -j 4 --locked --lib --tests -- -D warnings: finished, no warnings` has no attached log or CI check"
  },
  {
    "claim": "2.10",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:23-24: `from the earlier run on pull request #1206 ... not rerun on this cut`; no PR #1206 log or git history is in the workspace"
  },
  {
    "claim": "2.11",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:8,26: `4eb1d3f` and git history are absent from exported `tree/`"
  },
  {
    "claim": "2.12",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:148-151 (`u.eq_ignore_ascii_case(&h.id)`), src/resources/cloud_instance/destroy.rs:52,58-59 (`\"${{#h}}\" -ne 8`, `echo \"destroyed $ids\"`), src/resources/package/cargo.rs:325,333 (`Some(v) => (v.to_string(), ...)`)"
  },
  {
    "claim": "2.13",
    "verdict": "SURVIVED",
    "evidence": "PR.stat:1-8 and ANGLE.diff:1-359 do not touch src/cli/check_marker.rs:1 (`//! #1187: ...`) or docs/roadmaps/roadmap.yaml:8220-8237 (`PMAT-1187`)"
  },
  {
    "claim": "2.14",
    "verdict": "SURVIVED",
    "evidence": "PR.stat:1,8: `docs/roadmaps/roadmap.yaml | 54 +++...` and `7 files changed, 135 insertions(+), 66 deletions(-)` (201 lines total)"
  },
  {
    "claim": "2.15",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:38: `4 commits (c33c01b8, 31be7b82, 23f79e35, 56ecdf0)`; exported `tree/` has no `.git` directory"
  },
  {
    "claim": "2.16",
    "verdict": "SURVIVED",
    "evidence": "src/resources/tests_package_c.rs:264-273,283-286 discards `o.status` and `o.stderr`, omits `set -euo pipefail` (present at src/resources/package/cargo.rs:155), and `assert!(!unpinned_cargo(\"\").contains(\"--version\"))` passes vacuously on empty stdout"
  },
  {
    "claim": "2.17",
    "verdict": "SURVIVED",
    "evidence": "src/resources/tests_package_c.rs:265,280 (`echo \"ARGS $*\"` and `o.contains(\"--version =0.24.0 \")`): bash verification confirms outer-quoting `\"${_FJ_PIN:+--version =$_FJ_PIN}\"` passes single arg `[--version =0.24.0]` / empty arg `[]` while still printing `ARGS ... --version =0.24.0 bat` and `ARGS ...  bat`"
  },
  {
    "claim": "2.18",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_b4.rs:312,318 (`sir-0a1b2c3d` always precedes `{bad}`, only checks `count(&log, \"cancel-spot\") == 1`) and src/resources/cloud_instance/destroy.rs:52 (`continue`): no other test supplies a malformed spot id, so `continue` -> `break` passes all tests"
  },
  {
    "claim": "2.19",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/destroy.rs:11,43-45 (`{spot}` runs `sirs=\"$({cmd})\" || { ... exit 1; }` before `aws ec2 terminate-instances`) vs docs/roadmaps/roadmap.yaml:8275-8277 (`id: PMAT-1185`, `title: 'destroy blocks terminate on a cancel-only lookup'`) and PR_BODY.md:3 (`Closes #1185`)"
  },
  {
    "claim": "2.20",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/destroy.rs:40,47-56 (`set -eu`, unguarded `aws ec2 cancel-spot-instance-requests ...` at line 53 before `if [ -n \"$bad\" ]` at line 55 and `{relookup}` at line 56): a failing cancel aborts the script before subsequent tokens, diagnostics, and relookup"
  },
  {
    "claim": "2.21",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/destroy.rs:58-59 (`echo \"destroyed $ids\"` then `[ -z \"$bad\" ] || exit 1`), ANGLE.diff:280 (`- assert!(!out.contains(\"destroyed\"), \"{bad}: {out}\");`), and PR_BODY.md:10 (GATE CHANGE does not mention removing that assertion)"
  },
  {
    "claim": "2.22",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans_tests.rs:134-139 (`assert_eq!(s.unknown_runners.len(), 2);`, both runners unmatched) and src/core/pool_orphans.rs:141 (`.map(|r| (r.name.clone(), r.host_id.clone()))`): replacing `r.name.clone()` with `String::new()` or `r.host_id.clone()` preserves `(_, u)` for line 151 and passes all 21 tests; no test mixes exact-match and unknown runners"
  },
  {
    "claim": "2.23",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:8 (`//! It returns hosts to terminate only; runner deregistration is not part of it.`) and src/core/pool_orphans.rs:138-154 (only exact match or `u.eq_ignore_ascii_case(&h.id)` protects hosts; no fail-closed guard on busy or non-case-mismatched runners)"
  },
  {
    "claim": "3.1",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:34,276,325,333,336,348: `is_safe_cargo_version` permits `^`, `Some(v)` sets `pin_arg = \"\"` and keys `_CACHE_KEY` on raw `v`, while `None` passes `--version \"=$_FJ_PIN\"`"
  },
  {
    "claim": "3.2",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:8,26 (`4eb1d3f`, PR #1206): exported `tree/` has no `.git` directory or PR #1206 history"
  },
  {
    "claim": "3.3",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans_tests.rs:137-138 (`assert_eq!(s.terminate_hosts, [\"o1\", \"o2\"]);`, `assert_eq!(s.unknown_runners.len(), 2);`) vs PR_BODY.md:10 (`lists the unknown runner`): line 138 asserts only length 2, never checking any runner name or host id"
  },
  {
    "claim": "3.4",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_b4.rs:300-318: 8 malformed spellings on 2-instance fixture asserting non-zero exit, `\"not a whole spot request id\"`, 1 `terminate-instances`, and 1 `cancel-spot` without checking which id was cancelled"
  },
  {
    "claim": "3.5",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:138-158: unmatched runners go to `unknown_runners` rather than returning `Err`, and among unknown runners only `u.eq_ignore_ascii_case(&h.id)` excludes a listed host from `terminate_hosts`"
  },
  {
    "claim": "3.6",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:275-281,324-336,359: `_FJ_PIN=\"$_FJ_LATEST\"`, `pin_arg = version.map_or(\" ${_FJ_PIN:+--version \\\"=$_FJ_PIN\\\"}\", |_| \"\")`; verified in both `bash` and `dash` to emit `[--version] [=0.24.0]` when set and nothing when empty"
  },
  {
    "claim": "3.7",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/destroy.rs:11,43-59: `lookup_sirs` still exits 1 before terminate; spot-id check moved into post-terminate cancel loop (`bad=1; continue`), reporting on stderr and exiting non-zero after `{relookup}`"
  },
  {
    "claim": "3.8",
    "verdict": "REFUTED",
    "evidence": "src/lib.rs:38 (`pub mod core;`), src/core/mod.rs:23 (`pub mod parser;`), src/core/parser/mod.rs:30-31 (`#[cfg(test)] mod tests_cloud_instance_validate;`), and src/core/parser/tests_cloud_instance_validate.rs:30,108,119,136,145,154 (6 `#[test]` functions) plus 133 `#[test]` functions in src/resources/cloud_instance/*.rs = 139 tests matching `--lib cloud_instance`, refuting the claim that `static #[test] counts match exactly (21, 133, 100)`"
  },
  {
    "claim": "3.9",
    "verdict": "SURVIVED",
    "evidence": "docs/roadmaps/roadmap.yaml:8238-8291 (`- id: PMAT-1186`, `- id: PMAT-1188`, `- id: PMAT-1185`); verified directly that `yq . tree/docs/roadmaps/roadmap.yaml` exits 0"
  },
  {
    "claim": "3.10",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:18,22: `cargo clippy -j 4 --locked --lib --tests -- -D warnings` has no attached log or CI check and was not executed"
  },
  {
    "claim": "3.11",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:23-24: red-before-fix and 5-of-5 mutant run from PR #1206 have no attached logs or git history in the workspace"
  },
  {
    "claim": "3.12",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:145-152 (`u.eq_ignore_ascii_case(&h.id)`) and src/core/pool_orphans_tests.rs:133-139 (no test for `i-0ab` vs `i-0abc`)"
  },
  {
    "claim": "3.13",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/destroy.rs:52,58-59 (`if [ \"${{#h}}\" -ne 8 ]`, `echo \"destroyed $ids\"`, `[ -z \"$bad\" ] || exit 1`) and src/resources/cloud_instance/tests_b4.rs:301 (`\"sir-0a1b2c3d4e5f6a7b8\"`)"
  },
  {
    "claim": "3.14",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests.rs:50-84 (`const STUB`), src/resources/tests_package_c.rs:265 (`cargo() {{ ... }}`), PR.stat:1-8 (does not touch `#1187` / `src/cli/check_marker.rs`)"
  },
  {
    "claim": "3.15",
    "verdict": "SURVIVED",
    "evidence": "PR.stat:1,8 (`docs/roadmaps/roadmap.yaml | 54 +++...`, `7 files changed, 135 insertions(+), 66 deletions(-)`) and docs/roadmaps/roadmap.yaml:8238-8291 (3 rows, 54 lines)"
  },
  {
    "claim": "3.16",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:38 (`4 commits (c33c01b8, 31be7b82, 23f79e35, 56ecdf0)`); exported `tree/` has no `.git` directory"
  },
  {
    "claim": "3.17",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:8 (`//! It returns hosts to terminate only; runner deregistration is not part of it.`) contradicts lines 5-6, 45-51, 128-129, and 155-158"
  },
  {
    "claim": "3.18",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:43 (`/// What one sweep removes. The list keeps input order, without duplicates.`) contradicts `pub struct Sweep` at lines 45-51 having two vectors (`terminate_hosts` and `unknown_runners`) where `unknown_runners` are not removed"
  },
  {
    "claim": "3.19",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:138-152 and src/core/pool_orphans_tests.rs:135-138: case-variant busy runner `r(\"b\", \"I-0ABC\", true)` protects host `\"i-0abc\"` via line 151 while being emitted in `unknown_runners` identically to stale `r(\"ghost\", \"gone\", false)`"
  },
  {
    "claim": "3.20",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans_tests.rs:138 (`assert_eq!(s.unknown_runners.len(), 2);`) and src/core/pool_orphans.rs:141: dropping `r.name` to `String::new()` or reversing `unknown_runners` preserves `(_, u)` for line 151 and passes all 21 `pool_orphans` tests"
  },
  {
    "claim": "3.21",
    "verdict": "SURVIVED",
    "evidence": "src/core/pool_orphans.rs:120-126,143-154 and ANGLE.diff:178: removing `SweepError::UnknownRunnerHost` allows `sweep` to return `Ok` and schedule idle/old hosts for termination even when all busy runners fail to match due to non-case ID differences"
  },
  {
    "claim": "3.22",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/destroy.rs:25-27,40,52-56: under `set -eu`, an unguarded non-zero `cancel-spot-instance-requests` on line 53 aborts immediately, dropping line 55's `spot-request-malformed` diagnostic, remaining cancellations, and line 56's `{relookup}`"
  },
  {
    "claim": "3.23",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/destroy.rs:58-59 (`echo \"destroyed $ids\"` then `[ -z \"$bad\" ] || exit 1`), ANGLE.diff:280 (`- assert!(!out.contains(\"destroyed\"), \"{bad}: {out}\");`), and PR_BODY.md:10 (GATE CHANGE omits that removed assertion)"
  },
  {
    "claim": "3.24",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/destroy.rs:11,43-45,52, docs/roadmaps/roadmap.yaml:8274-8277, PR_BODY.md:3, and src/resources/cloud_instance/tests_b4.rs:301: `{spot}` still exits 1 before terminate on lookup failure, and `\"${{#h}}\" -ne 8` rejects 17-char `sir-` IDs"
  },
  {
    "claim": "3.25",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_b4.rs:299-320: only checks `rc != 0`, `out.contains(\"not a whole spot request id\")`, `count(&log, \"terminate-instances\") == 1`, and `count(&log, \"cancel-spot\") == 1`, never checking cancelled id, terminated instance ids, or call order"
  },
  {
    "claim": "3.26",
    "verdict": "SURVIVED",
    "evidence": "src/resources/tests_package_c.rs:264-273,283-286: omits `set -euo pipefail` (present at src/resources/package/cargo.rs:155), ignores `o.status.success()`, and line 284 never checks `ARGS `, `_FJ_NC=1`, or `KEY=bat-unresolved-`, passing vacuously on empty stdout"
  },
  {
    "claim": "3.27",
    "verdict": "SURVIVED",
    "evidence": "src/resources/tests_package_c.rs:265,280 (`echo \"ARGS $*\"`, `o.contains(\"--version =0.24.0 \")`): bash verification confirms outer-quoting `\"${_FJ_PIN:+--version \"=$_FJ_PIN\"}\"` collapses args into `[--version =0.24.0]` (or `[]` when unset) yet still passes both assertions"
  }
]
VERDICT: FAIL
text_only: yes
refuter: 3
reviewer_model: gemini-4-neon-high
