use molten_core::content_replication::*;

use super::*;

const RECORD_IDENTITY_CONTEXT: &str = "onixresearch.molten.content-replication.record.v1";

#[derive(Debug, Clone)]
pub struct CanonicalReplicationRecord {
    pub record_ref: String,
    pub value: preserves::IOValue,
    pub bytes: Vec<u8>,
}

// r[impl molten.content_replication.manifest]
pub fn canonical_manifest(manifest: &Manifest) -> crate::error::Result<CanonicalReplicationRecord> {
    canonical(
        "manifest",
        record("content-replication-manifest-v1", vec![
            field("service-id", string(&manifest.service_id)),
            field("generation", number(manifest.generation)),
            field("membership-epoch", number(manifest.membership_epoch)),
            field("placement-epoch", number(manifest.placement_epoch)),
            field("authority-ref", string(&manifest.authority_ref)),
            field("identity-ref", string(&manifest.identity_ref)),
            field("content-profile-ref", string(&manifest.content_profile_ref)),
            field("transport-profile-ref", string(&manifest.transport_profile_ref)),
            field("retention-policy-ref", string(&manifest.retention_policy_ref)),
            field("evidence-profile-ref", string(&manifest.evidence_profile_ref)),
            field("ports", strings(&manifest.ports)),
            field("desired-replicas", count(manifest.policy.desired_replicas)?),
            field("minimum-verified", count(manifest.policy.minimum_verified_replicas)?),
            field("minimum-fault-domains", count(manifest.policy.minimum_fault_domains)?),
            field("contents", sequence(content_values(&manifest.contents))),
            non_claims(),
        ]),
    )
}

pub fn canonical_plan(plan: &Plan) -> crate::error::Result<CanonicalReplicationRecord> {
    canonical(
        "plan",
        record("content-replication-plan-v1", vec![
            field("plan-ref", string(&plan.plan_ref)),
            field("decision", string(plan.decision.as_str())),
            field("generation", number(plan.generation)),
            field("membership-epoch", number(plan.membership_epoch)),
            field("placement-epoch", number(plan.placement_epoch)),
            field("actions", sequence(plan.actions.iter().map(action_value).collect())),
            field("desired-replicas", count(plan.desired_replicas)?),
            field("verified-replicas", count(plan.verified_replicas)?),
            field("under-replicated", strings(&plan.under_replicated)),
            field("deferred", strings(&plan.deferred)),
            field("required-pins", strings(&plan.required_pins)),
            field("cleanup-candidates", strings(&plan.cleanup_candidates)),
            field("issues", issues(&plan.issues)),
            non_claims(),
        ]),
    )
}

pub fn canonical_operation(operation: &PriorOperation) -> crate::error::Result<CanonicalReplicationRecord> {
    canonical(
        "operation",
        record("content-replication-operation-v1", vec![
            field("operation-id", string(&operation.operation_id)),
            field("content-ref", string(&operation.content_ref)),
            field("source-peer", optional(operation.source_peer.as_deref())),
            field("target-peer", string(&operation.target_peer)),
            field("generation", number(operation.generation)),
            field("membership-epoch", number(operation.membership_epoch)),
            field("placement-epoch", number(operation.placement_epoch)),
            field("attempt", number(u64::from(operation.attempt))),
            field("outcome", string(operation.outcome.as_str())),
            field("result-ref", optional(operation.result_ref.as_deref())),
            non_claims(),
        ]),
    )
}

pub fn canonical_status(status: &Status) -> crate::error::Result<CanonicalReplicationRecord> {
    canonical(
        "status",
        record("content-replication-status-v1", vec![
            field("plan-ref", string(&status.plan_ref)),
            field("generation", number(status.generation)),
            field("placement-epoch", number(status.placement_epoch)),
            field("desired-replicas", count(status.desired_replicas)?),
            field("verified-replicas", count(status.verified_replicas)?),
            field("under-replicated", strings(&status.under_replicated)),
            field("active-operations", strings(&status.active_operations)),
            field("failures", strings(&status.failures)),
            field("pins", strings(&status.pins)),
            non_claims(),
        ]),
    )
}

pub fn canonical_operator_status(view: &OperatorStatusView) -> crate::error::Result<CanonicalReplicationRecord> {
    canonical(
        "operator-status",
        record("content-replication-operator-status-v1", vec![
            field("service-id", string(&view.service_id)),
            field("generation", number(view.generation)),
            field("placement-epoch", number(view.placement_epoch)),
            field("desired-replicas", count(view.desired_replicas)?),
            field("verified-replicas", count(view.verified_replicas)?),
            field("under-replicated", strings(&view.under_replicated)),
            field("active-plan-ref", string(&view.active_plan_ref)),
            field("active-operations", strings(&view.active_operations)),
            field("resource-refs", strings(&view.resource_refs)),
            field("failures", strings(&view.failures)),
            field("pins", strings(&view.pins)),
            field("evidence-refs", strings(&view.evidence_refs)),
            non_claims(),
        ]),
    )
}

