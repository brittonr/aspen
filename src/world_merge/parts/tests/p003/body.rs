fn exercise_positive() {
    // r[verify molten.audit_f06.binding]
    // r[verify molten.audit_f06.publication]
    // r[verify molten.audit_f06.validation]
    let (request, memory) = fixture();
    let original_source = request.roots[0].left.clone();
    let mut adapter = MigrationAdapter {
        case: "positive",
        calls: 0,
    };
    let plan = prepare_world_merge_plan(
        &mut MemoryPort(memory.clone()),
        &mut adapter,
        &mut MemoryPort(memory.clone()),
        &request,
    )
    .expect("mixed schema preparation");
    assert_eq!(adapter.calls, 1, "target-schema values must not be migrated");
    assert_eq!(plan.migrations.len(), 1);
    let WorldMergeOutput::Generated(generated) = &plan.outputs[0].output else {
        panic!("converted bytes cannot retain old root identity");
    };
    assert_eq!(generated.bytes.as_deref(), Some(b"modern-left".as_slice()));
    assert_eq!(request.roots[0].left, original_source, "original request mutated");
    assert_conversion_evidence(&plan, &request);
    let result = publish_world_merge(
        &mut MemoryPort(memory.clone()),
        &mut MemoryPort(memory.clone()),
        &mut MemoryPort(memory.clone()),
        &mut MemoryPort(memory.clone()),
        &WorldMergePublicationRequest {
            plan: &plan,
            policy_ref: &request.profile.policy_ref,
        },
    )
    .expect("publish converted root");
    let memory = memory.borrow();
    assert_eq!(memory.events, vec!["authority", "persist", "commit"]);
    let mut expected_sources = request.source_heads.clone();
    expected_sources.sort();
    let expected_parents = std::iter::once(request.base_head.clone()).chain(expected_sources).collect::<Vec<_>>();
    assert_eq!(memory.parents, expected_parents);
    assert_eq!(memory.heads, vec![result.result_commit.clone().expect("published commit")]);
    assert_eq!(memory.commits[&memory.heads[0]], result.output_roots);
    let output = &result.output_roots[0];
    assert_ne!(Some(output), original_source.root.as_ref());
    let persisted = &memory.objects[output];
    assert_eq!(output.as_str(), identity(persisted));
    let decoded = codec::parse_canonical_bytes(persisted).expect("persisted generated root");
    let fields = codec::simple_record_fields(&decoded, "world-merge-generated-root", 5).expect("root payload");
    assert_eq!(
        codec::record_string_field(&fields[2], "schema-ref", "schema").expect("target schema"),
        schema_ref("v2").as_str()
    );
    let expected = codec::record("handler-bytes", vec![codec::record("some", vec![codec::sequence(
        b"modern-left".iter().map(|byte| codec::u64_value(u64::from(*byte))).collect(),
    )])]);
    assert_eq!(
        codec::value_to_iovalue(&fields[4]),
        expected,
        "published payload must be converted content, not original bytes"
    );
}

fn assert_conversion_evidence(plan: &WorldMergePlan, request: &WorldMergeRequest) {
    let original_source = &request.roots[0].left;
    let evidence = &plan.migrations[0];
    let conversion = evidence.conversion();
    assert_eq!(conversion.side(), WorldMergeInputSide::Left);
    assert_eq!(conversion.original_root(), original_source.root.as_ref().expect("source root"));
    assert_eq!(conversion.binding(), &request.profile.migrations[&RootKind::DurableState]);
    assert_eq!(conversion.source_ref(), identity(b"legacy-left"));
    assert_eq!(conversion.max_output_bytes(), request.bounds.max_value_bytes);
    let canonical = canonical_world_merge_plan(plan).expect("canonical plan evidence");
    let decoded = codec::parse_canonical_bytes(&canonical.bytes).expect("decoded plan");
    let fields = codec::simple_record_fields(&decoded, "world-merge-plan", 8).expect("plan fields");
    let migrations = codec::value_to_iovalue(&fields[4]);
    let migration_field = codec::simple_record_fields(&migrations, "migrations", 1).expect("migrations field");
    let sequence = codec::required_sequence_field(&migration_field[0], "migrations").expect("evidence sequence");
    let first = codec::value_to_iovalue(&sequence[0]);
    let record = codec::simple_record_fields(&first, "merge-migration", 11).expect("migration record");
    for (index, name, expected) in [
        (0, "conversion-ref", conversion.conversion_ref()),
        (2, "side", "left"),
        (3, "original-root", original_source.root.as_ref().expect("source").as_str()),
        (4, "source-schema", original_source.schema_ref.as_ref().expect("source schema").as_str()),
        (5, "source-ref", conversion.source_ref()),
        (6, "target-schema", request.roots[0].base.schema_ref.as_ref().expect("target schema").as_str()),
        (10, "result-ref", evidence.result_ref()),
    ] {
        assert_eq!(codec::record_string_field(&record[index], name, name).expect("evidence field"), expected);
    }
}

