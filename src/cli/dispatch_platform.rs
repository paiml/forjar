//! Dispatch for platform features:
//! remote state, recipe registry, service catalog,
//! multi-config apply, stack dependency graph.

use super::commands::*;

/// Dispatch platform-level commands.
pub(crate) fn dispatch_platform_cmd(cmd: Commands) -> Result<(), String> {
    match cmd {
        Commands::StateBackend(StateBackendArgs {
            state_dir,
            prefix,
            json,
        }) => super::remote_state::cmd_state_backend(
            &state_dir,
            prefix.as_deref(),
            json,
        ),
        Commands::RegistryList(RegistryListArgs { registry_dir, json }) => {
            let dir = registry_dir
                .unwrap_or_else(super::recipe_registry::default_registry_dir);
            super::recipe_registry::cmd_registry_list(&dir, json)
        }
        Commands::CatalogList(CatalogListArgs {
            catalog_dir,
            category,
            json,
        }) => {
            let dir = catalog_dir
                .unwrap_or_else(super::recipe_registry::default_registry_dir);
            super::service_catalog::cmd_catalog_list(
                &dir,
                category.as_deref(),
                json,
            )
        }
        Commands::MultiApply(MultiConfigArgs { file, json }) => {
            super::multi_config::cmd_multi_config(&file, json)
        }
        Commands::StackGraph(StackGraphArgs { file, json }) => {
            super::stack_dep_graph::cmd_stack_graph(&file, json)
        }
        _ => Err("unknown command".to_string()),
    }
}
