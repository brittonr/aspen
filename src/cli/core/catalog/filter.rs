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

pub(super) fn filters(input: Input) -> Vec<molten::catalog::CatalogFilter> {
    let mut filters = Vec::new();
    if let Some(value) = input.artifact_kind {
        filters.push(molten::catalog::CatalogFilter::ArtifactKind(value));
    }
    if let Some(value) = input.ledger_kind {
        filters.push(molten::catalog::CatalogFilter::LedgerKind(value));
    }
    if let Some(value) = input.schema_ref {
        filters.push(molten::catalog::CatalogFilter::SchemaRef(value));
    }
    if let Some(value) = input.structural_fingerprint {
        filters.push(molten::catalog::CatalogFilter::StructuralFingerprint(value));
    }
    if let Some(value) = input.effect_ref {
        filters.push(molten::catalog::CatalogFilter::EffectRef(value));
    }
    if let Some(value) = input.policy_ref {
        filters.push(molten::catalog::CatalogFilter::PolicyRef(value));
    }
    if let Some(value) = input.capability_ref {
        filters.push(molten::catalog::CatalogFilter::CapabilityRef(value));
    }
    if let Some(value) = input.evidence_ref {
        filters.push(molten::catalog::CatalogFilter::EvidenceRef(value));
    }
    if let Some(value) = input.dependency_ref {
        filters.push(molten::catalog::CatalogFilter::DependencyRef(value));
    }
    if let Some(value) = input.dependent_ref {
        filters.push(molten::catalog::CatalogFilter::DependentRef(value));
    }
    if let Some(value) = input.receipt_operation {
        filters.push(molten::catalog::CatalogFilter::ReceiptOperation(value));
    }
    if let Some(value) = input.receipt_decision {
        filters.push(molten::catalog::CatalogFilter::ReceiptDecision(value));
    }
    if let Some(value) = input.transcript_status {
        filters.push(molten::catalog::CatalogFilter::TranscriptStatus(value));
    }
    if let Some(value) = input.upgrade_status {
        filters.push(molten::catalog::CatalogFilter::UpgradeStatus(value));
    }
    if let Some(value) = input.text {
        filters.push(molten::catalog::CatalogFilter::Text(value));
    }
    filters
}
