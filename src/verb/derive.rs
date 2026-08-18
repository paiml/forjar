//! Derive the verb registry from the clap command tree.
//!
//! This module is the whole reason the surface can be trusted. It reads
//! [`crate::cli::Cli::command()`] — the exact [`clap::Command`] that `main`
//! parses argv with — and projects it into [`VerbSpec`]s. There is no second
//! list of verbs to keep in step, because there is no first one: the enum in
//! `src/cli/commands/mod.rs` is the only place a verb is declared.

use super::effects;
use super::schema;
use super::spec::VerbSpec;
use crate::cli::Cli;
use clap::{Command, CommandFactory};
use std::sync::OnceLock;

/// Stack reserved for building the clap tree.
///
/// Constructing a [`clap::Command`] with 159 subcommands — one of which
/// (`validate`) declares over two hundred flags — overflows the 2 MiB a thread
/// gets by default, in debug builds where nothing is inlined. It aborts the
/// process rather than unwinding, because this crate builds with
/// `panic = "abort"`.
///
/// That is not a test-only concern: the HTTP transport serves each connection
/// on its own thread, so any code path that rebuilt the tree per request would
/// crash the server. [`registry`] therefore builds once, here, on a thread
/// sized for the job, and every later caller reads the cache.
const BUILD_STACK_BYTES: usize = 16 * 1024 * 1024;

static REGISTRY: OnceLock<Vec<VerbSpec>> = OnceLock::new();

/// The clap tree the CLI parses with, built on a thread sized for it.
///
/// Callers on the main thread could call [`clap::CommandFactory::command`]
/// directly — `main` does, via `Cli::parse()`, and gets the process's 8 MiB
/// stack. Anywhere else the default 2 MiB is not enough (see
/// [`BUILD_STACK_BYTES`]), and the failure is a `SIGABRT`, not a catchable
/// panic. Routing every library-side construction through here means no caller
/// has to know that.
///
/// Prefer [`registry`], which builds this once and caches the projection.
#[must_use]
pub fn cli_command() -> Command {
    std::thread::Builder::new()
        .name("forjar-clap-tree".into())
        .stack_size(BUILD_STACK_BYTES)
        .spawn(Cli::command)
        .expect("spawn clap tree builder")
        .join()
        .expect("clap tree builder panicked")
}

/// The full registry, built once and cached.
///
/// Verbs are sorted by name so the manifest is stable across clap versions and
/// across changes to enum declaration order.
#[must_use]
pub fn registry() -> &'static [VerbSpec] {
    REGISTRY.get_or_init(|| {
        std::thread::Builder::new()
            .name("forjar-verb-registry".into())
            .stack_size(BUILD_STACK_BYTES)
            .spawn(build_registry)
            .expect("spawn registry builder")
            .join()
            .expect("registry builder panicked")
    })
}

fn build_registry() -> Vec<VerbSpec> {
    let root = cli_command();
    let mut verbs: Vec<VerbSpec> = root
        .get_subcommands()
        .filter(|s| s.get_name() != "help")
        .map(spec_of)
        .collect();
    verbs.sort_by(|a, b| a.name.cmp(&b.name));
    verbs
}

/// Project one clap subcommand into a [`VerbSpec`].
fn spec_of(sub: &Command) -> VerbSpec {
    let name = sub.get_name().to_string();

    let subcommands: Vec<String> = sub
        .get_subcommands()
        .filter(|s| s.get_name() != "help")
        .map(|s| s.get_name().to_string())
        .collect();

    // Global flags are declared on the root and propagated by clap into every
    // subcommand. They are the transport's business, not the verb's, so they
    // are excluded from per-verb params.
    let params: Vec<_> = sub
        .get_arguments()
        .filter(|a| !a.is_global_set())
        .filter(|a| a.get_id() != "help" && a.get_id() != "version")
        .map(schema::param_of)
        .collect();

    VerbSpec {
        description: sub
            .get_about()
            .map(|a| a.to_string())
            .unwrap_or_else(|| name.clone()),
        params_schema: schema::params_schema(&params, &subcommands),
        output_schema: schema::output_schema(),
        effects: effects::classify(&name),
        params,
        subcommands,
        name,
    }
}

/// Check that an argv is one the CLI accepts, using the real parser.
///
/// `argv` excludes the program name, which is supplied here.
///
/// Parsing builds the clap tree, so this runs on a thread sized for it for the
/// reason given on [`cli_command`].
///
/// # Errors
///
/// The [`clap::error::ErrorKind`] clap rejected the argv with. Note that
/// `MissingRequiredArgument` means the *shape* was accepted and only a value
/// was absent — callers checking that a constructed argv is well-formed
/// usually want to treat that as success.
pub fn check_argv(argv: &[String]) -> Result<(), clap::error::ErrorKind> {
    let full: Vec<String> = std::iter::once("forjar".to_string())
        .chain(argv.iter().cloned())
        .collect();
    std::thread::Builder::new()
        .name("forjar-argv-check".into())
        .stack_size(BUILD_STACK_BYTES)
        .spawn(move || {
            use clap::Parser;
            Cli::try_parse_from(&full).map(|_| ()).map_err(|e| e.kind())
        })
        .expect("spawn argv checker")
        .join()
        .expect("argv checker panicked")
}