fn exercise_absent_positive() {
    let (mut request, memory) = fixture();
    let absent = WorldMergeValue {
        root: None,
        schema_ref: None,
        available: true,
        canonical_bytes: None,
        keyed_values: BTreeMap::new(),
    };
    request.roots[0].base = absent.clone();
    request.roots[0].right = absent;
    let mut adapter = MigrationAdapter {
        case: "positive",
        calls: 0,
    };
    let plan = prepare_world_merge_plan(
        &mut MemoryPort(memory.clone()),
        &mut adapter,
        &mut MemoryPort(memory.clone()),
        &request,
    )
    .expect("absent sides do not need migration");
    assert_eq!(adapter.calls, 1, "absent sides must not invoke the adapter");
    assert_eq!(plan.migrations.len(), 1);
    assert_eq!(plan.migrations[0].conversion().side(), WorldMergeInputSide::Left);
    let WorldMergeOutput::Generated(generated) = &plan.outputs[0].output else {
        panic!("converted present side is generated data");
    };
    assert_eq!(generated.bytes.as_deref(), Some(b"modern-left".as_slice()));
    let result = publish_world_merge(
        &mut MemoryPort(memory.clone()),
        &mut MemoryPort(memory.clone()),
        &mut MemoryPort(memory.clone()),
        &mut MemoryPort(memory.clone()),
        &WorldMergePublicationRequest {
            plan: &plan,
            policy_ref: &request.profile.policy_ref,
        },
    )
    .expect("publish converted present side");
    let memory = memory.borrow();
    assert_eq!(memory.events, vec!["authority", "persist", "commit"]);
    assert_ne!(Some(&result.output_roots[0]), request.roots[0].left.root.as_ref());
    let payload = codec::parse_canonical_bytes(&memory.objects[&result.output_roots[0]]).expect("persisted payload");
    let fields = codec::simple_record_fields(&payload, "world-merge-generated-root", 5).expect("payload fields");
    let expected = codec::record("handler-bytes", vec![codec::record("some", vec![codec::sequence(
        b"modern-left".iter().map(|byte| codec::u64_value(u64::from(*byte))).collect(),
    )])]);
    assert_eq!(codec::value_to_iovalue(&fields[4]), expected);
}

