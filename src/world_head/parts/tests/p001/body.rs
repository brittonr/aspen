
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
    let outcomes = [
        (WorldHeadMutationOutcome::Applied, WorldHeadExecutionStatus::Applied, "applied"),
        (WorldHeadMutationOutcome::Stale, WorldHeadExecutionStatus::Stale, "stale"),
        (WorldHeadMutationOutcome::Uncertain, WorldHeadExecutionStatus::Uncertain, "uncertain"),
        (WorldHeadMutationOutcome::AlreadyApplied, WorldHeadExecutionStatus::AlreadyApplied, "already-applied"),
    ];
    let mut receipts = Vec::with_capacity(outcomes.len());
    for (outcome, status, decision) in outcomes {
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
