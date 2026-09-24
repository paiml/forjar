//! FJ-031: User/group resource handler.
//!
//! Manages local system users and groups via useradd/usermod/userdel/groupadd.

use crate::core::shell_escape::{sh_squote, sh_write_file};
use crate::core::types::Resource;
use crate::resources::verdict;

/// A check asks for every DECLARED attribute, not just presence.
///
/// It used to be `id <name>` alone: a declared uid, shell, home, primary group or
/// supplementary group set was applied once and never asked about again, so a
/// user hand-added to another group still read converged. For an isolation user
/// that is the entire defect. `state: absent` was judged backwards too.
/// Undeclared fields are not asserted. `groups` is the exact supplementary set,
/// because `usermod --groups` (the apply) replaces the set rather than adding.
pub fn check_script(resource: &Resource) -> String {
    let username = resource.name.as_deref().unwrap_or("unknown");
    let u = sh_squote(username);
    if resource.state.as_deref() == Some("absent") {
        return verdict::check_script_from(&[verdict::assert_that(
            &format!("! id {u} >/dev/null 2>&1"),
            &format!("absent:{username}"),
            &format!("exists:{username}"),
        )]);
    }
    let mut a = vec![verdict::assert_that(
        &format!("id {u} >/dev/null 2>&1"),
        &format!("exists:{username}"),
        &format!("missing:{username}"),
    )];
    let mut field = |name: &str, live: String, want: &str| {
        a.push(verdict::assert_that(
            &format!("[ \"$({live})\" = {} ]", sh_squote(want)),
            &format!("{name}={want}"),
            &format!("{name}!={want}"),
        ));
    };
    if let Some(uid) = resource.uid {
        field("uid", format!("id -u {u} 2>/dev/null"), &uid.to_string());
    }
    if let Some(ref shell) = resource.shell {
        field("shell", format!("getent passwd {u} | cut -d: -f7"), shell);
    }
    if let Some(ref home) = resource.home {
        field("home", format!("getent passwd {u} | cut -d: -f6"), home);
    }
    if let Some(ref group) = resource.group {
        field("group", format!("id -gn {u} 2>/dev/null"), group);
    }
    if !resource.groups.is_empty() {
        let mut want: Vec<&str> = resource.groups.iter().map(String::as_str).collect();
        want.sort_unstable();
        want.dedup();
        // supplementary = `id -Gn` minus the primary group, sorted, comma-joined
        let live = format!(
            "p=$(id -gn {u} 2>/dev/null); id -Gn {u} 2>/dev/null | tr ' ' '\\n' | grep -vxF \"$p\" | sort -u | paste -sd,"
        );
        field("groups", live, &want.join(","));
    }
    verdict::check_script_from(&a)
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
