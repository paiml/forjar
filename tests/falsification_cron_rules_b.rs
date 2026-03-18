//! FJ-3103/3106: Cron source parsing and rules runtime evaluation.
//! Usage: cargo test --test falsification_cron_rules

use forjar::core::types::{EventPattern, EventType, InfraEvent};
use std::collections::HashMap;

fn file_changed_event() -> InfraEvent {
    let mut payload = HashMap::new();
    payload.insert("path".into(), "/etc/nginx/nginx.conf".into());
    InfraEvent {
        event_type: EventType::FileChanged,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("web1".into()),
        payload,
    }
}

#[test]
fn event_type_display() {
    assert_eq!(EventType::FileChanged.to_string(), "file_changed");
    assert_eq!(EventType::CronFired.to_string(), "cron_fired");
    assert_eq!(EventType::Manual.to_string(), "manual");
}

// ============================================================================
// Event pattern matching
// ============================================================================

#[test]
fn pattern_match_with_payload() {
    use forjar::core::types::event_matches_pattern;
    let event = file_changed_event();
    let pattern = EventPattern {
        event_type: EventType::FileChanged,
        match_fields: {
            let mut m = HashMap::new();
            m.insert("path".into(), "/etc/nginx/nginx.conf".into());
            m
        },
    };
    assert!(event_matches_pattern(&event, &pattern));
}

#[test]
fn pattern_no_match_wrong_type() {
    use forjar::core::types::event_matches_pattern;
    let event = file_changed_event();
    let pattern = EventPattern {
        event_type: EventType::Manual,
        match_fields: HashMap::new(),
    };
    assert!(!event_matches_pattern(&event, &pattern));
}

#[test]
fn pattern_no_match_wrong_payload() {
    use forjar::core::types::event_matches_pattern;
    let event = file_changed_event();
    let pattern = EventPattern {
        event_type: EventType::FileChanged,
        match_fields: {
            let mut m = HashMap::new();
            m.insert("path".into(), "/etc/other.conf".into());
            m
        },
    };
    assert!(!event_matches_pattern(&event, &pattern));
}
