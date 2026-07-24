use std::collections::BTreeSet;

use preserves::IOValue;

use super::ConsistencyGroupLifecycle;
use super::ConsistencyReadMode;
use super::INITIAL_CONSISTENCY_EPOCH;
use super::MAX_CONSISTENCY_COMMAND_BYTES;
use super::MAX_CONSISTENCY_IDENTIFIER_BYTES;
use super::MAX_CONSISTENCY_IN_FLIGHT_OPERATIONS;
use super::MAX_CONSISTENCY_NON_CLAIM_BYTES;
use super::MAX_CONSISTENCY_NON_CLAIMS;
use super::MAX_CONSISTENCY_POLICY_REFS;
use super::canonical::binding_value;
use crate::error::MoltenError;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyGroupBindingInput {
    pub group_id: String,
    pub extension_id: String,
    pub service_id: String,
    pub service_generation: u64,
    pub application_manifest_ref: String,
    pub engine_algorithm_profile: String,
    pub engine_implementation_profile: String,
    pub membership_ref: String,
    pub config_epoch: u64,
    pub placement_ref: String,
    pub fencing_ref: String,
    pub fencing_epoch: u64,
    pub resource_profile_ref: String,
    pub policy_refs: Vec<String>,
    pub non_claims: Vec<String>,
    pub supported_read_modes: Vec<ConsistencyReadMode>,
    pub max_command_bytes: u64,
    pub max_in_flight_operations: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyGroupBinding {
    pub binding_ref: String,
    pub group_id: String,
    pub extension_id: String,
    pub service_id: String,
    pub service_generation: u64,
    pub application_manifest_ref: String,
    pub engine_algorithm_profile: String,
    pub engine_implementation_profile: String,
    pub membership_ref: String,
    pub config_epoch: u64,
    pub placement_ref: String,
    pub fencing_ref: String,
    pub fencing_epoch: u64,
    pub resource_profile_ref: String,
    pub policy_refs: Vec<String>,
    pub non_claims: Vec<String>,
    pub supported_read_modes: Vec<ConsistencyReadMode>,
    pub max_command_bytes: u64,
    pub max_in_flight_operations: u32,
    pub lifecycle: ConsistencyGroupLifecycle,
    pub value: IOValue,
}

// r[impl molten.fabric_consistency.extension_port]
// r[impl molten.fabric_consistency.group_isolation]
pub fn canonical_consistency_group_binding(input: ConsistencyGroupBindingInput) -> Result<ConsistencyGroupBinding> {
    validate_binding_input(&input)?;
    let lifecycle = ConsistencyGroupLifecycle::Declared;
    let value = binding_value(&input, lifecycle);
    let binding_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ConsistencyGroupBinding {
        binding_ref,
        group_id: input.group_id,
        extension_id: input.extension_id,
        service_id: input.service_id,
        service_generation: input.service_generation,
        application_manifest_ref: input.application_manifest_ref,
        engine_algorithm_profile: input.engine_algorithm_profile,
        engine_implementation_profile: input.engine_implementation_profile,
        membership_ref: input.membership_ref,
        config_epoch: input.config_epoch,
        placement_ref: input.placement_ref,
        fencing_ref: input.fencing_ref,
        fencing_epoch: input.fencing_epoch,
        resource_profile_ref: input.resource_profile_ref,
        policy_refs: input.policy_refs,
        non_claims: input.non_claims,
        supported_read_modes: input.supported_read_modes,
        max_command_bytes: input.max_command_bytes,
        max_in_flight_operations: input.max_in_flight_operations,
        lifecycle,
        value,
    })
}

pub(super) fn validate_identifier(value: &str, label: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_CONSISTENCY_IDENTIFIER_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "{label} must be non-empty and at most {MAX_CONSISTENCY_IDENTIFIER_BYTES} bytes"
        )));
    }
    if !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')) {
        return Err(MoltenError::invalid_harness(format!("{label} contains unsupported characters")));
    }
    Ok(())
}