fn exercise_original_denial(case: &'static str) {
    // r[verify molten.audit_f06.admission]
    // r[verify molten.audit_f06.validation]
    let (mut request, memory) = fixture();
    match case {
        "unadmitted" => request.profile.migrations.get_mut(&RootKind::DurableState).expect("binding").admitted = false,
        "missing" => {
            request.profile.migrations.clear();
        }
        "malformed" => request.profile.migrations.get_mut(&RootKind::DurableState).expect("binding").profile_id.clear(),
        "wrong-source" => request.roots[0].left.schema_ref = Some(schema_ref("unbound-schema")),
        "empty-source" => request.roots[0].left.canonical_bytes = Some(Vec::new()),
        "unavailable" => request.roots[0].left.available = false,
        "unavailable-target" => request.roots[0].right.available = false,
        "oversized-target" => {
            request.roots[0].right.canonical_bytes = Some(vec![0; 100]);
            request.bounds.max_value_bytes = 32;
        }
        "missing-object" => {
            memory.borrow_mut().objects.remove(request.roots[0].left.root.as_ref().expect("source"));
        }
        "oversized-original" => request.bounds.max_value_bytes = 3,
        "oversized-inline" => {
            request.roots[0].left.canonical_bytes = Some(vec![0; 100]);
            request.bounds.max_value_bytes = 32;
        }
        _ => panic!("unknown original denial"),
    }
    let application = WorldApplicationHandlerProfile {
        handler_ref: WorldMergeHandlerRef::new(identity(b"app-handler")).expect("handler"),
        behavior_ref: WorldMergeHandlerRef::new(identity(b"app-behavior")).expect("behavior"),
        input_schema: schema_ref("app"),
        output_schema: schema_ref("app"),
        policy_ref: request.profile.policy_ref.clone(),
        max_output_bytes: request.bounds.max_value_bytes,
        pure: true,
    };
    request.profile.root_modes.insert(RootKind::Schema, WorldMergeMode::ApplicationHandler);
    request.profile.handlers.insert(RootKind::Schema, application);
    let app_value = WorldMergeValue {
        root: Some(WorldRootRef::parse(RootKind::Schema, identity(b"a")).expect("app root")),
        schema_ref: Some(schema_ref("app")),
        available: true,
        canonical_bytes: Some(b"a".to_vec()),
        keyed_values: BTreeMap::new(),
    };
    request.roots.push(WorldMergeRootInput {
        kind: RootKind::Schema,
        base: app_value.clone(),
        left: app_value.clone(),
        right: app_value,
    });
    let before_objects = memory.borrow().objects.clone();
    let before_heads = memory.borrow().heads.clone();
    let mut adapter = MigrationAdapter {
        case: "positive",
        calls: 0,
    };
    assert!(!prepare_and_publish(&request, &memory, &mut adapter), "accepted {case}");
    assert_eq!(adapter.calls, 0, "{case} reached migration adapter");
    let memory = memory.borrow();
    assert_eq!(memory.handler_calls, 0, "{case} reached application handler");
    assert!(memory.events.is_empty(), "{case} published artifacts or rechecked authority");
    assert_eq!(memory.objects, before_objects, "{case} persisted output");
    assert_eq!(memory.heads, before_heads, "{case} changed heads");
    assert!(memory.commits.is_empty());
}

fn exercise_result_denial(case: &'static str) {
    // r[verify molten.audit_f06.binding]
    // r[verify molten.audit_f06.publication]
    let (request, memory) = fixture();
    let before_objects = memory.borrow().objects.clone();
    let before_heads = memory.borrow().heads.clone();
    let mut adapter = MigrationAdapter { case, calls: 0 };
    assert!(!prepare_and_publish(&request, &memory, &mut adapter), "accepted {case}");
    assert_eq!(adapter.calls, 1);
    let memory = memory.borrow();
    assert_eq!(memory.handler_calls, 0);
    assert!(memory.events.is_empty(), "{case} reached publication");
    assert_eq!(memory.objects, before_objects);
    assert_eq!(memory.heads, before_heads);
    assert!(memory.commits.is_empty());
}

#[test]
fn mixed_schema_migration_publishes_new_payload_and_preserves_causal_parents() {
    exercise_positive();
}

#[test]
fn absent_input_sides_do_not_block_conversion_of_present_source() {
    exercise_absent_positive();
}

#[test]
fn original_denials_precede_adapters_handlers_and_publication() {
    for case in [
        "unadmitted",
        "missing",
        "malformed",
        "wrong-source",
        "empty-source",
        "unavailable",
        "unavailable-target",
        "missing-object",
        "oversized-original",
        "oversized-inline",
        "oversized-target",
    ] {
        exercise_original_denial(case);
    }
}

#[test]
fn invalid_or_failed_conversion_never_publishes_or_moves_heads() {
    for case in [
        "empty",
        "oversized",
        "wrong-target",
        "substituted-content",
        "stale-conversion",
        "keyed-substitution",
        "adapter-failure",
    ] {
        exercise_result_denial(case);
    }
}
