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
const WORLD_HEAD_SCHEMA_FIELD_COUNT: usize = 6;
const RECORD_LABEL_FIELD_INDEX: usize = 3;
const SCHEMA_ID_FIELD_INDEX: usize = 4;
const WORLD_HEAD_SCHEMA_ARTIFACTS: [&str; WORLD_HEAD_SCHEMA_COUNT] = [
    include_str!("../../schemas/preserves-boundaries/molten-world-head-claim-v1.preserves"),
    include_str!("../../schemas/preserves-boundaries/molten-world-head-conflict-v1.preserves"),
    include_str!("../../schemas/preserves-boundaries/molten-world-head-transition-receipt-v1.preserves"),
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
    LocalWorldHeadSigningAdapter::new(
        secrets,
        reference("crypto-profile"),
        reference("entropy-profile"),
        reference("crypto-backend"),
        "molten".to_string(),
        true,
    )
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

#[test]
fn canonical_claim_and_artifact_auth_statement_bind_exact_transition_bytes() {
    // r[verify molten.world_heads.authentication]
    let canonical = canonical_world_head_claim(&advance_claim("left")).expect("canonical claim");
    let parsed = parse_canonical_world_head_claim(&canonical.bytes).expect("parse canonical claim");
    assert_eq!(parsed.claim, canonical.claim);
    assert_eq!(parsed.claim_ref, canonical.claim_ref);

    let key = artifact_auth_ed25519::public_key_identity(
        &[TEST_PUBLIC_KEY_BYTE; artifact_auth_ed25519::ED25519_PUBLIC_KEY_BYTES],
    );
    let (statement, statement_ref) = world_head_artifact_statement(&canonical, WorldHeadArtifactAuthInput {
        producer_id: "molten",
        key_id: "maintainer",
        key_identity: key,
    })
    .expect("Artifact Auth statement");
    let bytes = artifact_auth_core::canonical_statement_bytes(&statement).expect("statement bytes");
    assert!(!bytes.is_empty());
    assert_eq!(
        statement.scope.subject.digest_hex,
        crate::preserves_rail::content_ref_hex(canonical.claim_ref.as_str()).unwrap()
    );
    assert!(statement_ref.as_str().starts_with("blake3:"));

    let mut tampered = canonical.bytes.clone();
    tampered.push(0);
    assert!(parse_canonical_world_head_claim(&tampered).is_err());
}

#[test]
fn local_store_atomically_creates_advances_and_survives_restart() {
    // r[verify molten.world_heads.cas]
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = NodeStateRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NodeStateNamespaceKind::Storage).expect("storage namespace");
    let secrets = root.namespace(NodeStateNamespaceKind::Secrets).expect("secrets namespace");
    let mut signer = signing_adapter(&secrets);
    let mut store = LocalWorldHeadStore::open(&storage).expect("world-head store");
    let mut authority = TestAuthority {
        admitted: true,
        calls: 0,
    };

    let create = signed_request(&mut signer, create_claim());
    let created = execute_world_head_transition(&mut store, &mut authority, &create).expect("create world head");
    assert_eq!(created.status, WorldHeadExecutionStatus::Applied);
    assert_eq!(authority.calls, EXPECTED_AUTHORITY_RECHECKS);
    assert_eq!(store.read_head(&branch()).unwrap().unwrap().head, commit("root"));

    let advance = signed_request(&mut signer, advance_claim("left"));
    let advanced = execute_world_head_transition(&mut store, &mut authority, &advance).expect("advance world head");
    assert_eq!(advanced.status, WorldHeadExecutionStatus::Applied);
    let state = store.read_head(&branch()).unwrap().unwrap();
    assert_eq!(state.head, commit("left"));
    assert_eq!(state.generation, NEXT_GENERATION);
    assert!(store.transition_receipt(&advanced.receipt.receipt_ref).expect("transition receipt read").is_some());

    drop(store);
    let reopened = LocalWorldHeadStore::open(&storage).expect("reopened world-head store");
    assert_eq!(reopened.read_head(&branch()).unwrap(), Some(state));
}

#[test]
fn valid_signature_never_overrides_denied_authority_or_stale_state() {
    // r[verify molten.world_heads.authentication]
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = NodeStateRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NodeStateNamespaceKind::Storage).expect("storage namespace");
    let secrets = root.secrets().expect("secrets namespace");
    let mut signer = signing_adapter(&secrets);
    let mut store = LocalWorldHeadStore::open(&storage).expect("world-head store");
    let create = signed_request(&mut signer, create_claim());
    let mut denied_authority = TestAuthority {
        admitted: false,
        calls: 0,
    };
    let denied =
        execute_world_head_transition(&mut store, &mut denied_authority, &create).expect("authority denial result");
    assert_eq!(denied.status, WorldHeadExecutionStatus::Denied);
    assert!(store.read_head(&branch()).unwrap().is_none());

    let mut admitted_authority = TestAuthority {
        admitted: true,
        calls: 0,
    };
    execute_world_head_transition(&mut store, &mut admitted_authority, &create).expect("create head");
    let left = signed_request(&mut signer, advance_claim("left"));
    execute_world_head_transition(&mut store, &mut admitted_authority, &left).expect("advance left");
    let stale = signed_request(&mut signer, advance_claim("right"));
    let stale_result =
        execute_world_head_transition(&mut store, &mut admitted_authority, &stale).expect("stale claim result");
    assert_eq!(stale_result.status, WorldHeadExecutionStatus::Denied);
    assert_eq!(store.read_head(&branch()).unwrap().unwrap().head, commit("left"));
}

