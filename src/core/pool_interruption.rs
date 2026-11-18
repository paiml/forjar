//! Pool spot-interruption decisions (#894, slice 2): pure functions. No I/O,
//! no clock read (the caller passes `now_secs`); the timer-driven caller
//! applies the results.

/// Trailing window over which interruptions are counted.
pub const WINDOW_SECS: u64 = 24 * 3600;
/// Interruptions above this percent of launches force on-demand.
pub const FALLBACK_PERCENT: u64 = 5;
/// An interrupted job is re-run at most this many times.
pub const MAX_RERUNS: u32 = 1;

/// What a host does when the interruption notice arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoticeResponse {
    /// Seats stop accepting new jobs.
    pub stop_accepting: bool,
    /// The host deregisters its runners.
    pub deregister: bool,
}

/// Respond to the 2-minute notice with `secs_left` seconds remaining. The
/// response never depends on it: a late notice (even 0 s) still drains and
/// deregisters.
pub fn on_notice(_secs_left: u64) -> NoticeResponse {
    NoticeResponse {
        stop_accepting: true,
        deregister: true,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RerunDecision {
    Rerun,
    /// Already re-run `MAX_RERUNS` times: fail it, never loop.
    GiveUp,
}

/// Decide for a job interrupted with `prior_reruns` earlier re-runs.
pub fn decide_rerun(prior_reruns: u32) -> RerunDecision {
    if prior_reruns < MAX_RERUNS {
        RerunDecision::Rerun
    } else {
        RerunDecision::GiveUp
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityMode {
    Spot,
    OnDemand,
}

/// A timestamp after `now_secs` is a typed error, not an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptionError {
    FutureTimestamp { ts: u64, now_secs: u64 },
}

/// Choose the capacity mode from launch and interruption timestamps (seconds).
/// An event counts when its age is strictly under `WINDOW_SECS`. A timestamp in
/// the future is refused (a unit slip would otherwise count forever). Over 5%
/// means on-demand; exactly 5% stays spot; zero launches in the window is spot.
pub fn capacity_mode(
    launches: &[u64],
    interruptions: &[u64],
    now_secs: u64,
) -> Result<CapacityMode, InterruptionError> {
    let all = launches.iter().chain(interruptions);
    if let Some(&ts) = all.clone().find(|&&t| t > now_secs) {
        return Err(InterruptionError::FutureTimestamp { ts, now_secs });
    }
    let in_window = |ts: &&u64| now_secs - **ts < WINDOW_SECS;
    let l = launches.iter().filter(in_window).count() as u64;
    let i = interruptions.iter().filter(in_window).count() as u64;
    Ok(if l > 0 && i * 100 > l * FALLBACK_PERCENT {
        CapacityMode::OnDemand
    } else {
        CapacityMode::Spot
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_000_000;

    #[test]
    fn notice_stops_accepting_and_deregisters() {
        let r = on_notice(120);
        assert!(r.stop_accepting && r.deregister);
    }

    #[test]
    fn late_notice_still_drains_and_deregisters() {
        for left in [0, 1, u64::MAX] {
            let r = on_notice(left);
            assert!(r.stop_accepting && r.deregister, "{left}");
        }
    }

    #[test]
    fn job_is_rerun_once_and_never_a_second_time() {
        assert_eq!(decide_rerun(0), RerunDecision::Rerun);
        assert_eq!(decide_rerun(MAX_RERUNS), RerunDecision::GiveUp);
        assert_eq!(decide_rerun(7), RerunDecision::GiveUp);
    }

    #[test]
    fn exactly_5_percent_stays_spot_above_falls_back() {
        let launches: Vec<u64> = vec![NOW - 10; 100];
        let five = vec![NOW - 5; 5];
        let six = vec![NOW - 5; 6];
        assert_eq!(capacity_mode(&launches, &five, NOW), Ok(CapacityMode::Spot));
        assert_eq!(
            capacity_mode(&launches, &six, NOW),
            Ok(CapacityMode::OnDemand)
        );
    }

    #[test]
    fn zero_launches_is_spot_even_with_stray_interruptions() {
        assert_eq!(capacity_mode(&[], &[], NOW), Ok(CapacityMode::Spot));
        assert_eq!(capacity_mode(&[], &[NOW - 1], NOW), Ok(CapacityMode::Spot));
    }

    #[test]
    fn events_outside_the_24h_window_are_ignored() {
        let launches = vec![NOW - 10; 10];
        let old = vec![NOW - WINDOW_SECS; 10];
        assert_eq!(capacity_mode(&launches, &old, NOW), Ok(CapacityMode::Spot));
        let edge = vec![NOW - WINDOW_SECS + 1; 1];
        assert_eq!(
            capacity_mode(&launches, &edge, NOW),
            Ok(CapacityMode::OnDemand)
        );
    }

    #[test]
    fn old_launches_do_not_dilute_recent_interruptions() {
        let mut launches = vec![NOW - WINDOW_SECS - 1; 1000];
        launches.push(NOW - 1);
        let hit = vec![NOW - 1];
        assert_eq!(
            capacity_mode(&launches, &hit, NOW),
            Ok(CapacityMode::OnDemand)
        );
    }

    #[test]
    fn future_timestamps_are_refused_not_counted_as_age_zero() {
        let ok = vec![NOW - 1];
        let bad = vec![NOW + 5];
        let e = Err(InterruptionError::FutureTimestamp {
            ts: NOW + 5,
            now_secs: NOW,
        });
        assert_eq!(capacity_mode(&bad, &ok, NOW), e);
        assert_eq!(capacity_mode(&ok, &bad, NOW), e);
    }

    #[test]
    fn timestamp_equal_to_now_is_accepted() {
        assert_eq!(capacity_mode(&[NOW], &[], NOW), Ok(CapacityMode::Spot));
    }
}
