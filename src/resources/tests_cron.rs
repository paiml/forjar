//! FJ-154 / GH #154: shell-injection hardening tests for the cron handler.
//!
//! Asserts on generated script text only; never spawns a shell or crontab.

use super::cron::*;
use crate::core::types::{MachineTarget, Resource, ResourceType};

fn cron_resource(name: &str) -> Resource {
    Resource {
        resource_type: ResourceType::Cron,
        machine: MachineTarget::Single("m1".to_string()),
        owner: Some("root".to_string()),
        name: Some(name.to_string()),
        schedule: Some("0 * * * *".to_string()),
        command: Some("/usr/local/bin/backup.sh".to_string()),
        ..Default::default()
    }
}

#[test]
fn fj154_cron_user_quote_neutralized() {
    let mut r = cron_resource("job");
    r.owner = Some("x';reboot;'".to_string());
    let script = apply_script(&r);
    assert!(script.contains("'x'\"'\"';reboot;'\"'\"''"), "{script}");
    assert!(!script.contains("crontab -u 'x';reboot"), "{script}");
}

#[test]
fn fj154_cron_name_quote_neutralized() {
    let r = cron_resource("n';reboot;'");
    let script = apply_script(&r);
    assert!(script.contains("'\"'\"'"), "{script}");
    // The grep marker stays a single quoted word.
    assert!(!script.contains("grep -v '# forjar:n';reboot"), "{script}");
}

#[test]
fn fj154_cron_command_quote_neutralized() {
    let mut r = cron_resource("job");
    // Even though command is intentionally arbitrary, a stray quote must not
    // break the `echo '...'` that writes the crontab line.
    r.command = Some("/bin/x';reboot;'".to_string());
    let script = apply_script(&r);
    assert!(script.contains("'\"'\"'"), "{script}");
    assert!(
        !script.contains("echo '0 * * * * /bin/x';reboot"),
        "{script}"
    );
}

#[test]
fn fj154_cron_benign_unchanged() {
    let r = cron_resource("backup");
    let script = apply_script(&r);
    assert!(script.contains("crontab -u 'root'"));
    assert!(script.contains("# forjar:backup"));
    assert!(script.contains("0 * * * * /usr/local/bin/backup.sh"));
    assert!(check_script(&r).contains("crontab -u 'root' -l"));
    assert!(state_query_script(&r).contains("cron=MISSING:backup"));
}
