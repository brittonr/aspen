
fn near_far_calls(objects: &FixtureObjects) -> Result<Vec<VatCallEvidence>> {
    let near_call = crate::runtime::evaluate_near_far_refs(&crate::runtime::RuntimeNearFarRefState {
        reference_ref: objects.helper_ref.clone(),
        reference_kind: crate::runtime::RuntimeReferenceKind::Near,
        is_live: true,
        caller_vat_id: LOCAL_VAT_ID.to_string(),
        target_vat_id: LOCAL_VAT_ID.to_string(),
        call_mode: crate::runtime::RuntimeReferenceCallMode::Synchronous,
    })?;
    let far_sync_denial = crate::runtime::evaluate_near_far_refs(&crate::runtime::RuntimeNearFarRefState {
        reference_ref: objects.far_ref.clone(),
        reference_kind: crate::runtime::RuntimeReferenceKind::Far,
        is_live: true,
        caller_vat_id: LOCAL_VAT_ID.to_string(),
        target_vat_id: REMOTE_VAT_ID.to_string(),
        call_mode: crate::runtime::RuntimeReferenceCallMode::Synchronous,
    })?;
    let far_async = crate::runtime::evaluate_near_far_refs(&crate::runtime::RuntimeNearFarRefState {
        reference_ref: objects.far_ref.clone(),
        reference_kind: crate::runtime::RuntimeReferenceKind::Far,
        is_live: true,
        caller_vat_id: LOCAL_VAT_ID.to_string(),
        target_vat_id: REMOTE_VAT_ID.to_string(),
        call_mode: crate::runtime::RuntimeReferenceCallMode::Asynchronous,
    })?;

    Ok(vec![
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
    ])
}

fn actormap_calls(objects: &FixtureObjects) -> Result<Vec<VatCallEvidence>> {
    let committed = crate::runtime::evaluate_actormap_transaction(&crate::runtime::RuntimeActormapTransactionState {
        outcome: crate::runtime::RuntimeActormapTransactionOutcome::Committed,
        before_object_refs: sorted_refs(vec![objects.root_ref.clone(), objects.helper_ref.clone()]),
        after_object_refs: sorted_refs(vec![
            objects.root_ref.clone(),
            objects.helper_ref.clone(),
            objects.spawned_ref.clone(),
        ]),
        spawned_object_refs: vec![objects.spawned_ref.clone()],
        removed_object_refs: Vec::new(),
        visible_object_refs: sorted_refs(vec![
            objects.root_ref.clone(),
            objects.helper_ref.clone(),
            objects.spawned_ref.clone(),
        ]),
        used_object_refs: vec![objects.helper_ref.clone()],
    })?;
    let rollback = crate::runtime::evaluate_actormap_transaction(&crate::runtime::RuntimeActormapTransactionState {
        outcome: crate::runtime::RuntimeActormapTransactionOutcome::RolledBack,
        before_object_refs: sorted_refs(vec![objects.root_ref.clone(), objects.helper_ref.clone()]),
        after_object_refs: sorted_refs(vec![objects.root_ref.clone(), objects.helper_ref.clone()]),
        spawned_object_refs: vec![objects.spawned_ref.clone()],
        removed_object_refs: Vec::new(),
        visible_object_refs: sorted_refs(vec![objects.root_ref.clone(), objects.helper_ref.clone()]),
        used_object_refs: vec![objects.helper_ref.clone()],
    })?;

    Ok(vec![
        VatCallEvidence {
            name: "actormap-commit".to_string(),
            receipt: committed.receipt,
        },
        VatCallEvidence {
            name: "actormap-rollback".to_string(),
            receipt: rollback.receipt,
        },
    ])
}

fn pipeline_call(far_ref: &str) -> Result<VatCallEvidence> {
    let pipeline = crate::runtime::evaluate_promise_pipeline(&crate::runtime::RuntimePromisePipelineState::new(
        crate::runtime::RuntimePromiseState::pending("promise:far-call"),
        PIPELINE_MAX_QUEUE,
        vec![
            crate::runtime::RuntimePromisePipelineEntry::new(1, far_ref.to_string(), "get"),
            crate::runtime::RuntimePromisePipelineEntry::new(2, far_ref.to_string(), "subscribe"),
        ],
    ))?;
    Ok(VatCallEvidence {
        name: "promise-pipeline".to_string(),
        receipt: pipeline.receipt,
    })
}

fn revocation_call(proxy_ref: &str) -> Result<VatCallEvidence> {
    let revoked = crate::runtime::evaluate_revocation_cleanup(&crate::runtime::RuntimeRevocationCleanupState {
        revoked_refs: vec![proxy_ref.to_string()],
        attempted_use_refs: vec![proxy_ref.to_string()],
        remaining_assertion_refs: Vec::new(),
        remaining_subscription_refs: Vec::new(),
        remaining_pending_call_refs: Vec::new(),
        remaining_child_refs: Vec::new(),
    })?;
    Ok(VatCallEvidence {
        name: "revocation-cleanup".to_string(),
        receipt: revoked.receipt,
    })
}

fn fixture_calls(objects: &FixtureObjects) -> Result<Vec<VatCallEvidence>> {
    let mut calls = near_far_calls(objects)?;
    calls.extend(actormap_calls(objects)?);
    calls.push(pipeline_call(&objects.far_ref)?);
    calls.push(revocation_call(&objects.proxy_ref)?);
    Ok(calls)
}

