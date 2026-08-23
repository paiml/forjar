//! FJ-031: User/group resource handler.
//!
//! Manages local system users and groups via useradd/usermod/userdel/groupadd.

use crate::core::shell_escape::{sh_squote, sh_write_file};
use crate::core::types::Resource;
use crate::resources::verdict;

/// Generate shell script to check if a user exists and its properties.
pub fn check_script(resource: &Resource) -> String {
    let username = resource.name.as_deref().unwrap_or("unknown");
    verdict::single(
        &format!("id {} >/dev/null 2>&1", sh_squote(username)),
        &format!("exists:{username}"),
        &format!("missing:{username}"),
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
             if id '{username}' >/dev/null 2>&1; then\n\
               $SUDO userdel -r '{username}' 2>/dev/null || $SUDO userdel '{username}'\n\
             fi"
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
                    "getent group '{g}' >/dev/null 2>&1 || $SUDO groupadd '{g}'"
                ));
            }

            // Build useradd/usermod command
            let mut create_args = Vec::new();
            let mut modify_args = Vec::new();

            if resource.system_user {
                create_args.push("--system".to_string());
            }

            if let Some(ref shell) = resource.shell {
                create_args.push(format!("--shell '{shell}'"));
                modify_args.push(format!("--shell '{shell}'"));
            }

            if let Some(ref home) = resource.home {
                create_args.push(format!("--home-dir '{home}' --create-home"));
                modify_args.push(format!("--home '{home}'"));
            } else if !resource.system_user {
                create_args.push("--create-home".to_string());
            }

            if let Some(uid) = resource.uid {
                create_args.push(format!("--uid {uid}"));
                modify_args.push(format!("--uid {uid}"));
            }

            if let Some(ref group) = resource.group {
                create_args.push(format!("--gid '{group}'"));
                modify_args.push(format!("--gid '{group}'"));
            }

            if !resource.groups.is_empty() {
                let groups_str = resource.groups.join(",");
                create_args.push(format!("--groups '{groups_str}'"));
                modify_args.push(format!("--groups '{groups_str}'"));
            }

            let create_cmd = format!("$SUDO useradd {} '{}'", create_args.join(" "), username);
            let modify_cmd = format!("$SUDO usermod {} '{}'", modify_args.join(" "), username);

            lines.push(format!(
                "if ! id '{username}' >/dev/null 2>&1; then\n  {create_cmd}\nelse\n  {modify_cmd}\nfi"
            ));

            // SSH authorized keys
            if !resource.ssh_authorized_keys.is_empty() {
                let home_dir = resource
                    .home
                    .as_deref()
                    .map(|h| h.to_string())
                    .unwrap_or_else(|| format!("/home/{username}"));

                lines.push(format!("$SUDO mkdir -p '{home_dir}'/.ssh"));
                lines.push(format!("$SUDO chmod 700 '{home_dir}'/.ssh"));

                let keys = resource
                    .ssh_authorized_keys
                    .iter()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");

                // C8, second instance (GH #296): keys are DATA and must never reach
                // the target's shell parser. This was a `<<'FORJAR_EOF'` heredoc, and
                // a heredoc body is literal only until a line EQUALS the delimiter —
                // so a key entry containing `FORJAR_EOF` closed it and the remainder
                // executed as shell. Worse here than in file.rs: the lines that follow
                // are `$SUDO`, so an injected command lands beside privileges the
                // operator already granted, and the authorized_keys actually written
                // is silently truncated to whatever preceded the delimiter.
                //
                // `sh_write_file` has no delimiter to hit and is byte-exact.
                lines.push(sh_write_file("/tmp/forjar-authkeys", keys.as_bytes()));
                lines.push(format!(
                    "$SUDO mv /tmp/forjar-authkeys '{}'/.ssh/authorized_keys\n\
                     $SUDO chmod 600 '{}'/.ssh/authorized_keys\n\
                     $SUDO chown -R '{}':'{}' '{}'/.ssh",
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
        "id '{username}' >/dev/null 2>&1 && {{\n  \
         echo \"user={username}\"\n  \
         echo \"uid=$(id -u '{username}')\"\n  \
         echo \"gid=$(id -g '{username}')\"\n  \
         echo \"groups=$(id -Gn '{username}' | tr ' ' ',')\"\n  \
         echo \"shell=$(getent passwd '{username}' | cut -d: -f7)\"\n  \
         echo \"home=$(getent passwd '{username}' | cut -d: -f6)\"\n\
         }} || echo 'user=MISSING'"
    )
}
