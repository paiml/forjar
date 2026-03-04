//! Coverage tests: dispatch_notify channels, secrets, check, doctor.
use super::dispatch_notify_custom::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_routing() {
        send_custom_routing_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_routing_notification(Some("http://127.0.0.1:1"), &Ok(()), Path::new("/t.yaml"));
        send_custom_routing_notification(
            Some("http://127.0.0.1:1|P:slack"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_dedup_window() {
        send_custom_dedup_window_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_dedup_window_notification(
            Some("http://127.0.0.1:1|120"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_dedup_window_notification(
            Some("http://127.0.0.1:1|60"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_rate_limit() {
        send_custom_rate_limit_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_rate_limit_notification(
            Some("http://127.0.0.1:1|50"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_rate_limit_notification(
            Some("http://127.0.0.1:1|10"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_backoff() {
        send_custom_backoff_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_backoff_notification(
            Some("http://127.0.0.1:1|linear"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_backoff_notification(
            Some("http://127.0.0.1:1|exp"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_circuit_breaker() {
        send_custom_circuit_breaker_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_circuit_breaker_notification(
            Some("http://127.0.0.1:1|10"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_circuit_breaker_notification(
            Some("http://127.0.0.1:1|3"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_dead_letter() {
        send_custom_dead_letter_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_dead_letter_notification(
            Some("http://127.0.0.1:1|dlq"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_dead_letter_notification(
            Some("http://127.0.0.1:1|dlq"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_escalation() {
        send_custom_escalation_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_escalation_notification(
            Some("http://127.0.0.1:1|warn"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_escalation_notification(
            Some("http://127.0.0.1:1|info"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
        send_custom_escalation_notification(
            Some("http://127.0.0.1:1"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_correlation() {
        send_custom_correlation_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_correlation_notification(
            Some("http://127.0.0.1:1|120s"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_correlation_notification(
            Some("http://127.0.0.1:1|30s"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_sampling() {
        send_custom_sampling_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_sampling_notification(
            Some("http://127.0.0.1:1|50"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_sampling_notification(
            Some("http://127.0.0.1:1|25"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_digest() {
        send_custom_digest_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_digest_notification(
            Some("http://127.0.0.1:1|4h"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_digest_notification(
            Some("http://127.0.0.1:1|1h"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_severity_filter() {
        send_custom_severity_filter_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1|info"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1|warn"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1|critical"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1|warn"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1|info"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
}
