
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretCleanupInput {
    pub secret_ref: String,
    pub revocation_ref: String,
    pub tombstone_ref: String,
    pub retention_refs: Vec<String>,
    pub retention_receipts: Vec<IoValue>,
    pub retention_tombstones: Vec<IoValue>,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretCleanupReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub secret_ref: String,
    pub revocation_ref: String,
    pub tombstone_ref: String,
    pub retention_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}
