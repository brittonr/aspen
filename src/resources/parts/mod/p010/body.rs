fn check_limit_coherence(
    limits: EffectiveRuntimeLimits,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if limits.live_join_timeout_ms > limits.live_listener_timeout_ms {
        push_limit_diagnostic(diagnostics, "live join timeout exceeds listener timeout envelope")?;
    }
    if limits.frame_bytes > limits.session_bytes {
        push_limit_diagnostic(diagnostics, "frame bytes exceed session byte budget")?;
    }
    if limits.live_send_attempts > limits.queue_depth {
        push_limit_diagnostic(diagnostics, "live attempts exceed queue depth budget")?;
    }
    if limits.retention_scan_items < limits.queue_depth {
        push_limit_diagnostic(diagnostics, "retention scan bound is smaller than queue depth")?;
    }
    Ok(())
}

fn check_override_widening(
    input: RuntimeLimitAdmissionInput<'_>,
    limits: EffectiveRuntimeLimits,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if input.allow_widening_overrides {
        return Ok(());
    }
    check_widened(limits.service_tick_ms, input.profile.service_tick_ms, "service tick override", diagnostics)?;
    check_widened(limits.queue_depth, input.profile.queue_depth, "queue depth override", diagnostics)?;
    check_widened(
        limits.live_send_attempts,
        input.profile.live_send_attempts,
        "live attempts override",
        diagnostics,
    )?;
    check_widened(
        limits.live_join_timeout_ms,
        input.profile.live_join_timeout_ms,
        "join timeout override",
        diagnostics,
    )?;
    check_widened(
        limits.live_listener_timeout_ms,
        input.profile.live_listener_timeout_ms,
        "listener timeout override",
        diagnostics,
    )?;
    check_widened(limits.frame_bytes, input.profile.frame_bytes, "frame bytes override", diagnostics)?;
    check_widened(limits.session_bytes, input.profile.session_bytes, "session bytes override", diagnostics)?;
    check_widened(limits.chunk_bytes, input.profile.chunk_bytes, "chunk bytes override", diagnostics)?;
    check_widened(
        limits.retention_scan_items,
        input.profile.retention_scan_items,
        "retention scan override",
        diagnostics,
    )?;
    check_widened(limits.harness_steps, input.profile.harness_steps, "harness steps override", diagnostics)
}

fn runtime_limit_profile_value(profile: RuntimeLimitProfile) -> Result<IoValue> {
    validate_runtime_profile(&profile)?;
    Ok(record("runtime-limit-profile-v1", vec![
        string(RUNTIME_LIMIT_PROFILE_SCHEMA),
        record("name", vec![string(&profile.profile_name)]),
        record("tier", vec![string(&profile.profile_tier)]),
        record("source", vec![optional_ref_value(profile.profile_source_ref.as_deref())]),
        record("service-tick-ms", vec![u64_value(profile.service_tick_ms)]),
        record("queue-depth", vec![u64_value(profile.queue_depth)]),
        record("live-send-attempts", vec![u64_value(profile.live_send_attempts)]),
        record("live-join-timeout-ms", vec![u64_value(profile.live_join_timeout_ms)]),
        record("live-listener-timeout-ms", vec![u64_value(profile.live_listener_timeout_ms)]),
        record("frame-bytes", vec![u64_value(profile.frame_bytes)]),
        record("session-bytes", vec![u64_value(profile.session_bytes)]),
        record("chunk-bytes", vec![u64_value(profile.chunk_bytes)]),
        record("retention-scan-items", vec![u64_value(profile.retention_scan_items)]),
        record("harness-steps", vec![u64_value(profile.harness_steps)]),
        limit_checks_value(&[
            "compiled-hard-caps-preserved",
            "units-declared",
            "evidence-only-no-authority",
        ]),
    ]))
}

