use super::ssh::*;
use super::ExecOutput;
#[allow(unused_imports)]
use crate::core::types::Machine;

fn make_machine(addr: &str, user: &str, ssh_key: Option<&str>) -> Machine {
    Machine {
        hostname: "test-host".to_string(),
        addr: addr.to_string(),
        user: user.to_string(),
        arch: "x86_64".to_string(),
        ssh_key: ssh_key.map(|s| s.to_string()),
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    }
}

#[test]
fn test_fj011_ssh_key_expansion() {
    let key = "~/.ssh/id_ed25519";
    let expanded = if let Some(rest) = key.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            format!("{home}/{rest}")
        } else {
            key.to_string()
        }
    } else {
        key.to_string()
    };
    assert!(expanded.contains(".ssh/id_ed25519"));
    assert!(!expanded.starts_with('~'));
}

#[test]
fn test_fj011_ssh_key_expansion_no_tilde() {
    // Absolute path should be returned unchanged
    let key = "/home/deploy/.ssh/id_ed25519";
    let expanded = if let Some(rest) = key.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            format!("{home}/{rest}")
        } else {
            key.to_string()
        }
    } else {
        key.to_string()
    };
    assert_eq!(expanded, "/home/deploy/.ssh/id_ed25519");
}

#[test]
fn test_fj011_build_args_basic() {
    let m = make_machine("10.0.0.1", "root", None);
    let args = build_ssh_args(&m);
    assert!(args.contains(&"BatchMode=yes".to_string()));
    assert!(args.contains(&"ConnectTimeout=5".to_string()));
    assert!(args.contains(&"StrictHostKeyChecking=accept-new".to_string()));
    assert!(args.contains(&"root@10.0.0.1".to_string()));
    assert!(args.contains(&"bash".to_string()));
    // No -i flag without ssh_key
    assert!(!args.contains(&"-i".to_string()));
}

#[test]
fn test_fj011_build_args_with_key() {
    let m = make_machine("10.0.0.1", "deploy", Some("/home/deploy/.ssh/id_ed25519"));
    let args = build_ssh_args(&m);
    assert!(args.contains(&"-i".to_string()));
    assert!(args.contains(&"/home/deploy/.ssh/id_ed25519".to_string()));
    assert!(args.contains(&"deploy@10.0.0.1".to_string()));
}

#[test]
fn test_fj011_build_args_with_tilde_key() {
    let m = make_machine("10.0.0.1", "admin", Some("~/.ssh/id_rsa"));
    let args = build_ssh_args(&m);
    // Should have expanded ~ to $HOME
    let key_idx = args.iter().position(|a| a == "-i").unwrap();
    let key_path = &args[key_idx + 1];
    assert!(!key_path.starts_with('~'), "tilde should be expanded");
    assert!(key_path.ends_with(".ssh/id_rsa"));
}

#[test]
fn test_fj011_build_args_user_at_host_format() {
    let m = make_machine("web.example.com", "deployer", None);
    let args = build_ssh_args(&m);
    assert!(args.contains(&"deployer@web.example.com".to_string()));
}

#[test]
fn test_fj011_build_args_ipv6() {
    let m = make_machine("::1", "root", None);
    let args = build_ssh_args(&m);
    assert!(args.contains(&"root@::1".to_string()));
}

#[test]
fn test_fj011_exec_output_captures_nonzero_exit() {
    let output = ExecOutput {
        exit_code: 127,
        stdout: String::new(),
        stderr: "command not found".to_string(),
    };
    assert!(!output.success());
    assert_eq!(output.exit_code, 127);
}

#[test]
fn test_fj011_exec_output_captures_signal() {
    let output = ExecOutput {
        exit_code: -1,
        stdout: String::new(),
        stderr: "killed".to_string(),
    };
    assert!(!output.success());
    assert_eq!(output.exit_code, -1);
}

