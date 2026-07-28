//! FJ-2726 (PMAT-199): `forjar import-makefile` — turn a Makefile into a config.
//!
//! # What this claims
//!
//! It imports a **single-makefile, non-recursive** build: explicit and
//! pattern-derived rules, order-only prerequisites, `.PHONY` targets, and
//! recipes as make would expand them. That is enough to take the trivial
//! C-project shape — mkdir, compile, link, clean, test — and rebuild it with
//! `forjar make`.
//!
//! # What it refuses, and why refusing is the feature
//!
//! An importer that silently mistranslates is worse than no importer: the
//! output looks like a build and is not one. Every construct listed in
//! [`refusals`] is detected and reported, and the import fails rather than
//! emitting a config that would run something different from what make runs.
//!
//! The refusals are not a wish list — each has a signal measured against real
//! `make -p --trace` output.

use super::makefile_parse::{self as mk, MakeTarget};
use std::path::Path;

/// Minimum GNU make. 3.81 (still shipped by macOS) writes
/// "commands to execute" with a backtick quote instead of "recipe to execute",
/// so a 4.x parser finds no recipes at all and would emit an empty build.
const MIN_MAKE: (u32, u32) = (4, 0);

/// Run make once with both `-p` and `--trace`, capturing combined stdout.
///
/// `-B` is not optional. Measured: in an already-built tree,
/// `make --trace -n all` prints `Nothing to be done` and every compile and link
/// command disappears from the trace, so the import would produce structure
/// with no commands for exactly the targets that matter. `-k` keeps going past
/// a rule that errors during the dry run.
fn run_make(dir: &Path, makefile: &Path, goals: &[String]) -> Result<String, String> {
    let mut cmd = std::process::Command::new("make");
    cmd.arg("-p")
        .arg("-n")
        .arg("--trace")
        .arg("-k")
        .arg("-f")
        .arg(makefile)
        .current_dir(dir)
        // `-p` dumps every environment variable as a make variable, and
        // expansions can capture ambient state, so the import must not depend
        // on the caller's environment.
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("HOME", std::env::var("HOME").unwrap_or_default());

    if !goals.is_empty() {
        cmd.arg("-B");
        for g in goals {
            cmd.arg(g);
        }
    }

    let out = cmd
        .output()
        .map_err(|e| format!("cannot run make: {e}. Is GNU make installed?"))?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Targets worth asking make to materialise in pass 2.
///
/// Special targets (`.PHONY`, `.SUFFIXES`, …) are make's own directives, and a
/// target with neither prerequisites nor a recipe is a source file.
fn goal_candidates(targets: &[MakeTarget]) -> Vec<String> {
    targets
        .iter()
        .filter(|t| !t.name.starts_with('.'))
        .filter(|t| !t.prereqs.is_empty() || !t.order_only.is_empty() || t.has_recipe() || t.phony)
        .map(|t| t.name.clone())
        .collect()
}

/// Constructs this importer will not translate, with the reason.
///
/// Each check is against a signal actually present in make's output.
pub fn refusals(raw: &str, targets: &[MakeTarget]) -> Vec<String> {
    let mut out = Vec::new();

    if raw.contains("Entering directory") || raw.contains("Leaving directory") {
        out.push(
            "recursive make (a `$(MAKE)` sub-invocation). Its `+` recipe lines run \
             even under `-n`, and the sub-make interleaves its own trace with \
             `Makefile:N:` labels that collide with the parent's — the parse is \
             corrupted, not merely incomplete."
                .to_string(),
        );
    }

    if raw.contains("\n.ONESHELL:") {
        out.push(
            ".ONESHELL. The trace is byte-identical with and without it, so there \
             is no signal to translate from; importing a one-shell recipe as \
             separate lines would silently change what `cd` and `set -e` do."
                .to_string(),
        );
    }

    // Detected from the PARSED targets, not by scanning the raw dump: make's
    // own built-in implicit rules include `%:: %,v` and `%:: RCS/%`, so a text
    // scan reports a double-colon rule in every Makefile ever written.
    // FJ-2727: goal-only phony means an unrequested phony resource is dropped
    // and the edge to it scrubbed. That is correct for forjar and WRONG for a
    // Makefile where a real file target lists a .PHONY prerequisite: make runs
    // the action when it reaches it, forjar cannot, and the imported config
    // fails at the first command that needed the action's side effect.
    // Verified: `app.txt: stamp` (phony) imports and then dies on
    // `cp: cannot stat 'version.txt'`. Refusing is honest; silently emitting a
    // config that cannot build is exactly what this list exists to prevent.
    let phony: std::collections::HashSet<&str> = targets
        .iter()
        .filter(|t| t.phony)
        .map(|t| t.name.as_str())
        .collect();
    for t in targets.iter().filter(|t| !t.phony) {
        for dep in t.prereqs.iter().chain(t.order_only.iter()) {
            if phony.contains(dep.as_str()) {
                out.push(format!(
                    "target '{}' depends on the .PHONY target '{dep}'. make runs a \
                     phony prerequisite when it reaches it; forjar's phony targets \
                     are goal-only (running them as prerequisites is not \
                     convergent — a `clean` that a `build` depends on deletes the \
                     outputs that make `build` stale, forever). The imported config \
                     would build without '{dep}' ever running.",
                    t.name
                ));
            }
        }
    }

    for t in targets.iter().filter(|t| t.double_colon) {
        out.push(format!(
            "target '{}' is a double-colon rule. It declares independent recipes \
             for one target name, and forjar resource ids are unique.",
            t.name
        ));
    }

    if raw.contains("\n# General ('VPATH' variable) search path:") {
        out.push(
            "VPATH. Prerequisite paths in the config would not resolve the way \
             make resolves them."
                .to_string(),
        );
    }

    out
}

/// A forjar resource id derived from a make target name.
///
/// `build/main.o` -> `build-main-o`. NOT injective — see [`id_map`], which is
/// what callers must use.
fn slug(target: &str) -> String {
    let id: String = target
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let id = id.trim_matches('-').to_lowercase();
    if id.is_empty() {
        "target".to_string()
    } else {
        id
    }
}

/// Map every target name to a UNIQUE forjar resource id.
///
/// FJ-2728: slugging is not injective. `a-b.txt` and `a.b.txt` both slug to
/// `a-b-txt`, which emitted a duplicate YAML key; the parser kept one and the
/// other target vanished from the build with no error at all. Verified: make
/// produced both files, the imported config produced one.
///
/// Colliding ids are disambiguated with a short digest of the ORIGINAL target
/// name. The digest is used for every member of a collision, not just the later
/// ones, so an id never depends on iteration order — a config re-imported from
/// the same Makefile is byte-identical.
pub fn id_map(targets: &[MakeTarget]) -> std::collections::HashMap<String, String> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for t in targets {
        *counts.entry(slug(&t.name)).or_insert(0) += 1;
    }
    targets
        .iter()
        .map(|t| {
            let base = slug(&t.name);
            let id = if counts.get(&base).copied().unwrap_or(0) > 1 {
                format!("{base}-{}", short_digest(&t.name))
            } else {
                base
            };
            (t.name.clone(), id)
        })
        .collect()
}

