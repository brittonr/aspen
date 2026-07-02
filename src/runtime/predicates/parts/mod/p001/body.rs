
impl RuntimeRevocationCleanupState {
    pub fn cleanup_ref(&self) -> Result<String> {
        crate::preserves_rail::canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IoValue {
        crate::preserves_rail::record("runtime-revocation-cleanup-state-v1", vec![
            ref_list_value("revoked-refs", &self.revoked_refs),
            ref_list_value("attempted-use-refs", &self.attempted_use_refs),
            ref_list_value("remaining-assertion-refs", &self.remaining_assertion_refs),
            ref_list_value("remaining-subscription-refs", &self.remaining_subscription_refs),
            ref_list_value("remaining-pending-call-refs", &self.remaining_pending_call_refs),
            ref_list_value("remaining-child-refs", &self.remaining_child_refs),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevocationCleanupResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeActormapTransactionOutcome {
    Committed,
    RolledBack,
}

impl RuntimeActormapTransactionOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Committed => "committed",
            Self::RolledBack => "rolled-back",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeActormapTransactionState {
    pub outcome: RuntimeActormapTransactionOutcome,
    pub before_object_refs: Vec<String>,
    pub after_object_refs: Vec<String>,
    pub spawned_object_refs: Vec<String>,
    pub removed_object_refs: Vec<String>,
    pub visible_object_refs: Vec<String>,
    pub used_object_refs: Vec<String>,
}

impl RuntimeActormapTransactionState {
    pub fn transaction_ref(&self) -> Result<String> {
        crate::preserves_rail::canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IoValue {
        crate::preserves_rail::record("runtime-actormap-transaction-state-v1", vec![
            crate::preserves_rail::string(self.outcome.as_str()),
            ref_list_value("before-object-refs", &self.before_object_refs),
            ref_list_value("after-object-refs", &self.after_object_refs),
            ref_list_value("spawned-object-refs", &self.spawned_object_refs),
            ref_list_value("removed-object-refs", &self.removed_object_refs),
            ref_list_value("visible-object-refs", &self.visible_object_refs),
            ref_list_value("used-object-refs", &self.used_object_refs),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActormapTransactionResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeReferenceKind {
    Near,
    Far,
}

impl RuntimeReferenceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Near => "near",
            Self::Far => "far",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeReferenceCallMode {
    Synchronous,
    Asynchronous,
}

impl RuntimeReferenceCallMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Synchronous => "synchronous",
            Self::Asynchronous => "asynchronous",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeNearFarRefState {
    pub reference_ref: String,
    pub reference_kind: RuntimeReferenceKind,
    pub is_live: bool,
    pub caller_vat_id: String,
    pub target_vat_id: String,
    pub call_mode: RuntimeReferenceCallMode,
}

impl RuntimeNearFarRefState {
    pub fn call_ref(&self) -> Result<String> {
        crate::preserves_rail::canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IoValue {
        crate::preserves_rail::record("runtime-near-far-ref-state-v1", vec![
            crate::preserves_rail::record("reference-ref", vec![crate::preserves_rail::string(&self.reference_ref)]),
            crate::preserves_rail::string(self.reference_kind.as_str()),
            crate::preserves_rail::bool_value(self.is_live),
            crate::preserves_rail::string(&self.caller_vat_id),
            crate::preserves_rail::string(&self.target_vat_id),
            crate::preserves_rail::string(self.call_mode.as_str()),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NearFarRefsResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeObjectAuthorityKind {
    Filesystem,
    Network,
    Clock,
    Process,
    Dataspace,
    Store,
    Blob,
    Consensus,
    Choreography,
    HostResource,
}

impl RuntimeObjectAuthorityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Filesystem => "filesystem",
            Self::Network => "network",
            Self::Clock => "clock",
            Self::Process => "process",
            Self::Dataspace => "dataspace",
            Self::Store => "store",
            Self::Blob => "blob",
            Self::Consensus => "consensus",
            Self::Choreography => "choreography",
            Self::HostResource => "host-resource",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeObjectAuthorityState {
    pub object_ref: String,
    pub requested_authority_ref: String,
    pub requested_authority_kind: RuntimeObjectAuthorityKind,
    pub endowed_authority_refs: Vec<String>,
    pub admitted_authority_refs: Vec<String>,
}

impl RuntimeObjectAuthorityState {
    pub fn authority_ref(&self) -> Result<String> {
        crate::preserves_rail::canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IoValue {
        crate::preserves_rail::record("runtime-object-authority-state-v1", vec![
            crate::preserves_rail::record("object-ref", vec![crate::preserves_rail::string(&self.object_ref)]),
            crate::preserves_rail::record("requested-authority-ref", vec![crate::preserves_rail::string(
                &self.requested_authority_ref,
            )]),
            crate::preserves_rail::string(self.requested_authority_kind.as_str()),
            ref_list_value("endowed-authority-refs", &self.endowed_authority_refs),
            ref_list_value("admitted-authority-refs", &self.admitted_authority_refs),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectAuthorityResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRightsAmplificationState {
    pub holder_object_ref: String,
    pub sealed_value_ref: String,
    pub sealer_brand_ref: String,
    pub unsealer_brand_ref: String,
    pub sealed_authority_refs: Vec<String>,
    pub recovered_authority_refs: Vec<String>,
}

impl RuntimeRightsAmplificationState {
    pub fn amplification_ref(&self) -> Result<String> {
        crate::preserves_rail::canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IoValue {
        crate::preserves_rail::record("runtime-rights-amplification-state-v1", vec![
            crate::preserves_rail::record("holder-object-ref", vec![crate::preserves_rail::string(
                &self.holder_object_ref,
            )]),
            crate::preserves_rail::record("sealed-value-ref", vec![crate::preserves_rail::string(
                &self.sealed_value_ref,
            )]),
            crate::preserves_rail::record("sealer-brand-ref", vec![crate::preserves_rail::string(
                &self.sealer_brand_ref,
            )]),
            crate::preserves_rail::record("unsealer-brand-ref", vec![crate::preserves_rail::string(
                &self.unsealer_brand_ref,
            )]),
            ref_list_value("sealed-authority-refs", &self.sealed_authority_refs),
            ref_list_value("recovered-authority-refs", &self.recovered_authority_refs),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RightsAmplificationResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDistributedRefLifetimeState {
    pub far_ref: String,
    pub session_ref: String,
    pub replacement_ref: Option<String>,
    pub is_session_live: bool,
    pub is_handoff_admitted: bool,
    pub pending_call_refs: Vec<String>,
    pub failed_pending_call_refs: Vec<String>,
    pub attempted_use_refs: Vec<String>,
}

impl RuntimeDistributedRefLifetimeState {
    pub fn lifetime_ref(&self) -> Result<String> {
        crate::preserves_rail::canonical_hash(&self.to_value())
    }

    fn to_value(&self) -> IoValue {
        crate::preserves_rail::record("runtime-distributed-ref-lifetime-state-v1", vec![
            crate::preserves_rail::record("far-ref", vec![crate::preserves_rail::string(&self.far_ref)]),
            crate::preserves_rail::record("session-ref", vec![crate::preserves_rail::string(&self.session_ref)]),
            optional_ref_record("replacement-ref", self.replacement_ref.as_deref()),
            crate::preserves_rail::bool_value(self.is_session_live),
            crate::preserves_rail::bool_value(self.is_handoff_admitted),
            ref_list_value("pending-call-refs", &self.pending_call_refs),
            ref_list_value("failed-pending-call-refs", &self.failed_pending_call_refs),
            ref_list_value("attempted-use-refs", &self.attempted_use_refs),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedRefLifetimeResult {
    pub is_allowed: bool,
    pub receipt: RuntimePredicateReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSnapshotAuthorityState {
    pub snapshot_ref: String,
    pub admitted_authority_refs: Vec<String>,
    pub claimed_authority_refs: Vec<String>,
    pub requested_assertion_refs: Vec<String>,
    pub readable_assertion_refs: Vec<String>,
    pub redacted_assertion_refs: Vec<String>,
}