#[test]
fn test_fj011_build_args_order() {
    let m = make_machine("10.0.0.5", "root", Some("/root/.ssh/key"));
    let args = build_ssh_args(&m);
    let batch_idx = args.iter().position(|a| a == "BatchMode=yes").unwrap();
    let user_idx = args.iter().position(|a| a == "root@10.0.0.5").unwrap();
    let bash_idx = args.iter().position(|a| a == "bash").unwrap();
    assert!(batch_idx < user_idx, "options must come before user@host");
    assert!(user_idx < bash_idx, "user@host must come before bash");
}

#[test]
fn test_fj011_stdin_piping_design() {
    let m = make_machine("10.0.0.1", "root", None);
    let args = build_ssh_args(&m);
    assert_eq!(args.last().unwrap(), "bash");
    assert!(!args.contains(&"-c".to_string()));
}

#[test]
fn test_fj011_build_args_count_without_key() {
    let m = make_machine("10.0.0.1", "root", None);
    let args = build_ssh_args(&m);
    assert_eq!(args.len(), 8, "6 option args + user@host + bash");
}

#[test]
fn test_fj011_build_args_count_with_key() {
    let m = make_machine("10.0.0.1", "root", Some("/root/.ssh/key"));
    let args = build_ssh_args(&m);
    assert_eq!(args.len(), 10, "6 option args + -i key + user@host + bash");
}

#[test]
fn test_fj011_batch_mode_prevents_password_prompt() {
    let m = make_machine("10.0.0.1", "root", None);
    let args = build_ssh_args(&m);
    let batch_pos = args.iter().position(|a| a == "BatchMode=yes").unwrap();
    assert_eq!(args[batch_pos - 1], "-o");
}

#[test]
fn test_fj011_connect_timeout_value() {
    let m = make_machine("10.0.0.1", "root", None);
    let args = build_ssh_args(&m);
    assert!(
        args.contains(&"ConnectTimeout=5".to_string()),
        "must set 5s connect timeout"
    );
}

#[test]
fn test_fj011_strict_host_key_accept_new() {
    let m = make_machine("10.0.0.1", "root", None);
    let args = build_ssh_args(&m);
    assert!(args.contains(&"StrictHostKeyChecking=accept-new".to_string()));
}

#[test]
fn test_fj011_dns_hostname_addr() {
    let m = make_machine("db.prod.internal", "admin", None);
    let args = build_ssh_args(&m);
    assert!(args.contains(&"admin@db.prod.internal".to_string()));
}

#[test]
fn test_fj011_nonstandard_user() {
    let m = make_machine("10.0.0.1", "deploy-bot", None);
    let args = build_ssh_args(&m);
    assert!(args.contains(&"deploy-bot@10.0.0.1".to_string()));
}

#[test]
fn test_fj011_build_args_all_options_are_dash_o() {
    let m = make_machine("10.0.0.1", "root", None);
    let args = build_ssh_args(&m);
    let option_values = [
        "BatchMode=yes",
        "ConnectTimeout=5",
        "StrictHostKeyChecking=accept-new",
    ];
    for opt in &option_values {
        let pos = args.iter().position(|a| a == opt).unwrap();
        assert_eq!(args[pos - 1], "-o", "option '{opt}' must be preceded by -o");
    }
}

#[test]
fn test_fj011_build_args_key_before_user_host() {
    let m = make_machine("10.0.0.1", "root", Some("/root/.ssh/key"));
    let args = build_ssh_args(&m);
    let key_idx = args.iter().position(|a| a == "-i").unwrap();
    let user_idx = args.iter().position(|a| a == "root@10.0.0.1").unwrap();
    assert!(key_idx < user_idx, "-i must come before user@host");
}

#[test]
fn test_fj011_build_args_bash_is_last() {
    let m = make_machine("10.0.0.1", "root", Some("/root/.ssh/key"));
    let args = build_ssh_args(&m);
    assert_eq!(args.last().unwrap(), "bash");
}

#[test]
fn test_fj011_exec_output_success_zero() {
    let output = ExecOutput {
        exit_code: 0,
        stdout: "ok".to_string(),
        stderr: String::new(),
    };
    assert!(output.success());
}

