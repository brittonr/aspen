#[test]
fn signer_role_and_authority_come_from_trusted_policy_and_authority_port() {
    // r[verify molten.world_commit.world_head_truthful_receipts_and_verified_signers]
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NamespaceKind::Storage).expect("storage namespace");
    let secrets = root.secrets().expect("secrets namespace");
    let mut signer = signing_adapter(&secrets);
    let mut store = LocalWorldHeadStore::open(&storage).expect("world-head store");

    // Branch authority is admitted, but the authority port does not admit the signer.
    let create = signed_request(&mut signer, create_claim());
    let mut signer_denied = TestAuthority::new(true);
    signer_denied.signer_admitted = false;
    let denied = execute_world_head_transition(&mut store, &mut signer_denied, &create).expect("signer denial");
    assert_eq!(denied.status, WorldHeadExecutionStatus::Denied);
    assert!(denied.issues.contains(&WorldHeadIssue::SignerAuthorityDenied));
    assert_eq!(receipt_decision(&denied.receipt), "denied");

    // A recovery key's signature carries its trusted recovery role, which this policy excludes.
    let (_, recovery, _) = sign_world_head_claim(&mut signer, &create.claim, WorldHeadSignerRole::Recovery)
        .expect("recovery signature");
    let mut recovery_request = create.clone();
    recovery_request.authentication_policy = authentication_policy(&recovery);
    recovery_request.signatures = vec![recovery.clone()];
    let mut authority = TestAuthority::new(true);
    let wrong_role =
        execute_world_head_transition(&mut store, &mut authority, &recovery_request).expect("wrong role result");
    assert_eq!(wrong_role.status, WorldHeadExecutionStatus::Denied);
    assert!(wrong_role.issues.contains(&WorldHeadIssue::UnknownSignerRole));
    assert_eq!(authority.signer_roles, vec![WorldHeadSignerRole::Recovery]);

    // Relabeling the carrier's key id as the maintainer key breaks the signed statement, even when
    // the policy trusts the same key identity under the maintainer key id.
    let mut relabeled = recovery.clone();
    relabeled.key_id = world_head_signer_key_id(WorldHeadSignerRole::Maintainer);
    let mut forged_request = recovery_request;
    forged_request.authentication_policy.trusted_keys.push(authentication_policy(&relabeled).trusted_keys.remove(0));
    forged_request.signatures = vec![relabeled];
    let forged = execute_world_head_transition(&mut store, &mut TestAuthority::new(true), &forged_request)
        .expect("forged role result");
    assert_eq!(forged.status, WorldHeadExecutionStatus::Denied);
    assert!(forged.issues.contains(&WorldHeadIssue::AuthenticationDenied));
    assert!(store.read_head(&branch()).unwrap().is_none());

    assert_eq!(world_head_signer_role(&recovery.key_id), Some(WorldHeadSignerRole::Recovery));
    assert_eq!(world_head_signer_role("operator-key"), None);
}

#[test]
fn competing_plans_are_stored_as_a_stable_conflict_set() {
    // r[verify molten.world_heads.conflicts]
    let left = admitted_plan("left", "left-claim");
    let right = admitted_plan("right", "right-claim");
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NamespaceKind::Storage).expect("storage namespace");
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
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NamespaceKind::Storage).expect("storage namespace");
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
                role: Some(WorldHeadSignerRole::Maintainer),
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
