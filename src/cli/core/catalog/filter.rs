pub(super) struct Input {
    pub(super) artifact_kind: Option<String>,
    pub(super) ledger_kind: Option<String>,
    pub(super) schema_ref: Option<String>,
    pub(super) structural_fingerprint: Option<String>,
    pub(super) effect_ref: Option<String>,
    pub(super) policy_ref: Option<String>,
    pub(super) capability_ref: Option<String>,
    pub(super) evidence_ref: Option<String>,
    pub(super) dependency_ref: Option<String>,
    pub(super) dependent_ref: Option<String>,
    pub(super) receipt_operation: Option<String>,
    pub(super) receipt_decision: Option<String>,
    pub(super) transcript_status: Option<String>,
    pub(super) upgrade_status: Option<String>,
    pub(super) text: Option<String>,
}

pub(super) fn filters(input: Input) -> Vec<molten::catalog::Filter> {
    let mut filters = Vec::new();
    if let Some(value) = input.artifact_kind {
        filters.push(molten::catalog::Filter::ArtifactKind(value));
    }
    if let Some(value) = input.ledger_kind {
        filters.push(molten::catalog::Filter::LedgerKind(value));
    }
    if let Some(value) = input.schema_ref {
        filters.push(molten::catalog::Filter::SchemaRef(value));
    }
    if let Some(value) = input.structural_fingerprint {
        filters.push(molten::catalog::Filter::StructuralFingerprint(value));
    }
    if let Some(value) = input.effect_ref {
        filters.push(molten::catalog::Filter::EffectRef(value));
    }
    if let Some(value) = input.policy_ref {
        filters.push(molten::catalog::Filter::PolicyRef(value));
    }
    if let Some(value) = input.capability_ref {
        filters.push(molten::catalog::Filter::CapabilityRef(value));
    }
    if let Some(value) = input.evidence_ref {
        filters.push(molten::catalog::Filter::EvidenceRef(value));
    }
    if let Some(value) = input.dependency_ref {
        filters.push(molten::catalog::Filter::DependencyRef(value));
    }
    if let Some(value) = input.dependent_ref {
        filters.push(molten::catalog::Filter::DependentRef(value));
    }
    if let Some(value) = input.receipt_operation {
        filters.push(molten::catalog::Filter::ReceiptOperation(value));
    }
    if let Some(value) = input.receipt_decision {
        filters.push(molten::catalog::Filter::ReceiptDecision(value));
    }
    if let Some(value) = input.transcript_status {
        filters.push(molten::catalog::Filter::TranscriptStatus(value));
    }
    if let Some(value) = input.upgrade_status {
        filters.push(molten::catalog::Filter::UpgradeStatus(value));
    }
    if let Some(value) = input.text {
        filters.push(molten::catalog::Filter::Text(value));
    }
    filters
}
