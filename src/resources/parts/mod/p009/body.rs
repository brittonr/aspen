pub const RUNTIME_LIMIT_PROFILE_SCHEMA: &str = "molten.resources.runtime-limit-profile.v1";
pub const RUNTIME_LIMIT_ADMISSION_SCHEMA: &str = "molten.resources.runtime-limit-admission.v1";

const MAX_CONTROL_TICK_MS: u64 = 60_000;
const MAX_LIVE_SEND_ATTEMPTS: u64 = 16;
const MAX_LIVE_TIMEOUT_MS: u64 = 120_000;
const MAX_FRAME_BYTES: u64 = 1_048_576;
const MAX_SESSION_BYTES: u64 = 16_777_216;
const MAX_CHUNK_BYTES: u64 = 8_388_608;
const MAX_RETENTION_SCAN_ITEMS: u64 = 100_000;
const MAX_HARNESS_STEPS: u64 = 10_000;
const MAX_QUEUE_DEPTH: u64 = 4_096;
const DEFAULT_CONTROL_TICK_MS: u64 = 1_000;
const DEFAULT_LIVE_SEND_ATTEMPTS: u64 = 3;
const DEFAULT_LIVE_JOIN_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_LIVE_LISTENER_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_FRAME_BYTES: u64 = 65_536;
const DEFAULT_SESSION_BYTES: u64 = 1_048_576;
const DEFAULT_CHUNK_BYTES: u64 = 262_144;
const DEFAULT_RETENTION_SCAN_ITEMS: u64 = 10_000;
const DEFAULT_HARNESS_STEPS: u64 = 1_000;
const DEFAULT_QUEUE_DEPTH: u64 = 128;
const LIMIT_DIAGNOSTIC_LIMIT: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeLimitHardCaps {
    pub control_tick_ms: u64,
    pub live_send_attempts: u64,
    pub live_join_timeout_ms: u64,
    pub live_listener_timeout_ms: u64,
    pub frame_bytes: u64,
    pub session_bytes: u64,
    pub chunk_bytes: u64,
    pub retention_scan_items: u64,
    pub harness_steps: u64,
    pub queue_depth: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLimitProfile {
    pub profile_name: String,
    pub profile_tier: String,
    pub profile_source_ref: Option<String>,
    pub service_tick_ms: u64,
    pub queue_depth: u64,
    pub live_send_attempts: u64,
    pub live_join_timeout_ms: u64,
    pub live_listener_timeout_ms: u64,
    pub frame_bytes: u64,
    pub session_bytes: u64,
    pub chunk_bytes: u64,
    pub retention_scan_items: u64,
    pub harness_steps: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeLimitOverrides {
    pub service_tick_ms: Option<u64>,
    pub queue_depth: Option<u64>,
    pub live_send_attempts: Option<u64>,
    pub live_join_timeout_ms: Option<u64>,
    pub live_listener_timeout_ms: Option<u64>,
    pub frame_bytes: Option<u64>,
    pub session_bytes: Option<u64>,
    pub chunk_bytes: Option<u64>,
    pub retention_scan_items: Option<u64>,
    pub harness_steps: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveRuntimeLimits {
    pub service_tick_ms: u64,
    pub queue_depth: u64,
    pub live_send_attempts: u64,
    pub live_join_timeout_ms: u64,
    pub live_listener_timeout_ms: u64,
    pub frame_bytes: u64,
    pub session_bytes: u64,
    pub chunk_bytes: u64,
    pub retention_scan_items: u64,
    pub harness_steps: u64,
    pub used_default_budget: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimeLimitAdmissionInput<'a> {
    pub profile: &'a RuntimeLimitProfile,
    pub hard_caps: &'a RuntimeLimitHardCaps,
    pub overrides: &'a RuntimeLimitOverrides,
    pub allow_widening_overrides: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLimitAdmission {
    pub decision: String,
    pub profile_ref: String,
    pub limits: EffectiveRuntimeLimits,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

pub fn default_runtime_limit_hard_caps() -> RuntimeLimitHardCaps {
    RuntimeLimitHardCaps {
        control_tick_ms: MAX_CONTROL_TICK_MS,
        live_send_attempts: MAX_LIVE_SEND_ATTEMPTS,
        live_join_timeout_ms: MAX_LIVE_TIMEOUT_MS,
        live_listener_timeout_ms: MAX_LIVE_TIMEOUT_MS,
        frame_bytes: MAX_FRAME_BYTES,
        session_bytes: MAX_SESSION_BYTES,
        chunk_bytes: MAX_CHUNK_BYTES,
        retention_scan_items: MAX_RETENTION_SCAN_ITEMS,
        harness_steps: MAX_HARNESS_STEPS,
        queue_depth: MAX_QUEUE_DEPTH,
    }
}

pub fn default_runtime_limit_profile() -> RuntimeLimitProfile {
    RuntimeLimitProfile {
        profile_name: "local-default".to_string(),
        profile_tier: "local-fixture".to_string(),
        profile_source_ref: None,
        service_tick_ms: DEFAULT_CONTROL_TICK_MS,
        queue_depth: DEFAULT_QUEUE_DEPTH,
        live_send_attempts: DEFAULT_LIVE_SEND_ATTEMPTS,
        live_join_timeout_ms: DEFAULT_LIVE_JOIN_TIMEOUT_MS,
        live_listener_timeout_ms: DEFAULT_LIVE_LISTENER_TIMEOUT_MS,
        frame_bytes: DEFAULT_FRAME_BYTES,
        session_bytes: DEFAULT_SESSION_BYTES,
        chunk_bytes: DEFAULT_CHUNK_BYTES,
        retention_scan_items: DEFAULT_RETENTION_SCAN_ITEMS,
        harness_steps: DEFAULT_HARNESS_STEPS,
    }
}

pub fn admit_runtime_limits(input: RuntimeLimitAdmissionInput<'_>) -> Result<RuntimeLimitAdmission> {
    validate_runtime_profile(input.profile)?;
    validate_hard_caps(*input.hard_caps)?;
    let limits = apply_limit_overrides(input)?;
    let mut diagnostics = Vec::new();
    collect_limit_diagnostics(input, limits, &mut diagnostics)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let profile_value = runtime_limit_profile_value(input.profile.clone())?;
    let profile_ref = canonical_hash(&profile_value)?;
    let value = runtime_limit_admission_value(&decision, &profile_ref, limits, &diagnostics)?;
    Ok(RuntimeLimitAdmission {
        decision,
        profile_ref,
        limits,
        diagnostics,
        value,
    })
}

fn apply_limit_overrides(input: RuntimeLimitAdmissionInput<'_>) -> Result<EffectiveRuntimeLimits> {
    let profile = input.profile;
    let overrides = input.overrides;
    Ok(EffectiveRuntimeLimits {
        service_tick_ms: select_limit(profile.service_tick_ms, overrides.service_tick_ms, "service tick")?,
        queue_depth: select_limit(profile.queue_depth, overrides.queue_depth, "queue depth")?,
        live_send_attempts: select_limit(profile.live_send_attempts, overrides.live_send_attempts, "live attempts")?,
        live_join_timeout_ms: select_limit(
            profile.live_join_timeout_ms,
            overrides.live_join_timeout_ms,
            "live join timeout",
        )?,
        live_listener_timeout_ms: select_limit(
            profile.live_listener_timeout_ms,
            overrides.live_listener_timeout_ms,
            "live listener timeout",
        )?,
        frame_bytes: select_limit(profile.frame_bytes, overrides.frame_bytes, "frame bytes")?,
        session_bytes: select_limit(profile.session_bytes, overrides.session_bytes, "session bytes")?,
        chunk_bytes: select_limit(profile.chunk_bytes, overrides.chunk_bytes, "chunk bytes")?,
        retention_scan_items: select_limit(
            profile.retention_scan_items,
            overrides.retention_scan_items,
            "retention scan",
        )?,
        harness_steps: select_limit(profile.harness_steps, overrides.harness_steps, "harness steps")?,
        used_default_budget: profile.profile_source_ref.is_none(),
    })
}

fn collect_limit_diagnostics(
    input: RuntimeLimitAdmissionInput<'_>,
    limits: EffectiveRuntimeLimits,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    check_cap(limits.service_tick_ms, input.hard_caps.control_tick_ms, "service tick ms", diagnostics)?;
    check_cap(limits.queue_depth, input.hard_caps.queue_depth, "queue depth", diagnostics)?;
    check_cap(limits.live_send_attempts, input.hard_caps.live_send_attempts, "live send attempts", diagnostics)?;
    check_cap(
        limits.live_join_timeout_ms,
        input.hard_caps.live_join_timeout_ms,
        "live join timeout ms",
        diagnostics,
    )?;
    check_cap(
        limits.live_listener_timeout_ms,
        input.hard_caps.live_listener_timeout_ms,
        "live listener timeout ms",
        diagnostics,
    )?;
    check_cap(limits.frame_bytes, input.hard_caps.frame_bytes, "frame bytes", diagnostics)?;
    check_cap(limits.session_bytes, input.hard_caps.session_bytes, "session bytes", diagnostics)?;
    check_cap(limits.chunk_bytes, input.hard_caps.chunk_bytes, "chunk bytes", diagnostics)?;
    check_cap(
        limits.retention_scan_items,
        input.hard_caps.retention_scan_items,
        "retention scan items",
        diagnostics,
    )?;
    check_cap(limits.harness_steps, input.hard_caps.harness_steps, "harness steps", diagnostics)?;
    check_limit_coherence(limits, diagnostics)?;
    check_override_widening(input, limits, diagnostics)
}
