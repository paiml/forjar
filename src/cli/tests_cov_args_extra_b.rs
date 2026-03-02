//! Tests: Coverage for remaining args structs (misc_ops_args, plan_args, state_args, commands/mod.rs).

use super::commands::*;
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;

    // ── misc_ops_args.rs (49 uncov) ──

    #[test]
    fn test_cov_doctor_args_construct() {
        let a = DoctorArgs {
            file: None,
            json: false,
            fix: false,
            network: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_watch_args_construct() {
        let a = WatchArgs {
            file: PathBuf::from("f.yaml"),
            state_dir: PathBuf::from("s"),
            interval: 2,
            apply: false,
            yes: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_explain_args_construct() {
        let a = ExplainArgs {
            file: PathBuf::from("f.yaml"),
            resource: "r".to_string(),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_env_args_construct() {
        let a = EnvArgs {
            file: PathBuf::from("f.yaml"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_test_args_construct() {
        let a = TestArgs {
            file: PathBuf::from("f.yaml"),
            machine: None,
            resource: None,
            tag: None,
            group: None,
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_inventory_args_construct() {
        let a = InventoryArgs {
            file: PathBuf::from("f.yaml"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_retry_failed_args_construct() {
        let a = RetryFailedArgs {
            file: PathBuf::from("f.yaml"),
            state_dir: PathBuf::from("s"),
            params: vec![],
            timeout: None,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_rolling_args_construct() {
        let a = RollingArgs {
            file: PathBuf::from("f.yaml"),
            state_dir: PathBuf::from("s"),
            batch_size: 1,
            params: vec![],
            timeout: None,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_canary_args_construct() {
        let a = CanaryArgs {
            file: PathBuf::from("f.yaml"),
            state_dir: PathBuf::from("s"),
            machine: "m".to_string(),
            auto_proceed: false,
            params: vec![],
            timeout: None,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_audit_args_construct() {
        let a = AuditArgs {
            state_dir: PathBuf::from("s"),
            machine: None,
            limit: 20,
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_compliance_args_construct() {
        let a = ComplianceArgs {
            file: PathBuf::from("f.yaml"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_export_args_construct() {
        let a = ExportArgs {
            state_dir: PathBuf::from("s"),
            format: "csv".to_string(),
            machine: None,
            output: None,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_suggest_args_construct() {
        let a = SuggestArgs {
            file: PathBuf::from("f.yaml"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_compare_args_construct() {
        let a = CompareArgs {
            file1: PathBuf::from("a.yaml"),
            file2: PathBuf::from("b.yaml"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_env_diff_args_construct() {
        let a = EnvDiffArgs {
            env1: "dev".to_string(),
            env2: "prod".to_string(),
            state_dir: PathBuf::from("s"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_template_args_construct() {
        let a = TemplateArgs {
            recipe: PathBuf::from("r.yaml"),
            vars: vec![],
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    // ── plan_args.rs (9 uncov) ──

    #[test]
    fn test_cov_plan_args_construct() {
        let a = PlanArgs {
            file: PathBuf::from("f.yaml"),
            machine: None,
            resource: None,
            tag: None,
            group: None,
            state_dir: PathBuf::from("s"),
            json: false,
            output_dir: None,
            env_file: None,
            workspace: None,
            no_diff: false,
            target: None,
            cost: false,
            what_if: vec![],
            out: None,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_plan_compact_args_construct() {
        let a = PlanCompactArgs {
            file: PathBuf::from("f.yaml"),
            state_dir: PathBuf::from("s"),
            machine: None,
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    // ── state_args.rs (8 uncov) ──

    #[test]
    fn test_cov_state_list_args_construct() {
        let a = StateListArgs {
            state_dir: PathBuf::from("s"),
            machine: None,
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_state_mv_args_construct() {
        let a = StateMvArgs {
            old_id: "old".to_string(),
            new_id: "new".to_string(),
            state_dir: PathBuf::from("s"),
            machine: None,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_state_rm_args_construct() {
        let a = StateRmArgs {
            resource_id: "r".to_string(),
            state_dir: PathBuf::from("s"),
            machine: None,
            force: false,
        };
        let _ = format!("{:?}", a);
    }

    // ── commands/mod.rs (23 uncov) — Commands enum ──

    #[test]
    fn test_cov_commands_init_variant() {
        let cmd = Commands::Init(InitArgs {
            path: PathBuf::from("."),
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_validate_variant() {
        let cmd = Commands::Schema;
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_workspace_variant() {
        let cmd = Commands::Workspace(WorkspaceCmd::List);
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_workspace_current() {
        let cmd = Commands::Workspace(WorkspaceCmd::Current);
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_completion_bash() {
        let cmd = Commands::Completion(CompletionArgs {
            shell: CompletionShell::Bash,
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_completion_zsh() {
        let cmd = Commands::Completion(CompletionArgs {
            shell: CompletionShell::Zsh,
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_completion_fish() {
        let cmd = Commands::Completion(CompletionArgs {
            shell: CompletionShell::Fish,
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_snapshot_save() {
        let cmd = Commands::Snapshot(SnapshotCmd::Save {
            name: "s1".to_string(),
            state_dir: PathBuf::from("s"),
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_snapshot_list() {
        let cmd = Commands::Snapshot(SnapshotCmd::List {
            state_dir: PathBuf::from("s"),
            json: false,
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_snapshot_restore() {
        let cmd = Commands::Snapshot(SnapshotCmd::Restore {
            name: "s1".to_string(),
            state_dir: PathBuf::from("s"),
            yes: false,
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_snapshot_delete() {
        let cmd = Commands::Snapshot(SnapshotCmd::Delete {
            name: "s1".to_string(),
            state_dir: PathBuf::from("s"),
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_secrets_keygen() {
        let cmd = Commands::Secrets(SecretsCmd::Keygen);
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_secrets_encrypt() {
        let cmd = Commands::Secrets(SecretsCmd::Encrypt {
            value: "secret".to_string(),
            recipient: vec!["age1key".to_string()],
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_secrets_decrypt() {
        let cmd = Commands::Secrets(SecretsCmd::Decrypt {
            value: "ENC[age,abc]".to_string(),
            identity: None,
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_secrets_view() {
        let cmd = Commands::Secrets(SecretsCmd::View {
            file: PathBuf::from("f.yaml"),
            identity: None,
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_secrets_rekey() {
        let cmd = Commands::Secrets(SecretsCmd::Rekey {
            file: PathBuf::from("f.yaml"),
            identity: None,
            recipient: vec!["age1key".to_string()],
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_secrets_rotate() {
        let cmd = Commands::Secrets(SecretsCmd::Rotate {
            file: PathBuf::from("f.yaml"),
            identity: None,
            recipient: vec!["age1key".to_string()],
            re_encrypt: false,
            state_dir: PathBuf::from("s"),
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_workspace_new() {
        let cmd = Commands::Workspace(WorkspaceCmd::New {
            name: "dev".to_string(),
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_workspace_select() {
        let cmd = Commands::Workspace(WorkspaceCmd::Select {
            name: "dev".to_string(),
        });
        let _ = format!("{:?}", cmd);
    }

    #[test]
    fn test_cov_commands_workspace_delete() {
        let cmd = Commands::Workspace(WorkspaceCmd::Delete {
            name: "dev".to_string(),
            yes: false,
        });
        let _ = format!("{:?}", cmd);
    }
}