/// Look up one verb by name.
#[must_use]
pub fn find(name: &str) -> Option<&'static VerbSpec> {
    registry().iter().find(|v| v.name == name)
}

/// Every verb name, sorted.
#[must_use]
pub fn verb_names() -> Vec<String> {
    registry().iter().map(|v| v.name.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verb::spec::Effects;
    use std::collections::HashSet;

    #[test]
    fn registry_covers_every_cli_subcommand() {
        // The equality that makes this a *unified* surface. Derived from the
        // same tree on both sides, so it can only fail if `spec_of` drops a
        // verb — which is exactly the bug worth catching.
        let cli: HashSet<String> = cli_command()
            .get_subcommands()
            .map(|s| s.get_name().to_string())
            .filter(|n| n != "help")
            .collect();
        let reg: HashSet<String> = verb_names().into_iter().collect();
        assert_eq!(cli, reg, "registry and CLI subcommands must be equal");
    }

    #[test]
    fn registry_is_large_and_not_accidentally_empty() {
        // A derivation bug that yields zero verbs would make every "for each
        // verb" test below pass vacuously. Pin the order of magnitude.
        assert!(
            registry().len() >= 150,
            "expected ~159 verbs, got {}",
            registry().len()
        );
    }

    #[test]
    fn names_are_unique() {
        let names = verb_names();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len(), "duplicate verb name");
    }

    #[test]
    fn registry_is_sorted() {
        let names = verb_names();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn every_verb_has_a_description() {
        for v in registry() {
            assert!(!v.description.trim().is_empty(), "{} has none", v.name);
        }
    }

    #[test]
    fn every_verb_has_a_well_formed_params_schema() {
        for v in registry() {
            assert_eq!(v.params_schema["type"], "object", "{}", v.name);
            assert_eq!(v.params_schema["additionalProperties"], false, "{}", v.name);
            assert!(v.params_schema["properties"].is_object(), "{}", v.name);
        }
    }

    #[test]
    fn kebab_case_command_names_are_honoured() {
        // `#[command(name = "lock-verify")]` on variant LockVerify. If the
        // derivation read the variant name instead of clap's, this is what
        // would break.
        let names = verb_names();
        assert!(names.contains(&"lock-verify".to_string()));
        assert!(names.contains(&"import-makefile".to_string()));
        assert!(!names.contains(&"LockVerify".to_string()));
    }

    #[test]
    fn grouped_verbs_expose_their_nested_subcommands() {
        let ws = find("workspace").expect("workspace verb");
        assert!(ws.is_grouped());
        for expected in ["new", "list", "select", "delete"] {
            assert!(
                ws.subcommands.contains(&expected.to_string()),
                "workspace missing {expected}: {:?}",
                ws.subcommands
            );
        }
        assert_eq!(
            ws.params_schema["required"],
            serde_json::json!(["subcommand"])
        );
    }

    #[test]
    fn global_flags_are_not_verb_parameters() {
        // --verbose and --no-color belong to the process, not to `plan`.
        let plan = find("plan").expect("plan verb");
        assert!(plan.param("verbose").is_none());
        assert!(plan.param("no_color").is_none());
        assert!(plan.param("state_dir").is_some());
    }

    #[test]
    fn help_is_not_a_verb_and_not_a_parameter() {
        assert!(find("help").is_none());
        for v in registry() {
            assert!(v.param("help").is_none(), "{} exposes help", v.name);
        }
    }

    #[test]
    fn plan_params_match_the_cli_flags_exactly() {
        let plan = find("plan").expect("plan verb");
        let file = plan.param("file").expect("plan --file");
        assert_eq!(file.long.as_deref(), Some("file"));
        assert_eq!(file.default.as_deref(), Some("forjar.yaml"));
        let json = plan.param("json").expect("plan --json");
        assert_eq!(json.kind, crate::verb::spec::ParamKind::Flag);
        assert_eq!(plan.params_schema["properties"]["json"]["type"], "boolean");
    }

    #[test]
    fn effects_are_assigned_and_transports_are_marked() {
        assert_eq!(find("mcp").unwrap().effects, Effects::Transport);
        assert_eq!(find("plan").unwrap().effects, Effects::ReadOnly);
        assert_eq!(find("apply").unwrap().effects, Effects::Mutating);
    }

    #[test]
    fn the_effects_allowlist_names_only_live_verbs() {
        // Catches a rename: `lock-verify` becoming `lock-check` leaves a dead
        // entry claiming a verb is read-only when no such verb exists.
        let stale = effects::allowlist_is_live(&verb_names());
        assert!(stale.is_empty(), "stale effect classifications: {stale:?}");
    }

    #[test]
    fn find_returns_none_for_an_unknown_verb() {
        assert!(find("definitely-not-a-verb").is_none());
    }
}
