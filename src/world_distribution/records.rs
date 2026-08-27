use artifact_binding_core::RetirementClassification;
use molten_core::world_distribution::*;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;

const WORLD_DISTRIBUTION_RECORD_CONTEXT: &str = "onixresearch.molten.world-distribution.record.v1";
const SYNC_REQUEST_RECORD: &str = "molten-world-sync-request-v1";
const CLOSURE_PLAN_RECORD: &str = "molten-world-closure-plan-v1";
const CLAIM_ADMISSION_RECORD: &str = "molten-world-claim-exchange-v1";
const RETENTION_REPORT_RECORD: &str = "molten-world-retention-roots-v1";
const SYNC_RECEIPT_RECORD: &str = "molten-world-closure-report-v1";
const REACHABILITY_RECEIPT_RECORD: &str = "molten-world-reachability-receipt-v1";

#[derive(Debug, Clone)]
pub struct CanonicalWorldDistributionRecord {
    pub record_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

pub fn canonical_world_sync_request(plan: &WorldClosurePlan) -> Result<CanonicalWorldDistributionRecord> {
    require_non_claims(&plan.non_claims)?;
    canonical(
        "sync-request",
        record(SYNC_REQUEST_RECORD, vec![
            field("commit-ref", string(plan.projection.requested.as_str())),
            field(
                "inventory",
                sequence(
                    plan.request
                        .inventory
                        .available
                        .iter()
                        .map(|object| record("dag-object", vec![string(object.kind()), string(object.as_str())]))
                        .collect(),
                ),
            ),
            field("strategy", string(plan.request.strategy.as_str())),
            field("epoch-ref", string(plan.request.epoch_ref.as_str())),
            field("generation", number(plan.request.generation)),
            field("policy-ref", string(plan.request.policy_ref.as_str())),
            field("peers", sequence(plan.request.peers.iter().map(|peer| string(peer.as_str())).collect())),
            field("object-limit", usize_value(plan.request.bounds.max_nodes)),
            non_claims(),
        ]),
    )
}

pub fn canonical_world_closure_plan(plan: &WorldClosurePlan) -> Result<CanonicalWorldDistributionRecord> {
    require_non_claims(&plan.non_claims)?;
    canonical(
        "closure-plan",
        record(CLOSURE_PLAN_RECORD, vec![
            field("commit-ref", string(plan.projection.requested.as_str())),
            field("dag-plan-ref", string(plan.shared_plan.plan_ref.as_str())),
            field("complete", boolean(plan.complete)),
            field(
                "missing",
                sequence(
                    plan.missing
                        .iter()
                        .map(|object| {
                            record("world-object", vec![string(object.domain().as_str()), string(object.as_str())])
                        })
                        .collect(),
                ),
            ),
            field("activation-authorized", boolean(plan.activation_authorized)),
            non_claims(),
        ]),
    )
}

pub fn canonical_world_claim_admission(admission: &WorldClaimAdmission) -> Result<CanonicalWorldDistributionRecord> {
    require_non_claims(&admission.non_claims)?;
    let conflict = admission.conflict.as_ref().map_or_else(
        || record("none", Vec::new()),
        |conflict| {
            record("some", vec![record("world-head-conflict", vec![
                string(&conflict.conflict_ref),
                sequence(
                    conflict
                        .members
                        .iter()
                        .map(|member| {
                            record("member", vec![
                                string(member.claim_ref.as_str()),
                                string(member.successor_head.as_str()),
                            ])
                        })
                        .collect(),
                ),
            ])])
        },
    );
    canonical(
        "claim-admission",
        record(CLAIM_ADMISSION_RECORD, vec![
            field(
                "admitted",
                sequence(admission.admitted.iter().map(|plan| string(plan.claim_ref.as_str())).collect()),
            ),
            field(
                "denied",
                sequence(
                    admission
                        .denied
                        .iter()
                        .map(|denial| {
                            record("denial", vec![
                                string(denial.claim_ref.as_str()),
                                sequence(denial.issues.iter().map(|issue| string(format!("{issue:?}"))).collect()),
                            ])
                        })
                        .collect(),
                ),
            ),
            field("conflict", conflict),
            field(
                "selected-claim",
                admission
                    .selected_claim
                    .as_ref()
                    .map_or_else(|| record("none", Vec::new()), |claim| record("some", vec![string(claim.as_str())])),
            ),
            field("head-mutation-authorized", boolean(admission.head_mutation_authorized)),
            non_claims(),
        ]),
    )
}

pub fn canonical_world_retention_report(report: &WorldRetentionReport) -> Result<CanonicalWorldDistributionRecord> {
    require_non_claims(&report.non_claims)?;
    canonical(
        "retention-report",
        record(RETENTION_REPORT_RECORD, vec![
            field("snapshot-ref", string(&report.snapshot_ref)),
            field("generation-ref", string(&report.generation_ref)),
            field("retained", refs(&report.retained_refs)),
            field("remote", refs(&report.remote_refs)),
            field("evidence", refs(&report.evidence_refs)),
            field(
                "missing-classes",
                sequence(report.missing_classes.iter().map(|class| string(class.as_str())).collect()),
            ),
            field("unresolved-remote", refs(&report.unresolved_remote)),
            field("reference-index-complete", boolean(report.reference_index_complete)),
            field("shared-classification", string(classification(report.shared_classification))),
            field("observation-only", boolean(report.observation_only)),
            field("retention-authorized", boolean(report.retention_authorized)),
            field("deletion-authorized", boolean(report.deletion_authorized)),
            non_claims(),
        ]),
    )
}

pub fn canonical_world_reachability_receipt(report: &WorldRetentionReport) -> Result<CanonicalWorldDistributionRecord> {
    let roots = canonical_world_retention_report(report)?;
    canonical(
        "reachability-receipt",
        record(REACHABILITY_RECEIPT_RECORD, vec![
            field("retention-roots-ref", string(&roots.record_ref)),
            field("snapshot-ref", string(&report.snapshot_ref)),
            field("generation-ref", string(&report.generation_ref)),
            field("classification", string(classification(report.shared_classification))),
            field("retained", refs(&report.retained_refs)),
            field("observation-only", boolean(report.observation_only)),
            field("deletion-authorized", boolean(report.deletion_authorized)),
            non_claims(),
        ]),
    )
}

pub fn canonical_world_sync_receipt(
    plan: &WorldClosurePlan,
    dag_receipt_ref: &str,
    complete: bool,
    verified: usize,
) -> Result<CanonicalWorldDistributionRecord> {
    crate::preserves_rail::validate_content_ref(dag_receipt_ref)
        .map_err(|_| MoltenError::invalid_harness("world sync DAG receipt ref is invalid"))?;
    require_non_claims(&plan.non_claims)?;
    canonical(
        "sync-receipt",
        record(SYNC_RECEIPT_RECORD, vec![
            field("commit-ref", string(plan.projection.requested.as_str())),
            field("dag-plan-ref", string(plan.shared_plan.plan_ref.as_str())),
            field("dag-receipt-ref", string(dag_receipt_ref)),
            field("complete", boolean(complete)),
            field("verified", usize_value(verified)),
            field("activation-authorized", boolean(false)),
            non_claims(),
        ]),
    )
}

fn require_non_claims(non_claims: &[String]) -> Result<()> {
    let expected = distribution_non_claims();
    if non_claims != expected {
        return Err(MoltenError::invalid_harness("world distribution non-claims are incomplete"));
    }
    Ok(())
}

fn classification(value: RetirementClassification) -> &'static str {
    match value {
        RetirementClassification::Retired => "retired",
        RetirementClassification::Live => "live",
        RetirementClassification::Incomplete => "incomplete",
        RetirementClassification::Unknown => "unknown",
    }
}

