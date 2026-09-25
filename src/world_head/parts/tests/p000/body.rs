use std::collections::BTreeSet;

use artifact_auth_core::ALGORITHM_BLAKE3;
use artifact_auth_core::ArtifactRef;
use artifact_auth_core::AuthenticationPolicy;
use artifact_auth_core::KeyCurrentness;
use artifact_auth_core::POLICY_SCHEMA_V1;
use artifact_auth_core::TrustedKeyObservation;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_head::MAX_WORLD_HEAD_CONFLICTS;
use molten_core::world_head::WORLD_HEAD_ARTIFACT_AUTH_PROFILE;
use molten_core::world_head::WORLD_HEAD_ARTIFACT_AUTH_PURPOSE;
use molten_core::world_head::WorldBranchClass;
use molten_core::world_head::WorldBranchId;
use molten_core::world_head::WorldCommitHistoryNode;
use molten_core::world_head::WorldHeadAuthenticationDecisionRef;
use molten_core::world_head::WorldHeadAuthenticationObservation;
use molten_core::world_head::WorldHeadAuthorityObservation;
use molten_core::world_head::WorldHeadAuthorityRef;
use molten_core::world_head::WorldHeadBounds;
use molten_core::world_head::WorldHeadClaim;
use molten_core::world_head::WorldHeadClaimRef;
use molten_core::world_head::WorldHeadCurrentnessObservation;
use molten_core::world_head::WorldHeadDecision;
use molten_core::world_head::WorldHeadPlanRequest;
use molten_core::world_head::WorldHeadPolicy;
use molten_core::world_head::WorldHeadPolicyRef;
use molten_core::world_head::WorldHeadPurpose;
use molten_core::world_head::WorldHeadSignerObservation;
use molten_core::world_head::WorldHeadSignerRole;
use molten_core::world_head::WorldHeadState;
use molten_core::world_head::WorldHeadStatementRef;
use molten_core::world_head::WorldHeadTransitionPlan;
use molten_core::world_head::plan_world_head_transition;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStateRoot;

use super::*;

const INITIAL_GENERATION: u64 = 1;
const NEXT_GENERATION: u64 = 2;
const SIGNATURE_THRESHOLD: u16 = 1;
const TEST_PUBLIC_KEY_BYTE: u8 = 7;
const SIGNATURE_TAMPER_MASK: u8 = 1;
const EXPECTED_AUTHORITY_RECHECKS: usize = 2;
const WORLD_HEAD_SCHEMA_COUNT: usize = 3;
const WORLD_HEAD_SCHEMA_FIELD_COUNT: u64 = 6;
const RECORD_LABEL_FIELD_INDEX: usize = 3;
const SCHEMA_ID_FIELD_INDEX: usize = 4;
const WORLD_HEAD_SCHEMA_ARTIFACTS: [&str; WORLD_HEAD_SCHEMA_COUNT] = [
    include_str!("../../../../../schemas/preserves-boundaries/molten-world-head-claim-v1.preserves"),
    include_str!("../../../../../schemas/preserves-boundaries/molten-world-head-conflict-v1.preserves"),
    include_str!("../../../../../schemas/preserves-boundaries/molten-world-head-transition-receipt-v1.preserves"),
];

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn commit(label: &str) -> WorldCommitRef {
    WorldCommitRef::new(reference(label)).expect("commit ref")
}

fn branch() -> WorldBranchId {
    WorldBranchId::new("main").expect("branch")
}

fn policy_ref() -> WorldHeadPolicyRef {
    WorldHeadPolicyRef::new(reference("head-policy")).expect("policy ref")
}

fn history() -> Vec<WorldCommitHistoryNode> {
    vec![
        WorldCommitHistoryNode {
            commit: commit("root"),
            parents: Vec::new(),
        },
        WorldCommitHistoryNode {
            commit: commit("left"),
            parents: vec![commit("root")],
        },
        WorldCommitHistoryNode {
            commit: commit("right"),
            parents: vec![commit("root")],
        },
    ]
}

fn world_policy() -> WorldHeadPolicy {
    WorldHeadPolicy {
        policy_ref: policy_ref(),
        allowed_branch_classes: BTreeSet::from([WorldBranchClass::Local]),
        allowed_purposes: BTreeSet::from([WorldHeadPurpose::Create, WorldHeadPurpose::Advance]),
        allowed_signer_roles: BTreeSet::from([WorldHeadSignerRole::Maintainer]),
        signature_threshold: usize::from(SIGNATURE_THRESHOLD),
        max_conflicts: MAX_WORLD_HEAD_CONFLICTS,
        allow_recovery: false,
        require_independent_recovery_currentness: true,
    }
}

fn create_claim() -> WorldHeadClaim {
    WorldHeadClaim {
        branch_id: branch(),
        branch_class: WorldBranchClass::Local,
        expected_head: None,
        successor_head: commit("root"),
        expected_generation: 0,
        successor_generation: INITIAL_GENERATION,
        purpose: WorldHeadPurpose::Create,
        policy_ref: policy_ref(),
        source_heads: Vec::new(),
    }
}

fn advance_claim(successor: &str) -> WorldHeadClaim {
    WorldHeadClaim {
        branch_id: branch(),
        branch_class: WorldBranchClass::Local,
        expected_head: Some(commit("root")),
        successor_head: commit(successor),
        expected_generation: INITIAL_GENERATION,
        successor_generation: NEXT_GENERATION,
        purpose: WorldHeadPurpose::Advance,
        policy_ref: policy_ref(),
        source_heads: Vec::new(),
    }
}