/// Six hex characters of the target name's BLAKE3 digest.
fn short_digest(name: &str) -> String {
    crate::tripwire::hasher::hash_string(name)
        .rsplit(':')
        .next()
        .unwrap_or("000000")
        .chars()
        .take(6)
        .collect()
}

/// Import a Makefile into forjar YAML.
pub fn import(dir: &Path, makefile: &Path, machine: &str) -> Result<String, String> {
    // Pass 1 — enumerate. Pattern rules are not instantiated yet, so this only
    // learns the target NAMES.
    let pass1 = run_make(dir, makefile, &[])?;
    let (_, db1) = mk::split_streams(&pass1);
    match mk::parse_version(db1) {
        Some(v) if v >= MIN_MAKE => {}
        Some((maj, min)) => {
            return Err(format!(
                "GNU make {maj}.{min} is too old to import from (need >= {}.{}). \
                 3.81 writes \"commands to execute\" instead of \"recipe to \
                 execute\", so the recipes would be silently missed rather than \
                 reported. macOS ships 3.81; install a newer make (`brew install \
                 make`, then `gmake`).",
                MIN_MAKE.0, MIN_MAKE.1
            ));
        }
        None => {
            return Err(
                "make produced no database. Check that the Makefile parses: \
                 `make -n -f <makefile>`."
                    .to_string(),
            );
        }
    }

    let goals = goal_candidates(&mk::parse_database(db1));
    if goals.is_empty() {
        return Err("no targets found in this Makefile".to_string());
    }

    // Pass 2 — materialise. Asking for every target by name forces pattern
    // rules to instantiate, and `-B` forces every recipe into the trace.
    let pass2 = run_make(dir, makefile, &goals)?;
    let (trace, db2) = mk::split_streams(&pass2);
    let mut targets = mk::parse_database(db2);
    let blocks = mk::parse_trace(trace);
    mk::join(&mut targets, &blocks);
    for t in targets.iter_mut() {
        t.recipe = mk::fold_continuations(&t.recipe);
        t.recipe_raw = mk::fold_continuations(&t.recipe_raw);
    }

    targets.retain(|t| !t.name.starts_with('.') && (t.has_recipe() || t.phony));

    let refused = refusals(&pass2, &targets);
    if !refused.is_empty() {
        return Err(format!(
            "this Makefile uses constructs the importer will not translate \
             faithfully, so nothing was written:\n\n  - {}\n\nAn importer that \
             silently mistranslates produces a config that looks like your build \
             and is not one. Port these rules by hand.",
            refused.join("\n  - ")
        ));
    }

    if targets.is_empty() {
        return Err("no importable targets (every rule was a built-in)".to_string());
    }

    Ok(emit(&targets, dir, machine))
}

