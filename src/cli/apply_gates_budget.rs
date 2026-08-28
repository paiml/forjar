//! forjar#334: the apply gate for an unhonourable `disk_budget` preview request.
//!
//! Lives beside `apply_gates` rather than inside it only because that file is
//! at the repo's 500-line ceiling; the purpose is the same — pure decision
//! logic, testable without CLI orchestration.

/// Refuse rather than silently ignore a preview request.
///
/// `FORJAR_BUDGET_DRY_RUN` is a variable of the GENERATED REAPER, evaluated on
/// the target at the far end of a chain that strips it: `sudo bash
/// <<'FORJAR_SUDO'` resets the environment (sudo's `env_reset`) and `ssh host
/// bash` carries no `SendEnv`. forjar's own process never reads it. So an
/// operator who exported it in their shell got a real reclaim — 1.5 TB in the
/// reported incident — behind a `1 converged` line indistinguishable from the
/// preview they had asked for.
///
/// forjar cannot make an ambient variable survive that hop, so it says so
/// instead of proceeding. This is the repo's standing answer to "we cannot do
/// what you asked": refuse before mutating, and name the surfaces that can.
///
/// Scoped to runs that actually contain a `disk_budget`, so an unrelated apply
/// in a shell that happens to export the variable is not blocked; and exempt
/// from `--dry-run`, which mutates nothing and is itself one of the answers.
pub(crate) fn budget_dry_run_env_is_unhonoured(
    env_value: Option<&str>,
    has_disk_budget: bool,
) -> Option<String> {
    let requested = env_value.is_some_and(|v| v != "0");
    if !requested || !has_disk_budget {
        return None;
    }
    Some(
        "FORJAR_BUDGET_DRY_RUN is set, but this apply would reclaim disk anyway. \
         That variable belongs to the generated reaper and is read on the TARGET; \
         it survives neither `sudo` (env_reset) nor `ssh` (no SendEnv), so forjar \
         cannot honour it and will not pretend to. Two previews do work: \
         `forjar apply --dry-run` plans and executes nothing, and \
         `forjar codegen -r <id> --phase reaper > /tmp/reaper.sh && sh /tmp/reaper.sh` \
         runs the real reclaim logic in preview mode and deletes nothing. \
         Unset FORJAR_BUDGET_DRY_RUN to apply for real."
            .to_string(),
    )
}

/// Does the (already filtered and scoped) config still hold a `disk_budget`?
///
/// Honours the same selectors the executor does, so the gate cannot refuse an
/// apply that would not have touched a budget at all.
pub(crate) fn scope_holds_a_disk_budget(
    config: &crate::core::types::ForjarConfig,
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
) -> bool {
    config.resources.iter().any(|(id, r)| {
        r.resource_type == crate::core::types::ResourceType::DiskBudget
            && resource_filter.is_none_or(|f| id == f)
            && tag_filter.is_none_or(|t| r.tags.iter().any(|x| x == t))
            && machine_filter.is_none_or(|m| r.machine.iter().any(|x| x == m))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, Resource, ResourceType};

    #[test]
    fn refuses_when_a_budget_is_in_scope() {
        let msg = budget_dry_run_env_is_unhonoured(Some("1"), true).expect("must refuse");
        assert!(
            msg.contains("forjar codegen"),
            "must name a real preview: {msg}"
        );
        assert!(msg.contains("--dry-run"));
        assert!(msg.contains("--phase reaper"));
    }

    #[test]
    fn zero_is_not_a_request() {
        assert!(budget_dry_run_env_is_unhonoured(Some("0"), true).is_none());
    }

    #[test]
    fn unset_is_not_a_request() {
        assert!(budget_dry_run_env_is_unhonoured(None, true).is_none());
    }

    #[test]
    fn does_not_block_an_apply_that_holds_no_budget() {
        assert!(budget_dry_run_env_is_unhonoured(Some("1"), false).is_none());
    }

    fn cfg(kinds: &[(&str, ResourceType)]) -> crate::core::types::ForjarConfig {
        let mut c = crate::core::types::ForjarConfig::default();
        for (id, t) in kinds {
            c.resources.insert(
                (*id).to_string(),
                Resource {
                    resource_type: t.clone(),
                    machine: MachineTarget::Single("intel".into()),
                    tags: vec!["disk".into()],
                    ..Default::default()
                },
            );
        }
        c
    }

    #[test]
    fn scope_sees_a_budget_only_when_the_filters_select_it() {
        let c = cfg(&[
            ("pkg", ResourceType::Package),
            ("budget", ResourceType::DiskBudget),
        ]);
        assert!(scope_holds_a_disk_budget(&c, None, None, None));
        assert!(scope_holds_a_disk_budget(&c, None, Some("budget"), None));
        assert!(!scope_holds_a_disk_budget(&c, None, Some("pkg"), None));
        assert!(scope_holds_a_disk_budget(&c, Some("intel"), None, None));
        assert!(!scope_holds_a_disk_budget(&c, Some("lambda"), None, None));
        assert!(scope_holds_a_disk_budget(&c, None, None, Some("disk")));
        assert!(!scope_holds_a_disk_budget(&c, None, None, Some("web")));
    }

    #[test]
    fn a_config_with_no_budget_is_never_in_scope() {
        let c = cfg(&[("pkg", ResourceType::Package)]);
        assert!(!scope_holds_a_disk_budget(&c, None, None, None));
    }
}
