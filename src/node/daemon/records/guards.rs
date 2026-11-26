//! Ref, decision, schema, and bound validators.

pub(super) fn validate_decision(decision: &str) -> crate::error::Result<()> {
    if matches!(decision, "pass" | "deny" | "fail") {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!("invalid node control decision `{decision}`")))
    }
}

pub(super) fn validate_live_send_timeout(timeout_ms: u64) -> crate::error::Result<()> {
    if timeout_ms == 0 {
        return Err(crate::error::Failure::invalid_harness("node control live send timeout must be positive"));
    }
    if timeout_ms > super::settings::MAX_CONTROL_LIVE_SEND_TIMEOUT_MS {
        return Err(crate::error::Failure::invalid_harness(format!(
            "node control live send timeout exceeds bounded limit {MAX_CONTROL_LIVE_SEND_TIMEOUT_MS}",
            MAX_CONTROL_LIVE_SEND_TIMEOUT_MS = super::settings::MAX_CONTROL_LIVE_SEND_TIMEOUT_MS
        )));
    }
    Ok(())
}

pub(super) fn validate_live_send_attempts(max_attempts: u64) -> crate::error::Result<()> {
    if max_attempts == 0 {
        return Err(crate::error::Failure::invalid_harness("node control live send attempts must be positive"));
    }
    if max_attempts > super::settings::MAX_CONTROL_LIVE_SEND_ATTEMPTS {
        return Err(crate::error::Failure::invalid_harness(format!(
            "node control live send attempts exceed bounded limit {MAX_CONTROL_LIVE_SEND_ATTEMPTS}",
            MAX_CONTROL_LIVE_SEND_ATTEMPTS = super::settings::MAX_CONTROL_LIVE_SEND_ATTEMPTS
        )));
    }
    Ok(())
}

pub(super) fn validate_listener_event_limit(max_events: u64) -> crate::error::Result<()> {
    if max_events > super::settings::MAX_CONTROL_LIVE_LISTENER_EVENTS {
        return Err(crate::error::Failure::invalid_harness(format!(
            "node control live listener max events exceeds bounded limit {MAX_CONTROL_LIVE_LISTENER_EVENTS}",
            MAX_CONTROL_LIVE_LISTENER_EVENTS = super::settings::MAX_CONTROL_LIVE_LISTENER_EVENTS
        )));
    }
    Ok(())
}

pub(super) fn validate_supervisor_policy_bounds(value: u64, label: &str) -> crate::error::Result<()> {
    if value > super::settings::MAX_CONTROL_SERVICE_TICKS {
        return Err(crate::error::Failure::invalid_harness(format!(
            "node control supervisor policy {label} exceeds bounded limit {MAX_CONTROL_SERVICE_TICKS}",
            MAX_CONTROL_SERVICE_TICKS = super::settings::MAX_CONTROL_SERVICE_TICKS
        )));
    }
    Ok(())
}

pub(super) fn validate_service_tick_limit(max_ticks: u64) -> crate::error::Result<usize> {
    if max_ticks == 0 {
        return Err(crate::error::Failure::invalid_harness("node control service max ticks must be positive"));
    }
    if max_ticks > super::settings::MAX_CONTROL_SERVICE_TICKS {
        return Err(crate::error::Failure::invalid_harness(format!(
            "node control service max ticks exceeds bounded limit {MAX_CONTROL_SERVICE_TICKS}",
            MAX_CONTROL_SERVICE_TICKS = super::settings::MAX_CONTROL_SERVICE_TICKS
        )));
    }
    usize::try_from(max_ticks).map_err(|_| {
        crate::error::Failure::invalid_harness("node control service max ticks does not fit this platform")
    })
}

pub(super) fn validate_loop_request_limit(max_requests: u64) -> crate::error::Result<usize> {
    if max_requests == 0 {
        return Err(crate::error::Failure::invalid_harness("node control loop max requests must be positive"));
    }
    if max_requests > super::settings::MAX_CONTROL_LOOP_REQUESTS {
        return Err(crate::error::Failure::invalid_harness(format!(
            "node control loop max requests exceeds bounded limit {MAX_CONTROL_LOOP_REQUESTS}",
            MAX_CONTROL_LOOP_REQUESTS = super::settings::MAX_CONTROL_LOOP_REQUESTS
        )));
    }
    usize::try_from(max_requests).map_err(|_| {
        crate::error::Failure::invalid_harness("node control loop max requests does not fit this platform")
    })
}

pub(super) fn validate_ingress_refs(refs: &[String], label: &str) -> crate::error::Result<()> {
    for reference in refs {
        validate_ingress_ref(reference, label)?;
    }
    Ok(())
}

pub(super) fn validate_ingress_ref(reference: &str, label: &str) -> crate::error::Result<()> {
    crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
        crate::error::Failure::invalid_harness(format!("{label} must be a canonical blake3 content ref: {error}"))
    })
}

pub(super) fn validate_member_ref(actual: &str, expected: &str, label: &str) -> crate::error::Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!("{label} ref {actual} does not match {expected}")))
    }
}

pub(super) fn validate_optional_member_ref(
    value: Option<&preserves::IOValue>,
    expected_ref: Option<&str>,
    label: &str,
) -> crate::error::Result<()> {
    match (value, expected_ref) {
        (Some(value), Some(expected)) => {
            validate_member_ref(&crate::preserves_rail::canonical_hash(value)?, expected, label)
        }
        (Some(_), None) => Err(crate::error::Failure::invalid_harness(format!("{label} value present without ref"))),
        (None, Some(expected)) => {
            Err(crate::error::Failure::invalid_harness(format!("{label} ref {expected} present without value")))
        }
        (None, None) => Ok(()),
    }
}

pub(super) fn require_schema(
    value: &preserves::Value<preserves::IOValue>,
    expected: &str,
    context: &str,
) -> crate::error::Result<()> {
    let actual = value
        .as_string()
        .ok_or_else(|| crate::error::Failure::invalid_harness(format!("{context} schema must be a string")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!(
            "{context} schema mismatch: expected {expected}, got {actual}"
        )))
    }
}
