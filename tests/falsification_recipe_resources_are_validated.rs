//! A recipe's resources go through `validate_config`, like every other resource.
//!
//! forjar#357. `load_config` validated, and only then expanded. `includes` were
//! given the opposite order by FJ-254 — `merge_includes` runs ABOVE the
//! validation call precisely so included resources are checked — but recipes
//! were never moved with them. So every resource a recipe supplied reached
//! `plan` and `apply` having been validated by nothing at all.
//!
//! It is not a narrow hole. `validate_config` is where forjar's whole
//! config-time contract lives, so the contract held only for authors who did
//! not use recipes — and recipes are the mechanism forjar documents for fleet
//! reuse, which makes the most widely deployed resources the least checked
//! ones. `docs/book/src/04-recipes.md` even tells the user expansion begins
//! with "Config YAML is parsed and validated", which was true of the config
//! and false of the recipe.
//!
//! The probe is forjar#335's narrowed `lifecycle.ignore_drift`, because that
//! one is a HARD validation error inline and so makes the asymmetry a clean
//! pass/fail: identical declarations, one inline and one behind a recipe.
//!
//! DRIVEN THROUGH THE REAL BINARY, because the defect is that a user-visible
//! surface printed a clean verdict over a declaration it had never examined.

use std::fs;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

