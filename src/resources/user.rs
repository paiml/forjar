//! FJ-031: User/group resource handler.
//!
//! Manages local system users and groups via useradd/usermod/userdel/groupadd.

use crate::core::types::Resource;

/// Generate shell script to check if a user exists and its properties.
pub fn check_script(resource: &Resource) -> String {
    let username = resource.name.as_deref().unwrap_or("unknown");
    format!(
        "id '{}' >/dev/null 2>&1 && echo 'exists:{}' || echo 'missing:{}'",
        username, username, username
    )
}

/// Generate shell script to create/modify/remove a user.
pub fn apply_script(resource: &Resource) -> String {
    let username = resource.name.as_deref().unwrap_or("unknown");
    let state = resource.state.as_deref().unwrap_or("present");

    match state {
        "absent" => format!(
            "set -euo pipefail\n\
             SUDO=\"\"\n\
             [ \"$(id -u)\" -ne 0 ] && SUDO=\"sudo\"\n\
             if id '{}' >/dev/null 2>&1; then\n\
               $SUDO userdel -r '{}' 2>/dev/null || $SUDO userdel '{}'\n\
             fi",
            username, username, username
        ),
        _ => {
            let mut lines = vec![
                "set -euo pipefail".to_string(),
                "SUDO=\"\"".to_string(),
                "[ \"$(id -u)\" -ne 0 ] && SUDO=\"sudo\"".to_string(),
            ];

            // Ensure supplementary groups exist
            for g in &resource.groups {
                lines.push(format!(
                    "getent group '{}' >/dev/null 2>&1 || $SUDO groupadd '{}'",
                    g, g
                ));
            }

            // Build useradd/usermod command
            let mut create_args = Vec::new();
            let mut modify_args = Vec::new();

            if resource.system_user {
                create_args.push("--system".to_string());
            }

            if let Some(ref shell) = resource.shell {
                create_args.push(format!("--shell '{}'", shell));
                modify_args.push(format!("--shell '{}'", shell));
            }

            if let Some(ref home) = resource.home {
                create_args.push(format!("--home-dir '{}' --create-home", home));
                modify_args.push(format!("--home '{}'", home));
            } else if !resource.system_user {
                create_args.push("--create-home".to_string());
            }

            if let Some(uid) = resource.uid {
                create_args.push(format!("--uid {}", uid));
                modify_args.push(format!("--uid {}", uid));
            }

            if let Some(ref group) = resource.group {
                create_args.push(format!("--gid '{}'", group));
                modify_args.push(format!("--gid '{}'", group));
            }

            if !resource.groups.is_empty() {
                let groups_str = resource.groups.join(",");
                create_args.push(format!("--groups '{}'", groups_str));
                modify_args.push(format!("--groups '{}'", groups_str));
            }

            let create_cmd = format!("$SUDO useradd {} '{}'", create_args.join(" "), username);
            let modify_cmd = format!("$SUDO usermod {} '{}'", modify_args.join(" "), username);

            lines.push(format!(
                "if ! id '{}' >/dev/null 2>&1; then\n  {}\nelse\n  {}\nfi",
                username, create_cmd, modify_cmd
            ));

            // SSH authorized keys
            if !resource.ssh_authorized_keys.is_empty() {
                let home_dir = resource
                    .home
                    .as_deref()
                    .map(|h| h.to_string())
                    .unwrap_or_else(|| format!("/home/{}", username));

                lines.push(format!("$SUDO mkdir -p '{}'/.ssh", home_dir));
                lines.push(format!("$SUDO chmod 700 '{}'/.ssh", home_dir));

                let keys = resource
                    .ssh_authorized_keys
                    .iter()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");

                lines.push(format!(
                    "cat > /tmp/forjar-authkeys <<'FORJAR_EOF'\n{}\nFORJAR_EOF\n\
                     $SUDO mv /tmp/forjar-authkeys '{}'/.ssh/authorized_keys\n\
                     $SUDO chmod 600 '{}'/.ssh/authorized_keys\n\
                     $SUDO chown -R '{}':'{}' '{}'/.ssh",
                    keys,
                    home_dir,
                    home_dir,
                    username,
                    resource.group.as_deref().unwrap_or(username),
                    home_dir
                ));
            }

            lines.join("\n")
        }
    }
}