pub(super) fn validate_content_ref(value: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label}: {error}")))
}

pub(super) fn validate_content_refs(
    refs: &[String],
    maximum: usize,
    label: &str,
    require_non_empty: bool,
) -> Result<()> {
    if require_non_empty && refs.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{label} must not be empty")));
    }
    if refs.len() > maximum {
        return Err(MoltenError::invalid_harness(format!("{label} exceeds maximum count {maximum}")));
    }
    let mut unique = BTreeSet::new();
    for reference in refs {
        validate_content_ref(reference, label)?;
        if !unique.insert(reference.as_str()) {
            return Err(MoltenError::invalid_harness(format!("{label} contains a duplicate")));
        }
    }
    Ok(())
}

fn validate_binding_input(input: &ConsistencyGroupBindingInput) -> Result<()> {
    for (value, label) in [
        (&input.group_id, "consistency group id"),
        (&input.extension_id, "consistency extension id"),
        (&input.service_id, "consistency service id"),
        (&input.engine_algorithm_profile, "consistency algorithm profile"),
        (&input.engine_implementation_profile, "consistency implementation profile"),
    ] {
        validate_identifier(value, label)?;
    }
    if input.service_generation < INITIAL_CONSISTENCY_EPOCH
        || input.config_epoch < INITIAL_CONSISTENCY_EPOCH
        || input.fencing_epoch < INITIAL_CONSISTENCY_EPOCH
    {
        return Err(MoltenError::invalid_harness("consistency generations and epochs must be positive"));
    }
    for (reference, label) in [
        (&input.application_manifest_ref, "application manifest ref"),
        (&input.membership_ref, "membership ref"),
        (&input.placement_ref, "placement ref"),
        (&input.fencing_ref, "fencing ref"),
        (&input.resource_profile_ref, "resource profile ref"),
    ] {
        validate_content_ref(reference, label)?;
    }
    validate_content_refs(&input.policy_refs, MAX_CONSISTENCY_POLICY_REFS, "consistency policy refs", true)?;
    validate_non_claims(&input.non_claims)?;
    validate_read_modes(&input.supported_read_modes)?;
    if input.max_command_bytes == 0 || input.max_command_bytes > MAX_CONSISTENCY_COMMAND_BYTES {
        return Err(MoltenError::invalid_harness("consistency command byte bound is outside the admitted range"));
    }
    if input.max_in_flight_operations == 0 || input.max_in_flight_operations > MAX_CONSISTENCY_IN_FLIGHT_OPERATIONS {
        return Err(MoltenError::invalid_harness(
            "consistency in-flight operation bound is outside the admitted range",
        ));
    }
    Ok(())
}

fn validate_non_claims(non_claims: &[String]) -> Result<()> {
    if non_claims.is_empty() || non_claims.len() > MAX_CONSISTENCY_NON_CLAIMS {
        return Err(MoltenError::invalid_harness("consistency non-claims must be non-empty and bounded"));
    }
    let mut unique = BTreeSet::new();
    for non_claim in non_claims {
        if non_claim.is_empty() || non_claim.len() > MAX_CONSISTENCY_NON_CLAIM_BYTES {
            return Err(MoltenError::invalid_harness("consistency non-claim is empty or over-bound"));
        }
        if !unique.insert(non_claim.as_str()) {
            return Err(MoltenError::invalid_harness("consistency non-claims contain a duplicate"));
        }
    }
    Ok(())
}

fn validate_read_modes(read_modes: &[ConsistencyReadMode]) -> Result<()> {
    if read_modes.is_empty() {
        return Err(MoltenError::invalid_harness("consistency read modes must not be empty"));
    }
    let mut unique = BTreeSet::new();
    for mode in read_modes {
        if *mode == ConsistencyReadMode::Lease {
            return Err(MoltenError::invalid_harness("lease reads are not admitted by the initial consistency port"));
        }
        if !unique.insert(mode.as_str()) {
            return Err(MoltenError::invalid_harness("consistency read modes contain a duplicate"));
        }
    }
    Ok(())
}
