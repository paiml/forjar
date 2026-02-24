//! FJ-033: Cron resource handler.
//!
//! Manages scheduled tasks via crontab entries.

use crate::core::types::Resource;

/// Generate shell script to check if a cron job exists.
pub fn check_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let user = resource.owner.as_deref().unwrap_or("root");
    format!(
        "crontab -u '{}' -l 2>/dev/null | grep -qF '# forjar:{}' && echo 'exists:{}' || echo 'missing:{}'",
        user, name, name, name
    )
}

/// Generate shell script to add/remove a cron entry.
pub fn apply_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let user = resource.owner.as_deref().unwrap_or("root");
    let state = resource.state.as_deref().unwrap_or("present");

    match state {
        "absent" => format!(
            "set -euo pipefail\n\
             SUDO=\"\"\n\
             [ \"$(id -u)\" -ne 0 ] && SUDO=\"sudo\"\n\
             EXISTING=$($SUDO crontab -u '{}' -l 2>/dev/null || true)\n\
             echo \"$EXISTING\" | grep -v '# forjar:{}' | grep -v '# forjar-cmd:{}' | $SUDO crontab -u '{}' -",
            user, name, name, user
        ),
        _ => {
            let schedule = resource.schedule.as_deref().unwrap_or("* * * * *");
            let command = resource.command.as_deref().unwrap_or("true");

            format!(
                "set -euo pipefail\n\
                 SUDO=\"\"\n\
                 [ \"$(id -u)\" -ne 0 ] && SUDO=\"sudo\"\n\
                 EXISTING=$($SUDO crontab -u '{}' -l 2>/dev/null | grep -v '# forjar:{}' | grep -v '# forjar-cmd:{}' || true)\n\
                 {{\n\
                   echo \"$EXISTING\"\n\
                   echo '# forjar:{}'\n\
                   echo '# forjar-cmd:{}'  \n\
                   echo '{} {}'\n\
                 }} | $SUDO crontab -u '{}' -",
                user, name, name, name, name, schedule, command, user
            )
        }
    }
}

/// Generate shell to query cron state (for BLAKE3 hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let user = resource.owner.as_deref().unwrap_or("root");
    format!(
        "crontab -u '{}' -l 2>/dev/null | grep -A1 '# forjar:{}' || echo 'cron=MISSING:{}'",
        user, name, name
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn make_cron_resource(name: &str) -> Resource {
        Resource {
            resource_type: ResourceType::Cron,
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
            owner: Some("root".to_string()),
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
            schedule: Some("0 * * * *".to_string()),
            command: Some("/usr/local/bin/backup.sh".to_string()),
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
        }
    }

    #[test]
    fn test_fj033_check_cron() {
        let r = make_cron_resource("backup");
        let script = check_script(&r);
        assert!(script.contains("crontab -u 'root' -l"));
        assert!(script.contains("forjar:backup"));
        assert!(script.contains("exists:backup"));
        assert!(script.contains("missing:backup"));
    }

    #[test]
    fn test_fj033_apply_present() {
        let r = make_cron_resource("backup");
        let script = apply_script(&r);
        assert!(script.contains("set -euo pipefail"));
        assert!(script.contains("crontab -u 'root'"));
        assert!(script.contains("0 * * * *"));
        assert!(script.contains("/usr/local/bin/backup.sh"));
        assert!(script.contains("# forjar:backup"));
    }

    #[test]
    fn test_fj033_apply_absent() {
        let mut r = make_cron_resource("old-job");
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        assert!(script.contains("grep -v '# forjar:old-job'"));
    }

    #[test]
    fn test_fj033_state_query() {
        let r = make_cron_resource("backup");
        let script = state_query_script(&r);
        assert!(script.contains("crontab -u 'root' -l"));
        assert!(script.contains("forjar:backup"));
        assert!(script.contains("cron=MISSING:backup"));
    }

    #[test]
    fn test_fj033_custom_user() {
        let mut r = make_cron_resource("deploy-sync");
        r.owner = Some("deploy".to_string());
        let script = apply_script(&r);
        assert!(script.contains("crontab -u 'deploy'"));
    }

    #[test]
    fn test_fj033_sudo_detection() {
        let r = make_cron_resource("job");
        let script = apply_script(&r);
        assert!(script.contains("SUDO=\"\""));
        assert!(script.contains("$SUDO crontab"));
    }

    /// Verify tagging prevents cron entry collisions.
    #[test]
    fn test_fj033_unique_tagging() {
        let r1 = make_cron_resource("job-a");
        let r2 = make_cron_resource("job-b");
        let s1 = apply_script(&r1);
        let s2 = apply_script(&r2);
        assert!(s1.contains("# forjar:job-a"));
        assert!(s2.contains("# forjar:job-b"));
        assert!(!s1.contains("# forjar:job-b"));
    }
}
