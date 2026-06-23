use preserves::IOValue;

use super::PredicateDecision;
use super::RuntimeActormapTransactionOutcome;
use super::RuntimeActormapTransactionState;
use super::RuntimeDistributedRefLifetimeState;
use super::RuntimeNearFarRefState;
use super::RuntimeObjectAuthorityKind;
use super::RuntimeObjectAuthorityState;
use super::RuntimePredicateReceipt;
use super::RuntimePromisePipelineEntry;
use super::RuntimePromisePipelineState;
use super::RuntimePromiseState;
use super::RuntimeReferenceCallMode;
use super::RuntimeReferenceKind;
use super::RuntimeRevocationCleanupState;
use super::RuntimeRightsAmplificationState;
use super::RuntimeSnapshotAuthorityState;
use super::evaluate_actormap_transaction;
use super::evaluate_distributed_ref_lifetime;
use super::evaluate_near_far_refs;
use super::evaluate_object_authority;
use super::evaluate_promise_pipeline;
use super::evaluate_promise_state_transition;
use super::evaluate_revocation_cleanup;
use super::evaluate_rights_amplification;
use super::evaluate_snapshot_authority;
use crate::deterministic_replay;
use crate::error::Result;
use crate::preserves_rail::RUNTIME_VAT_AMBIENT_AUTHORITY_FIXTURE_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_AUTHORITY_GRAPH_FIXTURE_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_DISTRIBUTED_REF_FIXTURE_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_FIXTURE_RUN_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_OBJECT_REF_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_OBJECT_UPGRADE_RECIPE_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_PORTABLE_STORAGE_FIXTURE_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_PROMISE_FIXTURE_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_REPLAY_FIXTURE_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_RESTORE_RECEIPT_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_RIGHTS_FIXTURE_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_SNAPSHOT_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_TIME_TRAVEL_FIXTURE_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;

const LOCAL_VAT_ID: &str = "vat:fixture:local";
const REMOTE_VAT_ID: &str = "vat:fixture:remote";
const ROOT_OBJECT_ID: &str = "object:root";
const HELPER_OBJECT_ID: &str = "object:helper";
const SPAWNED_OBJECT_ID: &str = "object:spawned";
const FAR_OBJECT_ID: &str = "object:remote";
const PROXY_OBJECT_ID: &str = "object:proxy";
const PIPELINE_MAX_QUEUE: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VatReferenceKind {
    Near,
    Far,
    Proxy,
}

impl VatReferenceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Near => "near",
            Self::Far => "far",
            Self::Proxy => "proxy",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatObjectRef {
    pub vat_id: String,
    pub object_id: String,
    pub kind: VatReferenceKind,
    pub authority_refs: Vec<String>,
}

impl VatObjectRef {
    pub fn new(
        vat_id: impl Into<String>,
        object_id: impl Into<String>,
        kind: VatReferenceKind,
        authority_refs: Vec<String>,
    ) -> Self {
        let mut sorted_authority_refs = authority_refs;
        sorted_authority_refs.sort();
        sorted_authority_refs.dedup();
        Self {
            vat_id: vat_id.into(),
            object_id: object_id.into(),
            kind,
            authority_refs: sorted_authority_refs,
        }
    }

    pub fn value(&self) -> IOValue {
        record("vat-object-ref-v1", vec![
            string(RUNTIME_VAT_OBJECT_REF_SCHEMA),
            string(&self.vat_id),
            string(&self.object_id),
            string(self.kind.as_str()),
            sequence(self.authority_refs.iter().map(string).collect()),
        ])
    }

