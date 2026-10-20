fn exercise_mutated_plan_denial(case: &'static str) {
    let (request, memory) = fixture();
    let before_objects = memory.borrow().objects.clone();
    let before_heads = memory.borrow().heads.clone();
    let mut adapter = MigrationAdapter {
        case: "positive",
        calls: 0,
    };
    let mut plan = prepare_world_merge_plan(
        &mut MemoryPort(memory.clone()),
        &mut adapter,
        &mut MemoryPort(memory.clone()),
        &request,
    )
    .expect("valid conversion plan");
    match case {
        "output" => plan.outputs[0].generated_bytes = Some(b"substituted after planning".to_vec()),
        "evidence" => plan.migrations.clear(),
        "head" => plan.source_heads[0] = head("substituted-head"),
        _ => panic!("unknown plan mutation"),
    }
    assert!(validate_world_merge_plan(&plan).is_err(), "mutated {case} plan validated");
    let result = publish_world_merge(
        &mut MemoryPort(memory.clone()),
        &mut MemoryPort(memory.clone()),
        &mut MemoryPort(memory.clone()),
        &mut MemoryPort(memory.clone()),
        &WorldMergePublicationRequest {
            plan: &plan,
            policy_ref: &request.profile.policy_ref,
        },
    );
    assert!(result.is_err(), "mutated {case} plan published");
    let memory = memory.borrow();
    assert!(memory.events.is_empty(), "mutated {case} plan reached a publication port");
    assert_eq!(memory.objects, before_objects);
    assert_eq!(memory.heads, before_heads);
    assert!(memory.commits.is_empty());
}

#[test]
fn mutated_conversion_plan_denies_before_publication_ports() {
    for case in ["output", "evidence", "head"] {
        exercise_mutated_plan_denial(case);
    }
}

