use preserves::IOValue;

use super::PredicateDecision;
use super::RuntimeActormapTransactionOutcome;
use super::RuntimeActormapTransactionState;
use super::RuntimeNearFarRefState;
use super::RuntimePredicateReceipt;
use super::RuntimePromisePipelineEntry;
use super::RuntimePromisePipelineState;
use super::RuntimePromiseState;
use super::RuntimeReferenceCallMode;
use super::RuntimeReferenceKind;
use super::RuntimeRevocationCleanupState;
use super::RuntimeSnapshotAuthorityState;
use super::evaluate_actormap_transaction;
use super::evaluate_near_far_refs;
use super::evaluate_promise_pipeline;
use super::evaluate_promise_state_transition;
use super::evaluate_revocation_cleanup;
use super::evaluate_snapshot_authority;
use crate::error::Result;
use crate::preserves_rail::RUNTIME_VAT_FIXTURE_RUN_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_OBJECT_REF_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_OBJECT_UPGRADE_RECIPE_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_PROMISE_FIXTURE_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_RESTORE_RECEIPT_SCHEMA;
use crate::preserves_rail::RUNTIME_VAT_SNAPSHOT_SCHEMA;
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

pub fn run_vat_fixture() -> Result<VatFixtureRun> {
    let root = VatObjectRef::new(LOCAL_VAT_ID, ROOT_OBJECT_ID, VatReferenceKind::Near, Vec::new());
    let helper = VatObjectRef::new(LOCAL_VAT_ID, HELPER_OBJECT_ID, VatReferenceKind::Near, vec![root.object_ref()?]);
    let spawned =
        VatObjectRef::new(LOCAL_VAT_ID, SPAWNED_OBJECT_ID, VatReferenceKind::Near, vec![helper.object_ref()?]);
    let far = VatObjectRef::new(REMOTE_VAT_ID, FAR_OBJECT_ID, VatReferenceKind::Far, Vec::new());
    let proxy = VatObjectRef::new(LOCAL_VAT_ID, PROXY_OBJECT_ID, VatReferenceKind::Proxy, vec![helper.object_ref()?]);

    let root_ref = root.object_ref()?;
    let helper_ref = helper.object_ref()?;
    let spawned_ref = spawned.object_ref()?;
    let far_ref = far.object_ref()?;
    let proxy_ref = proxy.object_ref()?;

    let near_call = evaluate_near_far_refs(&RuntimeNearFarRefState {
        reference_ref: helper_ref.clone(),
        reference_kind: RuntimeReferenceKind::Near,
        is_live: true,
        caller_vat_id: LOCAL_VAT_ID.to_string(),
        target_vat_id: LOCAL_VAT_ID.to_string(),
        call_mode: RuntimeReferenceCallMode::Synchronous,
    })?;
    let far_sync_denial = evaluate_near_far_refs(&RuntimeNearFarRefState {
        reference_ref: far_ref.clone(),
        reference_kind: RuntimeReferenceKind::Far,
        is_live: true,
        caller_vat_id: LOCAL_VAT_ID.to_string(),
        target_vat_id: REMOTE_VAT_ID.to_string(),
        call_mode: RuntimeReferenceCallMode::Synchronous,
    })?;
    let far_async = evaluate_near_far_refs(&RuntimeNearFarRefState {
        reference_ref: far_ref.clone(),
        reference_kind: RuntimeReferenceKind::Far,
        is_live: true,
        caller_vat_id: LOCAL_VAT_ID.to_string(),
        target_vat_id: REMOTE_VAT_ID.to_string(),
        call_mode: RuntimeReferenceCallMode::Asynchronous,
    })?;

    let committed = evaluate_actormap_transaction(&RuntimeActormapTransactionState {
        outcome: RuntimeActormapTransactionOutcome::Committed,
        before_object_refs: sorted_refs(vec![root_ref.clone(), helper_ref.clone()]),
        after_object_refs: sorted_refs(vec![root_ref.clone(), helper_ref.clone(), spawned_ref.clone()]),
        spawned_object_refs: vec![spawned_ref.clone()],
        removed_object_refs: Vec::new(),
        visible_object_refs: sorted_refs(vec![root_ref.clone(), helper_ref.clone(), spawned_ref.clone()]),
        used_object_refs: vec![helper_ref.clone()],
    })?;
    let rollback = evaluate_actormap_transaction(&RuntimeActormapTransactionState {
        outcome: RuntimeActormapTransactionOutcome::RolledBack,
        before_object_refs: sorted_refs(vec![root_ref.clone(), helper_ref.clone()]),
        after_object_refs: sorted_refs(vec![root_ref.clone(), helper_ref.clone()]),
        spawned_object_refs: vec![spawned_ref.clone()],
        removed_object_refs: Vec::new(),
        visible_object_refs: sorted_refs(vec![root_ref.clone(), helper_ref.clone()]),
        used_object_refs: vec![helper_ref.clone()],
    })?;

    let pipeline = evaluate_promise_pipeline(&RuntimePromisePipelineState::new(
        RuntimePromiseState::pending("promise:far-call"),
        PIPELINE_MAX_QUEUE,
        vec![
            RuntimePromisePipelineEntry::new(1, far_ref.clone(), "get"),
            RuntimePromisePipelineEntry::new(2, far_ref.clone(), "subscribe"),
        ],
    ))?;
    let revoked = evaluate_revocation_cleanup(&RuntimeRevocationCleanupState {
        revoked_refs: vec![proxy_ref.clone()],
        attempted_use_refs: vec![proxy_ref.clone()],
        remaining_assertion_refs: Vec::new(),
        remaining_subscription_refs: Vec::new(),
        remaining_pending_call_refs: Vec::new(),
        remaining_child_refs: Vec::new(),
    })?;

    let receipts = vec![
        near_call.receipt.clone(),
        far_sync_denial.receipt.clone(),
        far_async.receipt.clone(),
        committed.receipt.clone(),
        rollback.receipt.clone(),
        pipeline.receipt.clone(),
        revoked.receipt.clone(),
    ];
    let diagnostics = fixture_diagnostics(&receipts);
    let value = record("vat-fixture-run-v1", vec![
        string(RUNTIME_VAT_FIXTURE_RUN_SCHEMA),
        string(LOCAL_VAT_ID),
        sequence([root, helper, spawned, far, proxy].iter().map(VatObjectRef::value).collect()),
        sequence(
            [
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
                VatCallEvidence {
                    name: "actormap-commit".to_string(),
                    receipt: committed.receipt,
                },
                VatCallEvidence {
                    name: "actormap-rollback".to_string(),
                    receipt: rollback.receipt,
                },
                VatCallEvidence {
                    name: "promise-pipeline".to_string(),
                    receipt: pipeline.receipt,
                },
                VatCallEvidence {
                    name: "revocation-cleanup".to_string(),
                    receipt: revoked.receipt,
                },
            ]
            .iter()
            .map(VatCallEvidence::value)
            .collect(),
        ),
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
    use super::run_vat_fixture;
    use super::run_vat_promise_fixture;
    use super::run_vat_restore_fixture;
    use super::run_vat_snapshot_fixture;
    use super::vat_fixture_summary;
    use crate::preserves_rail::to_text;
    use crate::runtime::PredicateDecision;

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
}
