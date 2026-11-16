//! The tail of `cmd_drift_offline`: what happens once drift is found, and the
//! exit verdict. Extracted verbatim (forjar#851: complexity), no logic change.

use super::drift::run_drift_remediation;
use super::drift_report::{run_drift_alert, send_drift_notification};
use crate::core::error::ForjarError;
use crate::core::types;
use std::path::Path;

/// What a drift run does once it has found drift: alert, (maybe) remediate, notify.
pub(super) struct DriftFollowUp<'a> {
    pub(super) config_path: &'a Path,
    pub(super) state_dir: &'a Path,
    pub(super) machine_filter: Option<&'a str>,
    pub(super) alert_cmd: Option<&'a str>,
    pub(super) auto_remediate: bool,
    pub(super) json: bool,
    pub(super) verbose: bool,
}

impl DriftFollowUp<'_> {
    pub(super) fn run(
        &self,
        config: Option<&types::ForjarConfig>,
        total_drift: usize,
        version_drift: usize,
    ) -> Result<(), String> {
        if let Some(cmd) = self.alert_cmd {
            run_drift_alert(cmd, total_drift)?;
        }
        // forjar#613: remediation re-applies every resource, and for a live
        // binary AHEAD of its pin that is a silent downgrade. The fix for a
        // version finding is an edit to the pin, never an apply.
        if self.auto_remediate && version_drift > 0 {
            return Err(format!(
                "refusing --auto-remediate: {version_drift} version-pin finding(s); \
                 re-applying would move a live binary to its pin (possibly a downgrade). \
                 Update the pin in the config instead"
            ));
        }
        if self.auto_remediate {
            run_drift_remediation(
                self.config_path,
                self.state_dir,
                self.machine_filter,
                total_drift,
                self.json,
                self.verbose,
            )?;
        }
        if let Some(cfg) = config {
            send_drift_notification(cfg, total_drift, self.machine_filter);
        }
        Ok(())
    }
}

/// The exit verdict of a drift run.
///
/// PMAT-562: the verdict IS the exit code, with no flag. `Drift detected:
/// 2 resource(s)` followed by rc=0 was measured on yoga and gx10 (paiml/
/// infra#605): the default was fail-open and `--tripwire` was the way to
/// ask for the truth — the forjar#352 class, where the flag that makes the
/// tool honest is opt-in. A drift finding is a reject (1); a remediated
/// finding was still a finding.
pub(super) fn drift_verdict(
    total_drift: usize,
    total_unmeasured: usize,
) -> Result<(), ForjarError> {
    if total_drift > 0 {
        return Err(ForjarError::from_untyped(format!(
            "{total_drift} drift finding(s)"
        )));
    }
    // forjar#549: DRIFT WINS when both are present, because a definite finding
    // is the stronger statement. Unmeasured alone is not a pass: it exits with
    // the connection class (4), because the question was never answered.
    if total_unmeasured > 0 {
        return Err(ForjarError::connection(format!(
            "{}: {total_unmeasured} resource(s)",
            crate::core::error::DRIFT_UNMEASURED_MARKER
        )));
    }

    Ok(())
}

#[cfg(test)]
mod typed_tests {
    use super::*;
    use crate::core::error::ErrorClass;

    #[test]
    fn drift_verdict_clean_is_ok() {
        assert!(drift_verdict(0, 0).is_ok());
    }

    #[test]
    fn drift_wins_over_unmeasured_with_its_exact_message() {
        let e = drift_verdict(2, 3).unwrap_err();
        assert_eq!(e.message(), "2 drift finding(s)");
    }

    #[test]
    fn unmeasured_alone_is_the_connection_class() {
        let e = drift_verdict(0, 3).unwrap_err();
        assert_eq!(e.class(), ErrorClass::Connection);
        assert_eq!(
            e.message(),
            format!(
                "{}: 3 resource(s)",
                crate::core::error::DRIFT_UNMEASURED_MARKER
            )
        );
    }
}
