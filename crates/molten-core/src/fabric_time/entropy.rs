use super::AdmittedTimeProfile;
use super::TimeProfileKind;
use super::valid_time_id;
use super::valid_time_ref;

const CHOICE_SAMPLE_BYTES: u64 = 8;
const SPLITMIX_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;
const SPLITMIX_MIX_A: u64 = 0xBF58_476D_1CE4_E5B9;
const SPLITMIX_MIX_B: u64 = 0x94D0_49BB_1331_11EB;
const SPLITMIX_SHIFT_A: u32 = 30;
const SPLITMIX_SHIFT_B: u32 = 27;
const SPLITMIX_SHIFT_C: u32 = 31;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntropyMode {
    DeterministicSimulation,
    ProductionCryptographic,
}

impl EntropyMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeterministicSimulation => "deterministic-simulation",
            Self::ProductionCryptographic => "production-cryptographic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntropyReplayClass {
    RecomputeFromExplicitSeed,
    SecretInputRequired,
}

impl EntropyReplayClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RecomputeFromExplicitSeed => "recompute-from-explicit-seed",
            Self::SecretInputRequired => "secret-input-required",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntropyStreamRequest {
    pub profile_ref: String,
    pub stream_id: String,
    pub purpose: String,
    pub capability_ref: String,
    pub generation: u64,
    pub mode: EntropyMode,
    pub explicit_simulation_seed: Option<u64>,
    pub explicit_simulation_seed_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntropyStreamState {
    pub profile_ref: String,
    pub stream_id: String,
    pub purpose: String,
    pub capability_ref: String,
    pub generation: u64,
    pub mode: EntropyMode,
    pub replay_class: EntropyReplayClass,
    pub position_bytes: u64,
    pub deterministic_state: Option<u64>,
    pub deterministic_input_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntropyRequest {
    Bytes { count: u64 },
    BoundedChoice { upper_exclusive: u64 },
}

impl EntropyRequest {
    pub const fn requested_bytes(self) -> u64 {
        match self {
            Self::Bytes { count } => count,
            Self::BoundedChoice { .. } => CHOICE_SAMPLE_BYTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntropyValue {
    Bytes(Vec<u8>),
    Choice(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntropyTransition {
    pub next: EntropyStreamState,
    pub value: EntropyValue,
    pub request_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntropyEvidenceMetadata {
    pub profile_ref: String,
    pub stream_id: String,
    pub purpose: String,
    pub generation: u64,
    pub mode: EntropyMode,
    pub replay_class: EntropyReplayClass,
    pub deterministic_input_ref: Option<String>,
    pub start_position_bytes: u64,
    pub request_bytes: u64,
    pub end_position_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntropyError {
    MalformedStreamId(String),
    MalformedPurpose(String),
    MalformedCapabilityRef(String),
    ProfileMismatch,
    ZeroGeneration,
    StaleGeneration { expected: u64, actual: u64 },
    MissingSimulationSeed,
    MissingSimulationSeedRef,
    MalformedSimulationSeedRef(String),
    SeedForbiddenForProduction,
    WrongMode { expected: EntropyMode, actual: EntropyMode },
    EmptyRequest,
    InvalidChoiceBound,
    RequestLimitExceeded { actual: u64, maximum: u64 },
    TotalLimitExceeded { actual: u64, maximum: u64 },
    SuppliedByteCountMismatch { expected: u64, actual: u64 },
    Overflow,
}

// r[impl molten.fabric_time.entropy]
pub fn open_entropy_stream(
    profile: &AdmittedTimeProfile,
    active_generation: u64,
    request: &EntropyStreamRequest,
) -> Result<EntropyStreamState, EntropyError> {
    if request.profile_ref != profile.profile_ref {
        return Err(EntropyError::ProfileMismatch);
    }
    let expected_mode = match profile.kind {
        TimeProfileKind::Live => EntropyMode::ProductionCryptographic,
        TimeProfileKind::DeterministicSimulation => EntropyMode::DeterministicSimulation,
    };
    if request.mode != expected_mode {
        return Err(EntropyError::WrongMode {
            expected: expected_mode,
            actual: request.mode,
        });
    }
    if !valid_time_id(&request.stream_id) {
        return Err(EntropyError::MalformedStreamId(request.stream_id.clone()));
    }
    if !valid_time_id(&request.purpose) {
        return Err(EntropyError::MalformedPurpose(request.purpose.clone()));
    }
    if !valid_time_ref(&request.capability_ref) {
        return Err(EntropyError::MalformedCapabilityRef(request.capability_ref.clone()));
    }
    if request.generation == 0 {
        return Err(EntropyError::ZeroGeneration);
    }
    ensure_generation(active_generation, request.generation)?;
    let (replay_class, deterministic_state, deterministic_input_ref) = match request.mode {
        EntropyMode::DeterministicSimulation => {
            let seed_ref =
                request.explicit_simulation_seed_ref.clone().ok_or(EntropyError::MissingSimulationSeedRef)?;
            if !valid_time_ref(&seed_ref) {
                return Err(EntropyError::MalformedSimulationSeedRef(seed_ref));
            }
            (
                EntropyReplayClass::RecomputeFromExplicitSeed,
                Some(request.explicit_simulation_seed.ok_or(EntropyError::MissingSimulationSeed)?),
                Some(seed_ref),
            )
        }
        EntropyMode::ProductionCryptographic => {
            if request.explicit_simulation_seed.is_some() || request.explicit_simulation_seed_ref.is_some() {
                return Err(EntropyError::SeedForbiddenForProduction);
            }
            (EntropyReplayClass::SecretInputRequired, None, None)
        }
    };
    Ok(EntropyStreamState {
        profile_ref: request.profile_ref.clone(),
        stream_id: request.stream_id.clone(),
        purpose: request.purpose.clone(),
        capability_ref: request.capability_ref.clone(),
        generation: request.generation,
        mode: request.mode,
        replay_class,
        position_bytes: 0,
        deterministic_state,
        deterministic_input_ref,
    })
}

pub fn draw_deterministic_entropy(
    profile: &AdmittedTimeProfile,
    active_generation: u64,
    state: &EntropyStreamState,
    request: EntropyRequest,
) -> Result<EntropyTransition, EntropyError> {
    validate_request(profile, active_generation, state, request)?;
    if state.mode != EntropyMode::DeterministicSimulation {
        return Err(EntropyError::WrongMode {
            expected: EntropyMode::DeterministicSimulation,
            actual: state.mode,
        });
    }
    let byte_count = request.requested_bytes();
    let output_len = usize::try_from(byte_count).map_err(|_| EntropyError::Overflow)?;
    let seed = state.deterministic_state.ok_or(EntropyError::MissingSimulationSeed)?;
    let bytes = deterministic_stream_bytes(seed, state.position_bytes, output_len)?;
    finish_entropy_transition(state, request, bytes, Some(seed))
}

// The production shell obtains cryptographic bytes externally and supplies
// them here. This core validates purpose, capability, generation, and bounds;
// it neither reads ambient entropy nor treats output bytes as evidence.
pub fn consume_production_entropy(
    profile: &AdmittedTimeProfile,
    active_generation: u64,
    state: &EntropyStreamState,
    request: EntropyRequest,
    supplied_secret_bytes: Vec<u8>,
) -> Result<EntropyTransition, EntropyError> {
    validate_request(profile, active_generation, state, request)?;
    if state.mode != EntropyMode::ProductionCryptographic {
        return Err(EntropyError::WrongMode {
            expected: EntropyMode::ProductionCryptographic,
            actual: state.mode,
        });
    }
    let actual = u64::try_from(supplied_secret_bytes.len()).map_err(|_| EntropyError::Overflow)?;
    let expected = request.requested_bytes();
    if actual != expected {
        return Err(EntropyError::SuppliedByteCountMismatch { expected, actual });
    }
    finish_entropy_transition(state, request, supplied_secret_bytes, None)
}

pub fn entropy_evidence_metadata(
    previous: &EntropyStreamState,
    transition: &EntropyTransition,
) -> EntropyEvidenceMetadata {
    EntropyEvidenceMetadata {
        profile_ref: previous.profile_ref.clone(),
        stream_id: previous.stream_id.clone(),
        purpose: previous.purpose.clone(),
        generation: previous.generation,
        mode: previous.mode,
        replay_class: previous.replay_class,
        deterministic_input_ref: previous.deterministic_input_ref.clone(),
        start_position_bytes: previous.position_bytes,
        request_bytes: transition.request_bytes,
        end_position_bytes: transition.next.position_bytes,
    }
}

fn validate_request(
    profile: &AdmittedTimeProfile,
    active_generation: u64,
    state: &EntropyStreamState,
    request: EntropyRequest,
) -> Result<(), EntropyError> {
    if state.profile_ref != profile.profile_ref {
        return Err(EntropyError::ProfileMismatch);
    }
    ensure_generation(active_generation, state.generation)?;
    let request_bytes = request.requested_bytes();
    if request_bytes == 0 {
        return Err(EntropyError::EmptyRequest);
    }
    if matches!(request, EntropyRequest::BoundedChoice { upper_exclusive: 0 }) {
        return Err(EntropyError::InvalidChoiceBound);
    }
    if request_bytes > profile.max_entropy_request_bytes {
        return Err(EntropyError::RequestLimitExceeded {
            actual: request_bytes,
            maximum: profile.max_entropy_request_bytes,
        });
    }
    let total = state.position_bytes.checked_add(request_bytes).ok_or(EntropyError::Overflow)?;
    if total > profile.max_entropy_total_bytes {
        return Err(EntropyError::TotalLimitExceeded {
            actual: total,
            maximum: profile.max_entropy_total_bytes,
        });
    }
    Ok(())
}

fn finish_entropy_transition(
    state: &EntropyStreamState,
    request: EntropyRequest,
    bytes: Vec<u8>,
    deterministic_state: Option<u64>,
) -> Result<EntropyTransition, EntropyError> {
    let request_bytes = request.requested_bytes();
    let mut next = state.clone();
    next.position_bytes = next.position_bytes.checked_add(request_bytes).ok_or(EntropyError::Overflow)?;
    next.deterministic_state = deterministic_state;
    let value = match request {
        EntropyRequest::Bytes { .. } => EntropyValue::Bytes(bytes),
        EntropyRequest::BoundedChoice { upper_exclusive } => {
            let sample_bytes: [u8; CHOICE_SAMPLE_BYTES as usize] =
                bytes.as_slice().try_into().map_err(|_| EntropyError::Overflow)?;
            let sample = u64::from_le_bytes(sample_bytes);
            EntropyValue::Choice(bounded_choice(sample, upper_exclusive))
        }
    };
    Ok(EntropyTransition {
        next,
        value,
        request_bytes,
    })
}

fn deterministic_stream_bytes(seed: u64, start_position: u64, output_len: usize) -> Result<Vec<u8>, EntropyError> {
    let mut bytes = Vec::with_capacity(output_len);
    let mut absolute = start_position;
    while bytes.len() < output_len {
        let block_index = absolute / CHOICE_SAMPLE_BYTES;
        let byte_index = usize::try_from(absolute % CHOICE_SAMPLE_BYTES).map_err(|_| EntropyError::Overflow)?;
        let prior_blocks = SPLITMIX_GAMMA.wrapping_mul(block_index);
        let mut generator = seed.wrapping_add(prior_blocks);
        let sample_bytes = splitmix64_next(&mut generator).to_le_bytes();
        let remaining = output_len - bytes.len();
        let take = remaining.min(sample_bytes.len() - byte_index);
        bytes.extend_from_slice(&sample_bytes[byte_index..byte_index + take]);
        absolute = absolute
            .checked_add(u64::try_from(take).map_err(|_| EntropyError::Overflow)?)
            .ok_or(EntropyError::Overflow)?;
    }
    Ok(bytes)
}

fn splitmix64_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(SPLITMIX_GAMMA);
    let mut mixed = *state;
    mixed = (mixed ^ (mixed >> SPLITMIX_SHIFT_A)).wrapping_mul(SPLITMIX_MIX_A);
    mixed = (mixed ^ (mixed >> SPLITMIX_SHIFT_B)).wrapping_mul(SPLITMIX_MIX_B);
    mixed ^ (mixed >> SPLITMIX_SHIFT_C)
}

fn bounded_choice(sample: u64, upper_exclusive: u64) -> u64 {
    let product = u128::from(sample) * u128::from(upper_exclusive);
    (product >> u64::BITS) as u64
}

fn ensure_generation(expected: u64, actual: u64) -> Result<(), EntropyError> {
    if expected != actual {
        return Err(EntropyError::StaleGeneration { expected, actual });
    }
    Ok(())
}
