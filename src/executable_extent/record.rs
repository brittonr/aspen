//! Detached Molten consumer receipts.

// r[impl molten.world_extents.receipts]

const CONSUMER_RECEIPT_SCHEMA: &str = "molten-executable-extent-consumer-receipt-v1";
const RECEIPT_DOMAIN: &[u8] = b"onix.molten.executable-extent-consumer-receipt.v1\0";
const MAPPING_DOMAIN: &[u8] = b"onix.molten.executable-extent-live-mapping.v1\0";
const NON_CLAIMS: [&str; 6] = [
    "consumer receipt does not prove compiler correctness",
    "consumer receipt does not prove executable code semantics",
    "consumer receipt does not prove sandbox or host integrity",
    "consumer receipt does not grant storage or retention authority",
    "consumer receipt does not prove external authority freshness",
    "consumer receipt does not prove release eligibility",
];

/// One mapped and explicitly unmapped extent observation.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumerMappingObservation {
    /// Stable extent ordinal.
    pub ordinal: u32,
    /// Exact extent byte identity.
    pub extent_identity_blake3: String,
    /// Detached live mapping identity.
    pub mapping_identity_blake3: String,
    /// State observed after mapping and protection.
    pub mapped_state: String,
    /// State observed after explicit teardown.
    pub final_state: String,
}

/// Detached role-bound Molten consumer receipt.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumerReceipt {
    /// Exact receipt schema.
    pub schema: String,
    /// BLAKE3 identity over all receipt facts except this field.
    pub receipt_identity_blake3: String,
    /// Exact Mantle bundle identity.
    pub bundle_identity_blake3: String,
    /// Exact Mantle producer receipt identity.
    pub producer_receipt_identity_blake3: String,
    /// Molten semantic code identity.
    pub semantic_code_identity_blake3: String,
    /// Exact built artifact identity.
    pub built_artifact_identity_blake3: String,
    /// Shared executable layout identity.
    pub layout_identity_blake3: String,
    /// Current runtime cohort identity.
    pub runtime_cohort_identity_blake3: String,
    /// Current policy identity.
    pub policy_identity_blake3: String,
    /// Closed disposition: `mapped-and-unmapped` or `inert`.
    pub disposition: String,
    /// Optional current admission denial.
    pub denial: Option<String>,
    /// Ordered mapping observations.
    pub mappings: Vec<ConsumerMappingObservation>,
    /// Whether every bundle member was independently remeasured.
    pub extents_remeasured: bool,
    /// Whether detached Artifact Auth and pinned binding checks passed.
    pub detached_review_admitted: bool,
    /// Exact executable-extent source repository.
    pub executable_extent_repository: String,
    /// Exact executable-extent source revision.
    pub executable_extent_revision: String,
    /// Exact Mantle producer repository.
    pub mantle_producer_repository: String,
    /// Exact Mantle producer revision.
    pub mantle_producer_revision: String,
    /// Explicit bounded non-claims.
    pub non_claims: Vec<String>,
}

#[derive(serde::Serialize)]
struct ReceiptMaterial<'a> {
    schema: &'a str,
    bundle_identity_blake3: &'a str,
    producer_receipt_identity_blake3: &'a str,
    semantic_code_identity_blake3: &'a str,
    built_artifact_identity_blake3: &'a str,
    layout_identity_blake3: &'a str,
    runtime_cohort_identity_blake3: &'a str,
    policy_identity_blake3: &'a str,
    disposition: &'a str,
    denial: &'a Option<String>,
    mappings: &'a [ConsumerMappingObservation],
    extents_remeasured: bool,
    detached_review_admitted: bool,
    executable_extent_repository: &'a str,
    executable_extent_revision: &'a str,
    mantle_producer_repository: &'a str,
    mantle_producer_revision: &'a str,
    non_claims: &'a [String],
}

pub(crate) struct ReceiptInput<'a> {
    pub bundle: &'a super::producer::model::Bundle,
    pub producer: &'a super::producer::model::Receipt,
    pub profile: &'a molten_core::executable_extent::ExtentCodeRootProfile,
    pub disposition: &'a str,
    pub denial: Option<String>,
    pub mappings: Vec<ConsumerMappingObservation>,
}

