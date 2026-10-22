//! The publication eligibility gate.
//!
//! A profile states its own durability requirement before authoritative
//! metadata publication. The gate compares the admitted storage observations
//! against that requirement and emits a receipt that binds the exact content
//! identity, the operation, the receiver set, the retention generation, the
//! placement epoch, and the storage guarantee kind.
//!
//! A replica count alone never satisfies the gate: the receipt must name the
//! storage guarantee, and a local guarantee never satisfies a requirement that
//! asks for replicated availability across fault domains.

use super::*;

/// The storage guarantee the deployment observed for the content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StorageGuaranteeKind {
    /// The content is durable in one local store.
    Local,
    /// The content is durable in replicated stores across distinct fault
    /// domains.
    ReplicatedAcrossFaultDomains { replicas: usize, fault_domains: usize },
}

impl StorageGuaranteeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::ReplicatedAcrossFaultDomains { .. } => "replicated-across-fault-domains",
        }
    }
}

/// The storage observations the gate reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EligibilityObservations {
    /// The content the publication would publish.
    pub content_ref: String,
    /// The operation that publishes the content.
    pub operation_id: String,
    /// The receivers that hold the content.
    pub receiver_set: Vec<String>,
    /// The retention generation the content is protected in.
    pub retention_generation: u64,
    /// The placement epoch the receivers are admitted in.
    pub placement_epoch: u64,
    /// The storage guarantee the deployment observed.
    pub storage_guarantee: StorageGuaranteeKind,
}

/// The receipt the gate emits when the requirement is satisfied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EligibilityReceipt {
    pub content_ref: String,
    pub operation_id: String,
    pub receiver_set: Vec<String>,
    pub retention_generation: u64,
    pub placement_epoch: u64,
    pub storage_guarantee: StorageGuaranteeKind,
    pub receipt_ref: String,
}

/// Why the gate refused a publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EligibilityIssue {
    /// The content reference is not a canonical content reference.
    InvalidContentRef,
    /// The operation identity is empty.
    MissingOperationId,
    /// The receiver set is empty.
    EmptyReceiverSet,
    /// The placement epoch is zero.
    MissingPlacementEpoch,
    /// The observed guarantee is local where the profile requires replicated
    /// availability.
    StorageGuaranteeInsufficient,
    /// The observed guarantee does not cover the required fault domains.
    FaultDomainSpreadInsufficient,
    /// The observed guarantee does not cover the required replicas.
    ReplicaAvailabilityInsufficient,
}

impl EligibilityIssue {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidContentRef => "invalid-content-ref",
            Self::MissingOperationId => "missing-operation-id",
            Self::EmptyReceiverSet => "empty-receiver-set",
            Self::MissingPlacementEpoch => "missing-placement-epoch",
            Self::StorageGuaranteeInsufficient => "storage-guarantee-insufficient",
            Self::FaultDomainSpreadInsufficient => "fault-domain-spread-insufficient",
            Self::ReplicaAvailabilityInsufficient => "replica-availability-insufficient",
        }
    }
}

const ELIGIBILITY_RECEIPT_CONTEXT: &str = "onixresearch.molten.content-replication.eligibility-receipt.v1";

// r[impl molten.observations.eligibility]
/// Check one publication against the profile's eligibility requirement.
///
/// The requirement is the profile's replica policy: the minimum verified
/// replicas and the minimum fault domains the deployment must observe before
/// the metadata reference becomes authoritative.
pub fn check_publication_eligibility(
    policy: &ReplicaPolicy,
    observations: &EligibilityObservations,
) -> Result<EligibilityReceipt, EligibilityIssue> {
    if observations.content_ref.is_empty() || !observations.content_ref.starts_with("blake3:") {
        return Err(EligibilityIssue::InvalidContentRef);
    }
    if observations.operation_id.is_empty() {
        return Err(EligibilityIssue::MissingOperationId);
    }
    if observations.receiver_set.is_empty() {
        return Err(EligibilityIssue::EmptyReceiverSet);
    }
    if observations.placement_epoch == 0 {
        return Err(EligibilityIssue::MissingPlacementEpoch);
    }
    let (replicas, fault_domains) = match observations.storage_guarantee {
        StorageGuaranteeKind::Local => {
            if policy.minimum_verified_replicas > 1 || policy.minimum_fault_domains > 1 {
                return Err(EligibilityIssue::StorageGuaranteeInsufficient);
            }
            (1, 1)
        }
        StorageGuaranteeKind::ReplicatedAcrossFaultDomains {
            replicas,
            fault_domains,
        } => (replicas, fault_domains),
    };
    if replicas < policy.minimum_verified_replicas {
        return Err(EligibilityIssue::ReplicaAvailabilityInsufficient);
    }
    if fault_domains < policy.minimum_fault_domains {
        return Err(EligibilityIssue::FaultDomainSpreadInsufficient);
    }
    let mut receiver_set = observations.receiver_set.clone();
    receiver_set.sort();
    receiver_set.dedup();
    let receipt = EligibilityReceipt {
        content_ref: observations.content_ref.clone(),
        operation_id: observations.operation_id.clone(),
        receiver_set,
        retention_generation: observations.retention_generation,
        placement_epoch: observations.placement_epoch,
        storage_guarantee: observations.storage_guarantee,
        receipt_ref: String::new(),
    };
    Ok(EligibilityReceipt {
        receipt_ref: eligibility_receipt_identity(&receipt),
        ..receipt
    })
}

/// The canonical identity of an eligibility receipt.
pub fn eligibility_receipt_identity(receipt: &EligibilityReceipt) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(ELIGIBILITY_RECEIPT_CONTEXT);
    let receivers = receipt.receiver_set.join(",");
    for field in [
        receipt.content_ref.as_str(),
        receipt.operation_id.as_str(),
        receivers.as_str(),
        &receipt.retention_generation.to_string(),
        &receipt.placement_epoch.to_string(),
        receipt.storage_guarantee.as_str(),
    ] {
        let length = u64::try_from(field.len()).unwrap_or(u64::MAX);
        hasher.update(&length.to_be_bytes());
        hasher.update(field.as_bytes());
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}
