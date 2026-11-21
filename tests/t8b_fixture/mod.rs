//! Shared fixture for the T8b falsifiers (index redirect, curl shim, ledgers).
#![allow(dead_code)]
use super::fx::*;

pub(crate) const SHA2: &str = "0123456789abcdef0123456789abcdef01234567";
pub(crate) const TOML2: &str = "[dependencies]\nforjar = \"0.0\"\n";

pub(crate) fn lock_of(v: &str) -> String {
    format!("[[package]]\nname = \"forjar\"\nversion = \"{v}\"\n")
}

/// Run the gate with the index redirected to a document (or to a path that does
/// not exist when `index` is None). `marked` sets the test marker the script
/// requires before it honours any redirect.
pub(crate) fn run_env(fx: &Fixture, index: Option<&str>, marked: bool) -> Run {
    run_env_seen(fx, index, marked).0
}

/// As `run_env`, and whether the redirected curl was ever invoked.
pub(crate) fn run_env_seen(fx: &Fixture, index: Option<&str>, marked: bool) -> (Run, bool) {
    run_full(fx, index, marked, None)
}

/// Every jq the gate runs whose program contains `jq_fail` exits 5 (a jq that
/// cannot answer); all others are the real jq. When `serve_index` is true, a
/// `curl` shim on PATH rewrites crates.io requests to the fixture's local file.
pub(crate) fn run_full(
    fx: &Fixture,
    index: Option<&str>,
    serve_index: bool,
    jq_fail: Option<&str>,
) -> (Run, bool) {
    run_full_opts(fx, index, serve_index, jq_fail, None, None)
}