#[test]
fn threshold_tamper_revocation_and_wrong_purpose_fail_closed() {
    // r[verify molten.world_heads.verification]
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = NodeStateRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let secrets = root.secrets().expect("secrets namespace");
    let mut signer = signing_adapter(&secrets);
    let claim = canonical_world_head_claim(&create_claim()).expect("claim");
    let (_, mut carrier, _) =
        sign_world_head_claim(&mut signer, &claim.claim, WorldHeadSignerRole::Maintainer).expect("signed claim");
    carrier.authority_admitted = true;
    let policy = authentication_policy(&carrier);
    let passed = evaluate_world_head_authentication(&claim, &policy, &[carrier.clone()]).expect("authentication");
    assert!(passed.observation.passed);

    let mut tampered = carrier.clone();
    tampered.signature_bytes[0] ^= SIGNATURE_TAMPER_MASK;
    assert!(
        !evaluate_world_head_authentication(&claim, &policy, &[tampered])
            .expect("tampered authentication")
            .observation
            .passed
    );

    let mut revoked_policy = policy.clone();
    revoked_policy.trusted_keys[0].currentness = KeyCurrentness::Revoked;
    assert!(
        !evaluate_world_head_authentication(&claim, &revoked_policy, &[carrier.clone()])
            .expect("revoked authentication")
            .observation
            .passed
    );

    let mut wrong_purpose = policy.clone();
    wrong_purpose.trusted_keys[0].allowed_purposes = vec!["release-evidence".to_string()];
    assert!(
        !evaluate_world_head_authentication(&claim, &wrong_purpose, &[carrier])
            .expect("wrong purpose authentication")
            .observation
            .passed
    );
}

#[test]
fn uncertain_storage_outcome_enters_reconciliation_without_success_overclaim() {
    // r[verify molten.world_heads.rollback]
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = NodeStateRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let secrets = root.secrets().expect("secrets namespace");
    let mut signer = signing_adapter(&secrets);
    let request = signed_request(&mut signer, create_claim());
    let mut store = UncertainStore {
        reconciliation_recorded: false,
    };
    let mut authority = TestAuthority {
        admitted: true,
        calls: 0,
    };

    let result =
        execute_world_head_transition(&mut store, &mut authority, &request).expect("uncertain transition result");

    assert_eq!(result.status, WorldHeadExecutionStatus::Uncertain);
    assert!(store.reconciliation_recorded);
    let receipt_text = crate::preserves_rail::to_text(&result.receipt.value).expect("world-head receipt text");
    assert!(receipt_text.contains("does-not-prove-whole-store-rollback-detection"));
    assert!(!receipt_text.contains("remote-convergence-proven"));
}

#[test]
fn competing_plans_are_stored_as_a_stable_conflict_set() {
    // r[verify molten.world_heads.conflicts]
    let left = admitted_plan("left", "left-claim");
    let right = admitted_plan("right", "right-claim");
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = NodeStateRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NodeStateNamespaceKind::Storage).expect("storage namespace");
    let mut store = LocalWorldHeadStore::open(&storage).expect("world-head store");

    let (conflict, canonical) =
        record_world_head_conflict(&mut store, &[right.clone(), left.clone()], MAX_WORLD_HEAD_CONFLICTS)
            .expect("record conflict")
            .expect("conflict");
    assert_eq!(conflict.members.len(), 2);
    assert_eq!(store.read_conflicts(&branch()).unwrap(), vec![canonical.bytes]);

    let repeated = record_world_head_conflict(&mut store, &[left, right], MAX_WORLD_HEAD_CONFLICTS)
        .expect("repeat conflict")
        .expect("conflict");
    assert_eq!(repeated.0.conflict_ref, conflict.conflict_ref);
}

fn admitted_plan(successor: &str, claim_label: &str) -> WorldHeadTransitionPlan {
    let claim = advance_claim(successor);
    let request = WorldHeadPlanRequest {
        claim_ref: WorldHeadClaimRef::new(reference(claim_label)).expect("claim ref"),
        claim,
        current: Some(WorldHeadState {
            branch_id: branch(),
            branch_class: WorldBranchClass::Local,
            head: commit("root"),
            generation: INITIAL_GENERATION,
            policy_ref: policy_ref(),
        }),
        history: history(),
        policy: world_policy(),
        authentication: WorldHeadAuthenticationObservation {
            statement_ref: WorldHeadStatementRef::new(reference("statement")).expect("statement ref"),
            decision_ref: WorldHeadAuthenticationDecisionRef::new(reference("decision")).expect("decision ref"),
            passed: true,
            purpose_matches: true,
            policy_matches: true,
            signers: vec![WorldHeadSignerObservation {
                key_identity_ref: reference("key"),
                role: WorldHeadSignerRole::Maintainer,
                authenticated: true,
                current: true,
                revoked: false,
                authority_admitted: true,
            }],
        },
        authority: WorldHeadAuthorityObservation {
            authority_ref: WorldHeadAuthorityRef::new(reference("authority")).expect("authority ref"),
            policy_ref: policy_ref(),
            admitted: true,
            observed_generation: INITIAL_GENERATION,
        },
        currentness: WorldHeadCurrentnessObservation {
            durable_generation_observed: true,
            independent_ref: None,
        },
        bounds: WorldHeadBounds::standard(),
    };
    match plan_world_head_transition(&request) {
        WorldHeadDecision::Admitted(plan) => plan,
        decision => panic!("expected plan, got {decision:?}"),
    }
}
