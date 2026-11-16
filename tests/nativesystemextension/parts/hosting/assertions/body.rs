type ActorHost = molten::node_service_hosting::Host<
    Gate,
    molten::node_service_hosting::native::Runtime<crate::support::Port, crate::support::Journal>,
>;

#[derive(Clone)]
struct Gate {
    denied: std::rc::Rc<std::cell::Cell<bool>>,
    requests: std::rc::Rc<std::cell::RefCell<Vec<molten::node_service_hosting::OperationRequest>>>,
}
impl molten::node_service_hosting::AuthorityPort for Gate {
    fn admit(&mut self, request: &molten::node_service_hosting::OperationRequest) -> molten::error::Result<String> {
        self.requests.borrow_mut().push(request.clone());
        if self.denied.get() {
            return Err(molten::error::Failure::invalid_harness("test current authority denied"));
        }
        molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("test-actor-authority-v1", vec![
            molten::preserves_rail::string(&request.operation),
            molten::preserves_rail::string(&request.subject),
            molten::preserves_rail::u64_value(request.logical_tick),
            molten::preserves_rail::string(&request.state_ref),
            molten::preserves_rail::string(&request.input_ref),
        ]))
    }
}

fn assert_sleep_admission(
    host: &mut ActorHost,
    member: &str,
    gate: &Gate,
) -> TestResult<molten::system_extension::NativeInstanceRecord> {
    let before = host.actor_instance(member, 2)?;
    let preview = host.actor_preview(molten::node_service_hosting::native::actor::Input::Sleep(member), 3)?;
    gate.denied.set(true);
    assert!(host.actor_sleep(member, 3).is_err());
    assert_eq!(gate.requests.borrow().last(), Some(&preview));
    gate.denied.set(false);
    assert_eq!(host.actor_instance(member, 3)?, before, "denied effect must not alter native state");
    let dormant = host.actor_sleep(member, 4)?;
    assert_eq!(dormant.lifecycle.phase, molten::system_extension::LifecyclePhase::Drained);
    assert!(!dormant.is_accepting_ingress);
    assert!(host.actor_checkpoint(member, 5).is_err(), "checkpoint must not accidentally wake Drained");
    let checkpoint = host.actor_checkpoint_value(member, 5)?;
    assert_eq!(Some(&checkpoint.value_ref), dormant.checkpoint_ref.as_ref());
    let state = host.actor_state(member, 5)?;
    assert_eq!(Some(&state.value_ref), dormant.state_ref.as_ref());
    Ok(dormant)
}

fn stage_durable_work(
    host: &mut ActorHost,
    member: &str,
    dormant: &molten::system_extension::NativeInstanceRecord,
    gate: &Gate,
) -> TestResult<molten::node_service_hosting::DeliveryInput> {
    let retained_ingress = ingress(dormant.lifecycle.generation.get(), &dormant.manifest_ref)?;
    let presentation = molten::system_extension::WorkPresentation {
        work_ref: retained_ingress.payload.value_ref.clone(),
        class: molten::system_extension::WorkClass::Durable,
        generation: dormant.lifecycle.generation,
        incarnation: dormant.lifecycle.incarnation,
    };
    assert!(
        host.submit(member, presentation.clone(), retained_ingress.payload.clone(), 5).is_err(),
        "ordinary submit must remain closed while the actor is Drained"
    );
    let mut runtime_local = presentation.clone();
    runtime_local.class = molten::system_extension::WorkClass::RuntimeLocal;
    assert!(host.actor_submit_dormant_preview(member, &runtime_local, &retained_ingress.payload, 5).is_err());
    let mut stale = presentation.clone();
    stale.incarnation = stale.incarnation.checked_add(1).expect("incarnation increment");
    assert!(host.actor_submit_dormant_preview(member, &stale, &retained_ingress.payload, 5).is_err());
    let mut substituted = retained_ingress.payload.clone();
    substituted.bytes.push(0);
    assert!(host.actor_submit_dormant_preview(member, &presentation, &substituted, 5).is_err());
    let staging = host.actor_submit_dormant_preview(member, &presentation, &retained_ingress.payload, 5)?;
    gate.denied.set(true);
    assert!(
        host.actor_submit_dormant(member, presentation.clone(), retained_ingress.payload.clone(), 5)
            .is_err()
    );
    assert_eq!(gate.requests.borrow().last(), Some(&staging));
    gate.denied.set(false);
    host.actor_submit_dormant(member, presentation.clone(), retained_ingress.payload.clone(), 5)?;
    assert_eq!(host.actor_instance(member, 5)?, *dormant, "staging must not execute or reopen the native actor");
    assert!(
        host.actor_submit_dormant(member, presentation.clone(), retained_ingress.payload.clone(), 5)
            .is_err(),
        "the same retained work must not be staged twice"
    );
    assert!(host.readmit(member, &presentation.work_ref, 5).is_err(), "dormancy cannot mint a delivery claim");
    let delivery = molten::node_service_hosting::DeliveryInput {
        member: member.to_string(),
        presentation: presentation.clone(),
        claim_ref: None,
        ingress: retained_ingress,
        logical_tick: 7,
    };
    Ok(delivery)
}