/// Render the imported graph as forjar YAML.
pub fn emit(targets: &[MakeTarget], dir: &Path, machine: &str) -> String {
    let known: std::collections::HashSet<&str> = targets.iter().map(|t| t.name.as_str()).collect();
    let ids = id_map(targets);
    let id_of = |name: &str| -> String { ids.get(name).cloned().unwrap_or_else(|| slug(name)) };

    let mut y = String::new();
    y.push_str("# Generated by `forjar import-makefile`.\n");
    y.push_str("# Recipes are make's own expansion; review before applying.\n");
    y.push_str("version: \"1.0\"\nname: imported\n\nmachines:\n");
    // Absolute, so the emitted config does not silently depend on the
    // directory it is applied from.
    let proj = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    y.push_str(&format!(
        "  {machine}:\n    hostname: localhost\n    addr: localhost\n\nparams:\n  proj: \"{}\"\n\nresources:\n",
        proj.display()
    ));

    for t in targets {
        y.push_str(&format!("  {}:\n", id_of(&t.name)));
        y.push_str("    type: task\n");
        y.push_str(&format!("    machine: {machine}\n"));
        y.push_str("    working_dir: \"{{params.proj}}\"\n");

        if t.phony {
            // Names an action: no artifact to observe, goal-only.
            y.push_str("    phony: true\n");
        } else {
            y.push_str(&format!("    output_artifacts: [\"{}\"]\n", t.name));
        }

        // A prerequisite that is itself a target becomes an edge; one that is
        // not is a source file, and therefore an input to hash. Order-only
        // prerequisites are edges only — that is exactly what `|` means, and
        // treating them as inputs is what made `| build` an idempotency pump.
        let inputs: Vec<&String> = t
            .prereqs
            .iter()
            .filter(|p| !known.contains(p.as_str()))
            .collect();
        let edges: Vec<String> = t
            .prereqs
            .iter()
            .chain(t.order_only.iter())
            .filter(|p| known.contains(p.as_str()))
            .map(|p| id_of(p))
            .collect();

        if !inputs.is_empty() {
            y.push_str(&format!(
                "    task_inputs: [{}]\n",
                inputs
                    .iter()
                    .map(|i| format!("\"{i}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !edges.is_empty() {
            y.push_str(&format!("    depends_on: [{}]\n", edges.join(", ")));
        }

        if !t.recipe.is_empty() {
            y.push_str("    command: |\n");
            for (i, line) in t.recipe.iter().enumerate() {
                // `@` (silent) and `+` (run even under -n) are make recipe
                // prefixes, not shell, and the trace has already stripped them.
                let cleaned = line
                    .trim_start_matches(['@', '+'])
                    .trim_start_matches('-')
                    .trim();

                // One SUBSHELL per logical recipe line, with make's shell
                // options restored inside it.
                //
                // forjar wraps a command in `set -euo pipefail`; make sets
                // NOTHING. That difference is not merely "stricter": under
                // `pipefail`, `seq 1 100000 | head -1` fails with 141 because
                // `seq` takes SIGPIPE when `head` exits. make returns 0 and
                // `cmd | head` is a stock Makefile idiom, so importing it
                // unchanged turns a working build into a failing one. `set -e`
                // inside the line is wrong for the same reason — make checks
                // the LINE's exit status, it does not abort mid-line.
                //
                // The outer `set -e` still aborts the target when a subshell
                // exits non-zero, which IS make's rule.
                let ignore = t
                    .recipe_raw
                    .get(i)
                    .is_some_and(|raw| mk::ignores_errors(raw));
                // A `-` prefix means make ignores this line's exit status.
                // Dropping it would convert `-rm -f x` from a no-op into a
                // hard failure.
                let tail = if ignore { " || true" } else { "" };
                y.push_str(&format!(
                    "      ( set +e +u +o pipefail; {cleaned} ){tail}\n"
                ));
            }
        }
        y.push('\n');
    }
    y
}
