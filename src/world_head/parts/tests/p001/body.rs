
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
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NamespaceKind::Storage).expect("storage namespace");
    let secrets = root.namespace(NamespaceKind::Secrets).expect("secrets namespace");
    let mut signer = signing_adapter(&secrets);
    let mut store = LocalWorldHeadStore::open(&storage).expect("world-head store");
    let mut authority = TestAuthority::new(true);

    let create = signed_request(&mut signer, create_claim());
    let created = execute_world_head_transition(&mut store, &mut authority, &create).expect("create world head");
    assert_eq!(created.status, WorldHeadExecutionStatus::Applied);
    assert_eq!(receipt_decision(&created.receipt), "applied");
    assert_eq!(authority.calls, EXPECTED_AUTHORITY_RECHECKS);
    assert_eq!(store.read_head(&branch()).unwrap().unwrap().head, commit("root"));

    let advance = signed_request(&mut signer, advance_claim("left"));
    let advanced = execute_world_head_transition(&mut store, &mut authority, &advance).expect("advance world head");
    assert_eq!(advanced.status, WorldHeadExecutionStatus::Applied);
    let state = store.read_head(&branch()).unwrap().unwrap();
    assert_eq!(state.head, commit("left"));
    assert_eq!(state.generation, NEXT_GENERATION);
    assert_eq!(
        store.transition_receipt(&advanced.receipt.receipt_ref).expect("transition receipt read"),
        Some(advanced.receipt.bytes.clone())
    );
    assert_eq!(authority.signer_roles, vec![WorldHeadSignerRole::Maintainer; 4]);

    let replayed = execute_world_head_transition(&mut store, &mut authority, &advance).expect("replayed advance");
    assert_eq!(replayed.status, WorldHeadExecutionStatus::Denied);

    drop(store);
    let reopened = LocalWorldHeadStore::open(&storage).expect("reopened world-head store");
    assert_eq!(reopened.read_head(&branch()).unwrap(), Some(state));
}

#[test]
fn valid_signature_never_overrides_denied_authority_or_stale_state() {
    // r[verify molten.world_heads.authentication]
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NamespaceKind::Storage).expect("storage namespace");
    let secrets = root.secrets().expect("secrets namespace");
    let mut signer = signing_adapter(&secrets);
    let mut store = LocalWorldHeadStore::open(&storage).expect("world-head store");
    let create = signed_request(&mut signer, create_claim());
    let mut denied_authority = TestAuthority::new(false);
    let denied =
        execute_world_head_transition(&mut store, &mut denied_authority, &create).expect("authority denial result");
    assert_eq!(denied.status, WorldHeadExecutionStatus::Denied);
    assert!(store.read_head(&branch()).unwrap().is_none());

    let mut admitted_authority = TestAuthority::new(true);
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
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let secrets = root.secrets().expect("secrets namespace");
    let mut signer = signing_adapter(&secrets);
    let claim = canonical_world_head_claim(&create_claim()).expect("claim");
    let (_, carrier, _) =
        sign_world_head_claim(&mut signer, &claim.claim, WorldHeadSignerRole::Maintainer).expect("signed claim");
    let policy = authentication_policy(&carrier);
    let mut authority = TestAuthority::new(true);
    let passed = evaluate_world_head_authentication(&claim, &policy, std::slice::from_ref(&carrier), &mut authority)
        .expect("authentication");
    assert!(passed.observation.passed);
    assert_eq!(passed.observation.signers[0].role, Some(WorldHeadSignerRole::Maintainer));
    assert!(passed.observation.signers[0].authority_admitted);

    let mut tampered = carrier.clone();
    tampered.signature_bytes[0] ^= SIGNATURE_TAMPER_MASK;
    assert!(
        !evaluate_world_head_authentication(&claim, &policy, &[tampered], &mut authority)
            .expect("tampered authentication")
            .observation
            .passed
    );

    let mut revoked_policy = policy.clone();
    revoked_policy.trusted_keys[0].currentness = KeyCurrentness::Revoked;
    assert!(
        !evaluate_world_head_authentication(&claim, &revoked_policy, std::slice::from_ref(&carrier), &mut authority)
            .expect("revoked authentication")
            .observation
            .passed
    );

    let mut wrong_purpose = policy.clone();
    wrong_purpose.trusted_keys[0].allowed_purposes = vec!["release-evidence".to_string()];
    assert!(
        !evaluate_world_head_authentication(&claim, &wrong_purpose, &[carrier], &mut authority)
            .expect("wrong purpose authentication")
            .observation
            .passed
    );
}

#[test]
fn uncertain_storage_outcome_enters_reconciliation_without_success_overclaim() {
    // r[verify molten.world_heads.rollback]
    // r[verify molten.world_commit.world_head_truthful_receipts_and_verified_signers]
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let secrets = root.secrets().expect("secrets namespace");
    let mut signer = signing_adapter(&secrets);
    let request = signed_request(&mut signer, create_claim());
    let mut store = ScriptedStore::new(WorldHeadMutationOutcome::Uncertain);
    let mut authority = TestAuthority::new(true);

    let result =
        execute_world_head_transition(&mut store, &mut authority, &request).expect("uncertain transition result");

    assert_eq!(result.status, WorldHeadExecutionStatus::Uncertain);
    assert_eq!(receipt_decision(&result.receipt), "uncertain");
    let reconciliation = store.reconciliation_receipt.as_ref().expect("reconciliation receipt recorded");
    assert_eq!(reconciliation.bytes, result.receipt.bytes, "the uncertain table holds the returned receipt");
    assert_eq!(receipt_decision(reconciliation), "uncertain");
    let receipt_text = crate::preserves_rail::to_text(&result.receipt.value).expect("world-head receipt text");
    assert!(receipt_text.contains("does-not-prove-whole-store-rollback-detection"));
    assert!(!receipt_text.contains("remote-convergence-proven"));
}