fn canonical(kind: &str, value: IOValue) -> Result<CanonicalWorldDistributionRecord> {
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_DISTRIBUTION_RECORD_CONTEXT);
    update(&mut hasher, kind)?;
    let length = u64::try_from(bytes.len())
        .map_err(|_| MoltenError::invalid_harness("world distribution record length exceeds u64"))?;
    hasher.update(&length.to_be_bytes());
    hasher.update(&bytes);
    Ok(CanonicalWorldDistributionRecord {
        record_ref: format!("blake3:{}", hasher.finalize().to_hex()),
        value,
        bytes,
    })
}

fn update(hasher: &mut blake3::Hasher, value: &str) -> Result<()> {
    let length = u64::try_from(value.len())
        .map_err(|_| MoltenError::invalid_harness("world distribution identity length exceeds u64"))?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}

fn non_claims() -> IOValue {
    field("non-claims", sequence(WORLD_DISTRIBUTION_NON_CLAIMS.iter().map(string).collect()))
}

fn refs(values: &[String]) -> IOValue {
    sequence(values.iter().map(string).collect())
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    record(label, vec![value])
}

fn boolean(value: bool) -> IOValue {
    record(if value { "true" } else { "false" }, Vec::new())
}

fn usize_value(value: usize) -> IOValue {
    crate::preserves_rail::u64_value(u64::try_from(value).unwrap_or(u64::MAX))
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

fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::record(label, fields)
}
