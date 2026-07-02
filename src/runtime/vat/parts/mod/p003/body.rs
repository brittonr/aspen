
pub fn run_vat_replay_fixture() -> Result<VatReplayFixture> {
    let expected = case_run(RunCase::Baseline)?;
    let actual = case_run(RunCase::Baseline)?;
    let changed_input = case_run(RunCase::Input)?;
    let changed_effect = case_run(RunCase::Effect)?;
    let changed_sequence = case_run(RunCase::Sequence)?;
    let changed_policy = case_run(RunCase::Policy)?;
    let changed_state = case_run(RunCase::State)?;
    let generic_pass =
        crate::deterministic_replay::verify_fixture_value(crate::deterministic_replay::ReplayFixtureVariant::Baseline)?;
    let generic_deny = crate::deterministic_replay::verify_fixture_value(
        crate::deterministic_replay::ReplayFixtureVariant::ChangedEffectResponse,
    )?;
    let generic_first_divergence = generic_deny
        .first_divergence
        .clone()
        .unwrap_or_else(|| record("deterministic-first-divergence-v1", Vec::new()));
    let receipts = vec![
        vat_replay_receipt_value(&expected, &actual)?,
        vat_replay_receipt_value(&expected, &changed_input)?,
        vat_replay_receipt_value(&expected, &changed_effect)?,
        vat_replay_receipt_value(&expected, &changed_sequence)?,
        vat_replay_receipt_value(&expected, &changed_policy)?,
        vat_replay_receipt_value(&expected, &changed_state)?,
    ];
    let generic_receipts = vec![generic_pass.value, generic_deny.value, generic_first_divergence];
    let mut diagnostic_receipts = receipts.clone();
    diagnostic_receipts.extend(generic_receipts.clone());
    let diagnostics = debug_diagnostics(&diagnostic_receipts)?;
    let value = record("vat-replay-fixture-v1", vec![
        string(RUNTIME_VAT_REPLAY_FIXTURE_SCHEMA),
        record("profile", vec![string("replay")]),
        record("policy", vec![string("no-real-external-effects")]),
        record("expected-run-ref", vec![string(&expected.run_ref)]),
        sequence(
            [
                actual.value,
                changed_input.value,
                changed_effect.value,
                changed_sequence.value,
                changed_policy.value,
                changed_state.value,
            ]
            .to_vec(),
        ),
        sequence(receipts.clone()),
        sequence(generic_receipts),
        sequence(diagnostics.iter().map(string).collect()),
    ]);
    let fixture_ref = canonical_hash(&value)?;
    Ok(VatReplayFixture {
        value,
        fixture_ref,
        receipts,
        diagnostics,
    })
}

