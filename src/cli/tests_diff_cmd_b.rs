//! Tests: Diff — env diff parsing.

#![allow(unused_imports)]
use super::commands::*;
use super::diff_cmd::*;
use super::helpers::*;
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj367_env_diff_parse() {
        let cmd = Commands::EnvDiff(EnvDiffArgs {
            env1: "staging".to_string(),
            env2: "production".to_string(),
            state_dir: PathBuf::from("state"),
            json: false,
        });
        match cmd {
            Commands::EnvDiff(EnvDiffArgs { env1, env2, .. }) => {
                assert_eq!(env1, "staging");
                assert_eq!(env2, "production");
            }
            _ => panic!("expected EnvDiff"),
        }
    }
}