pub(crate) fn run_full_opts(
    fx: &Fixture,
    index: Option<&str>,
    serve_index: bool,
    jq_fail: Option<&str>,
    curl_home: Option<&std::path::Path>,
    curl_exit: Option<i32>,
) -> (Run, bool) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("forjar.json");
    if let Some(body) = index {
        std::fs::write(&path, body).expect("write index");
    }
    let seen = dir.path().join("seen");
    let stub = dir.path().join("curl-stub.sh");
    std::fs::write(
        &stub,
        format!("#!/bin/sh\ntouch {}\nexec curl \"$@\"\n", seen.display()),
    )
    .expect("stub");
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    let shim = dir.path().join("shim");
    std::fs::create_dir_all(&shim).expect("shim dir");
    if serve_index {
        std::fs::write(
            shim.join("curl"),
            "#!/bin/bash\nset -euo pipefail\nif [ -n \"${CURL_SHIM_EXIT:-}\" ]; then\n  cat \"$CURL_SHIM_INDEX\"\n  exit \"$CURL_SHIM_EXIT\"\nfi\nargs=()\nfor a in \"$@\"; do\n  if [ \"$a\" = \"https://crates.io/api/v1/crates/forjar\" ]; then\n    args+=(\"file://${CURL_SHIM_INDEX}\")\n  else\n    args+=(\"$a\")\n  fi\ndone\nPATH=\"$ORIG_PATH\" exec curl \"${args[@]}\"\n",
        )
        .expect("curl shim");
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(shim.join("curl"), std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
    }
    if !serve_index {
        // No live network, ever: this curl fails (exit 6) for the canonical crates.io URL
        // and serves the forged index for ANY other URL, so a gate that honoured an
        // exported CURL / CRATES_IO_API would go green and the test would catch it.
        std::fs::write(
            shim.join("curl"),
            format!(
                "#!/bin/sh\nfor a in \"$@\"; do\n  [ \"$a\" = \"https://crates.io/api/v1/crates/forjar\" ] && exit 6\ndone\ncat {}\nexit 0\n",
                path.display()
            ),
        )
        .expect("failing curl shim");
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(shim.join("curl"), std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
    }
    if jq_fail.is_some() {
        std::fs::write(
            shim.join("jq"),
            "#!/bin/bash\nfor a in \"$@\"; do case \"$a\" in *\"$JQ_FAIL_MARK\"*) exit 5;; esac; done\nPATH=\"$ORIG_PATH\" exec jq \"$@\"\n",
        )
        .expect("jq shim");
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(shim.join("jq"), std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
    }
    let orig = std::env::var("PATH").unwrap_or_default();
    let mut cmd = std::process::Command::new("bash");
    cmd.arg(fx.root.join("scripts/dogfood/tagged.sh"))
        .current_dir(&fx.root)
        .env("GH", &fx.gh)
        .env("DOGFOOD_NOW", (fx.cut + AN_HOUR).to_string())
        .env_remove("FORJAR_GATE_T_TEST")
        .env_remove("CURL_OVERRIDE")
        .env_remove("CRATES_IO_API_OVERRIDE")
        .env_remove("CURL_SHIM_EXIT")
        .env_remove("CURL_SHIM_INDEX");
    {
        cmd.env("PATH", format!("{}:{orig}", shim.display()))
            .env("ORIG_PATH", &orig);
    }
    if let Some(m) = jq_fail {
        cmd.env("JQ_FAIL_MARK", m);
    }
    if let Some(home) = curl_home {
        cmd.env("CURL_HOME", home);
    }
    let url = format!("file://{}", path.display());
    if serve_index {
        cmd.env("CURL_SHIM_INDEX", &path);
        if let Some(rc) = curl_exit {
            cmd.env("CURL_SHIM_EXIT", rc.to_string());
        }
    } else {
        // the names an operator or a CI job might plausibly export
        cmd.env("FORJAR_GATE_T_TEST", "1")
            .env("CRATES_IO_API", &url)
            .env("CRATES_IO_API_OVERRIDE", &url)
            .env("CURL", &stub)
            .env("CURL_OVERRIDE", &stub);
    }
    let out = cmd.output().expect("bash must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (
        Run {
            code: out.status.code().unwrap_or(-1),
            text,
        },
        seen.exists(),
    )
}

pub(crate) fn run_index(fx: &Fixture, index: Option<&str>) -> Run {
    run_env(fx, index, true)
}

pub(crate) fn unpublished_fx(crates_io: &'static str, lock: &str) -> Fixture {
    let mut fx = fixture(Case {
        cookbook_floor: FLOOR,
        cookbook: SHA2,
        crates_io,
        ..Case::default()
    });
    fx.cookbook_files(TOML2, &lock_of(lock));
    fx
}

/// An index document for crate `forjar` from (num, yanked) pairs.
pub(crate) fn idx(vs: &[(&str, bool)]) -> String {
    let items: Vec<String> = vs
        .iter()
        .map(|(n, y)| format!(r#"{{"num":"{n}","yanked":{y}}}"#))
        .collect();
    format!(
        r#"{{"crate":{{"id":"forjar"}},"versions":[{}]}}"#,
        items.join(",")
    )
}

/// Run the predecessor selection directly against an index with MANY versions,
/// which a fixture whose only tag is v0.0.1 cannot hold.
pub(crate) fn run_cratesio_below(ver: &str, index: &str) -> (i32, String) {
    use std::io::Write;
    let lib = format!(
        "{}/scripts/dogfood/lib/cratesio.sh",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut child = std::process::Command::new("bash")
        .args([
            "-c",
            &format!("fail() {{ echo \"$1\" >&2; exit 1; }}; . \"{lib}\" && cratesio_newest_stable_below \"$1\""),
            "bash",
            ver,
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("bash must run");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(index.as_bytes())
        .expect("write");
    let out = child.wait_with_output().expect("wait");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    )
}

pub(crate) fn newest_stable_below(ver: &str, index: &str) -> String {
    let (code, out) = run_cratesio_below(ver, index);
    assert_eq!(
        code, 0,
        "cratesio_newest_stable_below {ver} exited {code}: {out}"
    );
    out
}
