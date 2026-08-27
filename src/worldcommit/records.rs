use std::collections::BTreeSet;

use artifact_auth_core::ArtifactRef;
use artifact_auth_core::ArtifactStatement;
use artifact_auth_core::AuthenticationScope;
use molten_core::world_commit::ClosureReport;
use molten_core::world_commit::RestorePlan;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use preserves::IOValue;
use valence::preserves_evidence::PreservesBridgeRow;
use valence::preserves_evidence::PreservesEvidenceReport;

use super::CanonicalWorldCommit;
use super::WORLD_COMMIT_CLOSURE_REPORT_RECORD;
use super::WORLD_COMMIT_RESTORE_PLAN_RECORD;
use crate::error::MoltenError;
use crate::error::Result;

const VALENCE_MOLTEN_ENVELOPE_SCHEMA: &str = "molten.preserves-envelope.v1";
const VALENCE_ARTIFACT_ROLE: &str = "artifact_identity";
const VALENCE_ARTIFACT_KIND: &str = "molten-world-commit";
const VALENCE_VERIFICATION_ROLE: &str = "boundary";
const WORLD_COMMIT_AUTH_DOMAIN: &str = "molten.world-commit";
const WORLD_COMMIT_AUTH_PURPOSE: &str = "world-commit-attestation";
const WORLD_COMMIT_VERIFIER_CONTEXT_PROFILE: &str = "molten-world-commit-verifier-context-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalClosureReport {
    pub commit_ref: WorldCommitRef,
    pub report: ClosureReport,
    pub report_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalRestorePlan {
    pub plan: RestorePlan,
    pub plan_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldCommitValenceProjection {
    pub row: PreservesBridgeRow,
    pub report: PreservesEvidenceReport,
}

#[derive(Debug, Clone, Copy)]
pub struct WorldCommitArtifactAuthInput<'a> {
    pub producer_id: &'a str,
    pub key_id: &'a str,
    pub key_identity_ref: &'a str,
    pub verifier_context_ref: &'a str,
}

