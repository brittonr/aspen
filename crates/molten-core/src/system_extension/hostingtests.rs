#[test]
fn drained_members_can_checkpoint_before_generation_replacement() {
    let running = super::LifecycleState {
        generation: 4,
        incarnation: 9,
        phase: super::LifecyclePhase::Running,
        restart_attempts: 1,
        health: super::HealthState::Healthy,
        checkpoint_ref: None,
    };
    let usage = super::ResourceUsage::default();
    let drain = super::plan_lifecycle_transition(
        &running,
        &super::LifecycleEvent::simple(super::LifecycleEventKind::BeginDrain, running.generation, running.incarnation),
        usage,
        4,
    )
    .expect("running member begins draining");
    let drained = super::plan_lifecycle_transition(
        &drain,
        &super::LifecycleEvent::simple(super::LifecycleEventKind::DrainSucceeded, drain.generation, drain.incarnation),
        usage,
        4,
    )
    .expect("idle member drains");
    let checkpointing = super::plan_lifecycle_transition(
        &drained,
        &super::LifecycleEvent::simple(
            super::LifecycleEventKind::BeginCheckpoint,
            drained.generation,
            drained.incarnation,
        ),
        usage,
        4,
    )
    .expect("drained member must be checkpointable without accepting more work");
    let checkpoint_ref = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
    let completed = super::plan_lifecycle_transition(
        &checkpointing,
        &super::LifecycleEvent {
            kind: super::LifecycleEventKind::CheckpointSucceeded,
            generation: checkpointing.generation,
            incarnation: checkpointing.incarnation,
            next_generation: None,
            checkpoint_ref: Some(checkpoint_ref.clone()),
            failure_class: None,
        },
        usage,
        4,
    )
    .expect("checkpoint records its immutable state reference");
    assert_eq!(completed.checkpoint_ref.as_deref(), Some(checkpoint_ref.as_str()));
    let old_work = super::WorkPresentation {
        work_ref: checkpoint_ref,
        class: super::WorkClass::RuntimeLocal,
        generation: running.generation,
        incarnation: running.incarnation,
    };
    let upgraded = super::plan_lifecycle_transition(
        &completed,
        &super::LifecycleEvent {
            kind: super::LifecycleEventKind::BeginUpgrade,
            generation: completed.generation,
            incarnation: completed.incarnation,
            next_generation: Some(completed.generation + 1),
            checkpoint_ref: None,
            failure_class: None,
        },
        usage,
        4,
    )
    .expect("checkpoint permits admitted generation replacement");
    let issues = super::admit_work_delivery(&upgraded, &old_work, None).expect_err("pre-upgrade work is stale");
    assert!(issues.iter().any(|issue| matches!(issue, super::ReadmissionIssue::StaleGeneration { .. })));
}