struct Sandbox {
    dir: std::path::PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-357-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("recipes")).expect("sandbox");
        Self { dir }
    }

    fn write(&self, rel: &str, body: &str) {
        fs::write(self.dir.join(rel), body).expect("write");
    }

    fn validate(&self) -> (bool, String) {
        let out = Command::new(FORJAR)
            .arg("validate")
            .arg("-f")
            .arg(self.dir.join("forjar.yaml"))
            .output()
            .expect("run forjar validate");
        let merged = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        (out.status.success(), merged)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// The narrowed form, written inline. This is the control: it must be refused,
/// and it already was before #357 — the point is that the recipe case below
/// writes the very same thing.
#[test]
fn the_narrowed_form_is_refused_when_written_inline() {
    let sb = Sandbox::new("inline");
    sb.write(
        "forjar.yaml",
        "version: \"1.0\"\nname: recipe-validation-inline\nmachines:\n  sandbox:\n    \
         hostname: sandbox\n    addr: 127.0.0.1\nresources:\n  \
         a-file:\n    type: file\n    machine: sandbox\n    \
         path: /tmp/forjar-357-inline.txt\n    content: hello\n    \
         lifecycle:\n      ignore_drift:\n        - content\n",
    );
    let (ok, msg) = sb.validate();
    assert!(
        !ok && msg.contains("ignore_drift"),
        "inline narrowed ignore_drift must be a validation error; got ok={ok}, output:\n{msg}"
    );
}

/// The same declaration, supplied by a recipe. Before #357 this printed a clean
/// verdict and exited 0, because `expand_recipes` ran AFTER `validate_config`.
#[test]
fn the_narrowed_form_is_refused_when_supplied_by_a_recipe() {
    let sb = Sandbox::new("recipe");
    sb.write(
        "forjar.yaml",
        "version: \"1.0\"\nname: recipe-validation\nmachines:\n  sandbox:\n    \
         hostname: sandbox\n    addr: 127.0.0.1\nresources:\n  \
         viarecipe:\n    type: recipe\n    recipe: probe\n    machine: sandbox\n",
    );
    sb.write(
        "recipes/probe.yaml",
        "recipe:\n  name: probe\n  version: \"1.0\"\nresources:\n  \
         a-file:\n    type: file\n    machine: sandbox\n    \
         path: /tmp/forjar-357-recipe.txt\n    content: hello\n    \
         lifecycle:\n      ignore_drift:\n        - content\n",
    );
    let (ok, msg) = sb.validate();
    assert!(
        !ok,
        "a recipe-supplied resource must be validated like any other; \
         forjar validate accepted it (exit 0). output:\n{msg}"
    );
    assert!(
        msg.contains("ignore_drift"),
        "the refusal must name the offending field, so the operator can find it \
         in the recipe; output:\n{msg}"
    );
}

/// The expanded id is what the plan will carry, so it is what the message must
/// name. `viarecipe/a-file`, not the `a-file` the user never typed at top level.
#[test]
fn the_refusal_names_the_expanded_id() {
    let sb = Sandbox::new("attribution");
    sb.write(
        "forjar.yaml",
        "version: \"1.0\"\nname: recipe-validation\nmachines:\n  sandbox:\n    \
         hostname: sandbox\n    addr: 127.0.0.1\nresources:\n  \
         viarecipe:\n    type: recipe\n    recipe: probe\n    machine: sandbox\n",
    );
    sb.write(
        "recipes/probe.yaml",
        "recipe:\n  name: probe\n  version: \"1.0\"\nresources:\n  \
         a-file:\n    type: file\n    machine: sandbox\n    \
         path: /tmp/forjar-357-attr.txt\n    content: hello\n    \
         lifecycle:\n      ignore_drift:\n        - content\n",
    );
    let (_, msg) = sb.validate();
    assert!(
        msg.contains("viarecipe/a-file"),
        "the error must name the post-expansion id the plan will use; output:\n{msg}"
    );
}

/// A recipe with nothing wrong in it still loads. Guards against the obvious
/// over-correction: a second validation pass that rejects legitimate expansion
/// output (namespaced ids, resolved `{{inputs.*}}` templates) would make every
/// recipe unusable, which is worse than the hole it closes.
#[test]
fn a_clean_recipe_still_validates() {
    let sb = Sandbox::new("clean");
    sb.write(
        "forjar.yaml",
        "version: \"1.0\"\nname: recipe-validation-clean\nmachines:\n  sandbox:\n    \
         hostname: sandbox\n    addr: 127.0.0.1\nresources:\n  \
         viarecipe:\n    type: recipe\n    recipe: probe\n    machine: sandbox\n    \
         inputs:\n      body: hello\n",
    );
    sb.write(
        "recipes/probe.yaml",
        "recipe:\n  name: probe\n  version: \"1.0\"\n  inputs:\n    \
         body:\n      type: string\n      default: hi\nresources:\n  \
         a-file:\n    type: file\n    machine: sandbox\n    \
         path: /tmp/forjar-357-clean.txt\n    content: \"{{inputs.body}}\"\n",
    );
    let (ok, msg) = sb.validate();
    assert!(
        ok,
        "a valid recipe must still load after the post-expansion pass; output:\n{msg}"
    );
}

/// A numeric uid/gid is a valid owner, and `forjar validate` must accept it.
///
/// Not a hypothetical: `examples/dogfood-renacer.yaml` and
/// `dogfood-sovereign-stack.yaml` set `owner: 472` on Grafana's data directory,
/// because Grafana runs as uid 472 and the account has no passwd entry on the
/// host — the normal case for a directory bind-mounted into a container.
/// `chown 472 /path` is exactly what `src/resources/file.rs` emits for it.
///
/// `is_valid_unix_name` required `^[a-z_][a-z0-9_-]*$`, so the only correct way
/// to write that ownership was a validation error. Nobody noticed for the same
/// reason #357 exists: those resources come from a recipe, and recipes were
/// never validated. Fixing the bypass surfaced the over-strict rule underneath
/// it, so the two are pinned together here.
#[test]
fn a_numeric_uid_is_a_valid_owner() {
    let sb = Sandbox::new("numeric-owner");
    sb.write(
        "forjar.yaml",
        "version: \"1.0\"\nname: numeric-owner\nmachines:\n  sandbox:\n    \
         hostname: sandbox\n    addr: 127.0.0.1\nresources:\n  \
         viarecipe:\n    type: recipe\n    recipe: probe\n    machine: sandbox\n",
    );
    sb.write(
        "recipes/probe.yaml",
        "recipe:\n  name: probe\n  version: \"1.0\"\nresources:\n  \
         grafana-data:\n    type: file\n    machine: sandbox\n    \
         path: /var/lib/grafana\n    content: \"\"\n    \
         owner: \"472\"\n    group: \"472\"\n",
    );
    let (ok, msg) = sb.validate();
    assert!(
        ok,
        "a numeric uid/gid is what `chown` takes and what forjar emits; \
         rejecting it makes a correct config unwritable. output:\n{msg}"
    );
}

/// The relaxation is bounded: a name that is neither a Unix name nor a bare
/// number is still refused, so this did not become "accept anything".
#[test]
fn a_malformed_owner_is_still_refused() {
    let sb = Sandbox::new("bad-owner");
    sb.write(
        "forjar.yaml",
        "version: \"1.0\"\nname: bad-owner\nmachines:\n  sandbox:\n    \
         hostname: sandbox\n    addr: 127.0.0.1\nresources:\n  \
         a-file:\n    type: file\n    machine: sandbox\n    \
         path: /tmp/forjar-357-badowner.txt\n    content: hello\n    \
         owner: \"4x7;rm -rf /\"\n",
    );
    let (ok, msg) = sb.validate();
    assert!(
        !ok && msg.contains("invalid owner"),
        "a non-name, non-numeric owner must still be a validation error; \
         got ok={ok}, output:\n{msg}"
    );
}
