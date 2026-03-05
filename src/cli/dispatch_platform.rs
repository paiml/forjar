//! Dispatch for platform features:
//! remote state, recipe registry, service catalog,
//! multi-config/stack, query, signing, preservation, parallel apply.

use super::commands::*;

/// Dispatch platform-level commands.
pub(crate) fn dispatch_platform_cmd(cmd: Commands) -> Result<(), String> {
    match cmd {
        Commands::StateBackend(args) => dispatch_state_backend(args),
        Commands::RegistryList(args) => dispatch_registry(args),
        Commands::CatalogList(args) => dispatch_catalog(args),
        Commands::MultiApply(MultiConfigArgs { file, json }) => {
            super::multi_config::cmd_multi_config(&file, json)
        }
        Commands::StackGraph(StackGraphArgs { file, json }) => {
            super::stack_dep_graph::cmd_stack_graph(&file, json)
        }
        Commands::InfraQuery(args) => dispatch_query(args),
        Commands::RecipeSign(args) => dispatch_sign(args),
        Commands::Preservation(PreservationArgs { file, json }) => {
            super::preservation_check::cmd_preservation(&file, json)
        }
        Commands::ParallelApply(ParallelStackArgs {
            file,
            max_parallel,
            json,
        }) => super::parallel_multi_stack::cmd_parallel_stacks(&file, max_parallel, json),
        Commands::Saga(SagaArgs {
            file,
            state_dir,
            json,
        }) => super::saga_coordinator::cmd_saga_plan(&file, &state_dir, json),
        Commands::AgentRegistry(args) => dispatch_agent_registry(args),
        Commands::PullAgent(args) => dispatch_pull_agent(args),
        _ => Err("unknown command".to_string()),
    }
}

fn dispatch_state_backend(args: StateBackendArgs) -> Result<(), String> {
    super::remote_state::cmd_state_backend(&args.state_dir, args.prefix.as_deref(), args.json)
}

fn dispatch_registry(args: RegistryListArgs) -> Result<(), String> {
    let dir = args
        .registry_dir
        .unwrap_or_else(super::recipe_registry::default_registry_dir);
    super::recipe_registry::cmd_registry_list(&dir, args.json)
}

fn dispatch_catalog(args: CatalogListArgs) -> Result<(), String> {
    let dir = args
        .catalog_dir
        .unwrap_or_else(super::recipe_registry::default_registry_dir);
    super::service_catalog::cmd_catalog_list(&dir, args.category.as_deref(), args.json)
}

fn dispatch_query(args: InfraQueryArgs) -> Result<(), String> {
    if args.live {
        return super::infra_query_live::cmd_query_live(
            &args.file,
            args.pattern.as_deref(),
            args.json,
        );
    }
    let filter = super::infra_query::QueryFilter {
        pattern: args.pattern,
        resource_type: args.resource_type,
        machine: args.machine,
        tag: args.tag,
    };
    super::infra_query::cmd_query(
        &args.file,
        &args.state_dir,
        &filter,
        args.details,
        args.json,
    )
}

fn dispatch_agent_registry(args: AgentRegistryArgs) -> Result<(), String> {
    let dir = args
        .registry_dir
        .unwrap_or_else(super::recipe_registry::default_registry_dir);
    super::agent_registry::cmd_agent_registry(&dir, args.category.as_deref(), args.json)
}

fn dispatch_pull_agent(args: PullAgentArgs) -> Result<(), String> {
    let mode = if args.pull {
        super::pull_agent::ExecMode::Pull
    } else {
        super::pull_agent::ExecMode::Push
    };
    super::pull_agent::cmd_pull_agent(
        &args.file,
        &args.state_dir,
        args.interval,
        args.auto_apply,
        args.max_iterations,
        mode,
        args.json,
    )
}

fn dispatch_sign(args: RecipeSignArgs) -> Result<(), String> {
    if args.pq {
        super::pq_signing::cmd_dual_sign(
            &args.recipe,
            args.verify,
            args.signer.as_deref(),
            args.json,
        )
    } else {
        super::recipe_signing::cmd_recipe_sign(
            &args.recipe,
            args.verify,
            args.signer.as_deref(),
            args.json,
        )
    }
}
