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
            arch: vec![],
            tags: vec![],
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

    #[test]
    fn test_fj033_apply_preserves_existing_entries() {
        // Apply should filter out existing forjar entries before re-adding
        let r = make_cron_resource("backup");
        let script = apply_script(&r);
        assert!(
            script.contains("grep -v '# forjar:backup'"),
            "must remove old entry before re-adding"
        );
    }

    #[test]
    fn test_fj033_absent_preserves_other_entries() {
        // Absent should only remove the matching forjar entry
        let mut r = make_cron_resource("old-job");
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        assert!(script.contains("grep -v '# forjar:old-job'"));
        assert!(script.contains("grep -v '# forjar-cmd:old-job'"));
        // Should re-install the filtered crontab
        assert!(script.contains("crontab -u 'root' -"));
    }

    #[test]
    fn test_fj033_default_schedule_and_command() {
        let mut r = make_cron_resource("defaults");
        r.schedule = None;
        r.command = None;
        let script = apply_script(&r);
        assert!(
            script.contains("* * * * *"),
            "default schedule should be every minute"
        );
        assert!(script.contains("true"), "default command should be 'true'");
    }

    #[test]
    fn test_fj033_check_custom_user() {
        let mut r = make_cron_resource("sync");
        r.owner = Some("www-data".to_string());
        let script = check_script(&r);
        assert!(script.contains("crontab -u 'www-data' -l"));
    }

    #[test]
    fn test_fj033_state_query_custom_user() {
        let mut r = make_cron_resource("sync");
        r.owner = Some("deploy".to_string());
        let script = state_query_script(&r);
        assert!(script.contains("crontab -u 'deploy' -l"));
    }

    // ── Edge-case tests (FJ-123) ─────────────────────────────────

    #[test]
    fn test_fj033_no_name_defaults_to_unknown() {
        let mut r = make_cron_resource("placeholder");
        r.name = None;
        let check = check_script(&r);
        assert!(check.contains("forjar:unknown"));
        let apply = apply_script(&r);
        assert!(apply.contains("# forjar:unknown"));
        let query = state_query_script(&r);
        assert!(query.contains("forjar:unknown"));
    }

    #[test]
    fn test_fj033_no_owner_defaults_to_root() {
        let mut r = make_cron_resource("job");
        r.owner = None;
        let script = apply_script(&r);
        assert!(script.contains("crontab -u 'root'"));
    }

    #[test]
    fn test_fj033_absent_ignores_schedule_and_command() {
        // In absent state, schedule/command are irrelevant — script should only grep -v
        let mut r = make_cron_resource("old-job");
        r.state = Some("absent".to_string());
        r.schedule = Some("0 3 * * *".to_string());
        r.command = Some("/bin/cleanup".to_string());
        let script = apply_script(&r);
        assert!(
            !script.contains("0 3 * * *"),
            "absent should not include schedule"
        );
        assert!(
            !script.contains("/bin/cleanup"),
            "absent should not include command"
        );
        assert!(script.contains("grep -v '# forjar:old-job'"));
    }

    #[test]
    fn test_fj033_apply_cmd_tag_idempotency() {
        // Verify forjar-cmd tag is also filtered out on re-apply (prevents duplication)
        let r = make_cron_resource("backup");
        let script = apply_script(&r);
        assert!(script.contains("grep -v '# forjar-cmd:backup'"));
        assert!(script.contains("echo '# forjar-cmd:backup'"));
    }

    // ── FJ-132: Additional cron edge case tests ─────────────────

    #[test]
    fn test_fj132_check_script_default_name() {
        let mut r = make_cron_resource("placeholder");
        r.name = None;
        let script = check_script(&r);
        assert!(script.contains("forjar:unknown"));
    }

    #[test]
    fn test_fj132_state_query_default_owner() {
        let mut r = make_cron_resource("job");
        r.owner = None;
        let script = state_query_script(&r);
        assert!(script.contains("crontab -u 'root' -l"));
    }

    #[test]
    fn test_fj132_apply_special_chars_in_command() {
        let mut r = make_cron_resource("log-rotate");
        r.command = Some("/usr/sbin/logrotate /etc/logrotate.d/*.conf".to_string());
        let script = apply_script(&r);
        assert!(script.contains("/usr/sbin/logrotate /etc/logrotate.d/*.conf"));
    }

    #[test]
    fn test_fj132_apply_five_field_schedule() {
        let mut r = make_cron_resource("weekly");
        r.schedule = Some("0 3 * * 0".to_string());
        let script = apply_script(&r);
        assert!(script.contains("0 3 * * 0"));
    }

    #[test]
    fn test_fj132_check_and_query_same_user() {
        // check_script and state_query_script should use the same user
        let mut r = make_cron_resource("sync");
        r.owner = Some("www-data".to_string());
        let check = check_script(&r);
        let query = state_query_script(&r);
        assert!(check.contains("crontab -u 'www-data'"));
        assert!(query.contains("crontab -u 'www-data'"));
    }

    // ── FJ-036: Cron resource handler tests ─────────────────────

    #[test]
    fn test_fj036_cron_apply_contains_schedule() {
        let mut r = make_cron_resource("nightly-backup");
        r.schedule = Some("30 2 * * *".to_string());
        r.command = Some("/opt/backup/run.sh".to_string());
        let script = apply_script(&r);
        assert!(
            script.contains("30 2 * * *"),
            "apply script must include the schedule string"
        );
        assert!(
            script.contains("/opt/backup/run.sh"),
            "apply script must include the command"
        );
    }

    #[test]
    fn test_fj036_cron_apply_absent_removes() {
        let mut r = make_cron_resource("stale-job");
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        assert!(
            script.contains("grep -v '# forjar:stale-job'"),
            "absent state must generate crontab removal via grep -v"
        );
        assert!(
            script.contains("crontab -u 'root' -"),
            "absent state must reinstall filtered crontab"
        );
        // Absent must NOT contain schedule/command in the output
        assert!(
            !script.contains("0 * * * *"),
            "absent should not emit schedule"
        );
    }

    #[test]
    fn test_fj036_cron_state_query_lists_crontab() {
        let r = make_cron_resource("hourly-sync");
        let script = state_query_script(&r);
        assert!(
            script.contains("crontab -u 'root' -l"),
            "state_query must use 'crontab -l' to list entries"
        );
        assert!(
            script.contains("forjar:hourly-sync"),
            "state_query must filter by forjar tag"
        );
    }
}