fn authentication_policy(carrier: &WorldHeadSignatureCarrier) -> AuthenticationPolicy {
    let key_identity = artifact_auth_ed25519::public_key_identity(&carrier.public_key_bytes);
    AuthenticationPolicy {
        schema: POLICY_SCHEMA_V1.to_string(),
        profile_id: WORLD_HEAD_ARTIFACT_AUTH_PROFILE.to_string(),
        threshold: SIGNATURE_THRESHOLD,
        trusted_keys: vec![TrustedKeyObservation {
            producer_id: carrier.producer_id.clone(),
            key_id: carrier.key_id.clone(),
            key_identity,
            allowed_purposes: vec![WORLD_HEAD_ARTIFACT_AUTH_PURPOSE.to_string()],
            generation: carrier.key_generation,
            currentness: KeyCurrentness::Current,
            currentness_ref: artifact_ref("key-currentness", "current"),
        }],
    }
}

fn artifact_ref(profile: &str, label: &str) -> ArtifactRef {
    ArtifactRef {
        profile: profile.to_string(),
        algorithm: ALGORITHM_BLAKE3.to_string(),
        digest_hex: blake3::hash(label.as_bytes()).to_hex().to_string(),
    }
}

fn signing_adapter<'a>(
    secrets: &'a molten_node_host::node_state::NodeStateNamespace,
) -> LocalWorldHeadSigningAdapter<'a> {
    LocalWorldHeadSigningAdapter::new(secrets, SignerIdentity {
        profile_ref: reference("crypto-profile"),
        entropy_profile_ref: reference("entropy-profile"),
        backend_ref: reference("crypto-backend"),
        producer_id: "molten".to_string(),
        allow_generation: true,
    })
    .expect("world-head signing adapter")
}

fn signed_request(signer: &mut LocalWorldHeadSigningAdapter<'_>, claim: WorldHeadClaim) -> WorldHeadExecutionRequest {
    let (_, mut carrier, _) =
        sign_world_head_claim(signer, &claim, WorldHeadSignerRole::Maintainer).expect("signed world-head claim");
    carrier.authority_admitted = true;
    let policy = authentication_policy(&carrier);
    WorldHeadExecutionRequest {
        claim,
        history: history(),
        policy: world_policy(),
        authentication_policy: policy,
        signatures: vec![carrier],
        currentness: WorldHeadCurrentnessObservation {
            durable_generation_observed: true,
            independent_ref: None,
        },
        bounds: WorldHeadBounds::standard(),
    }
}

struct TestAuthority {
    admitted: bool,
    calls: usize,
}

impl WorldHeadAuthorityPort for TestAuthority {
    fn observe_authority(
        &mut self,
        _branch_id: &WorldBranchId,
        policy_ref: &WorldHeadPolicyRef,
        expected_generation: u64,
    ) -> std::result::Result<WorldHeadAuthorityObservation, WorldHeadPortError> {
        self.calls = self.calls.saturating_add(1);
        Ok(WorldHeadAuthorityObservation {
            authority_ref: WorldHeadAuthorityRef::new(reference("authority-observation")).expect("authority ref"),
            policy_ref: policy_ref.clone(),
            admitted: self.admitted,
            observed_generation: expected_generation,
        })
    }
}

struct UncertainStore {
    reconciliation_recorded: bool,
}

impl WorldHeadStatePort for UncertainStore {
    fn read_head(&self, _branch_id: &WorldBranchId) -> std::result::Result<Option<WorldHeadState>, WorldHeadPortError> {
        Ok(None)
    }

    fn apply_transition<F>(
        &mut self,
        _plan: &WorldHeadTransitionPlan,
        _receipt: &CanonicalWorldHeadTransitionReceipt,
        recheck: F,
    ) -> std::result::Result<WorldHeadMutationOutcome, WorldHeadPortError>
    where
        F: FnOnce(Option<&WorldHeadState>) -> std::result::Result<WorldHeadFreshAdmission, WorldHeadPortError>,
    {
        let fresh = recheck(None)?;
        assert!(fresh.authentication_passed);
        assert!(fresh.authority.admitted);
        Ok(WorldHeadMutationOutcome::Uncertain)
    }
}

impl WorldHeadReconciliationPort for UncertainStore {
    fn record_uncertain_transition(
        &mut self,
        _plan: &WorldHeadTransitionPlan,
        _receipt: &CanonicalWorldHeadTransitionReceipt,
    ) -> std::result::Result<(), WorldHeadPortError> {
        self.reconciliation_recorded = true;
        Ok(())
    }
}

#[test]
fn checked_schema_artifacts_match_world_head_boundary_specs() {
    // r[verify molten.world_heads.authentication]
    for (source, spec) in WORLD_HEAD_SCHEMA_ARTIFACTS.iter().zip(WORLD_HEAD_BOUNDARY_SCHEMAS) {
        let value = crate::preserves_rail::parse_text(source).expect("schema artifact");
        let fields = crate::preserves_rail::simple_record_fields(
            &value,
            "preserves-boundary-schema-artifact-v1",
            WORLD_HEAD_SCHEMA_FIELD_COUNT,
        )
        .expect("schema fields");
        assert_eq!(crate::preserves_rail::record_string_field(&fields[0], "family", "family").unwrap(), spec.family);
        assert_eq!(crate::preserves_rail::record_string_field(&fields[1], "version", "version").unwrap(), spec.version);
        assert_eq!(
            crate::preserves_rail::record_string_field(
                &fields[RECORD_LABEL_FIELD_INDEX],
                "record-label",
                "record label"
            )
            .unwrap(),
            spec.record_label
        );
        assert_eq!(
            crate::preserves_rail::record_string_field(&fields[SCHEMA_ID_FIELD_INDEX], "schema-id", "schema id",)
                .unwrap(),
            spec.schema_id
        );
    }
}
