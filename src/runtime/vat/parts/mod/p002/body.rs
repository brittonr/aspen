
fn dist_state(refs: &DistRefs, case: DistCase) -> crate::runtime::RuntimeDistributedRefLifetimeState {
    let mut state = crate::runtime::RuntimeDistributedRefLifetimeState {
        far_ref: refs.far_ref.clone(),
        session_ref: refs.session_ref.clone(),
        replacement_ref: None,
        is_session_live: false,
        is_handoff_admitted: false,
        pending_call_refs: Vec::new(),
        failed_pending_call_refs: Vec::new(),
        attempted_use_refs: Vec::new(),
    };
    match case {
        DistCase::Live => {
            state.is_session_live = true;
            state.attempted_use_refs.push(refs.far_ref.clone());
        }
        DistCase::Disconnected => {
            state.pending_call_refs.push(refs.pending_call_ref.clone());
            state.failed_pending_call_refs.push(refs.pending_call_ref.clone());
        }
        DistCase::Handoff => {
            state.replacement_ref = Some(refs.replacement_ref.clone());
            state.is_handoff_admitted = true;
            state.attempted_use_refs.push(refs.replacement_ref.clone());
        }
        DistCase::StaleUse => state.attempted_use_refs.push(refs.far_ref.clone()),
        DistCase::PendingOpen => state.pending_call_refs.push(refs.stale_call_ref.clone()),
    }
    state
}

fn dist_receipts(refs: &DistRefs) -> Result<Vec<crate::runtime::RuntimePredicateReceipt>> {
    [
        DistCase::Live,
        DistCase::Disconnected,
        DistCase::Handoff,
        DistCase::StaleUse,
        DistCase::PendingOpen,
    ]
    .into_iter()
    .map(|case| {
        crate::runtime::evaluate_distributed_ref_lifetime(&dist_state(refs, case)).map(|outcome| outcome.receipt)
    })
    .collect()
}

