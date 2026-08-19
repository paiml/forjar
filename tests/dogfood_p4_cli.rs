//! Refs #208 partition P4 — process-level regressions for the artifact
//! surfaces (build, image, oci-pack, dist, plugin, template).
//!
//! These run the real binary and check exit codes and stdout the way the
//! dogfood run did (`cmd > out 2>&1; rc=$?`), because that is the only level
//! at which "exit 0 while nothing happened" is visible. Every assertion here
//! fails against the published 1.12.3.

use std::path::Path;
use std::process::{Command, Output};

fn forjar(dir: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_forjar"));
    c.current_dir(dir);
    c
}

fn run(dir: &Path, args: &[&str]) -> Output {
    forjar(dir).args(args).output().expect("run forjar")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

const CONFIG: &str = r#"version: '1.0'
name: p4
params: {}
machines:
  local:
    hostname: sandbox-local
    addr: 127.0.0.1
    user: nobody
    arch: x86_64
resources:
  hello:
    type: file
    machine: local
    path: SANDBOX/hello.txt
    content: hi
  myimage:
    type: image
    machine: local
    tag: sandbox/test:0.1
    path: SANDBOX/hello.txt
"#;

const DIST_CONFIG: &str = r#"version: '1.0'
name: p4dist
params: {}
machines:
  local:
    hostname: sandbox-local
    addr: 127.0.0.1
    user: nobody
    arch: x86_64
resources: {}
dist:
  source: github_release
  repo: example/exampletool
  binary: exampletool
  description: Example tool
  homepage: https://example.invalid
"#;

/// Resource paths must be absolute, and the sandbox is a fresh temp dir, so
/// `SANDBOX` in the fixtures is rewritten to that dir. Nothing here touches a
/// path outside it.
fn sandbox(config: &str) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let body = config.replace("SANDBOX", &d.path().display().to_string());
    std::fs::write(d.path().join("forjar.yaml"), body).unwrap();
    d
}

// ── #212: build --json must emit JSON ─────────────────────────────────

#[test]
fn build_json_is_parseable() {
    let d = sandbox(CONFIG);
    let out = run(
        d.path(),
        &[
            "build",
            "-f",
            "forjar.yaml",
            "--resource",
            "myimage",
            "--json",
        ],
    );
    assert!(out.status.success(), "build failed: {}", stderr(&out));
    let text = stdout(&out);
    // RED on 1.12.3: first line was "Warning: --json is not yet implemented
    // for build output. Flag ignored." followed by the human build log.
    let doc: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("build --json did not emit JSON: {e}\n---\n{text}"));
    assert_eq!(doc["layout_exists"], serde_json::Value::Bool(true));
    assert!(doc["tag"].is_string(), "no tag in manifest: {text}");
}

#[test]
fn build_json_with_load_is_refused_rather_than_interleaved() {
    let d = sandbox(CONFIG);
    let out = run(
        d.path(),
        &[
            "build",
            "-f",
            "forjar.yaml",
            "--resource",
            "myimage",
            "--json",
            "--load",
        ],
    );
    assert!(!out.status.success(), "should refuse: {}", stdout(&out));
    assert!(stderr(&out).contains("--load"), "{}", stderr(&out));
}