struct RewakeInput<'a> {
    member: &'a str,
    dormant: &'a molten::system_extension::NativeInstanceRecord,
    delivery: molten::node_service_hosting::DeliveryInput,
}

fn assert_rewake_readmission(
    host: &mut ActorHost,
    gate: &Gate,
    cohort: &Cohort,
    input: RewakeInput<'_>,
) -> TestResult<()> {
    let RewakeInput { member, dormant, mut delivery } = input;
    let presentation = &delivery.presentation;
    let retained_ingress = &delivery.ingress;
    let running = host.actor_rewake(member, 6)?;
    assert_eq!(running.lifecycle.incarnation, dormant.lifecycle.incarnation.checked_add(1).expect("incarnation increment"));
    assert_eq!(running.lifecycle.generation, dormant.lifecycle.generation);
    assert_eq!(running.lifecycle.restart_attempts, dormant.lifecycle.restart_attempts);
    assert!(
        host.actor_submit_dormant_preview(member, presentation, &retained_ingress.payload, 6).is_err(),
        "dormant staging must not loosen running submission rules"
    );
    assert!(
        molten::system_extension::admit_work_delivery(&running.lifecycle, presentation, None).is_err(),
        "old-incarnation durable work requires actual readmission after wake"
    );
    assert!(host.actor_ingress(&delivery).is_err(), "waking cannot synthesize a delivery claim");
    gate.denied.set(true);
    assert!(host.readmit(member, &presentation.work_ref, 7).is_err());
    gate.denied.set(false);
    let claim = host.readmit(member, &presentation.work_ref, 7)?;
    assert_eq!(claim.incarnation, running.lifecycle.incarnation);
    assert_eq!(claim.generation, running.lifecycle.generation);
    molten::system_extension::admit_work_delivery(&running.lifecycle, presentation, Some(&claim))
        .or_fail("actual current Host claim admits old-incarnation durable work")?;
    delivery.claim_ref = Some(cohort.admitted.manifest_ref().to_string());
    assert!(host.actor_ingress(&delivery).is_err(), "delivery must supply the actual retained current claim");
    let already_running = host.actor_rewake(member, 7)?;
    assert_eq!(already_running, running, "StartRuntime after Restore must not issue another activation callback");
    assert_eq!(gate.requests.borrow().last().map(|request| request.operation.as_str()), Some("actor-rewake"));
    let selected = host.actor_checkpoint(member, 8)?;
    let drained = host.actor_drain(member, 9)?;
    assert_eq!(drained.checkpoint_ref, selected.checkpoint_ref);
    assert_eq!(drained.callback_sequence, selected.callback_sequence + 1, "drain must not repeat checkpoint");
    assert_eq!(drained.lifecycle.phase, molten::system_extension::LifecyclePhase::Drained);
    assert!(!drained.is_accepting_ingress);
    Ok(())
}
