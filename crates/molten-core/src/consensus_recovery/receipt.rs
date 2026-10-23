use super::*;

pub const STORAGE_FAULT_RECEIPT_SCHEMA: &str = "molten.consensus.storage-fault-receipt.v1";
pub const STORAGE_FAULT_RECEIPT_IDENTITY_DOMAIN: &str = "onixresearch.molten.consensus.storage-fault-receipt.v1";

/// The non-claims every storage-fault receipt carries.
pub const STORAGE_FAULT_NON_CLAIMS: &[&str] = &[
    "a local database repair does not prove retained committed history",
    "a checksum does not prove which replica is authoritative",
    "a passing campaign does not prove arbitrary hardware, kernels, filesystems, or schedules",
    "a passing campaign does not prove whole-cluster safety, liveness, or release readiness",
];

/// The exact cohort one storage-fault result holds for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageFaultCohort {
    pub candidate_ref: String,
    pub molten_profile_ref: String,
    pub redb_version: String,
    pub kernel_ref: String,
    pub filesystem_ref: String,
    pub mount_options_ref: String,
    pub device_profile_ref: String,
    pub fault_schedule_ref: String,
}

impl StorageFaultCohort {
    fn fields(&self) -> [(&'static str, &str); 8] {
        [
            ("candidate", &self.candidate_ref),
            ("molten-profile", &self.molten_profile_ref),
            ("redb-version", &self.redb_version),
            ("kernel", &self.kernel_ref),
            ("filesystem", &self.filesystem_ref),
            ("mount-options", &self.mount_options_ref),
            ("device-profile", &self.device_profile_ref),
            ("fault-schedule", &self.fault_schedule_ref),
        ]
    }
}

/// A storage-fault recovery receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageFaultReceipt {
    pub schema: String,
    pub receipt_ref: String,
    pub cohort: StorageFaultCohort,
    pub results: Vec<RecoveryProgress>,
    pub non_claims: Vec<String>,
}

/// Why a storage-fault receipt is rejected.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum StorageFaultReceiptIssue {
    InvalidSchema,
    MissingCohortIdentity(&'static str),
    /// The receipt's cohort is not the campaign's cohort.
    CohortChanged(&'static str),
    MissingNonClaim(String),
    IdentityMismatch,
}

/// The receipt's domain-separated BLAKE3 identity over its cohort and results.
// r[impl molten.consensus.storage_fault_claim_boundary]
pub fn identify_storage_fault_receipt(receipt: &StorageFaultReceipt) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(STORAGE_FAULT_RECEIPT_IDENTITY_DOMAIN);
    let mut text = |value: &str| {
        hasher.update(&(value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    };
    text(&receipt.schema);
    for (label, value) in receipt.cohort.fields() {
        text(label);
        text(value);
    }
    for result in &receipt.results {
        text(&result.participant_id);
        text(&result.required.entry_ref);
        text(&format!("{}:{}", result.required_index, result.required.term));
        text(&format!("{:?}:{:?}", result.verdict, result.finding));
    }
    for non_claim in &receipt.non_claims {
        text(non_claim);
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}

/// Validate a storage-fault receipt against the campaign's exact cohort.
///
/// Validation fails closed when a cohort identity is missing or differs from
/// the campaign's, a non-claim is dropped, or the identity does not match.
// r[impl molten.consensus.storage_fault_claim_boundary]
pub fn validate_storage_fault_receipt(
    receipt: &StorageFaultReceipt,
    campaign: &StorageFaultCohort,
) -> Vec<StorageFaultReceiptIssue> {
    let mut issues = Vec::with_capacity(STORAGE_FAULT_NON_CLAIMS.len() + 16);
    if receipt.schema != STORAGE_FAULT_RECEIPT_SCHEMA {
        issues.push(StorageFaultReceiptIssue::InvalidSchema);
    }
    for ((label, value), (_, expected)) in receipt.cohort.fields().into_iter().zip(campaign.fields()) {
        if value.trim().is_empty() {
            issues.push(StorageFaultReceiptIssue::MissingCohortIdentity(label));
        } else if value != expected {
            issues.push(StorageFaultReceiptIssue::CohortChanged(label));
        }
    }
    for non_claim in STORAGE_FAULT_NON_CLAIMS {
        if !receipt.non_claims.iter().any(|stated| stated == non_claim) {
            issues.push(StorageFaultReceiptIssue::MissingNonClaim((*non_claim).to_string()));
        }
    }
    if identify_storage_fault_receipt(receipt) != receipt.receipt_ref {
        issues.push(StorageFaultReceiptIssue::IdentityMismatch);
    }
    issues
}