#[test]
fn test_fj011_key_expansion_relative_path() {
    let m = make_machine("10.0.0.1", "root", Some("keys/deploy.pem"));
    let args = build_ssh_args(&m);
    assert!(args.contains(&"keys/deploy.pem".to_string()));
}

// -- FJ-252: SSH connection multiplexing --

#[test]
fn test_fj252_control_path_format() {
    let m = make_machine("10.0.0.1", "deploy", None);
    let path = control_path(&m);
    assert_eq!(path, "/tmp/forjar-ssh/deploy@10.0.0.1");
}

#[test]
fn test_fj252_control_path_different_machines() {
    let m1 = make_machine("10.0.0.1", "root", None);
    let m2 = make_machine("10.0.0.2", "deploy", None);
    assert_ne!(control_path(&m1), control_path(&m2));
}

#[test]
fn test_fj252_has_control_master_no_socket() {
    let m = make_machine("192.168.99.99", "nobody", None);
    assert!(!has_control_master(&m));
}

#[test]
fn test_fj252_no_mux_args_without_socket() {
    let m = make_machine("10.0.0.1", "root", None);
    let args = build_ssh_args(&m);
    assert!(
        !args.contains(&"ControlMaster=auto".to_string()),
        "no ControlMaster without socket"
    );
    assert!(
        !args.iter().any(|a| a.starts_with("ControlPath=")),
        "no ControlPath without socket"
    );
}

#[test]
fn test_fj252_mux_args_with_socket() {
    let dir = tempfile::tempdir().unwrap();
    let sock_path = dir.path().join("test-sock");
    std::fs::write(&sock_path, "").unwrap();

    let m = make_machine("10.0.0.1", "root", None);
    let sock = control_path(&m);
    let _ = std::fs::remove_file(&sock);

    let args = build_ssh_args(&m);
    assert!(!args.contains(&"ControlMaster=auto".to_string()));
}

#[test]
fn test_fj252_mux_args_injected_when_socket_exists() {
    let unique_addr = format!("252.252.{}.{}", std::process::id() % 255, 252);
    let m = make_machine(&unique_addr, "muxtest", None);
    let sock = control_path(&m);

    std::fs::create_dir_all(CONTROL_DIR).unwrap();
    std::fs::write(&sock, "fake-socket").unwrap();

    let args = build_ssh_args(&m);
    let _ = std::fs::remove_file(&sock);

    assert!(
        args.contains(&"ControlMaster=auto".to_string()),
        "ControlMaster=auto when socket exists"
    );
    assert!(
        args.iter().any(|a| a.starts_with("ControlPath=")),
        "ControlPath when socket exists"
    );
    assert_eq!(args.last().unwrap(), "bash");
}

#[test]
fn test_fj252_stop_control_master_no_socket() {
    let m = make_machine("192.168.99.99", "nobody", None);
    assert!(stop_control_master(&m).is_ok());
}

#[test]
fn test_fj252_stop_all_noop_when_empty() {
    stop_all_control_masters();
}

#[test]
fn test_fj252_expand_tilde_with_home() {
    let result = expand_tilde("~/.ssh/id_ed25519");
    assert!(!result.starts_with('~'));
    assert!(result.ends_with(".ssh/id_ed25519"));
}

#[test]
fn test_fj252_expand_tilde_absolute() {
    let result = expand_tilde("/etc/ssh/key");
    assert_eq!(result, "/etc/ssh/key");
}

#[test]
fn test_fj252_expand_tilde_relative() {
    let result = expand_tilde("keys/deploy.pem");
    assert_eq!(result, "keys/deploy.pem");
}

#[test]
fn test_fj252_control_persist_constant() {
    assert_eq!(CONTROL_PERSIST_SECS, 60);
}

#[test]
fn test_fj252_control_dir_constant() {
    assert_eq!(CONTROL_DIR, "/tmp/forjar-ssh");
}