pub fn run_vat_distributed_ref_fixture() -> Result<VatDistributedRefFixture> {
    let refs = dist_refs()?;
    let receipts = dist_receipts(&refs)?;
    let diagnostics = fixture_diagnostics(&receipts);
    let value = record("vat-distributed-ref-fixture-v1", vec![
        string(RUNTIME_VAT_DISTRIBUTED_REF_FIXTURE_SCHEMA),
        string(LOCAL_VAT_ID),
        sequence(vec![refs.far.value(), refs.replacement.value()]),
        sequence(receipts.iter().map(|receipt| receipt.value.clone()).collect()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatDistributedRefFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

pub fn run_vat_rights_fixture() -> Result<VatRightsFixture> {
    let root = VatObjectRef::new(LOCAL_VAT_ID, ROOT_OBJECT_ID, VatReferenceKind::Near, Vec::new());
    let root_ref = root.object_ref()?;
    let helper = VatObjectRef::new(LOCAL_VAT_ID, HELPER_OBJECT_ID, VatReferenceKind::Near, vec![root_ref.clone()]);
    let helper_ref = helper.object_ref()?;
    let brand_ref = canonical_hash(&record("vat-rights-brand-v1", vec![string("private-cooperator")]))?;
    let wrong_brand_ref = canonical_hash(&record("vat-rights-brand-v1", vec![string("unrelated-cooperator")]))?;
    let sealed_value = record("vat-sealed-value-v1", vec![
        record("brand-ref", vec![string(&brand_ref)]),
        record("sealed-authority-ref", vec![string(&root_ref)]),
    ]);
    let sealed_value_ref = canonical_hash(&sealed_value)?;

    let unsealed = crate::runtime::evaluate_rights_amplification(&crate::runtime::RuntimeRightsAmplificationState {
        holder_object_ref: helper_ref.clone(),
        sealed_value_ref: sealed_value_ref.clone(),
        sealer_brand_ref: brand_ref.clone(),
        unsealer_brand_ref: brand_ref.clone(),
        sealed_authority_refs: vec![root_ref.clone()],
        recovered_authority_refs: vec![root_ref.clone()],
    })?;
    let wrong_unsealer =
        crate::runtime::evaluate_rights_amplification(&crate::runtime::RuntimeRightsAmplificationState {
            holder_object_ref: helper_ref.clone(),
            sealed_value_ref: sealed_value_ref.clone(),
            sealer_brand_ref: brand_ref.clone(),
            unsealer_brand_ref: wrong_brand_ref,
            sealed_authority_refs: vec![root_ref.clone()],
            recovered_authority_refs: vec![root_ref.clone()],
        })?;
    let over_recovery_ref = canonical_hash(&string("unsealed-extra-authority"))?;
    let over_recovery =
        crate::runtime::evaluate_rights_amplification(&crate::runtime::RuntimeRightsAmplificationState {
            holder_object_ref: helper_ref,
            sealed_value_ref: sealed_value_ref.clone(),
            sealer_brand_ref: brand_ref.clone(),
            unsealer_brand_ref: brand_ref,
            sealed_authority_refs: vec![root_ref.clone()],
            recovered_authority_refs: sorted_refs(vec![root_ref, over_recovery_ref]),
        })?;

    let receipts = vec![unsealed.receipt, wrong_unsealer.receipt, over_recovery.receipt];
    let diagnostics = fixture_diagnostics(&receipts);
    let value = record("vat-rights-fixture-v1", vec![
        string(RUNTIME_VAT_RIGHTS_FIXTURE_SCHEMA),
        string(LOCAL_VAT_ID),
        sealed_value,
        sequence([root, helper].iter().map(VatObjectRef::value).collect()),
        sequence(receipts.iter().map(|receipt| receipt.value.clone()).collect()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatRightsFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

pub fn run_vat_ambient_authority_fixture() -> Result<VatAmbientAuthorityFixture> {
    let spawned = VatObjectRef::new(LOCAL_VAT_ID, SPAWNED_OBJECT_ID, VatReferenceKind::Near, Vec::new());
    let spawned_ref = spawned.object_ref()?;
    let authority_kinds = [
        crate::runtime::RuntimeObjectAuthorityKind::Filesystem,
        crate::runtime::RuntimeObjectAuthorityKind::Network,
        crate::runtime::RuntimeObjectAuthorityKind::Clock,
        crate::runtime::RuntimeObjectAuthorityKind::Process,
        crate::runtime::RuntimeObjectAuthorityKind::Dataspace,
        crate::runtime::RuntimeObjectAuthorityKind::Store,
        crate::runtime::RuntimeObjectAuthorityKind::Blob,
        crate::runtime::RuntimeObjectAuthorityKind::Consensus,
        crate::runtime::RuntimeObjectAuthorityKind::Choreography,
        crate::runtime::RuntimeObjectAuthorityKind::HostResource,
    ];
    let mut authority_refs = Vec::with_capacity(authority_kinds.len());
    let mut receipts = Vec::with_capacity(authority_kinds.len() + 1);
    for authority_kind in authority_kinds {
        let authority_ref = authority_descriptor_ref(authority_kind)?;
        let denied = crate::runtime::evaluate_object_authority(&crate::runtime::RuntimeObjectAuthorityState {
            object_ref: spawned_ref.clone(),
            requested_authority_ref: authority_ref.clone(),
            requested_authority_kind: authority_kind,
            endowed_authority_refs: Vec::new(),
            admitted_authority_refs: Vec::new(),
        })?;
        authority_refs.push(authority_ref);
        receipts.push(denied.receipt);
    }

    let clock_ref = authority_descriptor_ref(crate::runtime::RuntimeObjectAuthorityKind::Clock)?;
    let clock_pass = crate::runtime::evaluate_object_authority(&crate::runtime::RuntimeObjectAuthorityState {
        object_ref: spawned_ref,
        requested_authority_ref: clock_ref.clone(),
        requested_authority_kind: crate::runtime::RuntimeObjectAuthorityKind::Clock,
        endowed_authority_refs: vec![clock_ref.clone()],
        admitted_authority_refs: vec![clock_ref],
    })?;
    receipts.push(clock_pass.receipt);

    let diagnostics = fixture_diagnostics(&receipts);
    let value = record("vat-ambient-authority-fixture-v1", vec![
        string(RUNTIME_VAT_AMBIENT_AUTHORITY_FIXTURE_SCHEMA),
        string(LOCAL_VAT_ID),
        spawned.value(),
        sequence(authority_refs.iter().map(string).collect()),
        sequence(receipts.iter().map(|receipt| receipt.value.clone()).collect()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatAmbientAuthorityFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

pub fn run_vat_restore_fixture() -> Result<VatRestoreFixture> {
    let old_object = versioned_object_value("object:legacy", "schema:v1", "legacy-state");
    let old_object_ref = canonical_hash(&old_object)?;
    let upgraded_object = versioned_object_value("object:legacy", "schema:v2", "legacy-state");
    let upgraded_object_ref = canonical_hash(&upgraded_object)?;
    let recipe = vat_upgrade_recipe_value("schema:v1", "schema:v2", "schema-rename", &old_object_ref)?;
    let recipe_ref = canonical_hash(&recipe)?;
    let pass_receipt = vat_restore_receipt_value(VatRestoreReceiptInput {
        decision: "pass",
        snapshot_ref: &old_object_ref,
        recipe_ref: Some(&recipe_ref),
        restored_object_ref: Some(&upgraded_object_ref),
        diagnostics: Vec::new(),
    });
    let deny_diagnostics = vec!["missing-compatible-upgrade-recipe".to_string()];
    let deny_receipt = vat_restore_receipt_value(VatRestoreReceiptInput {
        decision: "deny",
        snapshot_ref: &old_object_ref,
        recipe_ref: None,
        restored_object_ref: None,
        diagnostics: deny_diagnostics.clone(),
    });
    let receipts = vec![pass_receipt, deny_receipt];
    let diagnostics = restore_diagnostics(&receipts)?;
    let value = record("vat-restore-fixture-v1", vec![
        string("molten.runtime.vat-restore-fixture.v1"),
        old_object,
        upgraded_object,
        recipe,
        sequence(receipts.clone()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatRestoreFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

pub fn run_vat_time_travel_fixture() -> Result<VatDebugFixture> {
    let root = VatObjectRef::new(LOCAL_VAT_ID, ROOT_OBJECT_ID, VatReferenceKind::Near, Vec::new());
    let helper = VatObjectRef::new(LOCAL_VAT_ID, HELPER_OBJECT_ID, VatReferenceKind::Near, vec![root.object_ref()?]);
    let root_ref = root.object_ref()?;
    let helper_ref = helper.object_ref()?;
    let turn_trace = record("vat-turn-trace-v1", vec![
        string(LOCAL_VAT_ID),
        string("turn:0001"),
        sequence([root_ref.clone(), helper_ref.clone()].iter().map(string).collect()),
        sequence(["message:deliver", "assertion:add", "turn:commit"].iter().map(string).collect()),
    ]);
    let turn_trace_ref = canonical_hash(&turn_trace)?;
    let snapshot = record("vat-debug-snapshot-v1", vec![
        string(LOCAL_VAT_ID),
        string("turn:0001"),
        record("trace-ref", vec![string(&turn_trace_ref)]),
        sequence([root_ref.clone(), helper_ref.clone()].iter().map(string).collect()),
    ]);
    let snapshot_ref = canonical_hash(&snapshot)?;
    let replay = record("vat-replay-hook-v1", vec![
        record("snapshot-ref", vec![string(&snapshot_ref)]),
        record("trace-ref", vec![string(&turn_trace_ref)]),
        string("deterministic-replay"),
    ]);
    let replay_ref = canonical_hash(&replay)?;
    let receipts = vec![
        vat_debug_receipt_value(VatDebugReceiptInput {
            kind: "vat-time-travel-debug-receipt-v1",
            schema: RUNTIME_VAT_TIME_TRAVEL_FIXTURE_SCHEMA,
            decision: "pass",
            subject_ref: &snapshot_ref,
            evidence_refs: sorted_refs(vec![turn_trace_ref.clone(), replay_ref]),
            diagnostics: vec![
                "reconstructs-prior-turn".to_string(),
                "correlates-trace-snapshot-replay".to_string(),
            ],
        }),
        vat_debug_receipt_value(VatDebugReceiptInput {
            kind: "vat-time-travel-debug-receipt-v1",
            schema: RUNTIME_VAT_TIME_TRAVEL_FIXTURE_SCHEMA,
            decision: "deny",
            subject_ref: &snapshot_ref,
            evidence_refs: vec![turn_trace_ref],
            diagnostics: vec![
                "debug-authority-missing".to_string(),
                "redacted-events-not-readable".to_string(),
            ],
        }),
    ];
    let diagnostics = debug_diagnostics(&receipts)?;
    let value = record("vat-time-travel-fixture-v1", vec![
        string(RUNTIME_VAT_TIME_TRAVEL_FIXTURE_SCHEMA),
        turn_trace,
        snapshot,
        replay,
        sequence([root, helper].iter().map(VatObjectRef::value).collect()),
        sequence(receipts.clone()),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatDebugFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}