pub(crate) fn build(input: ReceiptInput<'_>) -> Result<ConsumerReceipt, serde_json::Error> {
    let mut receipt = ConsumerReceipt {
        schema: CONSUMER_RECEIPT_SCHEMA.to_string(),
        receipt_identity_blake3: String::new(),
        bundle_identity_blake3: input.bundle.bundle_identity_blake3.clone(),
        producer_receipt_identity_blake3: input.producer.receipt_identity_blake3.clone(),
        semantic_code_identity_blake3: super::producer::admission::encode_digest(
            input.profile.semantic_code.as_bytes(),
        ),
        built_artifact_identity_blake3: super::producer::admission::encode_digest(
            input.profile.built_artifact.as_bytes(),
        ),
        layout_identity_blake3: input.bundle.layout_identity_blake3.clone(),
        runtime_cohort_identity_blake3: super::producer::admission::encode_digest(
            input.profile.runtime_cohort.as_bytes(),
        ),
        policy_identity_blake3: super::producer::admission::encode_digest(input.profile.policy.as_bytes()),
        disposition: input.disposition.to_string(),
        denial: input.denial,
        mappings: input.mappings,
        extents_remeasured: true,
        detached_review_admitted: true,
        executable_extent_repository: crate::executable_extent::EXECUTABLE_EXTENT_REPOSITORY.to_string(),
        executable_extent_revision: crate::executable_extent::EXECUTABLE_EXTENT_REVISION.to_string(),
        mantle_producer_repository: crate::executable_extent::MANTLE_PRODUCER_REPOSITORY.to_string(),
        mantle_producer_revision: crate::executable_extent::MANTLE_PRODUCER_REVISION.to_string(),
        non_claims: NON_CLAIMS.iter().map(|claim| (*claim).to_string()).collect(),
    };
    let material = material(&receipt);
    let bytes = serde_json::to_vec(&material)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(RECEIPT_DOMAIN);
    hasher.update(&bytes);
    receipt.receipt_identity_blake3 = super::producer::admission::encode_digest(hasher.finalize().as_bytes());
    assert!(!receipt.receipt_identity_blake3.is_empty());
    Ok(receipt)
}

pub(crate) fn mapping_identity(
    bundle_identity: &[u8; blake3::OUT_LEN],
    extent_identity: &[u8; blake3::OUT_LEN],
    runtime_identity: &[u8; blake3::OUT_LEN],
    policy_identity: &[u8; blake3::OUT_LEN],
    ordinal: u32,
) -> [u8; blake3::OUT_LEN] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(MAPPING_DOMAIN);
    hasher.update(bundle_identity);
    hasher.update(extent_identity);
    hasher.update(runtime_identity);
    hasher.update(policy_identity);
    hasher.update(&ordinal.to_le_bytes());
    *hasher.finalize().as_bytes()
}

fn material(receipt: &ConsumerReceipt) -> ReceiptMaterial<'_> {
    ReceiptMaterial {
        schema: &receipt.schema,
        bundle_identity_blake3: &receipt.bundle_identity_blake3,
        producer_receipt_identity_blake3: &receipt.producer_receipt_identity_blake3,
        semantic_code_identity_blake3: &receipt.semantic_code_identity_blake3,
        built_artifact_identity_blake3: &receipt.built_artifact_identity_blake3,
        layout_identity_blake3: &receipt.layout_identity_blake3,
        runtime_cohort_identity_blake3: &receipt.runtime_cohort_identity_blake3,
        policy_identity_blake3: &receipt.policy_identity_blake3,
        disposition: &receipt.disposition,
        denial: &receipt.denial,
        mappings: &receipt.mappings,
        extents_remeasured: receipt.extents_remeasured,
        detached_review_admitted: receipt.detached_review_admitted,
        executable_extent_repository: &receipt.executable_extent_repository,
        executable_extent_revision: &receipt.executable_extent_revision,
        mantle_producer_repository: &receipt.mantle_producer_repository,
        mantle_producer_revision: &receipt.mantle_producer_revision,
        non_claims: &receipt.non_claims,
    }
}
