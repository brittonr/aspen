// r[impl molten.consensus.live_register_profile]
//! Exact, bounded admission for the single public service-registry register.

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Register {
    pub schema: alloc::string::String,
    pub group: alloc::string::String,
    pub service: alloc::string::String,
    pub keys: alloc::vec::Vec<alloc::string::String>,
    pub initial_value: Option<i64>,
    pub values: alloc::vec::Vec<i64>,
    pub generator_ref: alloc::string::String,
    pub adapter_ref: alloc::string::String,
    pub model_ref: alloc::string::String,
    pub outcome_ref: alloc::string::String,
    pub seed: u64,
    pub read_weight: u32,
    pub write_weight: u32,
    pub clients: alloc::vec::Vec<alloc::string::String>,
    pub endpoints: alloc::vec::Vec<alloc::string::String>,
    pub concurrency: u32,
    pub operations: u32,
    pub control_operations: u32,
    pub consistency_model: alloc::string::String,
    pub recovery: RecoveryBounds,
    pub evidence: EvidenceBounds,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryBounds {
    pub timeout_ms: u64,
    pub stable_membership_observations: u32,
    pub require_all_endpoint_reads: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBounds {
    pub max_attempts_per_operation: u32,
    pub max_events: u32,
    pub max_uncertain: u32,
    pub max_bytes: u64,
    pub checker_states: u64,
    pub checker_branches: u64,
    pub checker_memo_bytes: u64,
    pub checker_reductions: u64,
}

/// A register profile that passed [`admit`], bound to its caller-computed canonical reference.
/// Only [`admit`] constructs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    profile: Register,
    profile_ref: alloc::string::String,
}

impl Admitted {
    pub fn profile(&self) -> &Register {
        &self.profile
    }
    pub fn profile_ref(&self) -> &str {
        &self.profile_ref
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issue {
    Schema,
    Identity,
    SingleKey,
    InitialValue,
    Values,
    Clients,
    Bounds,
    Model,
    Recovery,
    Encoding,
}

impl Issue {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Schema => "schema",
            Self::Identity => "identity",
            Self::SingleKey => "single-key",
            Self::InitialValue => "initial-value",
            Self::Values => "values",
            Self::Clients => "clients",
            Self::Bounds => "bounds",
            Self::Model => "model",
            Self::Recovery => "recovery",
            Self::Encoding => "encoding",
        }
    }
}

impl core::fmt::Display for Issue {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "register profile admission denied: {}", self.as_str())
    }
}

impl core::error::Error for Issue {}

/// Admits `profile` and binds the reference computed by `canonical_ref`.
///
/// The canonical identity (BLAKE3 over canonical packed Preserves) is a shell concern; the port is
/// called only after every structural bound passed, and its failure is reported unchanged.
// r[impl molten.testing.live_reliability_boundary]
pub fn admit<F>(profile: &Register, canonical_ref: F) -> Result<Admitted, Issue>
where F: FnOnce(&Register) -> Result<alloc::string::String, Issue> {
    if profile.schema != "molten.live-register-profile.v1" {
        return Err(Issue::Schema);
    }
    if profile.keys.len() != 1 || !bounded_name(&profile.keys[0]) {
        return Err(Issue::SingleKey);
    }
    if !bounded_name(&profile.group)
        || !bounded_name(&profile.service)
        || [
            &profile.generator_ref,
            &profile.adapter_ref,
            &profile.model_ref,
            &profile.outcome_ref,
        ]
        .iter()
        .any(|reference| !is_ref(reference))
    {
        return Err(Issue::Identity);
    }
    if profile.initial_value.is_some() {
        return Err(Issue::InitialValue);
    }
    if profile.values.is_empty() || profile.values.len() > 256 || !unique(&profile.values) {
        return Err(Issue::Values);
    }
    if profile.clients.is_empty()
        || profile.clients.len() > 64
        || !unique(&profile.clients)
        || profile.clients.iter().any(|value| !bounded_name(value))
        || profile.endpoints.is_empty()
        || profile.endpoints.len() > 64
        || !unique(&profile.endpoints)
        || profile.endpoints.iter().any(|value| !bounded_name(value))
    {
        return Err(Issue::Clients);
    }
    validate_bounds(profile)?;
    if profile.consistency_model != "register" {
        return Err(Issue::Model);
    }
    if profile.recovery.timeout_ms == 0
        || profile.recovery.timeout_ms > 3_600_000
        || profile.recovery.stable_membership_observations == 0
        || profile.recovery.stable_membership_observations > 1_024
        || !profile.recovery.require_all_endpoint_reads
    {
        return Err(Issue::Recovery);
    }
    Ok(Admitted {
        profile: profile.clone(),
        profile_ref: canonical_ref(profile)?,
    })
}

fn validate_bounds(profile: &Register) -> Result<(), Issue> {
    let evidence = &profile.evidence;
    let attempts = u64::from(profile.operations) * u64::from(evidence.max_attempts_per_operation);
    if profile.operations == 0
        || profile.operations > 4_096
        || profile.control_operations == 0
        || profile.control_operations > profile.operations
        || profile.concurrency == 0
        || u64::from(profile.concurrency) > profile.clients.len() as u64
        || profile.read_weight == 0
        || profile.write_weight == 0
        || profile.read_weight.checked_add(profile.write_weight).is_none()
        || evidence.max_attempts_per_operation == 0
        || evidence.max_attempts_per_operation > 16
        || u64::from(profile.control_operations) + profile.endpoints.len() as u64 > u64::from(profile.operations)
        || u64::from(evidence.max_events) < attempts * 2
        || evidence.max_events > 131_072
        || u64::from(evidence.max_uncertain) > attempts
        || evidence.max_bytes == 0
        || evidence.max_bytes > 134_217_728
        || evidence.checker_states == 0
        || evidence.checker_states > 1_000_000
        || evidence.checker_branches == 0
        || evidence.checker_branches > 10_000_000
        || evidence.checker_memo_bytes == 0
        || evidence.checker_memo_bytes > 134_217_728
        || evidence.checker_reductions == 0
        || evidence.checker_reductions > 1_000_000
    {
        return Err(Issue::Bounds);
    }
    Ok(())
}

pub fn is_ref(value: &str) -> bool {
    value.strip_prefix("blake3:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn bounded_name(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
}

fn unique<T: Ord>(values: &[T]) -> bool {
    values.iter().collect::<alloc::collections::BTreeSet<_>>().len() == values.len()
}