#[test]
fn applied_stale_and_uncertain_outcomes_emit_distinct_truthful_receipts() {
    // r[verify molten.world_commit.world_head_truthful_receipts_and_verified_signers]
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let secrets = root.secrets().expect("secrets namespace");
    let mut signer = signing_adapter(&secrets);
    let request = signed_request(&mut signer, create_claim());
    let mut receipts = Vec::new();
    for (outcome, status, decision) in [
        (WorldHeadMutationOutcome::Applied, WorldHeadExecutionStatus::Applied, "applied"),
        (WorldHeadMutationOutcome::Stale, WorldHeadExecutionStatus::Stale, "stale"),
        (WorldHeadMutationOutcome::Uncertain, WorldHeadExecutionStatus::Uncertain, "uncertain"),
        (WorldHeadMutationOutcome::AlreadyApplied, WorldHeadExecutionStatus::AlreadyApplied, "already-applied"),
    ] {
        let mut store = ScriptedStore::new(outcome);
        let mut authority = TestAuthority::new(true);
        let result = execute_world_head_transition(&mut store, &mut authority, &request).expect("transition result");
        assert_eq!(result.status, status);
        assert_eq!(receipt_decision(&result.receipt), decision);
        // The store is only ever offered the receipt its committing transaction makes true.
        let offered = store.offered_receipt.as_ref().expect("offered receipt");
        assert_eq!(receipt_decision(offered), "applied");
        assert_eq!(offered.bytes == result.receipt.bytes, outcome == WorldHeadMutationOutcome::Applied);
        assert_eq!(store.reconciliation_receipt.is_some(), outcome == WorldHeadMutationOutcome::Uncertain);
        let is_stale = outcome == WorldHeadMutationOutcome::Stale;
        assert_eq!(result.issues == vec![WorldHeadIssue::StaleExpectedHead], is_stale);
        receipts.push(result.receipt.receipt_ref);
    }
    let distinct = receipts.iter().collect::<BTreeSet<_>>();
    assert_eq!(distinct.len(), receipts.len(), "every outcome has its own canonical receipt identity");
}

#[test]
fn local_cas_conflict_returns_stale_receipt_and_persists_nothing() {
    // r[verify molten.world_commit.world_head_truthful_receipts_and_verified_signers]
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NamespaceKind::Storage).expect("storage namespace");
    let secrets = root.secrets().expect("secrets namespace");
    let mut signer = signing_adapter(&secrets);
    let mut store = LocalWorldHeadStore::open(&storage).expect("world-head store");
    let create = signed_request(&mut signer, create_claim());
    execute_world_head_transition(&mut store, &mut TestAuthority::new(true), &create).expect("create head");
    let before = store.read_head(&branch()).unwrap();

    let mut racing = TestAuthority::new(true);
    racing.stale_on_recheck = true;
    let advance = signed_request(&mut signer, advance_claim("left"));
    let stale = execute_world_head_transition(&mut store, &mut racing, &advance).expect("stale result");

    assert_eq!(stale.status, WorldHeadExecutionStatus::Stale);
    assert_eq!(receipt_decision(&stale.receipt), "stale");
    assert_eq!(stale.issues, vec![WorldHeadIssue::StaleExpectedHead]);
    assert_eq!(store.read_head(&branch()).unwrap(), before);
    assert!(store.transition_receipt(&stale.receipt.receipt_ref).expect("transition receipt read").is_none());
}

#[test]
fn signer_authority_withdrawn_before_commit_denies_inside_mutation_boundary() {
    // r[verify molten.world_commit.world_head_truthful_receipts_and_verified_signers]
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = DirectoryRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("node state layout");
    let storage = root.namespace(NamespaceKind::Storage).expect("storage namespace");
    let secrets = root.secrets().expect("secrets namespace");
    let mut signer = signing_adapter(&secrets);
    let mut store = LocalWorldHeadStore::open(&storage).expect("world-head store");
    let create = signed_request(&mut signer, create_claim());
    execute_world_head_transition(&mut store, &mut TestAuthority::new(true), &create).expect("create head");
    let before = store.read_head(&branch()).unwrap();

    // Branch authority stays admitted and the signature stays valid; only the signer's authority
    // is withdrawn between planning admission and the transactional recheck.
    let mut withdrawing = TestAuthority::new(true);
    withdrawing.withdraw_signer_on_recheck = true;
    let advance = signed_request(&mut signer, advance_claim("left"));
    let error = execute_world_head_transition(&mut store, &mut withdrawing, &advance)
        .expect_err("withdrawn signer authority denies the commit");
    assert!(error.to_string().contains("fresh-admission-denied"), "{error}");
    assert_eq!(withdrawing.signer_roles.len(), 2, "planning and the in-transaction recheck both consulted the port");
    assert_eq!(store.read_head(&branch()).unwrap(), before);
}

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
