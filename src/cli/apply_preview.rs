//! GH-210: `apply --preview` showed no scripts and applied nothing.
//!
//! `--preview` is documented as "FJ-360: Show generated scripts before
//! execution". The shipped implementation short-circuited into the plan
//! renderer and returned `Ok(())`, so it printed a plan (not a script),
//! executed nothing, and wrote no state — while exiting 0. `forjar status`
//! immediately afterwards failed with "cannot read state dir state", which is
//! how an operator eventually discovers the apply they thought they ran never
//! happened.
//!
//! Two things are wrong there and both are fixed:
//!
//! * it showed the wrong artifact — the script generator `--output-scripts`
//!   already uses was one call away, so `--preview` now prints the real
//!   generated scripts;
//! * "before execution" implies execution follows, so `--preview` no longer
//!   swallows the apply. It is a *louder* apply, not a silent no-op.
//!
//! `--output-scripts` keeps its early exit — writing scripts "for manual
//! review" is its whole purpose — but says so out loud instead of leaving the
//! caller to infer it from a missing state directory.

use super::commands::ApplyArgs;
use super::helpers::parse_and_validate;
use crate::core::{codegen, resolver, types};

/// Print the generated apply script for every resource in the config.
///
/// Errors from a single resource's codegen are reported inline rather than
/// aborting: a preview that dies on the first unsupported resource type shows
/// less than one that says which resource it could not render.
pub(super) fn print_generated_scripts(args: &ApplyArgs) -> Result<(), String> {
    let config = parse_and_validate(&args.file)?;
    println!("Generated scripts for {}:", config.name);
    for (id, resource) in &config.resources {
        let resolved = resolver::resolve_or_fallback(
            id,
            resource,
            &config.params,
            &config.machines,
            &config.secrets,
        );
        println!("\n# ── {} ({}) ──", id, machine_label(&resolved));
        match codegen::apply_script(&resolved) {
            Ok(script) => println!("{script}"),
            Err(e) => println!("# (no script could be generated: {e})"),
        }
    }
    println!("\n--preview: the apply follows.");
    Ok(())
}

/// Comma-joined machine list for a preview header.
fn machine_label(resource: &types::Resource) -> String {
    match &resource.machine {
        types::MachineTarget::Single(m) => m.clone(),
        types::MachineTarget::Multiple(ms) => ms.join(","),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_for(path: &std::path::Path) -> ApplyArgs {
        ApplyArgs {
            file: path.to_path_buf(),
            ..Default::default()
        }
    }

    fn write_config(dir: &std::path::Path) -> std::path::PathBuf {
        let p = dir.join("forjar.yaml");
        std::fs::write(
            &p,
            "version: \"1.0\"\n\
             name: prev\n\
             params:\n\
             \x20 sandbox: /tmp/fj-preview-unit\n\
             machines:\n\
             \x20 local:\n\
             \x20   hostname: localhost\n\
             \x20   addr: 127.0.0.1\n\
             \x20   user: nobody\n\
             \x20   arch: x86_64\n\
             resources:\n\
             \x20 a-file:\n\
             \x20   type: file\n\
             \x20   machine: local\n\
             \x20   path: \"{{params.sandbox}}/a.txt\"\n\
             \x20   content: \"aaa\\n\"\n",
        )
        .expect("fixture written");
        p
    }

    #[test]
    fn preview_renders_a_script_for_every_resource() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = write_config(dir.path());
        // Errors would mean the preview cannot show anything at all; the
        // printed content itself is asserted by the process-level test.
        print_generated_scripts(&args_for(&cfg)).expect("preview renders");
    }

    #[test]
    fn a_missing_config_is_an_error_not_an_empty_preview() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("nope.yaml");
        assert!(print_generated_scripts(&args_for(&missing)).is_err());
    }
}