// r[impl molten.content_replication.final_validation]
pub fn canonical_receipt(receipt: &ExecutionReceipt) -> crate::error::Result<CanonicalReplicationRecord> {
    let expected = NON_CLAIMS.iter().map(ToString::to_string).collect::<Vec<_>>();
    if receipt.non_claims != expected {
        return Err(crate::error::MoltenError::invalid_harness(
            "content-replication receipt non-claims are incomplete",
        ));
    }
    canonical(
        "receipt",
        record("content-replication-receipt-v1", vec![
            field("decision", string(receipt.decision.as_str())),
            field("service-id", string(&receipt.service_id)),
            field("generation", number(receipt.generation)),
            field("plan-ref", string(&receipt.plan_ref)),
            field("status-ref", string(&receipt.status_ref)),
            field(
                "operations",
                sequence(receipt.operations.iter().map(|operation| string(&operation.operation_id)).collect()),
            ),
            field("evidence-refs", strings(&receipt.evidence_refs)),
            field("issues", issues(&receipt.issues)),
            non_claims(),
        ]),
    )
}

fn content_values(contents: &[ReplicaRule]) -> Vec<preserves::IOValue> {
    contents
        .iter()
        .map(|content| {
            record("content-rule", vec![
                string(&content.content_ref),
                string(&content.manifest_ref),
                number(content.encoded_bytes),
                boolean(content.protected),
                optional(content.transform_ref.as_deref()),
                optional(content.cleanup_authority_ref.as_deref()),
            ])
        })
        .collect()
}

fn action_value(action: &Action) -> preserves::IOValue {
    record("replication-action", vec![
        string(&action.action_id),
        string(&action.operation_id),
        string(action.kind.as_str()),
        number(u64::from(action.attempt)),
        string(&action.content_ref),
        optional(action.source_peer.as_deref()),
        string(&action.target_peer),
        string(&action.fault_domain),
        number(action.encoded_bytes),
        boolean(action.pin_required),
        boolean(action.preserve_protected_form),
        optional(action.cleanup_authority_ref.as_deref()),
        optional(action.prior_result_ref.as_deref()),
        optional(action.diagnostic.as_deref()),
    ])
}

fn canonical(kind: &str, value: preserves::IOValue) -> crate::error::Result<CanonicalReplicationRecord> {
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let mut hasher = blake3::Hasher::new_derive_key(RECORD_IDENTITY_CONTEXT);
    update(&mut hasher, kind)?;
    let length = u64::try_from(bytes.len())
        .map_err(|_| crate::error::MoltenError::invalid_harness("replication record length exceeds u64"))?;
    hasher.update(&length.to_be_bytes());
    hasher.update(&bytes);
    Ok(CanonicalReplicationRecord {
        record_ref: format!("blake3:{}", hasher.finalize().to_hex()),
        value,
        bytes,
    })
}

fn update(hasher: &mut blake3::Hasher, value: &str) -> crate::error::Result<()> {
    let length = u64::try_from(value.len())
        .map_err(|_| crate::error::MoltenError::invalid_harness("replication identity field length exceeds u64"))?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}

fn non_claims() -> preserves::IOValue {
    field("non-claims", sequence(NON_CLAIMS.iter().map(string).collect()))
}

fn issues(values: &[Issue]) -> preserves::IOValue {
    sequence(values.iter().map(|issue| string(issue.as_str())).collect())
}

fn strings(values: &[String]) -> preserves::IOValue {
    sequence(values.iter().map(string).collect())
}

fn optional(value: Option<&str>) -> preserves::IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn boolean(value: bool) -> preserves::IOValue {
    record(if value { "true" } else { "false" }, Vec::new())
}

fn count(value: usize) -> crate::error::Result<preserves::IOValue> {
    u64::try_from(value)
        .map(number)
        .map_err(|_| crate::error::MoltenError::invalid_harness("replication count exceeds u64"))
}

fn field(label: &'static str, value: preserves::IOValue) -> preserves::IOValue {
    record(label, vec![value])
}

fn number(value: u64) -> preserves::IOValue {
    crate::preserves_rail::u64_value(value)
}

fn string(value: impl AsRef<str>) -> preserves::IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn sequence(values: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values)
}

fn record(label: &'static str, fields: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::record(label, fields)
}
