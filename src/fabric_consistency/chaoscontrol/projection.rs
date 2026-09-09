use crate::error::Result;

// The genesis and transition framing mirrors the archived ChaosControl
// `smr-chain` contract exactly: separate versioned BLAKE3 domains, big-endian
// length-prefixed parts, and a `blake3:` lowercase-hex digest string. Changing
// either side breaks cross-implementation digest equality.
pub const CHAOSCONTROL_SMR_GENESIS_DOMAIN: &[u8] = b"chaoscontrol.smr-chain.genesis.v1\0";
pub const CHAOSCONTROL_SMR_TRANSITION_DOMAIN: &[u8] = b"chaoscontrol.smr-chain.transition.v1\0";
pub const CHAOSCONTROL_SMR_DIGEST_PREFIX: &str = "blake3:";
pub const FIRST_CHAOSCONTROL_COMMAND_INDEX: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaosControlObservationMode {
    Lossless,
    Sampled,
}

impl ChaosControlObservationMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lossless => "lossless",
            Self::Sampled => "sampled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChaosControlConformanceProfile {
    pub chaoscontrol_profile_ref: String,
    pub observation_mode: ChaosControlObservationMode,
    pub max_command_bytes: u64,
    pub max_projected_observations: u64,
}

impl ChaosControlConformanceProfile {
    // r[impl molten.consensus.chaoscontrol_chain_observation]
    pub fn validate(&self) -> Result<()> {
        crate::preserves_rail::validate_content_ref(&self.chaoscontrol_profile_ref)?;
        if self.max_command_bytes == 0 || self.max_projected_observations == 0 {
            return Err(crate::error::MoltenError::invalid_harness(
                "ChaosControl conformance profile requires nonzero observation bounds",
            ));
        }
        Ok(())
    }

    // Accepted conformance uses lossless observation mode; a sampled profile
    // cannot support external conformance evidence.
    pub fn admits_conformance(&self) -> Result<()> {
        if self.observation_mode != ChaosControlObservationMode::Lossless {
            return Err(crate::error::MoltenError::invalid_harness(
                "ChaosControl conformance requires lossless observation mode",
            ));
        }
        Ok(())
    }
}

// One committed control-plane apply observed on the admitted Molten
// application path. The application receipt ref is the observer-path binding:
// a chain digest that did not flow through the committed application port
// cannot carry a valid receipt and is rejected before it can support
// external conformance evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedApplyObservation {
    pub group_ref: String,
    pub replica_ref: String,
    pub command_index: u64,
    pub operation_ref: String,
    pub command_ref: String,
    pub command_bytes: Vec<u8>,
    pub application_state_ref: String,
    pub lifecycle_generation: u64,
    pub application_receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChaosControlChainObservation {
    pub chaoscontrol_profile_ref: String,
    pub group_ref: String,
    pub replica_ref: String,
    pub command_index: u64,
    pub operation_ref: String,
    pub command_ref: String,
    pub command_bytes: Vec<u8>,
    pub prior_digest: String,
    pub next_digest: String,
    pub application_state_ref: String,
    pub lifecycle_generation: u64,
    pub application_receipt_ref: String,
}

// r[impl molten.consensus.chaoscontrol_chain_observation]
#[derive(Debug, Clone)]
pub struct ChaosControlChainProjector {
    profile: ChaosControlConformanceProfile,
    initial_state_ref: String,
    genesis_digest: String,
    next_command_index: u64,
    prior_digest: String,
    last_lifecycle_generation: u64,
    projected_count: u64,
}

impl ChaosControlChainProjector {
    // Binds the canonical initial-state ref and derives the cohort genesis
    // digest with the exact ChaosControl genesis framing.
    pub fn bind(profile: ChaosControlConformanceProfile, initial_state_ref: &str) -> Result<Self> {
        profile.validate()?;
        crate::preserves_rail::validate_content_ref(initial_state_ref)?;
        let genesis_digest = chaoscontrol_chain_genesis(&profile.chaoscontrol_profile_ref, initial_state_ref);
        Ok(Self {
            profile,
            initial_state_ref: initial_state_ref.to_string(),
            prior_digest: genesis_digest.clone(),
            genesis_digest,
            next_command_index: FIRST_CHAOSCONTROL_COMMAND_INDEX,
            last_lifecycle_generation: 0,
            projected_count: 0,
        })
    }

    pub fn initial_state_ref(&self) -> &str {
        &self.initial_state_ref
    }

    pub fn genesis_digest(&self) -> &str {
        &self.genesis_digest
    }