// Non-regression: a real build still builds when --json is absent.
#[test]
fn build_without_json_still_writes_a_layout() {
    let d = sandbox(CONFIG);
    let out = run(
        d.path(),
        &["build", "-f", "forjar.yaml", "--resource", "myimage"],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(d.path().join("state/images/myimage/index.json").is_file());
}

// ── #210: oci-pack must write what it reports ─────────────────────────

#[test]
fn oci_pack_creates_the_output_directory() {
    let d = sandbox(CONFIG);
    std::fs::create_dir_all(d.path().join("src")).unwrap();
    std::fs::write(d.path().join("src/a.txt"), b"x").unwrap();

    let out = run(d.path(), &["oci-pack", "src", "--tag", "t:1"]);
    assert!(out.status.success(), "{}", stderr(&out));
    // RED on 1.12.3: rc=0, and ./oci-output did not exist.
    assert!(
        d.path().join("oci-output/index.json").is_file(),
        "no layout written; stdout was:\n{}",
        stdout(&out)
    );
}

#[test]
fn oci_pack_json_describes_a_layout_that_exists() {
    let d = sandbox(CONFIG);
    std::fs::create_dir_all(d.path().join("src")).unwrap();
    std::fs::write(d.path().join("src/a.txt"), b"x").unwrap();

    let out = run(
        d.path(),
        &[
            "oci-pack", "src", "--tag", "t:3", "--output", "jout", "--json",
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    let doc: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert!(doc["manifest"]["layers"].is_array(), "{text}");
    // RED on 1.12.3: this manifest was printed for a directory that was never
    // created.
    assert!(d.path().join("jout/index.json").is_file(), "{text}");
}

// ── #213: oci-pack must name the right cause ──────────────────────────

#[test]
fn oci_pack_file_argument_is_not_reported_as_missing() {
    let d = sandbox(CONFIG);
    std::fs::create_dir_all(d.path().join("src")).unwrap();
    std::fs::write(d.path().join("src/a.txt"), b"x").unwrap();

    let out = run(d.path(), &["oci-pack", "src/a.txt", "--tag", "t:1"]);
    assert!(!out.status.success());
    let msg = format!("{}{}", stdout(&out), stderr(&out));
    assert!(!msg.contains("does not exist"), "{msg}");
    assert!(msg.contains("not a directory"), "{msg}");
}

// ── #210: plugin run must not report Converged from a stub ────────────

/// Only meaningful WITHOUT `wasm-runtime`: it asserts that a build lacking the
/// runtime refuses rather than reporting `Converged` from a stub.
///
/// It carried no cfg guard, so `cargo test --all-features` — which the release
/// dogfood gate runs — enabled `wasm-runtime`, the refusal never happened, and
/// the test failed parsing an empty stdout. A test that only holds in one
/// feature configuration has to say so, or it reports a false failure in every
/// other one.
#[cfg(not(feature = "wasm-runtime"))]
#[test]
fn plugin_run_on_a_stub_runtime_refuses() {
    let d = sandbox(CONFIG);
    assert!(run(d.path(), &["plugin", "init", "good"]).status.success());

    let out = run(
        d.path(),
        &["plugin", "run", "good", "--operation", "apply", "--json"],
    );
    // RED on 1.12.3: rc=0 with {"success":true,"status":"Converged"}.
    assert!(!out.status.success(), "stub run reported success");
    let text = stdout(&out);
    let doc: serde_json::Value =
        serde_json::from_str(text.trim()).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert_eq!(doc["success"], serde_json::Value::Bool(false));
    assert_eq!(doc["status"], "Unsupported");
    assert!(
        doc["message"]
            .as_str()
            .unwrap_or_default()
            .contains("wasm-runtime"),
        "the missing feature must be named: {text}"
    );
}

// ── #211/#213: plugin init --output and the empty name ────────────────

#[test]
fn plugin_init_output_is_the_plugin_directory() {
    let d = sandbox(CONFIG);
    let out = run(d.path(), &["plugin", "init", "b", "--output", "custom"]);
    assert!(out.status.success(), "{}", stderr(&out));
    // RED on 1.12.3: this landed at custom/b/plugin.yaml.
    assert!(d.path().join("custom/plugin.yaml").is_file());
    assert!(!d.path().join("custom/b").exists());
}

#[test]
fn plugin_init_default_still_nests_under_plugins() {
    let d = sandbox(CONFIG);
    assert!(run(d.path(), &["plugin", "init", "a"]).status.success());
    assert!(d.path().join("plugins/a/plugin.yaml").is_file());
    // …and `plugin list` can see it, which is the property the empty name broke.
    let listed = stdout(&run(d.path(), &["plugin", "list"]));
    assert!(listed.contains('a'), "plugin list saw nothing: {listed}");
}

#[test]
fn plugin_init_empty_name_is_refused() {
    let d = sandbox(CONFIG);
    let out = run(d.path(), &["plugin", "init", ""]);
    assert!(!out.status.success(), "empty name accepted");
    assert!(!d.path().join("plugins/plugin.yaml").exists());
}

// ── #212 tail: image --help must not name a flag that does not exist ──

#[test]
fn image_help_does_not_reference_a_nonexistent_iso_flag() {
    let d = sandbox(CONFIG);
    let help = stdout(&run(d.path(), &["image", "--help"]));
    assert!(help.contains("--base"), "no --base in help: {help}");
    // RED on 1.12.3: "--base <BASE> Path to base Ubuntu ISO (required for --iso)"
    // while `image --iso` exits 2 "unexpected argument".
    assert!(
        !help.contains("--iso"),
        "help still advertises --iso, which the parser rejects:\n{help}"
    );
    let out = run(d.path(), &["image", "-m", "local", "--iso"]);
    assert!(!out.status.success(), "--iso unexpectedly exists now");
}

// ── #212: image --user-data writes YAML a parser can read ─────────────

#[test]
fn image_user_data_file_is_valid_yaml() {
    let d = sandbox(CONFIG);
    let out = run(
        d.path(),
        &["image", "-m", "local", "--user-data", "-o", "ud.yaml"],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let body = std::fs::read_to_string(d.path().join("ud.yaml")).unwrap();
    serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&body)
        .unwrap_or_else(|e| panic!("ud.yaml is not YAML: {e}\n---\n{body}"));
}

// ── #211: dist -o must reach every generator ──────────────────────────

#[test]
fn dist_single_artifact_honours_output() {
    let d = sandbox(DIST_CONFIG);
    for (flag, name) in [
        ("--rpm", "MYRPM"),
        ("--binstall", "MYBIN"),
        ("--github-action", "MYACT"),
        ("--installer", "MYINST"),
    ] {
        let out = run(d.path(), &["dist", flag, "-o", name]);
        assert!(out.status.success(), "{flag}: {}", stderr(&out));
        // RED on 1.12.3: only --installer wrote to -o; the rest went to dist/.
        assert!(
            d.path().join(name).is_file(),
            "{flag} ignored -o (stdout: {})",
            stdout(&out)
        );
    }
}

#[test]
fn dist_all_treats_output_as_a_directory() {
    let d = sandbox(DIST_CONFIG);
    std::fs::write(
        d.path().join("SHA256SUMS"),
        format!(
            "{}  exampletool-v1.0.0-x86_64-unknown-linux-gnu.tar.gz\n",
            "0".repeat(64)
        ),
    )
    .unwrap();
    std::fs::create_dir_all(d.path().join("odir")).unwrap();

    let out = run(
        d.path(),
        &[
            "dist",
            "--all",
            "--version",
            "v1.0.0",
            "--checksums-file",
            "SHA256SUMS",
            "-o",
            "odir",
        ],
    );
    // RED on 1.12.3: rc=1 "write ./odir: Is a directory (os error 21)".
    assert!(out.status.success(), "{}", stderr(&out));
    for f in [
        "install.sh",
        "homebrew.rb",
        "binstall.toml",
        "flake.nix",
        "action.yml",
        "exampletool.spec",
    ] {
        assert!(d.path().join("odir").join(f).is_file(), "missing {f}");
    }
    assert!(d.path().join("odir/debian").is_dir());
    assert!(!d.path().join("dist").exists(), "artifacts leaked to dist/");
}

// ── #213: template -V must be KEY=VALUE ───────────────────────────────

#[test]
fn template_var_without_equals_is_refused_at_parse_time() {
    let d = sandbox(CONFIG);
    let out = run(d.path(), &["template", "forjar.yaml", "-V", "novaluehere"]);
    // RED on 1.12.3: rc=0, and the value was silently discarded.
    assert!(!out.status.success(), "accepted a --var with no '='");
    assert!(stderr(&out).contains("KEY=VALUE"), "{}", stderr(&out));
}

#[test]
fn template_expands_a_config_instead_of_echoing_it() {
    let d = tempfile::tempdir().unwrap();
    let body = CONFIG
        .replace(
            "path: SANDBOX/hello.txt",
            "path: \"{{params.root}}/hello.txt\"",
        )
        .replace("params: {}", "params:\n  root: /srv/app");
    std::fs::write(d.path().join("forjar.yaml"), &body).unwrap();

    let out = run(d.path(), &["template", "forjar.yaml"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    // RED on 1.12.3: output was the source file byte-for-byte.
    assert!(!text.contains("{{params."), "left unexpanded:\n{text}");
    assert!(text.contains("/srv/app/hello.txt"), "{text}");
}
