//! Pool reconcile decision (#894, slice 1): a pure function from a pool
//! snapshot to one scale action. No I/O; the timer-driven caller applies it.

/// Observed pool state at one reconcile tick. All ages are seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolSnapshot {
    pub hosts: u32,
    pub idle_seats: u32,
    pub busy_seats: u32,
    pub queued_eligible: u32,
    /// Age of the oldest queued eligible job, if any.
    pub oldest_queued_secs: Option<u64>,
    /// Seconds the pool has had 0 busy seats and an empty eligible queue.
    pub quiet_secs: u64,
    /// Age of the oldest host.
    pub oldest_host_age_secs: u64,
}

/// Guards from the pool declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolLimits {
    pub max_hosts: u32,
}

pub const SCALE_UP_AGE_SECS: u64 = 5 * 60;
pub const SCALE_DOWN_QUIET_SECS: u64 = 10 * 60;
pub const HARD_TTL_SECS: u64 = 12 * 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolAction {
    Hold,
    ScaleUp,
    /// Terminate all hosts (idle 10 min, or the 12 h hard TTL hit).
    ScaleToZero,
}

/// Decide one action. TTL first, then scale-to-zero, then scale-up (capped).
pub fn decide(s: &PoolSnapshot, l: &PoolLimits) -> PoolAction {
    if s.hosts > 0 && s.oldest_host_age_secs >= HARD_TTL_SECS && s.busy_seats == 0 {
        return PoolAction::ScaleToZero;
    }
    let queue_empty = s.queued_eligible == 0;
    if s.hosts > 0 && s.busy_seats == 0 && queue_empty && s.quiet_secs >= SCALE_DOWN_QUIET_SECS {
        return PoolAction::ScaleToZero;
    }
    let aged = s.oldest_queued_secs.is_some_and(|a| a > SCALE_UP_AGE_SECS);
    let starved = s.queued_eligible > s.idle_seats;
    if (aged || starved) && !queue_empty && s.hosts < l.max_hosts {
        return PoolAction::ScaleUp;
    }
    PoolAction::Hold
}

#[cfg(test)]
mod tests {
    use super::*;

    const L: PoolLimits = PoolLimits { max_hosts: 2 };
    fn snap() -> PoolSnapshot {
        PoolSnapshot {
            hosts: 0,
            idle_seats: 0,
            busy_seats: 0,
            queued_eligible: 0,
            oldest_queued_secs: None,
            quiet_secs: 0,
            oldest_host_age_secs: 0,
        }
    }

    #[test]
    fn scales_up_when_oldest_job_over_5_min() {
        let s = PoolSnapshot {
            queued_eligible: 1,
            idle_seats: 1,
            oldest_queued_secs: Some(301),
            ..snap()
        };
        assert_eq!(decide(&s, &L), PoolAction::ScaleUp);
        let s = PoolSnapshot {
            oldest_queued_secs: Some(300),
            ..s
        };
        assert_eq!(decide(&s, &L), PoolAction::Hold);
    }

    #[test]
    fn scales_up_when_queue_exceeds_idle_seats() {
        let s = PoolSnapshot {
            hosts: 1,
            queued_eligible: 3,
            idle_seats: 2,
            oldest_queued_secs: Some(1),
            ..snap()
        };
        assert_eq!(decide(&s, &L), PoolAction::ScaleUp);
    }

    #[test]
    fn max_hosts_caps_scale_up() {
        let s = PoolSnapshot {
            hosts: 2,
            queued_eligible: 9,
            oldest_queued_secs: Some(999),
            ..snap()
        };
        assert_eq!(decide(&s, &L), PoolAction::Hold);
    }

    #[test]
    fn scales_to_zero_after_10_min_quiet() {
        let s = PoolSnapshot {
            hosts: 1,
            idle_seats: 2,
            quiet_secs: 600,
            ..snap()
        };
        assert_eq!(decide(&s, &L), PoolAction::ScaleToZero);
        let s = PoolSnapshot {
            quiet_secs: 599,
            ..s
        };
        assert_eq!(decide(&s, &L), PoolAction::Hold);
        let s = PoolSnapshot {
            busy_seats: 1,
            quiet_secs: 9999,
            ..s
        };
        assert_eq!(decide(&s, &L), PoolAction::Hold);
    }

    #[test]
    fn hard_ttl_retires_idle_host_even_with_queue() {
        let s = PoolSnapshot {
            hosts: 1,
            queued_eligible: 1,
            oldest_host_age_secs: HARD_TTL_SECS,
            ..snap()
        };
        assert_eq!(decide(&s, &L), PoolAction::ScaleToZero);
    }

    #[test]
    fn empty_pool_empty_queue_holds() {
        assert_eq!(decide(&snap(), &L), PoolAction::Hold);
    }
}
