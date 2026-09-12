//! The stubbed `gh` the gate T fixture injects through `$GH`.
//!
//! Two questions, two files. The cookbook's `Cargo.toml` carries the forjar
//! REQUIREMENT, which is a range; its `Cargo.lock` carries what cargo actually
//! builds. A stub that answered both with the manifest would make every lock
//! case measure the wrong file — which is precisely the confusion PMAT-537 is
//! about — so this one dispatches on the path.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use super::FLOOR;

pub(crate) fn stub_gh(dir: &Path, json: &str) -> String {
    stub_gh_with_cookbook(dir, json, "")
}

/// The stub, and what it answers a cookbook contents request with (PMAT-241).
///
/// `cargo_toml` is the cookbook's `Cargo.toml` as the gate would receive it:
/// the stub returns it base64-encoded under `.content`, the way the GitHub
/// contents API does, so a case can drive the requirement parser and the caret
/// comparison. Empty means the stub cannot answer, which is what an
/// unreachable GitHub looks like and is a case of its own.
pub(crate) fn stub_gh_with_cookbook(dir: &Path, json: &str, cargo_toml: &str) -> String {
    // The lock a cookbook must carry for the fixture's own release to be
    // qualified against it: the gate reads BOTH files and demands the lock pin
    // exactly the released version (PMAT-537).
    let lock = format!(
        "[[package]]\nname = \"forjar\"\nversion = \"{}\"\n",
        FLOOR.trim_start_matches('v')
    );
    stub_gh_with_cookbook_files(dir, json, cargo_toml, &lock)
}

/// base64, the way the GitHub contents API returns a file.
fn b64(body: &str) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in body.as_bytes().chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(A[(n >> 18 & 63) as usize] as char);
        out.push(A[(n >> 12 & 63) as usize] as char);
        out.push(if c.len() > 1 {
            A[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            A[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// The stub, answering the cookbook's `Cargo.toml` AND its `Cargo.lock`.
///
/// The gate makes two contents requests and they are different questions: the
/// manifest carries the requirement, which is a RANGE, and the lock carries
/// what cargo actually builds. A stub that answered both with the manifest
/// would make every lock case measure the wrong file — which is exactly the
/// confusion PMAT-537 is about. Either body may be empty, which is what an
/// unreachable GitHub or a missing file looks like.
pub(crate) fn stub_gh_with_cookbook_files(
    dir: &Path,
    json: &str,
    cargo_toml: &str,
    cargo_lock: &str,
) -> String {
    let p = dir.join("gh");
    let answer = |body: &str| {
        if body.is_empty() {
            String::from("echo '{}'; exit 0")
        } else {
            format!("printf '%s' '{}'; exit 0", b64(body))
        }
    };
    std::fs::write(
        &p,
        format!(
            "#!/usr/bin/env bash\n             if [ \"${{1:-}}\" = api ]; then\n             \x20 case \"${{2:-}}\" in\n             \x20   *Cargo.lock*) {lock} ;;\n             \x20   *) {toml} ;;\n             \x20 esac\n             fi\n             cat <<'FIXTURE_JSON'\n{json}\nFIXTURE_JSON\n",
            lock = answer(cargo_lock),
            toml = answer(cargo_toml),
        ),
    )
    .expect("write stub");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    p.to_string_lossy().into_owned()
}