    pub fn object_ref(&self) -> Result<String> {
        canonical_hash(&self.value())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatCallEvidence {
    pub name: String,
    pub receipt: RuntimePredicateReceipt,
}

impl VatCallEvidence {
    fn value(&self) -> IOValue {
        record("vat-call-evidence-v1", vec![
            string(&self.name),
            record("receipt-ref", vec![string(&self.receipt.receipt_ref)]),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatFixtureRun {
    pub value: IOValue,
    pub run_ref: String,
    pub receipts: Vec<RuntimePredicateReceipt>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatSnapshotFixture {
    pub value: IOValue,
    pub snapshot_ref: String,
    pub fixture_ref: String,
    pub receipts: Vec<RuntimePredicateReceipt>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatRestoreFixture {
    pub value: IOValue,
    pub fixture_ref: String,
    pub receipts: Vec<IOValue>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatPromiseFixture {
    pub value: IOValue,
    pub fixture_ref: String,
    pub receipts: Vec<RuntimePredicateReceipt>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatAmbientAuthorityFixture {
    pub value: IOValue,
    pub fixture_ref: String,
    pub receipts: Vec<RuntimePredicateReceipt>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatRightsFixture {
    pub value: IOValue,
    pub fixture_ref: String,
    pub receipts: Vec<RuntimePredicateReceipt>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatDistributedRefFixture {
    pub value: IOValue,
    pub fixture_ref: String,
    pub receipts: Vec<RuntimePredicateReceipt>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatDebugFixture {
    pub value: IOValue,
    pub fixture_ref: String,
    pub receipts: Vec<IOValue>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatReplayFixture {
    pub value: IOValue,
    pub fixture_ref: String,
    pub receipts: Vec<IOValue>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VatReplayRun {
    value: IOValue,
    run_ref: String,
    trace_ref: String,
    effect_request_ref: String,
    effect_response_ref: String,
    random_request_ref: String,
    random_response_ref: String,
    policy_decision_ref: String,
    final_state_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VatReplayDivergenceKind {
    None,
    Input,
    EffectRequest,
    EffectResponse,
    PolicyDecision,
    StateHash,
}

impl VatReplayDivergenceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Input => "input",
            Self::EffectRequest => "effect-request",
            Self::EffectResponse => "effect-response",
            Self::PolicyDecision => "policy-decision",
            Self::StateHash => "state-hash",
        }
    }
}

struct FixtureObjects {
    root: VatObjectRef,
    helper: VatObjectRef,
    spawned: VatObjectRef,
    far: VatObjectRef,
    proxy: VatObjectRef,
    root_ref: String,
    helper_ref: String,
    spawned_ref: String,
    far_ref: String,
    proxy_ref: String,
}

impl FixtureObjects {
    fn object_values(&self) -> Vec<IOValue> {
        [&self.root, &self.helper, &self.spawned, &self.far, &self.proxy]
            .iter()
            .map(|object| object.value())
            .collect()
    }
}

fn fixture_objects() -> Result<FixtureObjects> {
    let root = VatObjectRef::new(LOCAL_VAT_ID, ROOT_OBJECT_ID, VatReferenceKind::Near, Vec::new());
    let helper = VatObjectRef::new(LOCAL_VAT_ID, HELPER_OBJECT_ID, VatReferenceKind::Near, vec![root.object_ref()?]);
    let spawned =
        VatObjectRef::new(LOCAL_VAT_ID, SPAWNED_OBJECT_ID, VatReferenceKind::Near, vec![helper.object_ref()?]);
    let far = VatObjectRef::new(REMOTE_VAT_ID, FAR_OBJECT_ID, VatReferenceKind::Far, Vec::new());
    let proxy = VatObjectRef::new(LOCAL_VAT_ID, PROXY_OBJECT_ID, VatReferenceKind::Proxy, vec![helper.object_ref()?]);

    Ok(FixtureObjects {
        root_ref: root.object_ref()?,
        helper_ref: helper.object_ref()?,
        spawned_ref: spawned.object_ref()?,
        far_ref: far.object_ref()?,
        proxy_ref: proxy.object_ref()?,
        root,
        helper,
        spawned,
        far,
        proxy,
    })
}

fn near_far_calls(objects: &FixtureObjects) -> Result<Vec<VatCallEvidence>> {
    let near_call = evaluate_near_far_refs(&RuntimeNearFarRefState {
        reference_ref: objects.helper_ref.clone(),
        reference_kind: RuntimeReferenceKind::Near,
        is_live: true,
        caller_vat_id: LOCAL_VAT_ID.to_string(),
        target_vat_id: LOCAL_VAT_ID.to_string(),
        call_mode: RuntimeReferenceCallMode::Synchronous,
    })?;
    let far_sync_denial = evaluate_near_far_refs(&RuntimeNearFarRefState {
        reference_ref: objects.far_ref.clone(),
        reference_kind: RuntimeReferenceKind::Far,
        is_live: true,
        caller_vat_id: LOCAL_VAT_ID.to_string(),
        target_vat_id: REMOTE_VAT_ID.to_string(),
        call_mode: RuntimeReferenceCallMode::Synchronous,
    })?;
    let far_async = evaluate_near_far_refs(&RuntimeNearFarRefState {
        reference_ref: objects.far_ref.clone(),
        reference_kind: RuntimeReferenceKind::Far,
        is_live: true,
        caller_vat_id: LOCAL_VAT_ID.to_string(),
        target_vat_id: REMOTE_VAT_ID.to_string(),
        call_mode: RuntimeReferenceCallMode::Asynchronous,
    })?;

    Ok(vec![
        VatCallEvidence {
            name: "near-sync-call".to_string(),
            receipt: near_call.receipt,
        },
        VatCallEvidence {
            name: "far-sync-denied".to_string(),
            receipt: far_sync_denial.receipt,
        },
        VatCallEvidence {
            name: "far-async-call".to_string(),
            receipt: far_async.receipt,
        },
    ])
}

fn actormap_calls(objects: &FixtureObjects) -> Result<Vec<VatCallEvidence>> {
    let committed = evaluate_actormap_transaction(&RuntimeActormapTransactionState {
        outcome: RuntimeActormapTransactionOutcome::Committed,
        before_object_refs: sorted_refs(vec![objects.root_ref.clone(), objects.helper_ref.clone()]),
        after_object_refs: sorted_refs(vec![
            objects.root_ref.clone(),
            objects.helper_ref.clone(),
            objects.spawned_ref.clone(),
        ]),
        spawned_object_refs: vec![objects.spawned_ref.clone()],
        removed_object_refs: Vec::new(),
        visible_object_refs: sorted_refs(vec![
            objects.root_ref.clone(),
            objects.helper_ref.clone(),
            objects.spawned_ref.clone(),
        ]),
        used_object_refs: vec![objects.helper_ref.clone()],
    })?;
    let rollback = evaluate_actormap_transaction(&RuntimeActormapTransactionState {
        outcome: RuntimeActormapTransactionOutcome::RolledBack,
        before_object_refs: sorted_refs(vec![objects.root_ref.clone(), objects.helper_ref.clone()]),
        after_object_refs: sorted_refs(vec![objects.root_ref.clone(), objects.helper_ref.clone()]),
        spawned_object_refs: vec![objects.spawned_ref.clone()],
        removed_object_refs: Vec::new(),
        visible_object_refs: sorted_refs(vec![objects.root_ref.clone(), objects.helper_ref.clone()]),
        used_object_refs: vec![objects.helper_ref.clone()],
    })?;

    Ok(vec![
        VatCallEvidence {
            name: "actormap-commit".to_string(),
            receipt: committed.receipt,
        },
        VatCallEvidence {
            name: "actormap-rollback".to_string(),
            receipt: rollback.receipt,
        },
    ])
}

fn pipeline_call(far_ref: &str) -> Result<VatCallEvidence> {
    let pipeline = evaluate_promise_pipeline(&RuntimePromisePipelineState::new(
        RuntimePromiseState::pending("promise:far-call"),
        PIPELINE_MAX_QUEUE,
        vec![
            RuntimePromisePipelineEntry::new(1, far_ref.to_string(), "get"),
            RuntimePromisePipelineEntry::new(2, far_ref.to_string(), "subscribe"),
        ],
    ))?;
    Ok(VatCallEvidence {
        name: "promise-pipeline".to_string(),
        receipt: pipeline.receipt,
    })
}

fn revocation_call(proxy_ref: &str) -> Result<VatCallEvidence> {
    let revoked = evaluate_revocation_cleanup(&RuntimeRevocationCleanupState {
        revoked_refs: vec![proxy_ref.to_string()],
        attempted_use_refs: vec![proxy_ref.to_string()],
        remaining_assertion_refs: Vec::new(),
        remaining_subscription_refs: Vec::new(),
        remaining_pending_call_refs: Vec::new(),
        remaining_child_refs: Vec::new(),
    })?;
    Ok(VatCallEvidence {
        name: "revocation-cleanup".to_string(),
        receipt: revoked.receipt,
    })
}

fn fixture_calls(objects: &FixtureObjects) -> Result<Vec<VatCallEvidence>> {
    let mut calls = near_far_calls(objects)?;
    calls.extend(actormap_calls(objects)?);
    calls.push(pipeline_call(&objects.far_ref)?);
    calls.push(revocation_call(&objects.proxy_ref)?);
    Ok(calls)
}

pub fn run_vat_fixture() -> Result<VatFixtureRun> {
    let objects = fixture_objects()?;
    let calls = fixture_calls(&objects)?;
    let receipts = calls.iter().map(|call| call.receipt.clone()).collect::<Vec<_>>();
    let diagnostics = fixture_diagnostics(&receipts);
    let value = record("vat-fixture-run-v1", vec![
        string(RUNTIME_VAT_FIXTURE_RUN_SCHEMA),
        string(LOCAL_VAT_ID),
        sequence(objects.object_values()),
        sequence(calls.iter().map(VatCallEvidence::value).collect()),
        sequence(receipts.iter().map(|receipt| receipt.value.clone()).collect()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let run_ref = canonical_hash(&value)?;
    Ok(VatFixtureRun {
        value,
        run_ref,
        receipts,
        diagnostics,
    })
}

pub fn run_vat_snapshot_fixture() -> Result<VatSnapshotFixture> {
    let root = VatObjectRef::new(LOCAL_VAT_ID, ROOT_OBJECT_ID, VatReferenceKind::Near, Vec::new());
    let helper = VatObjectRef::new(LOCAL_VAT_ID, HELPER_OBJECT_ID, VatReferenceKind::Near, vec![root.object_ref()?]);
    let far = VatObjectRef::new(REMOTE_VAT_ID, FAR_OBJECT_ID, VatReferenceKind::Far, Vec::new());
    let root_ref = root.object_ref()?;
    let helper_ref = helper.object_ref()?;
    let far_ref = far.object_ref()?;

    let snapshot_body = record("vat-snapshot-body-v1", vec![
        string(LOCAL_VAT_ID),
        sequence([root.clone(), helper.clone()].iter().map(VatObjectRef::value).collect()),
        sequence([root_ref.clone(), helper_ref.clone()].iter().map(string).collect()),
        sequence([helper_ref.clone()].iter().map(string).collect()),
    ]);
    let snapshot_ref = canonical_hash(&snapshot_body)?;
    let pass = evaluate_snapshot_authority(&RuntimeSnapshotAuthorityState {
        snapshot_ref: snapshot_ref.clone(),
        admitted_authority_refs: sorted_refs(vec![root_ref.clone(), helper_ref.clone()]),
        claimed_authority_refs: vec![helper_ref.clone()],
        requested_assertion_refs: vec![helper_ref.clone()],
        readable_assertion_refs: vec![helper_ref.clone()],
        redacted_assertion_refs: Vec::new(),
    })?;
    let denied = evaluate_snapshot_authority(&RuntimeSnapshotAuthorityState {
        snapshot_ref: snapshot_ref.clone(),
        admitted_authority_refs: sorted_refs(vec![root_ref.clone(), helper_ref.clone()]),
        claimed_authority_refs: vec![far_ref.clone()],
        requested_assertion_refs: vec![far_ref],
        readable_assertion_refs: Vec::new(),
        redacted_assertion_refs: Vec::new(),
    })?;
    let receipts = vec![pass.receipt.clone(), denied.receipt.clone()];
    let diagnostics = fixture_diagnostics(&receipts);
    let value = record("vat-snapshot-v1", vec![
        string(RUNTIME_VAT_SNAPSHOT_SCHEMA),
        record("snapshot-ref", vec![string(&snapshot_ref)]),
        snapshot_body,
        sequence([root, helper, far].iter().map(VatObjectRef::value).collect()),
        sequence(receipts.iter().map(|receipt| receipt.value.clone()).collect()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatSnapshotFixture {
        value,
        snapshot_ref,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

pub fn run_vat_promise_fixture() -> Result<VatPromiseFixture> {
    let result_ref = canonical_hash(&string("far-call-result"))?;
    let cause_ref = canonical_hash(&string("target-turn-aborted"))?;
    let pending = RuntimePromiseState::pending("promise:far-call");
    let resolved = RuntimePromiseState::resolved("promise:far-call", result_ref);
    let broken = RuntimePromiseState::broken("promise:failed-call", "target turn aborted", vec![cause_ref]);
    let cancelled = RuntimePromiseState::cancelled("promise:cancelled-call", "caller revoked interest");
    let timed_out = RuntimePromiseState::timed_out("promise:timeout-call", "logical timeout elapsed");
    let changed_terminal = RuntimePromiseState::broken("promise:far-call", "late failure", Vec::new());

    let resolve_receipt = evaluate_promise_state_transition(&pending, &resolved)?.receipt;
    let broken_receipt =
        evaluate_promise_state_transition(&RuntimePromiseState::pending("promise:failed-call"), &broken)?.receipt;
    let cancel_receipt =
        evaluate_promise_state_transition(&RuntimePromiseState::pending("promise:cancelled-call"), &cancelled)?.receipt;
    let timeout_receipt =
        evaluate_promise_state_transition(&RuntimePromiseState::pending("promise:timeout-call"), &timed_out)?.receipt;
    let terminal_denial = evaluate_promise_state_transition(&resolved, &changed_terminal)?.receipt;
    let pipeline_cleanup =
        evaluate_promise_pipeline(&RuntimePromisePipelineState::new(broken, PIPELINE_MAX_QUEUE, vec![
            RuntimePromisePipelineEntry::new(1, canonical_hash(&string("stale-target"))?, "after-break"),
        ]))?
        .receipt;

    let receipts = vec![
        resolve_receipt,
        broken_receipt,
        cancel_receipt,
        timeout_receipt,
        terminal_denial,
        pipeline_cleanup,
    ];
    let diagnostics = fixture_diagnostics(&receipts);
    let value = record("vat-promise-fixture-v1", vec![
        string(RUNTIME_VAT_PROMISE_FIXTURE_SCHEMA),
        string(LOCAL_VAT_ID),
        sequence(receipts.iter().map(|receipt| receipt.value.clone()).collect()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatPromiseFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

struct DistRefs {
    far: VatObjectRef,
    replacement: VatObjectRef,
    far_ref: String,
    replacement_ref: String,
    session_ref: String,
    pending_call_ref: String,
    stale_call_ref: String,
}

enum DistCase {
    Live,
    Disconnected,
    Handoff,
    StaleUse,
    PendingOpen,
}

fn dist_refs() -> Result<DistRefs> {
    let far = VatObjectRef::new(REMOTE_VAT_ID, FAR_OBJECT_ID, VatReferenceKind::Far, Vec::new());
    let replacement = VatObjectRef::new(REMOTE_VAT_ID, "object:remote:handoff", VatReferenceKind::Far, Vec::new());
    Ok(DistRefs {
        far_ref: far.object_ref()?,
        replacement_ref: replacement.object_ref()?,
        session_ref: canonical_hash(&record("vat-session-descriptor-v1", vec![string("session:primary")]))?,
        pending_call_ref: canonical_hash(&record("vat-pending-call-v1", vec![string("call:primary")]))?,
        stale_call_ref: canonical_hash(&record("vat-pending-call-v1", vec![string("call:stale")]))?,
        far,
        replacement,
    })
}

fn dist_state(refs: &DistRefs, case: DistCase) -> RuntimeDistributedRefLifetimeState {
    let mut state = RuntimeDistributedRefLifetimeState {
        far_ref: refs.far_ref.clone(),
        session_ref: refs.session_ref.clone(),
        replacement_ref: None,
        is_session_live: false,
        is_handoff_admitted: false,
        pending_call_refs: Vec::new(),
        failed_pending_call_refs: Vec::new(),
        attempted_use_refs: Vec::new(),
    };
    match case {
        DistCase::Live => {
            state.is_session_live = true;
            state.attempted_use_refs.push(refs.far_ref.clone());
        }
        DistCase::Disconnected => {
            state.pending_call_refs.push(refs.pending_call_ref.clone());
            state.failed_pending_call_refs.push(refs.pending_call_ref.clone());
        }
        DistCase::Handoff => {
            state.replacement_ref = Some(refs.replacement_ref.clone());
            state.is_handoff_admitted = true;
            state.attempted_use_refs.push(refs.replacement_ref.clone());
        }
        DistCase::StaleUse => state.attempted_use_refs.push(refs.far_ref.clone()),
        DistCase::PendingOpen => state.pending_call_refs.push(refs.stale_call_ref.clone()),
    }
    state
}

fn dist_receipts(refs: &DistRefs) -> Result<Vec<RuntimePredicateReceipt>> {
    [
        DistCase::Live,
        DistCase::Disconnected,
        DistCase::Handoff,
        DistCase::StaleUse,
        DistCase::PendingOpen,
    ]
    .into_iter()
    .map(|case| evaluate_distributed_ref_lifetime(&dist_state(refs, case)).map(|outcome| outcome.receipt))
    .collect()
}

pub fn run_vat_distributed_ref_fixture() -> Result<VatDistributedRefFixture> {
    let refs = dist_refs()?;
    let receipts = dist_receipts(&refs)?;
    let diagnostics = fixture_diagnostics(&receipts);
    let value = record("vat-distributed-ref-fixture-v1", vec![
        string(RUNTIME_VAT_DISTRIBUTED_REF_FIXTURE_SCHEMA),
        string(LOCAL_VAT_ID),
        sequence(vec![refs.far.value(), refs.replacement.value()]),
        sequence(receipts.iter().map(|receipt| receipt.value.clone()).collect()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatDistributedRefFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

pub fn run_vat_rights_fixture() -> Result<VatRightsFixture> {
    let root = VatObjectRef::new(LOCAL_VAT_ID, ROOT_OBJECT_ID, VatReferenceKind::Near, Vec::new());
    let root_ref = root.object_ref()?;
    let helper = VatObjectRef::new(LOCAL_VAT_ID, HELPER_OBJECT_ID, VatReferenceKind::Near, vec![root_ref.clone()]);
    let helper_ref = helper.object_ref()?;
    let brand_ref = canonical_hash(&record("vat-rights-brand-v1", vec![string("private-cooperator")]))?;
    let wrong_brand_ref = canonical_hash(&record("vat-rights-brand-v1", vec![string("unrelated-cooperator")]))?;
    let sealed_value = record("vat-sealed-value-v1", vec![
        record("brand-ref", vec![string(&brand_ref)]),
        record("sealed-authority-ref", vec![string(&root_ref)]),
    ]);
    let sealed_value_ref = canonical_hash(&sealed_value)?;

    let unsealed = evaluate_rights_amplification(&RuntimeRightsAmplificationState {
        holder_object_ref: helper_ref.clone(),
        sealed_value_ref: sealed_value_ref.clone(),
        sealer_brand_ref: brand_ref.clone(),
        unsealer_brand_ref: brand_ref.clone(),
        sealed_authority_refs: vec![root_ref.clone()],
        recovered_authority_refs: vec![root_ref.clone()],
    })?;
    let wrong_unsealer = evaluate_rights_amplification(&RuntimeRightsAmplificationState {
        holder_object_ref: helper_ref.clone(),
        sealed_value_ref: sealed_value_ref.clone(),
        sealer_brand_ref: brand_ref.clone(),
        unsealer_brand_ref: wrong_brand_ref,
        sealed_authority_refs: vec![root_ref.clone()],
        recovered_authority_refs: vec![root_ref.clone()],
    })?;
    let over_recovery_ref = canonical_hash(&string("unsealed-extra-authority"))?;
    let over_recovery = evaluate_rights_amplification(&RuntimeRightsAmplificationState {
        holder_object_ref: helper_ref,
        sealed_value_ref: sealed_value_ref.clone(),
        sealer_brand_ref: brand_ref.clone(),
        unsealer_brand_ref: brand_ref,
        sealed_authority_refs: vec![root_ref.clone()],
        recovered_authority_refs: sorted_refs(vec![root_ref, over_recovery_ref]),
    })?;

    let receipts = vec![unsealed.receipt, wrong_unsealer.receipt, over_recovery.receipt];
    let diagnostics = fixture_diagnostics(&receipts);
    let value = record("vat-rights-fixture-v1", vec![
        string(RUNTIME_VAT_RIGHTS_FIXTURE_SCHEMA),
        string(LOCAL_VAT_ID),
        sealed_value,
        sequence([root, helper].iter().map(VatObjectRef::value).collect()),
        sequence(receipts.iter().map(|receipt| receipt.value.clone()).collect()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatRightsFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

pub fn run_vat_ambient_authority_fixture() -> Result<VatAmbientAuthorityFixture> {
    let spawned = VatObjectRef::new(LOCAL_VAT_ID, SPAWNED_OBJECT_ID, VatReferenceKind::Near, Vec::new());
    let spawned_ref = spawned.object_ref()?;
    let authority_kinds = [
        RuntimeObjectAuthorityKind::Filesystem,
        RuntimeObjectAuthorityKind::Network,
        RuntimeObjectAuthorityKind::Clock,
        RuntimeObjectAuthorityKind::Process,
        RuntimeObjectAuthorityKind::Dataspace,
        RuntimeObjectAuthorityKind::Store,
        RuntimeObjectAuthorityKind::Blob,
        RuntimeObjectAuthorityKind::Consensus,
        RuntimeObjectAuthorityKind::Choreography,
        RuntimeObjectAuthorityKind::HostResource,
    ];
    let mut authority_refs = Vec::with_capacity(authority_kinds.len());
    let mut receipts = Vec::with_capacity(authority_kinds.len() + 1);
    for authority_kind in authority_kinds {
        let authority_ref = authority_descriptor_ref(authority_kind)?;
        let denied = evaluate_object_authority(&RuntimeObjectAuthorityState {
            object_ref: spawned_ref.clone(),
            requested_authority_ref: authority_ref.clone(),
            requested_authority_kind: authority_kind,
            endowed_authority_refs: Vec::new(),
            admitted_authority_refs: Vec::new(),
        })?;
        authority_refs.push(authority_ref);
        receipts.push(denied.receipt);
    }

    let clock_ref = authority_descriptor_ref(RuntimeObjectAuthorityKind::Clock)?;
    let clock_pass = evaluate_object_authority(&RuntimeObjectAuthorityState {
        object_ref: spawned_ref,
        requested_authority_ref: clock_ref.clone(),
        requested_authority_kind: RuntimeObjectAuthorityKind::Clock,
        endowed_authority_refs: vec![clock_ref.clone()],
        admitted_authority_refs: vec![clock_ref],
    })?;
    receipts.push(clock_pass.receipt);

    let diagnostics = fixture_diagnostics(&receipts);
    let value = record("vat-ambient-authority-fixture-v1", vec![
        string(RUNTIME_VAT_AMBIENT_AUTHORITY_FIXTURE_SCHEMA),
        string(LOCAL_VAT_ID),
        spawned.value(),
        sequence(authority_refs.iter().map(string).collect()),
        sequence(receipts.iter().map(|receipt| receipt.value.clone()).collect()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatAmbientAuthorityFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

pub fn run_vat_restore_fixture() -> Result<VatRestoreFixture> {
    let old_object = versioned_object_value("object:legacy", "schema:v1", "legacy-state");
    let old_object_ref = canonical_hash(&old_object)?;
    let upgraded_object = versioned_object_value("object:legacy", "schema:v2", "legacy-state");
    let upgraded_object_ref = canonical_hash(&upgraded_object)?;
    let recipe = vat_upgrade_recipe_value("schema:v1", "schema:v2", "schema-rename", &old_object_ref)?;
    let recipe_ref = canonical_hash(&recipe)?;
    let pass_receipt = vat_restore_receipt_value(VatRestoreReceiptInput {
        decision: "pass",
        snapshot_ref: &old_object_ref,
        recipe_ref: Some(&recipe_ref),
        restored_object_ref: Some(&upgraded_object_ref),
        diagnostics: Vec::new(),
    });
    let deny_diagnostics = vec!["missing-compatible-upgrade-recipe".to_string()];
    let deny_receipt = vat_restore_receipt_value(VatRestoreReceiptInput {
        decision: "deny",
        snapshot_ref: &old_object_ref,
        recipe_ref: None,
        restored_object_ref: None,
        diagnostics: deny_diagnostics.clone(),
    });
    let receipts = vec![pass_receipt, deny_receipt];
    let diagnostics = restore_diagnostics(&receipts)?;
    let value = record("vat-restore-fixture-v1", vec![
        string("molten.runtime.vat-restore-fixture.v1"),
        old_object,
        upgraded_object,
        recipe,
        sequence(receipts.clone()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatRestoreFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

pub fn run_vat_time_travel_fixture() -> Result<VatDebugFixture> {
    let root = VatObjectRef::new(LOCAL_VAT_ID, ROOT_OBJECT_ID, VatReferenceKind::Near, Vec::new());
    let helper = VatObjectRef::new(LOCAL_VAT_ID, HELPER_OBJECT_ID, VatReferenceKind::Near, vec![root.object_ref()?]);
    let root_ref = root.object_ref()?;
    let helper_ref = helper.object_ref()?;
    let turn_trace = record("vat-turn-trace-v1", vec![
        string(LOCAL_VAT_ID),
        string("turn:0001"),
        sequence([root_ref.clone(), helper_ref.clone()].iter().map(string).collect()),
        sequence(["message:deliver", "assertion:add", "turn:commit"].iter().map(string).collect()),
    ]);
    let turn_trace_ref = canonical_hash(&turn_trace)?;
    let snapshot = record("vat-debug-snapshot-v1", vec![
        string(LOCAL_VAT_ID),
        string("turn:0001"),
        record("trace-ref", vec![string(&turn_trace_ref)]),
        sequence([root_ref.clone(), helper_ref.clone()].iter().map(string).collect()),
    ]);
    let snapshot_ref = canonical_hash(&snapshot)?;
    let replay = record("vat-replay-hook-v1", vec![
        record("snapshot-ref", vec![string(&snapshot_ref)]),
        record("trace-ref", vec![string(&turn_trace_ref)]),
        string("deterministic-replay"),
    ]);
    let replay_ref = canonical_hash(&replay)?;
    let receipts = vec![
        vat_debug_receipt_value(VatDebugReceiptInput {
            kind: "vat-time-travel-debug-receipt-v1",
            schema: RUNTIME_VAT_TIME_TRAVEL_FIXTURE_SCHEMA,
            decision: "pass",
            subject_ref: &snapshot_ref,
            evidence_refs: sorted_refs(vec![turn_trace_ref.clone(), replay_ref]),
            diagnostics: vec![
                "reconstructs-prior-turn".to_string(),
                "correlates-trace-snapshot-replay".to_string(),
            ],
        }),
        vat_debug_receipt_value(VatDebugReceiptInput {
            kind: "vat-time-travel-debug-receipt-v1",
            schema: RUNTIME_VAT_TIME_TRAVEL_FIXTURE_SCHEMA,
            decision: "deny",
            subject_ref: &snapshot_ref,
            evidence_refs: vec![turn_trace_ref],
            diagnostics: vec![
                "debug-authority-missing".to_string(),
                "redacted-events-not-readable".to_string(),
            ],
        }),
    ];
    let diagnostics = debug_diagnostics(&receipts)?;
    let value = record("vat-time-travel-fixture-v1", vec![
        string(RUNTIME_VAT_TIME_TRAVEL_FIXTURE_SCHEMA),
        turn_trace,
        snapshot,
        replay,
        sequence([root, helper].iter().map(VatObjectRef::value).collect()),
        sequence(receipts.clone()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatDebugFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

pub fn run_vat_replay_fixture() -> Result<VatReplayFixture> {
    let expected = case_run(RunCase::Baseline)?;
    let actual = case_run(RunCase::Baseline)?;
    let changed_input = case_run(RunCase::Input)?;
    let changed_effect = case_run(RunCase::Effect)?;
    let changed_sequence = case_run(RunCase::Sequence)?;
    let changed_policy = case_run(RunCase::Policy)?;
    let changed_state = case_run(RunCase::State)?;
    let generic_pass =
        deterministic_replay::verify_fixture_value(deterministic_replay::ReplayFixtureVariant::Baseline)?;
    let generic_deny =
        deterministic_replay::verify_fixture_value(deterministic_replay::ReplayFixtureVariant::ChangedEffectResponse)?;
    let generic_first_divergence = generic_deny
        .first_divergence
        .clone()
        .unwrap_or_else(|| record("deterministic-first-divergence-v1", Vec::new()));
    let receipts = vec![
        vat_replay_receipt_value(&expected, &actual)?,
        vat_replay_receipt_value(&expected, &changed_input)?,
        vat_replay_receipt_value(&expected, &changed_effect)?,
        vat_replay_receipt_value(&expected, &changed_sequence)?,
        vat_replay_receipt_value(&expected, &changed_policy)?,
        vat_replay_receipt_value(&expected, &changed_state)?,
    ];
    let generic_receipts = vec![generic_pass.value, generic_deny.value, generic_first_divergence];
    let mut diagnostic_receipts = receipts.clone();
    diagnostic_receipts.extend(generic_receipts.clone());
    let diagnostics = debug_diagnostics(&diagnostic_receipts)?;
    let value = record("vat-replay-fixture-v1", vec![
        string(RUNTIME_VAT_REPLAY_FIXTURE_SCHEMA),
        record("profile", vec![string("replay")]),
        record("policy", vec![string("no-real-external-effects")]),
        record("expected-run-ref", vec![string(&expected.run_ref)]),
        sequence(
            [
                actual.value,
                changed_input.value,
                changed_effect.value,
                changed_sequence.value,
                changed_policy.value,
                changed_state.value,
            ]
            .to_vec(),
        ),
        sequence(receipts.clone()),
        sequence(generic_receipts),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatReplayFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

pub fn run_vat_authority_graph_fixture() -> Result<VatDebugFixture> {
    let root = VatObjectRef::new(LOCAL_VAT_ID, ROOT_OBJECT_ID, VatReferenceKind::Near, Vec::new());
    let helper = VatObjectRef::new(LOCAL_VAT_ID, HELPER_OBJECT_ID, VatReferenceKind::Near, vec![root.object_ref()?]);
    let proxy = VatObjectRef::new(LOCAL_VAT_ID, PROXY_OBJECT_ID, VatReferenceKind::Proxy, vec![helper.object_ref()?]);
    let root_ref = root.object_ref()?;
    let helper_ref = helper.object_ref()?;
    let proxy_ref = proxy.object_ref()?;
    let graph = record("vat-authority-graph-v1", vec![
        string(LOCAL_VAT_ID),
        sequence([root_ref.clone(), helper_ref.clone(), proxy_ref.clone()].iter().map(string).collect()),
        sequence(
            [
                authority_edge_value(&helper_ref, &root_ref, "holds"),
                authority_edge_value(&proxy_ref, &helper_ref, "attenuates"),
            ]
            .to_vec(),
        ),
        sequence([proxy_ref.clone()].iter().map(string).collect()),
    ]);
    let graph_ref = canonical_hash(&graph)?;
    let receipts = vec![
        vat_debug_receipt_value(VatDebugReceiptInput {
            kind: "vat-authority-graph-inspect-receipt-v1",
            schema: RUNTIME_VAT_AUTHORITY_GRAPH_FIXTURE_SCHEMA,
            decision: "pass",
            subject_ref: &graph_ref,
            evidence_refs: sorted_refs(vec![root_ref.clone(), helper_ref.clone(), proxy_ref.clone()]),
            diagnostics: vec![
                "authority-graph-readable".to_string(),
                "proxy-chain-visible".to_string(),
            ],
        }),
        vat_debug_receipt_value(VatDebugReceiptInput {
            kind: "vat-authority-graph-inspect-receipt-v1",
            schema: RUNTIME_VAT_AUTHORITY_GRAPH_FIXTURE_SCHEMA,
            decision: "deny",
            subject_ref: &graph_ref,
            evidence_refs: vec![proxy_ref],
            diagnostics: vec![
                "inspection-authority-missing".to_string(),
                "redacted-edge-not-disclosed".to_string(),
            ],
        }),
    ];
    let diagnostics = debug_diagnostics(&receipts)?;
    let value = record("vat-authority-graph-fixture-v1", vec![
        string(RUNTIME_VAT_AUTHORITY_GRAPH_FIXTURE_SCHEMA),
        graph,
        sequence([root, helper, proxy].iter().map(VatObjectRef::value).collect()),
        sequence(receipts.clone()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatDebugFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

pub fn run_vat_portable_storage_fixture() -> Result<VatDebugFixture> {
    let chunk_a = canonical_hash(&record("encrypted-chunk-v1", vec![string("ciphertext:a")]))?;
    let chunk_b = canonical_hash(&record("encrypted-chunk-v1", vec![string("ciphertext:b")]))?;
    let read_cap = canonical_hash(&record("storage-read-capability-v1", vec![string("snapshot-reader")]))?;
    let write_cap = canonical_hash(&record("storage-write-capability-v1", vec![string("snapshot-writer")]))?;
    let manifest = record("vat-portable-storage-manifest-v1", vec![
        string("fixed_v1"),
        string("encrypted-before-storage"),
        string("provider-independent"),
        sequence([chunk_a.clone(), chunk_b.clone()].iter().map(string).collect()),
        sequence([read_cap.clone(), write_cap.clone()].iter().map(string).collect()),
    ]);
    let manifest_ref = canonical_hash(&manifest)?;
    let receipts = vec![
        vat_debug_receipt_value(VatDebugReceiptInput {
            kind: "vat-portable-storage-receipt-v1",
            schema: RUNTIME_VAT_PORTABLE_STORAGE_FIXTURE_SCHEMA,
            decision: "pass",
            subject_ref: &manifest_ref,
            evidence_refs: sorted_refs(vec![chunk_a.clone(), chunk_b.clone(), read_cap.clone(), write_cap]),
            diagnostics: vec![
                "content-addressed-chunked-encrypted".to_string(),
                "provider-independent".to_string(),
            ],
        }),
        vat_debug_receipt_value(VatDebugReceiptInput {
            kind: "vat-portable-storage-receipt-v1",
            schema: RUNTIME_VAT_PORTABLE_STORAGE_FIXTURE_SCHEMA,
            decision: "deny",
            subject_ref: &manifest_ref,
            evidence_refs: vec![chunk_a, read_cap],
            diagnostics: vec![
                "plaintext-storage-denied".to_string(),
                "provider-bound-location-denied".to_string(),
            ],
        }),
    ];
    let diagnostics = debug_diagnostics(&receipts)?;
    let value = record("vat-portable-storage-fixture-v1", vec![
        string(RUNTIME_VAT_PORTABLE_STORAGE_FIXTURE_SCHEMA),
        manifest,
        sequence(receipts.clone()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatDebugFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

pub fn vat_fixture_summary(value: &IOValue) -> Result<String> {
    let artifact_ref = canonical_hash(value)?;
    Ok(format!("vat artifact: {artifact_ref}"))
}

struct VatRestoreReceiptInput<'a> {
    decision: &'a str,
    snapshot_ref: &'a str,
    recipe_ref: Option<&'a str>,
    restored_object_ref: Option<&'a str>,
    diagnostics: Vec<String>,
}

struct VatDebugReceiptInput<'a> {
    kind: &'static str,
    schema: &'static str,
    decision: &'a str,
    subject_ref: &'a str,
    evidence_refs: Vec<String>,
    diagnostics: Vec<String>,
}

fn vat_debug_receipt_value(input: VatDebugReceiptInput<'_>) -> IOValue {
    record(input.kind, vec![
        string(input.schema),
        string(input.decision),
        record("subject-ref", vec![string(input.subject_ref)]),
        sequence(input.evidence_refs.iter().map(string).collect()),
        sequence(input.diagnostics.iter().map(string).collect()),
    ])
}

struct VatReplayRunInput {
    seed: &'static str,
    input_message: &'static str,
    effect_response: &'static str,
    random_sequence: &'static str,
    random_response: &'static str,
    policy_decision: &'static str,
    state_marker: &'static str,
}

enum RunCase {
    Baseline,
    Input,
    Effect,
    Sequence,
    Policy,
    State,
}

fn case_run(case: RunCase) -> Result<VatReplayRun> {
    let mut input = VatReplayRunInput {
        seed: "seed:vat-replay:0001",
        input_message: "deliver:root-to-helper",
        effect_response: "clock:logical:42",
        random_sequence: "random-seq:0001",
        random_response: "random:seeded:7",
        policy_decision: "policy:allow",
        state_marker: "state:committed",
    };
    match case {
        RunCase::Baseline => {}
        RunCase::Input => input.input_message = "deliver:root-to-helper:changed",
        RunCase::Effect => input.effect_response = "clock:logical:43",
        RunCase::Sequence => input.random_sequence = "random-seq:changed",
        RunCase::Policy => input.policy_decision = "policy:deny",
        RunCase::State => input.state_marker = "state:diverged",
    }
    vat_replay_run(input)
}

fn vat_replay_run(run_input: VatReplayRunInput) -> Result<VatReplayRun> {
    let root = VatObjectRef::new(LOCAL_VAT_ID, ROOT_OBJECT_ID, VatReferenceKind::Near, Vec::new());
    let helper = VatObjectRef::new(LOCAL_VAT_ID, HELPER_OBJECT_ID, VatReferenceKind::Near, vec![root.object_ref()?]);
    let root_ref = root.object_ref()?;
    let helper_ref = helper.object_ref()?;
    let initial_state = record("vat-replay-initial-state-v1", vec![
        string(run_input.seed),
        sequence([root_ref.clone(), helper_ref.clone()].iter().map(string).collect()),
    ]);
    let initial_state_ref = canonical_hash(&initial_state)?;
    let input = record("vat-replay-input-v1", vec![
        string(run_input.input_message),
        record("sender-ref", vec![string(&root_ref)]),
        record("target-ref", vec![string(&helper_ref)]),
    ]);
    let input_ref = canonical_hash(&input)?;
    let effect_request = record("vat-replay-effect-request-v1", vec![
        string("clock"),
        string("logical-time"),
        record("input-ref", vec![string(&input_ref)]),
        record("profile", vec![string("replay")]),
    ]);
    let effect_request_ref = canonical_hash(&effect_request)?;
    let effect_response = record("vat-replay-effect-response-v1", vec![
        string(run_input.effect_response),
        record("request-ref", vec![string(&effect_request_ref)]),
        record("source", vec![string("recorded-effect-log")]),
    ]);
    let effect_response_ref = canonical_hash(&effect_response)?;
    let random_request = record("vat-replay-effect-request-v1", vec![
        string("random"),
        string(run_input.random_sequence),
        record("input-ref", vec![string(&input_ref)]),
        record("profile", vec![string("replay")]),
    ]);
    let random_request_ref = canonical_hash(&random_request)?;
    let random_response = record("vat-replay-effect-response-v1", vec![
        string(run_input.random_response),
        record("request-ref", vec![string(&random_request_ref)]),
        record("source", vec![string("seeded-prng")]),
    ]);
    let random_response_ref = canonical_hash(&random_response)?;
    let policy_decision = record("vat-replay-policy-decision-v1", vec![
        string(run_input.policy_decision),
        record("input-ref", vec![string(&input_ref)]),
        record("effect-response-ref", vec![string(&effect_response_ref)]),
        record("random-response-ref", vec![string(&random_response_ref)]),
    ]);
    let policy_decision_ref = canonical_hash(&policy_decision)?;
    let final_state = record("vat-replay-final-state-v1", vec![
        record("initial-state-ref", vec![string(&initial_state_ref)]),
        record("input-ref", vec![string(&input_ref)]),
        record("effect-response-ref", vec![string(&effect_response_ref)]),
        record("random-response-ref", vec![string(&random_response_ref)]),
        record("policy-decision-ref", vec![string(&policy_decision_ref)]),
        record("state-marker", vec![string(run_input.state_marker)]),
        sequence([root_ref.clone(), helper_ref.clone()].iter().map(string).collect()),
    ]);
    let final_state_hash = canonical_hash(&final_state)?;
    let trace = record("vat-replay-turn-trace-v1", vec![
        string("turn:replay:0001"),
        record("scheduler-key", vec![string("logical:0:priority:0:queue:0:vat:fixture:local")]),
        record("input-ref", vec![string(&input_ref)]),
        record("effect-request-ref", vec![string(&effect_request_ref)]),
        record("effect-response-ref", vec![string(&effect_response_ref)]),
        record("random-request-ref", vec![string(&random_request_ref)]),
        record("random-response-ref", vec![string(&random_response_ref)]),
        record("policy-decision-ref", vec![string(&policy_decision_ref)]),
        record("after-state-ref", vec![string(&final_state_hash)]),
    ]);
    let trace_ref = canonical_hash(&trace)?;
    let value = record("vat-deterministic-replay-run-v1", vec![
        string(RUNTIME_VAT_REPLAY_FIXTURE_SCHEMA),
        record("profile", vec![string("replay")]),
        record("seed", vec![string(run_input.seed)]),
        record("initial-state-ref", vec![string(&initial_state_ref)]),
        record("input-ref", vec![string(&input_ref)]),
        record("effect-request-ref", vec![string(&effect_request_ref)]),
        record("effect-response-ref", vec![string(&effect_response_ref)]),
        record("random-request-ref", vec![string(&random_request_ref)]),
        record("random-response-ref", vec![string(&random_response_ref)]),
        record("policy-decision-ref", vec![string(&policy_decision_ref)]),
        record("trace-ref", vec![string(&trace_ref)]),
        record("final-state-ref", vec![string(&final_state_hash)]),
        record("external-effects", vec![string("denied")]),
    ]);
    let run_ref = canonical_hash(&value)?;
    Ok(VatReplayRun {
        value,
        run_ref,
        trace_ref,
        effect_request_ref,
        effect_response_ref,
        random_request_ref,
        random_response_ref,
        policy_decision_ref,
        final_state_hash,
    })
}

fn vat_replay_divergence(expected: &VatReplayRun, actual: &VatReplayRun) -> VatReplayDivergenceKind {
    if expected.run_ref == actual.run_ref {
        return VatReplayDivergenceKind::None;
    }
    if expected.effect_request_ref != actual.effect_request_ref {
        return VatReplayDivergenceKind::Input;
    }
    if expected.effect_response_ref != actual.effect_response_ref {
        return VatReplayDivergenceKind::EffectResponse;
    }
    if expected.random_request_ref != actual.random_request_ref {
        return VatReplayDivergenceKind::EffectRequest;
    }
    if expected.random_response_ref != actual.random_response_ref {
        return VatReplayDivergenceKind::EffectResponse;
    }
    if expected.policy_decision_ref != actual.policy_decision_ref {
        return VatReplayDivergenceKind::PolicyDecision;
    }
    VatReplayDivergenceKind::StateHash
}

fn vat_replay_receipt_value(expected: &VatReplayRun, actual: &VatReplayRun) -> Result<IOValue> {
    let divergence = vat_replay_divergence(expected, actual);
    let decision = if divergence == VatReplayDivergenceKind::None {
        "pass"
    } else {
        "deny"
    };
    let diagnostics = vat_replay_diagnostics(divergence);
    Ok(record("vat-replay-receipt-v1", vec![
        string(RUNTIME_VAT_REPLAY_FIXTURE_SCHEMA),
        string(decision),
        record("profile", vec![string("replay")]),
        record("expected-run-ref", vec![string(&expected.run_ref)]),
        record("actual-run-ref", vec![string(&actual.run_ref)]),
        record("divergence", vec![string(divergence.as_str())]),
        record("expected-trace-ref", vec![string(&expected.trace_ref)]),
        record("actual-trace-ref", vec![string(&actual.trace_ref)]),
        record("expected-random-request-ref", vec![string(&expected.random_request_ref)]),
        record("actual-random-request-ref", vec![string(&actual.random_request_ref)]),
        record("expected-random-response-ref", vec![string(&expected.random_response_ref)]),
        record("actual-random-response-ref", vec![string(&actual.random_response_ref)]),
        record("expected-policy-decision-ref", vec![string(&expected.policy_decision_ref)]),
        record("actual-policy-decision-ref", vec![string(&actual.policy_decision_ref)]),
        record("expected-final-state-ref", vec![string(&expected.final_state_hash)]),
        record("actual-final-state-ref", vec![string(&actual.final_state_hash)]),
        sequence(diagnostics.iter().map(string).collect()),
    ]))
}

fn vat_replay_diagnostics(divergence: VatReplayDivergenceKind) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(3);
    diagnostics.push("replay-profile-denies-real-external-effects".to_string());
    diagnostics.push("logical-clock-response-stable".to_string());
    diagnostics.push("seeded-random-response-stable".to_string());
    match divergence {
        VatReplayDivergenceKind::None => diagnostics.push("deterministic-replay-identical-trace-and-state".to_string()),
        VatReplayDivergenceKind::Input => diagnostics.push("first-divergence-input".to_string()),
        VatReplayDivergenceKind::EffectRequest => diagnostics.push("first-divergence-effect-request".to_string()),
        VatReplayDivergenceKind::EffectResponse => diagnostics.push("first-divergence-effect-response".to_string()),
        VatReplayDivergenceKind::PolicyDecision => diagnostics.push("first-divergence-policy-decision".to_string()),
        VatReplayDivergenceKind::StateHash => diagnostics.push("first-divergence-state-hash".to_string()),
    }
    diagnostics
}

fn authority_edge_value(from_ref: &str, to_ref: &str, edge_kind: &str) -> IOValue {
    record("authority-edge-v1", vec![string(from_ref), string(to_ref), string(edge_kind)])
}

fn authority_descriptor_ref(authority_kind: RuntimeObjectAuthorityKind) -> Result<String> {
    canonical_hash(&record("vat-authority-descriptor-v1", vec![string(authority_kind.as_str())]))
}

fn versioned_object_value(object_id: &'static str, schema_version: &'static str, state: &'static str) -> IOValue {
    record("vat-versioned-object-v1", vec![string(object_id), string(schema_version), string(state)])
}

fn vat_upgrade_recipe_value(
    source_schema: &'static str,
    target_schema: &'static str,
    transformer: &'static str,
    evidence_ref: &str,
) -> Result<IOValue> {
    Ok(record("vat-object-upgrade-recipe-v1", vec![
        string(RUNTIME_VAT_OBJECT_UPGRADE_RECIPE_SCHEMA),
        string(source_schema),
        string(target_schema),
        string(transformer),
        record("evidence-ref", vec![string(evidence_ref)]),
    ]))
}

fn vat_restore_receipt_value(input: VatRestoreReceiptInput<'_>) -> IOValue {
    record("vat-restore-receipt-v1", vec![
        string(RUNTIME_VAT_RESTORE_RECEIPT_SCHEMA),
        string(input.decision),
        record("snapshot-ref", vec![string(input.snapshot_ref)]),
        optional_ref_value("recipe-ref", input.recipe_ref),
        optional_ref_value("restored-object-ref", input.restored_object_ref),
        sequence(input.diagnostics.iter().map(string).collect()),
    ])
}

fn optional_ref_value(label: &'static str, value: Option<&str>) -> IOValue {
    match value {
        Some(reference) => record(label, vec![string(reference)]),
        None => record(label, Vec::new()),
    }
}

fn restore_diagnostics(receipts: &[IOValue]) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(receipts.len());
    for receipt in receipts {
        let receipt_ref = canonical_hash(receipt)?;
        diagnostics.push(format!("restore-receipt:{receipt_ref}"));
    }
    Ok(diagnostics)
}

fn debug_diagnostics(receipts: &[IOValue]) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(receipts.len() + 1);
    for receipt in receipts {
        let receipt_ref = canonical_hash(receipt)?;
        diagnostics.push(format!("debug-receipt:{receipt_ref}"));
    }
    diagnostics.push("evidence-only-debugging-surface".to_string());
    Ok(diagnostics)
}

fn sorted_refs(mut refs: Vec<String>) -> Vec<String> {
    refs.sort();
    refs.dedup();
    refs
}

fn fixture_diagnostics(receipts: &[RuntimePredicateReceipt]) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if receipts.iter().any(|receipt| receipt.decision == PredicateDecision::Deny) {
        diagnostics.push("expected-denials-present".to_string());
    }
    if receipts.iter().all(|receipt| receipt.decision == PredicateDecision::Pass) {
        diagnostics.push("missing-negative-coverage".to_string());
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::PIPELINE_MAX_QUEUE;
    use super::canonical_hash;
    use super::run_vat_ambient_authority_fixture;
    use super::run_vat_authority_graph_fixture;
    use super::run_vat_distributed_ref_fixture;
    use super::run_vat_fixture;
    use super::run_vat_portable_storage_fixture;
    use super::run_vat_promise_fixture;
    use super::run_vat_replay_fixture;
    use super::run_vat_restore_fixture;
    use super::run_vat_rights_fixture;
    use super::run_vat_snapshot_fixture;
    use super::run_vat_time_travel_fixture;
    use super::sorted_refs;
    use super::string;
    use super::vat_fixture_summary;
    use crate::preserves_rail::to_text;
    use crate::runtime::PredicateDecision;
    use crate::runtime::RuntimeActormapTransactionOutcome;
    use crate::runtime::RuntimeActormapTransactionState;
    use crate::runtime::RuntimePromisePipelineEntry;
    use crate::runtime::RuntimePromisePipelineState;
    use crate::runtime::RuntimePromiseState;
    use crate::runtime::evaluate_actormap_transaction;
    use crate::runtime::evaluate_promise_pipeline;

    #[test]
    fn vat_fixture_binds_near_far_actormap_pipeline_and_revocation_predicates() {
        let run = run_vat_fixture().expect("vat fixture run");
        assert_eq!(run.receipts.len(), 7);
        assert!(run.receipts.iter().any(|receipt| receipt.predicate == "molten.trellis-runtime.near-far-refs.v1"));
        assert!(
            run.receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.actormap-transaction.v1")
        );
        assert!(run.receipts.iter().any(|receipt| receipt.predicate == "molten.trellis-runtime.promise-pipeline.v1"));
        assert!(
            run.receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.revocation-cleanup.v1")
        );
        assert!(run.receipts.iter().any(|receipt| receipt.decision == PredicateDecision::Deny));
        assert!(run.diagnostics.iter().any(|diagnostic| diagnostic == "expected-denials-present"));
        assert!(run.run_ref.starts_with("blake3:"));
    }

    #[test]
    fn vat_fixture_summary_uses_canonical_ref() {
        let run = run_vat_fixture().expect("vat fixture run");
        let summary = vat_fixture_summary(&run.value).expect("summary");
        assert!(summary.contains(&run.run_ref));
    }

    #[test]
    fn vat_snapshot_fixture_denies_unheld_authority() {
        let snapshot = run_vat_snapshot_fixture().expect("snapshot fixture");
        assert_eq!(snapshot.receipts.len(), 2);
        assert!(snapshot.snapshot_ref.starts_with("blake3:"));
        assert!(snapshot.fixture_ref.starts_with("blake3:"));
        assert!(
            snapshot
                .receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.snapshot-authority.v1")
        );
        assert!(snapshot.receipts.iter().any(|receipt| receipt.decision == PredicateDecision::Pass));
        assert!(snapshot.receipts.iter().any(|receipt| receipt.decision == PredicateDecision::Deny));
        assert!(
            snapshot
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "snapshot-claimed-authority-not-admitted")
        );
    }

    #[test]
    fn vat_restore_fixture_records_upgrade_and_missing_recipe_denial() {
        let restore = run_vat_restore_fixture().expect("restore fixture");
        assert_eq!(restore.receipts.len(), 2);
        assert!(restore.fixture_ref.starts_with("blake3:"));
        assert!(restore.diagnostics.iter().all(|diagnostic| diagnostic.starts_with("restore-receipt:blake3:")));
        let rendered = to_text(&restore.value).expect("render restore fixture");
        assert!(rendered.contains("vat-object-upgrade-recipe-v1"));
        assert!(rendered.contains("missing-compatible-upgrade-recipe"));
    }

    #[test]
    fn vat_distributed_ref_fixture_records_lifetime_and_handoff() {
        let distributed_ref = run_vat_distributed_ref_fixture().expect("distributed ref fixture");
        assert_eq!(distributed_ref.receipts.len(), 5);
        assert!(distributed_ref.fixture_ref.starts_with("blake3:"));
        assert!(
            distributed_ref
                .receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.distributed-ref-lifetime.v1")
        );
        assert!(distributed_ref.receipts.iter().any(|receipt| receipt.decision == PredicateDecision::Pass));
        assert!(distributed_ref.receipts.iter().any(|receipt| receipt.decision == PredicateDecision::Deny));
        assert!(
            distributed_ref
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "distributed-ref-stale-descriptor-used")
        );
        assert!(
            distributed_ref
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "distributed-ref-disconnected-pending-calls-not-failed")
        );
    }

    #[test]
    fn vat_rights_fixture_records_unseal_and_denials() {
        let rights = run_vat_rights_fixture().expect("rights fixture");
        assert_eq!(rights.receipts.len(), 3);
        assert!(rights.fixture_ref.starts_with("blake3:"));
        assert!(
            rights
                .receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.rights-amplification.v1")
        );
        assert!(rights.receipts.iter().any(|receipt| receipt.decision == PredicateDecision::Pass));
        assert!(rights.receipts.iter().any(|receipt| receipt.decision == PredicateDecision::Deny));
        assert!(
            rights
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "rights-amplification-brand-mismatch")
        );
        assert!(
            rights
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "rights-amplification-recovered-authority-not-sealed")
        );
    }

    #[test]
    fn vat_ambient_authority_fixture_denies_unendowed_authority() {
        let authority = run_vat_ambient_authority_fixture().expect("ambient authority fixture");
        assert_eq!(authority.receipts.len(), 11);
        assert!(authority.fixture_ref.starts_with("blake3:"));
        assert!(
            authority
                .receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.object-authority.v1")
        );
        assert!(authority.receipts.iter().any(|receipt| receipt.decision == PredicateDecision::Pass));
        assert!(authority.receipts.iter().any(|receipt| receipt.decision == PredicateDecision::Deny));
        assert!(
            authority
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "object-authority-not-endowed")
        );
        assert!(
            authority
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "object-authority-not-policy-admitted")
        );
    }

    #[test]
    fn vat_promise_fixture_records_terminal_results_and_denials() {
        let promise = run_vat_promise_fixture().expect("promise fixture");
        assert_eq!(promise.receipts.len(), 6);
        assert!(promise.fixture_ref.starts_with("blake3:"));
        assert!(
            promise
                .receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.promise-state.v1")
        );
        assert!(
            promise
                .receipts
                .iter()
                .any(|receipt| receipt.predicate == "molten.trellis-runtime.promise-pipeline.v1")
        );
        assert!(promise.receipts.iter().any(|receipt| receipt.decision == PredicateDecision::Pass));
        assert!(promise.receipts.iter().any(|receipt| receipt.decision == PredicateDecision::Deny));
        assert!(
            promise
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "terminal-promise-state-changed")
        );
        assert!(
            promise
                .receipts
                .iter()
                .flat_map(|receipt| receipt.diagnostics.iter())
                .any(|diagnostic| diagnostic == "terminal-promise-pipeline-not-cleaned")
        );
    }

    #[test]
    fn vat_time_travel_fixture_records_trace_snapshot_replay_hooks() {
        let debug = run_vat_time_travel_fixture().expect("time travel fixture");
        assert_eq!(debug.receipts.len(), 2);
        assert!(debug.fixture_ref.starts_with("blake3:"));
        assert!(debug.diagnostics.iter().any(|diagnostic| diagnostic == "evidence-only-debugging-surface"));
        let rendered = to_text(&debug.value).expect("render time travel fixture");
        assert!(rendered.contains("vat-time-travel-debug-receipt-v1"));
        assert!(rendered.contains("debug-authority-missing"));
        assert!(rendered.contains("deterministic-replay"));
    }

    #[test]
    fn vat_replay_fixture_reports_identity_and_first_divergence() {
        let replay = run_vat_replay_fixture().expect("replay fixture");
        assert_eq!(replay.receipts.len(), 6);
        assert!(replay.fixture_ref.starts_with("blake3:"));
        let rendered = to_text(&replay.value).expect("render replay fixture");
        assert!(rendered.contains("vat-replay-receipt-v1"));
        assert!(rendered.contains("deterministic-replay-verify-v1"));
        assert!(rendered.contains("deterministic-first-divergence-v1"));
        assert!(rendered.contains("evidence-only-debugging-surface"));
        assert!(rendered.contains("deterministic-replay-identical-trace-and-state"));
        assert!(rendered.contains("first-divergence-input"));
        assert!(rendered.contains("first-divergence-effect-response"));
        assert!(rendered.contains("first-divergence-effect-request"));
        assert!(rendered.contains("first-divergence-policy-decision"));
        assert!(rendered.contains("first-divergence-state-hash"));
        assert!(rendered.contains("logical-clock-response-stable"));
        assert!(rendered.contains("seeded-random-response-stable"));
        assert!(rendered.contains("replay-profile-denies-real-external-effects"));
    }

    #[test]
    fn vat_authority_graph_fixture_records_inspection_denials() {
        let graph = run_vat_authority_graph_fixture().expect("authority graph fixture");
        assert_eq!(graph.receipts.len(), 2);
        assert!(graph.fixture_ref.starts_with("blake3:"));
        let rendered = to_text(&graph.value).expect("render authority graph fixture");
        assert!(rendered.contains("vat-authority-graph-inspect-receipt-v1"));
        assert!(rendered.contains("proxy-chain-visible"));
        assert!(rendered.contains("inspection-authority-missing"));
    }

    #[test]
    fn vat_portable_storage_fixture_records_encrypted_chunked_storage() {
        let storage = run_vat_portable_storage_fixture().expect("portable storage fixture");
        assert_eq!(storage.receipts.len(), 2);
        assert!(storage.fixture_ref.starts_with("blake3:"));
        let rendered = to_text(&storage.value).expect("render portable storage fixture");
        assert!(rendered.contains("vat-portable-storage-receipt-v1"));
        assert!(rendered.contains("content-addressed-chunked-encrypted"));
        assert!(rendered.contains("plaintext-storage-denied"));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_promise_pipeline_ordering_bounds_and_terminal_cleanup(tc: TestCase) {
        let queue_len = tc.draw(generators::integers::<u64>().min_value(0).max_value(4));
        let queue = (0..queue_len)
            .map(|index| RuntimePromisePipelineEntry::new(index + 1, vat_test_ref(&format!("target-{index}")), "call"))
            .collect::<Vec<_>>();
        let pending = evaluate_promise_pipeline(&RuntimePromisePipelineState::new(
            RuntimePromiseState::pending("promise:hegel"),
            PIPELINE_MAX_QUEUE,
            queue,
        ))
        .expect("pending pipeline");
        assert_eq!(pending.receipt.decision, PredicateDecision::Pass);

        let overflow_len = tc.draw(generators::integers::<u64>().min_value(5).max_value(8));
        let overflow_queue = (0..overflow_len)
            .map(|index| {
                RuntimePromisePipelineEntry::new(index + 1, vat_test_ref(&format!("overflow-{index}")), "call")
            })
            .collect::<Vec<_>>();
        let overflow = evaluate_promise_pipeline(&RuntimePromisePipelineState::new(
            RuntimePromiseState::pending("promise:overflow"),
            PIPELINE_MAX_QUEUE,
            overflow_queue,
        ))
        .expect("overflow pipeline");
        assert_eq!(overflow.receipt.decision, PredicateDecision::Deny);

        let terminal = evaluate_promise_pipeline(&RuntimePromisePipelineState::new(
            RuntimePromiseState::broken("promise:terminal", "causal failure", Vec::new()),
            PIPELINE_MAX_QUEUE,
            vec![RuntimePromisePipelineEntry::new(
                1,
                vat_test_ref("stale-terminal"),
                "late-call",
            )],
        ))
        .expect("terminal pipeline");
        assert_eq!(terminal.receipt.decision, PredicateDecision::Deny);
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_actormap_commit_and_rollback_invariants(tc: TestCase) {
        let spawn_count = tc.draw(generators::integers::<u64>().min_value(1).max_value(4));
        let before = sorted_refs(vec![vat_test_ref("root"), vat_test_ref("helper")]);
        let spawned =
            sorted_refs((0..spawn_count).map(|index| vat_test_ref(&format!("spawned-{index}"))).collect::<Vec<_>>());
        let after = sorted_refs(before.iter().cloned().chain(spawned.iter().cloned()).collect());
        let committed = evaluate_actormap_transaction(&RuntimeActormapTransactionState {
            outcome: RuntimeActormapTransactionOutcome::Committed,
            before_object_refs: before.clone(),
            after_object_refs: after.clone(),
            spawned_object_refs: spawned.clone(),
            removed_object_refs: Vec::new(),
            visible_object_refs: after,
            used_object_refs: vec![before[0].clone()],
        })
        .expect("commit");
        assert_eq!(committed.receipt.decision, PredicateDecision::Pass);

        let rollback = evaluate_actormap_transaction(&RuntimeActormapTransactionState {
            outcome: RuntimeActormapTransactionOutcome::RolledBack,
            before_object_refs: before.clone(),
            after_object_refs: before.clone(),
            spawned_object_refs: spawned.clone(),
            removed_object_refs: Vec::new(),
            visible_object_refs: before.clone(),
            used_object_refs: Vec::new(),
        })
        .expect("rollback");
        assert_eq!(rollback.receipt.decision, PredicateDecision::Pass);

        let leaked_spawn = evaluate_actormap_transaction(&RuntimeActormapTransactionState {
            outcome: RuntimeActormapTransactionOutcome::RolledBack,
            before_object_refs: before.clone(),
            after_object_refs: before,
            spawned_object_refs: spawned.clone(),
            removed_object_refs: Vec::new(),
            visible_object_refs: spawned,
            used_object_refs: Vec::new(),
        })
        .expect("leaked rollback");
        assert_eq!(leaked_spawn.receipt.decision, PredicateDecision::Deny);
    }

    fn vat_test_ref(label: &str) -> String {
        canonical_hash(&string(format!("vat-test:{label}"))).expect("vat test ref")
    }
}