pub fn run_vat_authority_graph_fixture() -> Result<VatDebugFixture> {
    let root = VatObjectRef::new(LOCAL_VAT_ID, ROOT_OBJECT_ID, VatReferenceKind::Near, Vec::new());
    let helper = VatObjectRef::new(LOCAL_VAT_ID, HELPER_OBJECT_ID, VatReferenceKind::Near, vec![root.object_ref()?]);
    let proxy = VatObjectRef::new(LOCAL_VAT_ID, PROXY_OBJECT_ID, VatReferenceKind::Proxy, vec![helper.object_ref()?]);
    let root_ref = root.object_ref()?;
    let helper_ref = helper.object_ref()?;
    let proxy_ref = proxy.object_ref()?;
    let graph = record("vat-authority-graph-v1", vec![
        string(LOCAL_VAT_ID),
        sequence([root_ref.clone(), helper_ref.clone(), proxy_ref.clone()].iter().map(string).collect()),
        sequence(
            [
                authority_edge_value(&helper_ref, &root_ref, "holds"),
                authority_edge_value(&proxy_ref, &helper_ref, "attenuates"),
            ]
            .to_vec(),
        ),
        sequence([proxy_ref.clone()].iter().map(string).collect()),
    ]);
    let graph_ref = canonical_hash(&graph)?;
    let receipts = vec![
        vat_debug_receipt_value(VatDebugReceiptInput {
            kind: "vat-authority-graph-inspect-receipt-v1",
            schema: RUNTIME_VAT_AUTHORITY_GRAPH_FIXTURE_SCHEMA,
            decision: "pass",
            subject_ref: &graph_ref,
            evidence_refs: sorted_refs(vec![root_ref.clone(), helper_ref.clone(), proxy_ref.clone()]),
            diagnostics: vec![
                "authority-graph-readable".to_string(),
                "proxy-chain-visible".to_string(),
            ],
        }),
        vat_debug_receipt_value(VatDebugReceiptInput {
            kind: "vat-authority-graph-inspect-receipt-v1",
            schema: RUNTIME_VAT_AUTHORITY_GRAPH_FIXTURE_SCHEMA,
            decision: "deny",
            subject_ref: &graph_ref,
            evidence_refs: vec![proxy_ref],
            diagnostics: vec![
                "inspection-authority-missing".to_string(),
                "redacted-edge-not-disclosed".to_string(),
            ],
        }),
    ];
    let diagnostics = debug_diagnostics(&receipts)?;
    let value = record("vat-authority-graph-fixture-v1", vec![
        string(RUNTIME_VAT_AUTHORITY_GRAPH_FIXTURE_SCHEMA),
        graph,
        sequence([root, helper, proxy].iter().map(VatObjectRef::value).collect()),
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

pub fn run_vat_portable_storage_fixture() -> Result<VatDebugFixture> {
    let chunk_a = canonical_hash(&record("encrypted-chunk-v1", vec![string("ciphertext:a")]))?;
    let chunk_b = canonical_hash(&record("encrypted-chunk-v1", vec![string("ciphertext:b")]))?;
    let read_cap = canonical_hash(&record("storage-read-capability-v1", vec![string("snapshot-reader")]))?;
    let write_cap = canonical_hash(&record("storage-write-capability-v1", vec![string("snapshot-writer")]))?;
    let manifest = record("vat-portable-storage-manifest-v1", vec![
        string("fixed_v1"),
        string("encrypted-before-storage"),
        string("provider-independent"),
        sequence([chunk_a.clone(), chunk_b.clone()].iter().map(string).collect()),
        sequence([read_cap.clone(), write_cap.clone()].iter().map(string).collect()),
    ]);
    let manifest_ref = canonical_hash(&manifest)?;
    let receipts = vec![
        vat_debug_receipt_value(VatDebugReceiptInput {
            kind: "vat-portable-storage-receipt-v1",
            schema: RUNTIME_VAT_PORTABLE_STORAGE_FIXTURE_SCHEMA,
            decision: "pass",
            subject_ref: &manifest_ref,
            evidence_refs: sorted_refs(vec![chunk_a.clone(), chunk_b.clone(), read_cap.clone(), write_cap]),
            diagnostics: vec![
                "content-addressed-chunked-encrypted".to_string(),
                "provider-independent".to_string(),
            ],
        }),
        vat_debug_receipt_value(VatDebugReceiptInput {
            kind: "vat-portable-storage-receipt-v1",
            schema: RUNTIME_VAT_PORTABLE_STORAGE_FIXTURE_SCHEMA,
            decision: "deny",
            subject_ref: &manifest_ref,
            evidence_refs: vec![chunk_a, read_cap],
            diagnostics: vec![
                "plaintext-storage-denied".to_string(),
                "provider-bound-location-denied".to_string(),
            ],
        }),
    ];
    let diagnostics = debug_diagnostics(&receipts)?;
    let value = record("vat-portable-storage-fixture-v1", vec![
        string(RUNTIME_VAT_PORTABLE_STORAGE_FIXTURE_SCHEMA),
        manifest,
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

pub fn vat_fixture_summary(value: &IoValue) -> Result<String> {
    let artifact_ref = canonical_hash(value)?;
    Ok(format!("vat artifact: {artifact_ref}"))
}

struct VatRestoreReceiptInput<'a> {
    decision: &'a str,
    snapshot_ref: &'a str,
    recipe_ref: Option<&'a str>,
    restored_object_ref: Option<&'a str>,
    diagnostics: Vec<String>,
}

struct VatDebugReceiptInput<'a> {
    kind: &'static str,
    schema: &'static str,
    decision: &'a str,
    subject_ref: &'a str,
    evidence_refs: Vec<String>,
    diagnostics: Vec<String>,
}

fn vat_debug_receipt_value(input: VatDebugReceiptInput<'_>) -> IoValue {
    record(input.kind, vec![
        string(input.schema),
        string(input.decision),
        record("subject-ref", vec![string(input.subject_ref)]),
        sequence(input.evidence_refs.iter().map(string).collect()),
        sequence(input.diagnostics.iter().map(string).collect()),
    ])
}

struct RunInput {
    seed: &'static str,
    input_message: &'static str,
    effect_response: &'static str,
    random_sequence: &'static str,
    random_response: &'static str,
    policy_decision: &'static str,
    state_marker: &'static str,
}

enum RunCase {
    Baseline,
    Input,
    Effect,
    Sequence,
    Policy,
    State,
}

fn case_run(case: RunCase) -> Result<VatReplayRun> {
    let mut input = RunInput {
        seed: "seed:vat-replay:0001",
        input_message: "deliver:root-to-helper",
        effect_response: "clock:logical:42",
        random_sequence: "random-seq:0001",
        random_response: "random:seeded:7",
        policy_decision: "policy:allow",
        state_marker: "state:committed",
    };
    match case {
        RunCase::Baseline => {}
        RunCase::Input => input.input_message = "deliver:root-to-helper:changed",
        RunCase::Effect => input.effect_response = "clock:logical:43",
        RunCase::Sequence => input.random_sequence = "random-seq:changed",
        RunCase::Policy => input.policy_decision = "policy:deny",
        RunCase::State => input.state_marker = "state:diverged",
    }
    vat_replay_run(input)
}

struct Objects {
    root_ref: String,
    helper_ref: String,
}

struct Inputs {
    initial_state_ref: String,
    input_ref: String,
}

struct Effects {
    effect_request_ref: String,
    effect_response_ref: String,
    random_request_ref: String,
    random_response_ref: String,
}

struct Tail {
    policy_decision_ref: String,
    final_state_hash: String,
    trace_ref: String,
}

fn objects() -> Result<Objects> {
    let root = VatObjectRef::new(LOCAL_VAT_ID, ROOT_OBJECT_ID, VatReferenceKind::Near, Vec::new());
    let root_ref = root.object_ref()?;
    let helper = VatObjectRef::new(LOCAL_VAT_ID, HELPER_OBJECT_ID, VatReferenceKind::Near, vec![root_ref.clone()]);
    let helper_ref = helper.object_ref()?;
    Ok(Objects { root_ref, helper_ref })
}

fn inputs(run_input: &RunInput, objects: &Objects) -> Result<Inputs> {
    let initial_state = record("vat-replay-initial-state-v1", vec![
        string(run_input.seed),
        sequence([objects.root_ref.clone(), objects.helper_ref.clone()].iter().map(string).collect()),
    ]);
    let initial_state_ref = canonical_hash(&initial_state)?;
    let input = record("vat-replay-input-v1", vec![
        string(run_input.input_message),
        record("sender-ref", vec![string(&objects.root_ref)]),
        record("target-ref", vec![string(&objects.helper_ref)]),
    ]);
    let input_ref = canonical_hash(&input)?;
    Ok(Inputs {
        initial_state_ref,
        input_ref,
    })
}
