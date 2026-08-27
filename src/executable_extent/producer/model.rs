//! Mantle producer DTOs and canonical identity material.

pub(super) const BUNDLE_SCHEMA: &str = "mantle-executable-extent-bundle-v1";
pub(super) const RECEIPT_SCHEMA: &str = "mantle-executable-extent-producer-receipt-v1";
pub(super) const CLOSED_PROFILE_ID: &str = "mantle-flat-page-x86_64-linux-gnu-v1";
pub(super) const CLOSED_FORMAT: &str = "mantle-flat-page-v1";
pub(super) const CLOSED_ARCHITECTURE: &str = "x86_64";
pub(super) const CLOSED_ABI: &str = "linux-gnu";
pub(super) const CLOSED_ENDIANNESS: &str = "little";
pub(super) const CLOSED_RELOCATION_MODEL: &str = "none";
pub(super) const CLOSED_PERMISSION: &str = "executable-read-only";
pub(super) const PUBLISHED_DISPOSITION: &str = "published";
pub(super) const CLOSED_PAGE_BYTES: u64 = 4_096;
pub(super) const CLOSED_MEMBER_COUNT: usize = 1;
pub(super) const BUNDLE_IDENTITY_DOMAIN: &[u8] = b"onix.mantle.executable-extent-bundle.v1\0";
pub(super) const RECEIPT_IDENTITY_DOMAIN: &[u8] = b"onix.mantle.executable-extent-producer-receipt.v1\0";
pub(super) const BUNDLE_NON_CLAIMS: [&str; 6] = [
    "bundle production does not prove compiler correctness",
    "bundle production does not prove executable code semantics",
    "bundle production does not grant mapping or execution authority",
    "bundle production does not prove sandbox or host integrity",
    "bundle production does not grant retention or deletion authority",
    "bundle production does not prove release eligibility",
];
pub(super) const RECEIPT_NON_CLAIMS: [&str; 8] = [
    "producer receipt does not prove compiler correctness",
    "producer receipt does not prove semantic code equivalence",
    "producer receipt does not prove consumer mapping safety",
    "producer receipt does not grant execution authority",
    "producer receipt does not prove sandbox or host integrity",
    "producer receipt does not grant retention or deletion authority",
    "producer receipt does not grant deployment authority",
    "producer receipt does not prove release eligibility",
];

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Member {
    pub ordinal: u32,
    pub member_leaf: String,
    pub source_offset_bytes: u64,
    pub virtual_offset_bytes: u64,
    pub length_bytes: u64,
    pub content_blake3: String,
    pub permission: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Bundle {
    pub schema: String,
    pub bundle_identity_blake3: String,
    pub profile_id: String,
    pub source_artifact_blake3: String,
    pub source_length_bytes: u64,
    pub format: String,
    pub architecture: String,
    pub abi: String,
    pub endianness: String,
    pub page_size_bytes: u64,
    pub maximum_virtual_bytes: u64,
    pub parser_id: String,
    pub toolchain_id: String,
    pub relocation_model: String,
    pub layout_identity_blake3: String,
    pub plan_identity_blake3: String,
    pub extents: Vec<Member>,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Receipt {
    pub schema: String,
    pub receipt_identity_blake3: String,
    pub bundle_identity_blake3: String,
    pub bundle_manifest_leaf: String,
    pub source_artifact_blake3: String,
    pub layout_identity_blake3: String,
    pub plan_identity_blake3: String,
    pub producer_implementation_id: String,
    pub layout_corpus_identity_blake3: String,
    pub layout_receipt_identity_blake3: String,
    pub transition_corpus_identity_blake3: String,
    pub transition_receipt_identity_blake3: String,
    pub publication_disposition: String,
    pub source_remeasured: bool,
    pub extents_remeasured: bool,
    pub manifest_published: bool,
    pub non_claims: Vec<String>,
}

#[derive(serde::Serialize)]
pub(super) struct BundleMaterial<'a> {
    pub schema: &'a str,
    pub profile_id: &'a str,
    pub source_artifact_blake3: &'a str,
    pub source_length_bytes: u64,
    pub format: &'a str,
    pub architecture: &'a str,
    pub abi: &'a str,
    pub endianness: &'a str,
    pub page_size_bytes: u64,
    pub maximum_virtual_bytes: u64,
    pub parser_id: &'a str,
    pub toolchain_id: &'a str,
    pub relocation_model: &'a str,
    pub layout_identity_blake3: &'a str,
    pub plan_identity_blake3: &'a str,
    pub extents: &'a [Member],
    pub non_claims: &'a [String],
}

#[derive(serde::Serialize)]
pub(super) struct ReceiptMaterial<'a> {
    pub schema: &'a str,
    pub bundle_identity_blake3: &'a str,
    pub bundle_manifest_leaf: &'a str,
    pub source_artifact_blake3: &'a str,
    pub layout_identity_blake3: &'a str,
    pub plan_identity_blake3: &'a str,
    pub producer_implementation_id: &'a str,
    pub layout_corpus_identity_blake3: &'a str,
    pub layout_receipt_identity_blake3: &'a str,
    pub transition_corpus_identity_blake3: &'a str,
    pub transition_receipt_identity_blake3: &'a str,
    pub publication_disposition: &'a str,
    pub source_remeasured: bool,
    pub extents_remeasured: bool,
    pub manifest_published: bool,
    pub non_claims: &'a [String],
}
