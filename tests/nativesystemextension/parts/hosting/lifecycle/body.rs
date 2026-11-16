#[test]
fn native_actor_host_sleep_rewake_requires_exact_authority_and_does_not_restart_twice() -> TestResult<()> {
    let workspace = crate::test_support::process_workspace("native-actor-host-lifecycle")?;
    let root = molten::node_state::DirectoryRoot::open_existing(workspace.as_ref())?;
    root.create_layout()?;
    let cohort = Cohort::new()?;
    let member = cohort.admitted.manifest().service_id.clone();
    let runtime = molten::node_service_hosting::native::Runtime::new(std::collections::BTreeMap::from([(
        member.clone(),
        cohort.install()?,
    )]))?;
    let denied = std::rc::Rc::new(std::cell::Cell::new(false));
    let requests = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let gate = Gate { denied: denied.clone(), requests: requests.clone() };
    let mut host = molten::node_service_hosting::Host::open(
        &root,
        gate.clone(),
        runtime,
    )?;
    host.install(
        molten::system_extension::RecoveryGroupDeclaration {
            group_id: "actor-lifecycle".to_string(),
            members: vec![molten::system_extension::RecoveryGroupMember {
                member_id: member.clone(),
                restart_class: molten::system_extension::RestartClass::Permanent,
                after: Vec::new(),
            }],
            window_ticks: 100,
            backoff_ticks: 1,
            max_restarts_per_member: 3,
            max_restarts_per_group: 3,
            elapsed_source_ref: cohort.admitted.manifest_ref().to_string(),
        },
        1,
    )?;
    let dormant = assert_sleep_admission(&mut host, &member, &gate)?;
    let delivery = stage_durable_work(&mut host, &member, &dormant, &gate)?;
    assert_rewake_readmission(&mut host, &gate, &cohort, RewakeInput { member: &member, dormant: &dormant, delivery })
}

#[test]
fn failed_native_actor_recovery_materialization_keeps_fresh_incarnation_closed() -> TestResult<()> {
    let mut cohort = Cohort::new()?;
    let mut service = cohort.install()?;
    service.start(1).or_fail("start actor before storage loss")?;
    service.checkpoint(2).or_fail("checkpoint actor before storage loss")?;
    service.drain(3).or_fail("drain actor before storage loss")?;
    service.observe_process_loss().or_fail("record actual process loss")?;
    let before = service.instance().or_fail("persisted pre-recovery actor")?;
    drop(service);
    cohort.values = molten::system_extension::shared_callback_value_port(
        molten::system_extension::InMemoryCallbackValuePort::empty(),
    );
    let mut recovered = cohort.recovered(before.clone())?;
    assert!(recovered.recover(4).is_err());
    let after = recovered.instance().or_fail("failed recovery durable actor")?;
    assert_eq!(after.lifecycle.incarnation, before.lifecycle.incarnation.checked_add(1).expect("incarnation increment"));
    assert_eq!(after.lifecycle.generation, before.lifecycle.generation);
    assert!(!after.is_accepting_ingress, "failed recovery must not reopen ingress");
    assert_ne!(after.lifecycle.phase, molten::system_extension::LifecyclePhase::Running);
    assert_eq!(
        cohort
            .journal
            .lock()
            .or_fail("failed recovery journal")?
            .latest_instance(&before.instance_id)
            .or_fail("failed recovery journal observation")?,
        Some(after)
    );
    Ok(())
}

#[test]
fn successful_native_ingress_persists_exact_callback_receipt_before_terminal_ingress() -> TestResult<()> {
    let cohort = Cohort::new()?;
    let mut service = cohort.install()?;
    service.start(1).or_fail("start actual native service")?;
    let current = service.instance().or_fail("current native instance")?;
    let request = ingress(current.lifecycle.generation.get(), &current.manifest_ref)?;
    let result = service.ingress(&request, 2).or_fail("actual native ingress")?;
    let molten::system_extension::DispatchResult::Executed { receipt, .. } = result.dispatch else {
        return Err("native request callback did not execute".into());
    };
    let persisted = cohort
        .values
        .lock()
        .or_fail("native callback value port")?
        .materialize(&receipt.receipt_ref, MAX_VALUE_BYTES)
        .or_fail("persisted exact callback receipt")?;
    assert_eq!(persisted.value_ref, receipt.receipt_ref);
    assert_eq!(persisted.bytes, molten::preserves_rail::canonical_bytes(&receipt.value)?);
    let history = cohort
        .journal
        .lock()
        .or_fail("native receipt journal")?
        .history(&current.instance_id)
        .or_fail("native receipt publication history")?;
    let publication_ref = molten::system_extension::native_identity_ref(&[
        "native-value-publication-operation-v2",
        &request.request_ref,
        "ingress-callback-receipt",
        &receipt.receipt_ref,
        &request.generation.to_string(),
    ]);
    let publication = history
        .iter()
        .position(|record| {
            record.completed_operations.iter().any(|operation| operation.operation_ref == publication_ref)
        })
        .or_fail("receipt publication is durably completed")?;
    let terminal = history
        .iter()
        .position(|record| {
            record.completed_operations.iter().any(|operation| {
                operation.operation_ref == request.request_ref
                    && operation.terminal_ref.as_deref() == Some(receipt.receipt_ref.as_str())
            })
        })
        .or_fail("durable native ingress terminal receipt")?;
    assert!(publication < terminal, "receipt must be published before ingress completion is durable");
    Ok(())
}