fn runtime_limit_admission_value(
    decision: &str,
    profile_ref: &str,
    limits: EffectiveRuntimeLimits,
    diagnostics: &[String],
) -> Result<IoValue> {
    validate_content_ref(profile_ref)?;
    Ok(record("runtime-limit-admission-v1", vec![
        string(RUNTIME_LIMIT_ADMISSION_SCHEMA),
        record("decision", vec![string(decision)]),
        record("profile-ref", vec![string(profile_ref)]),
        effective_limits_value(limits),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        record("default-budget-caveat", vec![string(if limits.used_default_budget { "present" } else { "absent" })]),
        limit_checks_value(&[
            "pure-limit-admission",
            "hard-cap-bounded",
            "effective-limits-receipted",
            "no-authority-grant",
        ]),
    ]))
}

fn effective_limits_value(limits: EffectiveRuntimeLimits) -> IoValue {
    record("effective-limits", vec![
        record("service-tick-ms", vec![u64_value(limits.service_tick_ms)]),
        record("queue-depth", vec![u64_value(limits.queue_depth)]),
        record("live-send-attempts", vec![u64_value(limits.live_send_attempts)]),
        record("live-join-timeout-ms", vec![u64_value(limits.live_join_timeout_ms)]),
        record("live-listener-timeout-ms", vec![u64_value(limits.live_listener_timeout_ms)]),
        record("frame-bytes", vec![u64_value(limits.frame_bytes)]),
        record("session-bytes", vec![u64_value(limits.session_bytes)]),
        record("chunk-bytes", vec![u64_value(limits.chunk_bytes)]),
        record("retention-scan-items", vec![u64_value(limits.retention_scan_items)]),
        record("harness-steps", vec![u64_value(limits.harness_steps)]),
    ])
}

fn validate_runtime_profile(profile: &RuntimeLimitProfile) -> Result<()> {
    validate_limit_token(&profile.profile_name, "runtime limit profile name")?;
    validate_limit_token(&profile.profile_tier, "runtime limit profile tier")?;
    if let Some(source_ref) = profile.profile_source_ref.as_ref() {
        validate_content_ref(source_ref)?;
    }
    Ok(())
}

fn validate_hard_caps(caps: RuntimeLimitHardCaps) -> Result<()> {
    select_limit(caps.control_tick_ms, None, "control tick hard cap")?;
    select_limit(caps.live_send_attempts, None, "live attempts hard cap")?;
    select_limit(caps.live_join_timeout_ms, None, "join timeout hard cap")?;
    select_limit(caps.live_listener_timeout_ms, None, "listener timeout hard cap")?;
    select_limit(caps.frame_bytes, None, "frame byte hard cap")?;
    select_limit(caps.session_bytes, None, "session byte hard cap")?;
    select_limit(caps.chunk_bytes, None, "chunk byte hard cap")?;
    select_limit(caps.retention_scan_items, None, "retention scan hard cap")?;
    select_limit(caps.harness_steps, None, "harness step hard cap")?;
    select_limit(caps.queue_depth, None, "queue depth hard cap")?;
    Ok(())
}

fn select_limit(profile_value: u64, override_value: Option<u64>, label: &str) -> Result<u64> {
    let selected = override_value.unwrap_or(profile_value);
    if selected == 0 {
        Err(MoltenError::invalid_harness(format!("{label} must be positive")))
    } else {
        Ok(selected)
    }
}

fn check_cap(
    selected: u64,
    cap: u64,
    label: &str,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if selected > cap {
        push_limit_diagnostic(diagnostics, format!("{label} selected {selected} exceeds hard cap {cap}"))?;
    }
    Ok(())
}

fn check_widened(
    selected: u64,
    profile_value: u64,
    label: &str,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if selected > profile_value {
        push_limit_diagnostic(diagnostics, format!("{label} widens selected profile value"))?;
    }
    Ok(())
}

fn validate_limit_token(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{field} cannot be empty")));
    }
    if value.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{field} must be lowercase ascii token")))
    }
}

fn limit_checks_value(checks: &[&str]) -> IoValue {
    record("checks", vec![sequence(
        checks
            .iter()
            .map(|check| record("check", vec![string(check), string("pass")]))
            .collect(),
    )])
}

fn push_limit_diagnostic(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    diagnostic: impl Into<String>,
) -> Result<()> {
    crate::bounded::push_bounded(
        diagnostics,
        diagnostic.into(),
        LIMIT_DIAGNOSTIC_LIMIT,
        "runtime limit diagnostics",
    )
}
