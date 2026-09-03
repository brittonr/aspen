use molten_core::prolly_map::*;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;

pub const PROLLY_PUBLICATION_RECEIPT_SCHEMA: &str = "molten.prolly-map-publication-receipt.v1";
pub const PROLLY_PUBLICATION_RECEIPT_RECORD: &str = "molten-prolly-map-publication-receipt-v1";

const PROLLY_RECEIPT_DOMAIN: &str = "onixresearch.molten.prolly-map-publication-receipt.v1";
const MAX_PROLLY_RECEIPT_BYTES: usize = 262_144;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProllyPublicationStatus {
    Applied,
    AlreadyApplied,
    AppliedAfterReconciliation,
    NotAppliedAfterReconciliation,
    Stale,
    Unknown,
}

impl ProllyPublicationStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::AlreadyApplied => "already-applied",
            Self::AppliedAfterReconciliation => "applied-after-reconciliation",
            Self::NotAppliedAfterReconciliation => "not-applied-after-reconciliation",
            Self::Stale => "stale",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProllyPublicationReceipt {
    pub schema: String,
    pub map_id: String,
    pub prior_root_ref: Option<RootRef>,
    pub next_root_ref: RootRef,
    pub generation: u64,
    pub staged_block_refs: Vec<NodeRef>,
    pub status: ProllyPublicationStatus,
    pub authorizes_future_mutation: bool,
    pub deletion_authorized: bool,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CanonicalProllyPublicationReceipt {
    pub receipt_ref: String,
    pub status: ProllyPublicationStatus,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

pub fn prolly_receipt_non_claims() -> Vec<String> {
    vec![
        "receipt-does-not-grant-authority".to_string(),
        "receipt-does-not-authorize-deletion".to_string(),
        "receipt-does-not-prove-durability-beyond-observed-transaction".to_string(),
        "receipt-does-not-prove-database-correctness".to_string(),
        "receipt-does-not-establish-release-eligibility".to_string(),
    ]
}

pub fn canonical_prolly_publication_receipt(
    receipt: &ProllyPublicationReceipt,
) -> Result<CanonicalProllyPublicationReceipt> {
    if receipt.schema != PROLLY_PUBLICATION_RECEIPT_SCHEMA
        || receipt.map_id.is_empty()
        || receipt.authorizes_future_mutation
        || receipt.deletion_authorized
        || receipt.non_claims != prolly_receipt_non_claims()
    {
        return Err(MoltenError::invalid_harness("Prolly publication receipt is invalid"));
    }
    let value = record(PROLLY_PUBLICATION_RECEIPT_RECORD, vec![
        field("schema", string(&receipt.schema)),
        field("map-id", string(&receipt.map_id)),
        field("prior-root-ref", optional_ref(receipt.prior_root_ref.as_ref())),
        field("next-root-ref", string(receipt.next_root_ref.as_str())),
        field("generation", number(receipt.generation)),
        field(
            "staged-block-refs",
            sequence(receipt.staged_block_refs.iter().map(|item| string(item.as_str())).collect()),
        ),
        field("status", string(receipt.status.as_str())),
        field("authorizes-future-mutation", boolean(receipt.authorizes_future_mutation)),
        field("deletion-authorized", boolean(receipt.deletion_authorized)),
        field("non-claims", sequence(receipt.non_claims.iter().map(string).collect())),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if bytes.len() > MAX_PROLLY_RECEIPT_BYTES {
        return Err(MoltenError::invalid_harness("Prolly publication receipt exceeds its byte bound"));
    }
    let mut hasher = blake3::Hasher::new_derive_key(PROLLY_RECEIPT_DOMAIN);
    hasher.update(&bytes);
    Ok(CanonicalProllyPublicationReceipt {
        receipt_ref: format!("blake3:{}", hasher.finalize().to_hex()),
        status: receipt.status,
        value,
        bytes,
    })
}

fn optional_ref(value: Option<&RootRef>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value.as_str())]))
}

fn boolean(value: bool) -> IOValue {
    record(if value { "true" } else { "false" }, Vec::new())
}

fn number(value: u64) -> IOValue {
    crate::preserves_rail::u64_value(value)
}

fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    record(label, vec![value])
}

fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::record(label, fields)
}