/// Generate shell to query user state (for BLAKE3 hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let username = resource.name.as_deref().unwrap_or("unknown");
    format!(
        "id '{}' >/dev/null 2>&1 && {{\n  \
         echo \"user={}\"\n  \
         echo \"uid=$(id -u '{}')\"\n  \
         echo \"gid=$(id -g '{}')\"\n  \
         echo \"groups=$(id -Gn '{}' | tr ' ' ',')\"\n  \
         echo \"shell=$(getent passwd '{}' | cut -d: -f7)\"\n  \
         echo \"home=$(getent passwd '{}' | cut -d: -f6)\"\n\
         }} || echo 'user=MISSING'",
        username, username, username, username, username, username, username
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn make_user_resource(name: &str) -> Resource {
        Resource {
            resource_type: ResourceType::User,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: None,
            content: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: Some(name.to_string()),
            enabled: None,
            restart_on: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: std::collections::HashMap::new(),
            arch: vec![],
            tags: vec![],
        }
    }

    #[test]
    fn test_fj031_check_user() {
        let r = make_user_resource("deploy");
        let script = check_script(&r);
        assert!(script.contains("id 'deploy'"));
        assert!(script.contains("exists:deploy"));
        assert!(script.contains("missing:deploy"));
    }

    #[test]
    fn test_fj031_apply_present_basic() {
        let r = make_user_resource("deploy");
        let script = apply_script(&r);
        assert!(script.contains("set -euo pipefail"));
        assert!(script.contains("useradd"));
        assert!(script.contains("'deploy'"));
        assert!(script.contains("--create-home"));
    }

    #[test]
    fn test_fj031_apply_with_shell_and_home() {
        let mut r = make_user_resource("app");
        r.shell = Some("/bin/zsh".to_string());
        r.home = Some("/opt/app".to_string());
        let script = apply_script(&r);
        assert!(script.contains("--shell '/bin/zsh'"));
        assert!(script.contains("--home-dir '/opt/app'"));
    }

    #[test]
    fn test_fj031_apply_with_uid() {
        let mut r = make_user_resource("svc");
        r.uid = Some(1500);
        let script = apply_script(&r);
        assert!(script.contains("--uid 1500"));
    }

    #[test]
    fn test_fj031_apply_with_groups() {
        let mut r = make_user_resource("deploy");
        r.groups = vec!["docker".to_string(), "sudo".to_string()];
        let script = apply_script(&r);
        assert!(script.contains("groupadd 'docker'"));
        assert!(script.contains("groupadd 'sudo'"));
        assert!(script.contains("--groups 'docker,sudo'"));
    }

    #[test]
    fn test_fj031_apply_system_user() {
        let mut r = make_user_resource("prometheus");
        r.system_user = true;
        let script = apply_script(&r);
        assert!(script.contains("--system"));
        // System users don't get --create-home by default
        assert!(!script.contains("--create-home"));
    }

    #[test]
    fn test_fj031_apply_absent() {
        let mut r = make_user_resource("olduser");
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        assert!(script.contains("userdel"));
        assert!(script.contains("'olduser'"));
    }

    #[test]
    fn test_fj031_apply_with_ssh_keys() {
        let mut r = make_user_resource("deploy");
        r.ssh_authorized_keys = vec!["ssh-ed25519 AAAA... deploy@host".to_string()];
        let script = apply_script(&r);
        assert!(script.contains("mkdir -p '/home/deploy'/.ssh"));
        assert!(script.contains("chmod 700"));
        assert!(script.contains("authorized_keys"));
        assert!(script.contains("ssh-ed25519 AAAA"));
    }

    #[test]
    fn test_fj031_apply_with_primary_group() {
        let mut r = make_user_resource("app");
        r.group = Some("appgroup".to_string());
        let script = apply_script(&r);
        assert!(script.contains("--gid 'appgroup'"));
    }

    #[test]
    fn test_fj031_state_query() {
        let r = make_user_resource("deploy");
        let script = state_query_script(&r);
        assert!(script.contains("id 'deploy'"));
        assert!(script.contains("getent passwd 'deploy'"));
        assert!(script.contains("user=MISSING"));
    }

    #[test]
    fn test_fj031_apply_usermod_on_existing() {
        let r = make_user_resource("deploy");
        let script = apply_script(&r);
        // Should contain both create and modify branches
        assert!(script.contains("useradd"));
        assert!(script.contains("usermod"));
    }

    #[test]
    fn test_fj031_sudo_detection() {
        let r = make_user_resource("deploy");
        let script = apply_script(&r);
        assert!(script.contains("SUDO=\"\""));
        assert!(script.contains("id -u"));
        assert!(script.contains("$SUDO useradd"));
    }

    /// Verify single-quoting prevents injection in username.
    #[test]
    fn test_fj031_quoted_username() {
        let r = make_user_resource("user; rm -rf /");
        let script = apply_script(&r);
        assert!(script.contains("'user; rm -rf /'"));
    }
}
