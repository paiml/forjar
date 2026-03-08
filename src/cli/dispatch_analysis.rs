//! Dispatch for analysis, security, and audit commands.

use super::commands::*;

/// Dispatch analysis, security, and audit commands.
pub(crate) fn dispatch_analysis_cmd(cmd: Commands) -> Result<(), String> {
    match cmd {
        Commands::SecurityScan(SecurityScanArgs {
            file,
            json,
            fail_on,
        }) => super::security_scan::cmd_security_scan(&file, json, fail_on.as_deref()),
        Commands::Sbom(SbomArgs {
            file,
            state_dir,
            json,
        }) => super::sbom::cmd_sbom(&file, &state_dir, json),
        Commands::Cbom(CbomArgs {
            file,
            state_dir,
            json,
        }) => super::cbom::cmd_cbom(&file, &state_dir, json),
        Commands::Prove(ProveArgs {
            file,
            state_dir,
            machine,
            json,
        }) => super::prove::cmd_prove(&file, &state_dir, machine.as_deref(), json),
        Commands::PrivilegeAnalysis(PrivilegeAnalysisArgs {
            file,
            machine,
            json,
        }) => super::privilege_analysis::cmd_privilege_analysis(&file, machine.as_deref(), json),
        Commands::Provenance(ProvenanceArgs {
            file,
            state_dir,
            machine,
            json,
        }) => super::provenance::cmd_provenance(&file, &state_dir, machine.as_deref(), json),
        Commands::Lineage(LineageArgs { file, json }) => super::lineage::cmd_lineage(&file, json),
        Commands::Bundle(BundleArgs {
            file,
            output,
            include_state,
            verify,
        }) => {
            if verify {
                super::bundle::cmd_bundle_verify(&file)
            } else {
                super::bundle::cmd_bundle(&file, output.as_deref(), include_state)
            }
        }
        Commands::ModelCard(ModelCardArgs {
            file,
            state_dir,
            json,
        }) => super::model_card::cmd_model_card(&file, &state_dir, json),
        Commands::AgentSbom(AgentSbomArgs {
            file,
            state_dir,
            json,
        }) => super::agent_sbom::cmd_agent_sbom(&file, &state_dir, json),
        Commands::ReproProof(ReproProofArgs {
            file,
            state_dir,
            json,
        }) => super::repro_proof::cmd_repro_proof(&file, &state_dir, json),
        other => super::dispatch_misc_b::dispatch_data_cmd(other),
    }
}