    pub const fn next_command_index(&self) -> u64 {
        self.next_command_index
    }

    pub const fn projected_count(&self) -> u64 {
        self.projected_count
    }

    // Projects one bounded chain observation from a committed application
    // transition. The committed application path in Molten is contiguous, so
    // stale, duplicated, noncontiguous, or generation-regressed applies are
    // typed denials rather than observations.
    pub fn project(&mut self, apply: &CommittedApplyObservation) -> Result<ChaosControlChainObservation> {
        self.profile.admits_conformance()?;
        self.validate_committed_apply(apply)?;
        if apply.command_index < self.next_command_index {
            return Err(crate::error::MoltenError::invalid_harness(
                "ChaosControl projection denies a stale or duplicated committed apply",
            ));
        }
        if apply.command_index > self.next_command_index {
            return Err(crate::error::MoltenError::invalid_harness(
                "ChaosControl projection denies a noncontiguous committed apply",
            ));
        }
        if apply.lifecycle_generation < self.last_lifecycle_generation {
            return Err(crate::error::MoltenError::invalid_harness(
                "ChaosControl projection denies a lifecycle generation regression",
            ));
        }
        if self.projected_count >= self.profile.max_projected_observations {
            return Err(crate::error::MoltenError::invalid_harness("ChaosControl projection bound exceeded"));
        }
        let next_digest = chaoscontrol_chain_transition(
            &self.profile.chaoscontrol_profile_ref,
            apply.command_index,
            &self.prior_digest,
            &apply.command_bytes,
        );
        let observation = ChaosControlChainObservation {
            chaoscontrol_profile_ref: self.profile.chaoscontrol_profile_ref.clone(),
            group_ref: apply.group_ref.clone(),
            replica_ref: apply.replica_ref.clone(),
            command_index: apply.command_index,
            operation_ref: apply.operation_ref.clone(),
            command_ref: apply.command_ref.clone(),
            command_bytes: apply.command_bytes.clone(),
            prior_digest: self.prior_digest.clone(),
            next_digest,
            application_state_ref: apply.application_state_ref.clone(),
            lifecycle_generation: apply.lifecycle_generation,
            application_receipt_ref: apply.application_receipt_ref.clone(),
        };
        self.prior_digest = observation.next_digest.clone();
        self.next_command_index = self
            .next_command_index
            .checked_add(1)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("ChaosControl command index overflow"))?;
        self.last_lifecycle_generation = apply.lifecycle_generation;
        self.projected_count += 1;
        Ok(observation)
    }

    fn validate_committed_apply(&self, apply: &CommittedApplyObservation) -> Result<()> {
        for reference in [
            &apply.group_ref,
            &apply.replica_ref,
            &apply.operation_ref,
            &apply.command_ref,
            &apply.application_state_ref,
            &apply.application_receipt_ref,
        ] {
            crate::preserves_rail::validate_content_ref(reference)?;
        }
        if apply.command_bytes.is_empty() {
            return Err(crate::error::MoltenError::invalid_harness(
                "ChaosControl projection denies an empty committed command",
            ));
        }
        if apply.command_bytes.len() as u64 > self.profile.max_command_bytes {
            return Err(crate::error::MoltenError::invalid_harness(
                "ChaosControl committed command exceeds the profile command bound",
            ));
        }
        if apply.command_index < FIRST_CHAOSCONTROL_COMMAND_INDEX {
            return Err(crate::error::MoltenError::invalid_harness("ChaosControl command index must start at one"));
        }
        Ok(())
    }
}

pub fn chaoscontrol_chain_genesis(profile_ref: &str, initial_state_ref: &str) -> String {
    chaoscontrol_domain_hash(CHAOSCONTROL_SMR_GENESIS_DOMAIN, &[profile_ref.as_bytes(), initial_state_ref.as_bytes()])
}

pub fn chaoscontrol_chain_transition(
    profile_ref: &str,
    command_index: u64,
    prior_digest: &str,
    command: &[u8],
) -> String {
    chaoscontrol_domain_hash(CHAOSCONTROL_SMR_TRANSITION_DOMAIN, &[
        profile_ref.as_bytes(),
        &command_index.to_be_bytes(),
        prior_digest.as_bytes(),
        &(command.len() as u64).to_be_bytes(),
        command,
    ])
}

fn chaoscontrol_domain_hash(domain: &[u8], parts: &[&[u8]]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    for part in parts {
        hasher.update(&(part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    format!("{CHAOSCONTROL_SMR_DIGEST_PREFIX}{}", hasher.finalize().to_hex())
}