// r[impl molten.world_commit.restore]
pub fn canonical_closure_report(commit_ref: &WorldCommitRef, report: &ClosureReport) -> Result<CanonicalClosureReport> {
    if report.commit_ref != *commit_ref {
        return Err(MoltenError::invalid_harness("world commit closure report is bound to a different commit"));
    }
    let value = crate::preserves_rail::record(WORLD_COMMIT_CLOSURE_REPORT_RECORD, vec![
        crate::preserves_rail::string(molten_core::world_commit::WORLD_COMMIT_CLOSURE_REPORT_SCHEMA),
        crate::preserves_rail::record("commit-ref", vec![crate::preserves_rail::string(commit_ref.as_str())]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(if report.complete {
            "complete"
        } else {
            "incomplete"
        })]),
        crate::preserves_rail::record("first-missing-root", vec![optional_string_value(
            report.first_missing_root.map(|kind| kind.as_str()),
        )]),
        crate::preserves_rail::record("issues", vec![strings(report.issues.iter().map(|issue| format!("{issue:?}")))]),
        super::non_claims_value(),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    Ok(CanonicalClosureReport {
        commit_ref: commit_ref.clone(),
        report: report.clone(),
        report_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        value,
        bytes,
    })
}

// r[impl molten.world_commit.restore]
pub fn canonical_restore_plan(plan: &RestorePlan) -> Result<CanonicalRestorePlan> {
    let value = crate::preserves_rail::record(WORLD_COMMIT_RESTORE_PLAN_RECORD, vec![
        crate::preserves_rail::string(molten_core::world_commit::WORLD_COMMIT_RESTORE_PLAN_SCHEMA),
        crate::preserves_rail::record("commit-ref", vec![crate::preserves_rail::string(plan.commit_ref.as_str())]),
        crate::preserves_rail::record("steps", vec![crate::preserves_rail::sequence(
            plan.steps
                .iter()
                .map(|step| {
                    crate::preserves_rail::record("restore-step", vec![
                        crate::preserves_rail::string(step.kind.as_str()),
                        crate::preserves_rail::optional_ref_value(step.root.as_ref().map(WorldRootRef::as_str)),
                    ])
                })
                .collect(),
        )]),
        crate::preserves_rail::record("replay", vec![crate::preserves_rail::sequence(
            plan.replay
                .iter()
                .map(|item| {
                    crate::preserves_rail::record("root-replay", vec![
                        crate::preserves_rail::string(item.root_kind.as_str()),
                        crate::preserves_rail::string(item.class.as_str()),
                    ])
                })
                .collect(),
        )]),
        crate::preserves_rail::record("current-admission-required", vec![crate::preserves_rail::bool_value(
            plan.current_admission_required,
        )]),
        super::non_claims_value(),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    Ok(CanonicalRestorePlan {
        plan: plan.clone(),
        plan_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        value,
        bytes,
    })
}

// r[impl molten.world_commit.detached_evidence]
pub fn project_world_commit_to_valence(commit: &CanonicalWorldCommit) -> Result<WorldCommitValenceProjection> {
    let row = PreservesBridgeRow {
        schema_label: VALENCE_MOLTEN_ENVELOPE_SCHEMA.to_string(),
        canonical_bytes: commit.bytes.clone(),
        preserves_content_hash: blake3::hash(&commit.bytes).to_hex().to_string(),
        artifact_role: VALENCE_ARTIFACT_ROLE.to_string(),
        valence_artifact_kind: VALENCE_ARTIFACT_KIND.to_string(),
        verification_role: VALENCE_VERIFICATION_ROLE.to_string(),
        non_claims: vec![valence::preserves_evidence::PRESERVES_BRIDGE_REQUIRED_NON_CLAIM.to_string()],
    };
    let report = valence::preserves_evidence::validate_preserves_bridge(&row);
    if !report.valid {
        return Err(MoltenError::invalid_harness(format!(
            "world commit Valence projection denied: {:?}",
            report.issues
        )));
    }
    Ok(WorldCommitValenceProjection { row, report })
}

// r[impl molten.world_commit.detached_evidence]
pub fn project_world_commit_artifact_auth_statement(
    commit: &CanonicalWorldCommit,
    input: WorldCommitArtifactAuthInput<'_>,
) -> Result<ArtifactStatement> {
    if input.producer_id.is_empty() || input.key_id.is_empty() {
        return Err(MoltenError::invalid_harness("world commit artifact-auth producer and key ids must not be empty"));
    }
    let parents = commit
        .core
        .parents
        .iter()
        .map(|parent| artifact_ref(molten_core::world_commit::WORLD_COMMIT_ARTIFACT_AUTH_PROFILE, parent.as_str()))
        .collect::<Result<Vec<_>>>()?;
    Ok(ArtifactStatement {
        schema: artifact_auth_core::STATEMENT_SCHEMA_V1.to_string(),
        scope: AuthenticationScope {
            domain: WORLD_COMMIT_AUTH_DOMAIN.to_string(),
            purpose: WORLD_COMMIT_AUTH_PURPOSE.to_string(),
            profile_id: molten_core::world_commit::WORLD_COMMIT_ARTIFACT_AUTH_PROFILE.to_string(),
            subject: artifact_ref(
                molten_core::world_commit::WORLD_COMMIT_ARTIFACT_AUTH_PROFILE,
                commit.commit_ref.as_str(),
            )?,
            parents,
            verifier_context: artifact_ref(WORLD_COMMIT_VERIFIER_CONTEXT_PROFILE, input.verifier_context_ref)?,
        },
        producer_id: input.producer_id.to_string(),
        key_id: input.key_id.to_string(),
        key_identity: artifact_ref(artifact_auth_core::ED25519_PUBLIC_KEY_PROFILE_V1, input.key_identity_ref)?,
    })
}

fn artifact_ref(profile: &str, reference: &str) -> Result<ArtifactRef> {
    Ok(ArtifactRef {
        profile: profile.to_string(),
        algorithm: artifact_auth_core::ALGORITHM_BLAKE3.to_string(),
        digest_hex: crate::preserves_rail::content_ref_hex(reference)?.to_string(),
    })
}

pub(crate) fn preflight_shell(input: &super::CaptureShellInput) -> Vec<String> {
    let mut issues = molten_core::world_commit::validate_bounds(&input.bounds)
        .into_iter()
        .map(|issue| format!("{issue:?}"))
        .collect::<Vec<_>>();
    if !issues.is_empty() {
        return issues;
    }
    if input.parents.len() > input.bounds.max_parents {
        return vec!["parent-count-exceeds-bound".to_string()];
    }
    if input.root_kinds.len() > input.bounds.max_roots {
        return vec!["root-kind-count-exceeds-bound".to_string()];
    }
    let unique_parents = input.parents.iter().collect::<BTreeSet<_>>();
    if unique_parents.len() != input.parents.len() {
        issues.push("duplicate-parent".to_string());
    }
    let mut unique_kinds = BTreeSet::new();
    for kind in &input.root_kinds {
        if !unique_kinds.insert(*kind) {
            issues.push(format!("duplicate-root-kind:{}", kind.as_str()));
        }
    }
    for required in input.profile.kind.required_roots() {
        if !unique_kinds.contains(required) {
            issues.push(format!("missing-required-root-kind:{}", required.as_str()));
        }
    }
    for supplied in &unique_kinds {
        if !input.profile.kind.required_roots().contains(supplied)
            && *supplied != molten_core::world_commit::RootKind::AuthorityObservation
        {
            issues.push(format!("unexpected-root-kind:{}", supplied.as_str()));
        }
    }
    match (input.profile.kind, input.profile.cohort_ref.is_some()) {
        (molten_core::world_commit::SnapshotProfileKind::Logical, true) => {
            issues.push("logical-profile-has-cohort".to_string());
        }
        (
            molten_core::world_commit::SnapshotProfileKind::Opaque
            | molten_core::world_commit::SnapshotProfileKind::Mixed,
            false,
        ) => issues.push("opaque-profile-missing-cohort".to_string()),
        _ => {}
    }
    issues.sort();
    issues.dedup();
    issues
}

fn optional_string_value(value: Option<&str>) -> IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
    )
}

fn strings(values: impl IntoIterator<Item = String>) -> IOValue {
    crate::preserves_rail::sequence(values.into_iter().map(crate::preserves_rail::string).collect())
}
