//! forjar#660 rules 3 to 6: what resolution covers, how often, and from where.
//!
//!   3. `{{data.x}}` is resolved in EVERY string of the config (resource
//!      `version`, `tags`, ... not just params and outputs).
//!   4. Outputs are resolved exactly once: a data value that itself contains
//!      `{{user}}` is printed as text, not resolved a second time.
//!   5. `validate` (plain, `--exhaustive`, `--deep`) reports an unknown
//!      `{{data.KEY}}` as an error, like `{{params.}}`.
//!   6. One base dir: every command resolves `data:` file sources relative to
//!      the manifest's directory, wherever it is run from.

mod common_660;

use common_660::*;
use std::path::Path;

fn write(dir: &Path, name: &str, body: &str) {
    if let Some(p) = Path::new(name).parent() {
        std::fs::create_dir_all(dir.join(p)).unwrap();
    }
    std::fs::write(dir.join(name), body).unwrap();
}

fn pkg_manifest(version_cmd: &str) -> String {
    format!(
        "version: \"1.0\"\nname: res660\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [\"curl\"]\n    version: \"{{{{data.ver}}}}\"\n    tags: [\"t-{{{{data.ver}}}}\"]\ndata:\n  ver:\n    type: command\n    value: \"{version_cmd}\"\n"
    )
}

#[test]
fn resource_strings_resolve_data_in_pin_sbom_and_compare() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    write(d, "forjar.yaml", &pkg_manifest("echo 1.2.3"));
    write(d, "same_text_other_data.yaml", &pkg_manifest("echo 9.9.9"));

    let pin = run(d, &["pin", "-f", "forjar.yaml"]);
    assert!(pin.status.success(), "{}", text(&pin));
    let lock = std::fs::read_to_string(d.join("state/forjar.inputs.lock.yaml")).unwrap();
    assert!(
        lock.contains("version: 1.2.3"),
        "pin saw an unresolved version: {lock}"
    );
    assert!(!lock.contains("{{"), "pin left a template: {lock}");

    let sbom = run(d, &["sbom", "-f", "forjar.yaml", "--json"]);
    let body = text(&sbom);
    assert!(sbom.status.success(), "{body}");
    assert!(body.contains("\"versionInfo\": \"1.2.3\""), "sbom: {body}");
    assert!(!body.contains("{{"), "sbom left a template: {body}");

    // The two files are byte-identical but for the data command, so `compare`
    // can only see a difference through the RESOLVED version.
    let cmp = run(d, &["compare", "forjar.yaml", "same_text_other_data.yaml"]);
    let body = text(&cmp);
    assert!(
        body.contains("pkg") && body.contains("modified"),
        "compare missed a changed data source: {body}"
    );
    let same = run(d, &["compare", "forjar.yaml", "forjar.yaml"]);
    assert!(!text(&same).contains("modified"), "{}", text(&same));
}

fn double_resolve() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "double_resolve.yaml",
        "version: \"1.0\"\nname: dbl660\nmachines: {}\nresources: {}\nparams:\n  user: alice\noutputs:\n  g:\n    value: \"hello {{data.greeting}}\"\ndata:\n  greeting:\n    type: command\n    value: \"echo '{{user}}'\"\n",
    );
    dir
}

#[test]
fn outputs_resolve_exactly_once() {
    let dir = double_resolve();
    for args in [
        &["output", "-f", "double_resolve.yaml", "g"][..],
        &["show", "-f", "double_resolve.yaml"][..],
    ] {
        let out = run(dir.path(), args);
        let body = text(&out);
        assert!(out.status.success(), "forjar {}: {body}", args.join(" "));
        assert!(
            body.contains("hello {{user}}"),
            "forjar {} resolved the data value a second time: {body}",
            args.join(" ")
        );
        assert!(!body.contains("hello alice"), "double resolve: {body}");
    }
}

#[test]
fn validate_rejects_an_unknown_data_key_in_every_mode() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "forjar.yaml",
        "version: \"1.0\"\nname: unk660\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/unk660\n    content: \"x={{data.nope}}\"\n",
    );
    for flags in [&[][..], &["--exhaustive"][..], &["--deep"][..]] {
        let mut args = vec!["validate", "-f", "forjar.yaml"];
        args.extend_from_slice(flags);
        let out = run(dir.path(), &args);
        let body = text(&out);
        assert!(
            !out.status.success(),
            "forjar {} accepted an unknown data key: {body}",
            args.join(" ")
        );
        assert!(
            body.contains("nope"),
            "forjar {} did not name the key: {body}",
            args.join(" ")
        );
    }
}

#[test]
fn one_base_dir_for_plan_apply_and_lock() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    write(d, "subdir/greeting.txt", "from-the-manifest-dir\n");
    write(
        d,
        "subdir/file_data.yaml",
        &format!(
            "version: \"1.0\"\nname: base660\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: {}/out.txt\n    content: \"g={{{{data.g}}}}\"\ndata:\n  g:\n    type: file\n    value: greeting.txt\n",
            d.display()
        ),
    );
    // Run from the PARENT of the manifest's directory.
    for args in [
        &["plan", "-f", "subdir/file_data.yaml"][..],
        &["apply", "-f", "subdir/file_data.yaml", "--dry-run"][..],
        &["lock", "-f", "subdir/file_data.yaml", "--dry-run"][..],
        &["show", "-f", "subdir/file_data.yaml"][..],
    ] {
        let out = run(d, args);
        assert!(
            out.status.success(),
            "forjar {} resolved a data file against the working directory: {}",
            args.join(" "),
            text(&out)
        );
    }
}