pub fn run_vat_fixture() -> Result<VatFixtureRun> {
    let objects = fixture_objects()?;
    let calls = fixture_calls(&objects)?;
    let receipts = calls.iter().map(|call| call.receipt.clone()).collect::<Vec<_>>();
    let diagnostics = fixture_diagnostics(&receipts);
    let value = record("vat-fixture-run-v1", vec![
        string(RUNTIME_VAT_FIXTURE_RUN_SCHEMA),
        string(LOCAL_VAT_ID),
        sequence(objects.object_values()),
        sequence(calls.iter().map(VatCallEvidence::value).collect()),
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
    let pass = crate::runtime::evaluate_snapshot_authority(&crate::runtime::RuntimeSnapshotAuthorityState {
        snapshot_ref: snapshot_ref.clone(),
        admitted_authority_refs: sorted_refs(vec![root_ref.clone(), helper_ref.clone()]),
        claimed_authority_refs: vec![helper_ref.clone()],
        requested_assertion_refs: vec![helper_ref.clone()],
        readable_assertion_refs: vec![helper_ref.clone()],
        redacted_assertion_refs: Vec::new(),
    })?;
    let denied = crate::runtime::evaluate_snapshot_authority(&crate::runtime::RuntimeSnapshotAuthorityState {
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
    let pending = crate::runtime::RuntimePromiseState::pending("promise:far-call");
    let resolved = crate::runtime::RuntimePromiseState::resolved("promise:far-call", result_ref.clone());
    let broken =
        crate::runtime::RuntimePromiseState::broken("promise:failed-call", "target turn aborted", vec![cause_ref]);
    let cancelled = crate::runtime::RuntimePromiseState::cancelled("promise:cancelled-call", "caller revoked interest");
    let timed_out = crate::runtime::RuntimePromiseState::timed_out("promise:timeout-call", "logical timeout elapsed");
    let changed_terminal = crate::runtime::RuntimePromiseState::broken("promise:far-call", "late failure", Vec::new());

    let resolve_receipt = crate::runtime::evaluate_promise_state_transition(&pending, &resolved)?.receipt;
    let broken_receipt = crate::runtime::evaluate_promise_state_transition(
        &crate::runtime::RuntimePromiseState::pending("promise:failed-call"),
        &broken,
    )?
    .receipt;
    let cancel_receipt = crate::runtime::evaluate_promise_state_transition(
        &crate::runtime::RuntimePromiseState::pending("promise:cancelled-call"),
        &cancelled,
    )?
    .receipt;
    let timeout_receipt = crate::runtime::evaluate_promise_state_transition(
        &crate::runtime::RuntimePromiseState::pending("promise:timeout-call"),
        &timed_out,
    )?
    .receipt;
    let terminal_denial = crate::runtime::evaluate_promise_state_transition(&resolved, &changed_terminal)?.receipt;
    let pipeline_cleanup = crate::runtime::evaluate_promise_pipeline(
        &crate::runtime::RuntimePromisePipelineState::new(broken, PIPELINE_MAX_QUEUE, vec![
            crate::runtime::RuntimePromisePipelineEntry::new(
                1,
                canonical_hash(&string("stale-target"))?,
                "after-break",
            ),
        ]),
    )?
    .receipt;
    let dependent_call_ref = canonical_hash(&record("vat-dependent-call-v1", vec![string("use-promise-value")]))?;
    let resolved_use = crate::runtime::evaluate_promise_use(&crate::runtime::RuntimePromiseUseState {
        source: resolved.clone(),
        use_kind: crate::runtime::RuntimePromiseUseKind::ResolvedValue,
        dependent_call_ref: dependent_call_ref.clone(),
        admitted_resolution_ref: Some(result_ref),
        admitted_pipeline_ref: None,
    })?
    .receipt;
    let unresolved_use_denial = crate::runtime::evaluate_promise_use(&crate::runtime::RuntimePromiseUseState {
        source: pending.clone(),
        use_kind: crate::runtime::RuntimePromiseUseKind::ResolvedValue,
        dependent_call_ref,
        admitted_resolution_ref: None,
        admitted_pipeline_ref: None,
    })?
    .receipt;

    let receipts = vec![
        resolve_receipt,
        broken_receipt,
        cancel_receipt,
        timeout_receipt,
        terminal_denial,
        pipeline_cleanup,
        resolved_use,
        unresolved_use_denial,
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

struct DistRefs {
    far: VatObjectRef,
    replacement: VatObjectRef,
    far_ref: String,
    replacement_ref: String,
    session_ref: String,
    pending_call_ref: String,
    stale_call_ref: String,
}

enum DistCase {
    Live,
    Disconnected,
    Handoff,
    StaleUse,
    PendingOpen,
}

fn dist_refs() -> Result<DistRefs> {
    let far = VatObjectRef::new(REMOTE_VAT_ID, FAR_OBJECT_ID, VatReferenceKind::Far, Vec::new());
    let replacement = VatObjectRef::new(REMOTE_VAT_ID, "object:remote:handoff", VatReferenceKind::Far, Vec::new());
    Ok(DistRefs {
        far_ref: far.object_ref()?,
        replacement_ref: replacement.object_ref()?,
        session_ref: canonical_hash(&record("vat-session-descriptor-v1", vec![string("session:primary")]))?,
        pending_call_ref: canonical_hash(&record("vat-pending-call-v1", vec![string("call:primary")]))?,
        stale_call_ref: canonical_hash(&record("vat-pending-call-v1", vec![string("call:stale")]))?,
        far,
        replacement,
    })
}
