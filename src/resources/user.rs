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
