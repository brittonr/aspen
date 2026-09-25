
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

#[test]
fn conflict_reads_accept_the_record_limit_and_deny_one_past() {
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = NodeStateRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NodeStateNamespaceKind::Storage).expect("storage namespace");
    let mut store = LocalWorldHeadStore::open(&storage).expect("world-head store");
    let plans = [
        admitted_plan("left", "left-claim"),
        admitted_plan("right", "right-claim"),
    ];
    let (conflict, canonical) = record_world_head_conflict(&mut store, &plans, MAX_WORLD_HEAD_CONFLICTS)
        .expect("record conflict")
        .expect("conflict");
    let record_fabricated = |store: &mut LocalWorldHeadStore, index: usize| {
        let mut fabricated = canonical.clone();
        fabricated.conflict_ref = reference(&format!("fabricated-conflict-{index}"));
        fabricated.bytes = fabricated.conflict_ref.clone().into_bytes();
        store.record_conflict(&conflict, &fabricated).expect("record fabricated conflict");
    };
    for index in 1..MAX_WORLD_HEAD_CONFLICT_RECORDS {
        record_fabricated(&mut store, index);
    }
    let at_limit = store.read_conflicts(&branch()).expect("conflicts at the record limit");
    assert_eq!(at_limit.len(), MAX_WORLD_HEAD_CONFLICT_RECORDS);

    record_fabricated(&mut store, MAX_WORLD_HEAD_CONFLICT_RECORDS);
    assert_eq!(
        store.read_conflicts(&branch()),
        Err(WorldHeadPortError::new("conflict-record-limit", "branch conflict records exceed the read bound")),
        "one-past reads deny without returning a partial conflict list"
    );
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
