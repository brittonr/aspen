
/// Advances the live supervisor generation while a callback's inputs are materialized: after the callback intent
/// is committed and before execution is admitted, the window only independent execution admission can observe.
#[derive(Clone, Default)]
struct FencingValuePort {
    inner: std::sync::Arc<std::sync::Mutex<InMemoryCallbackValuePort>>,
    fence: std::sync::Arc<std::sync::Mutex<Option<std::sync::Arc<std::sync::Mutex<NativeInstanceRecord>>>>>,
}

impl NativeCallbackValuePort for FencingValuePort {
    fn materialize(
        &mut self,
        value_ref: &str,
        maximum_bytes: u64,
    ) -> std::result::Result<NativeCallbackValue, ValuePortFailure> {
        let unavailable =
            || ValuePortFailure::new(ValuePortFailureKind::RejectedBeforeAcceptance, "fencing value port is unavailable");
        if let Some(instance) = self.fence.lock().map_err(|_| unavailable())?.take() {
            let mut instance = instance.lock().map_err(|_| unavailable())?;
            let next = instance.lifecycle.generation.get().checked_add(1).ok_or_else(unavailable)?;
            instance.lifecycle.generation = molten::system_extension::Generation::new(next);
        }
        self.inner.lock().map_err(|_| unavailable())?.materialize(value_ref, maximum_bytes)
    }

    fn publish(
        &mut self,
        value: &NativeCallbackValue,
        maximum_bytes: u64,
    ) -> std::result::Result<ValuePublicationReceipt, ValuePortFailure> {
        self.inner
            .lock()
            .map_err(|_| {
                ValuePortFailure::new(ValuePortFailureKind::RejectedBeforeAcceptance, "fencing value port is unavailable")
            })?
            .publish(value, maximum_bytes)
    }
}

/// A journal that refuses every save once armed, standing in for durable storage failing after an effect.
struct FailingJournal {
    inner: InMemory,
    is_failing: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl NativeHostJournal for FailingJournal {
    fn save_instance(
        &mut self,
        record: &NativeInstanceRecord,
    ) -> std::result::Result<CanonicalInstanceRecord, Error> {
        if self.is_failing.load(std::sync::atomic::Ordering::Acquire) {
            return Err(Error::Storage("fixture journal refuses saves after the effect".to_string()));
        }
        self.inner.save_instance(record)
    }

    fn latest_instance(&self, instance_id: &str) -> std::result::Result<Option<NativeInstanceRecord>, Error> {
        self.inner.latest_instance(instance_id)
    }

    fn history(&self, instance_id: &str) -> std::result::Result<Vec<NativeInstanceRecord>, Error> {
        self.inner.history(instance_id)
    }
}

/// Performs the fixture effect, then breaks the journal so the routed outcome cannot be recorded.
struct JournalBreakingEffectPort {
    inner: EffectPort,
    is_failing: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl FabricEffectPort for JournalBreakingEffectPort {
    fn route(
        &mut self,
        binding: &molten::fabric::CanonicalPortBinding,
        effect: &TypedEffectRequest,
    ) -> molten::fabric::FabricPortResult<PortEffectOutput> {
        let output = self.inner.route(binding, effect)?;
        self.is_failing.store(true, std::sync::atomic::Ordering::Release);
        Ok(output)
    }
}

/// The callback admission detail, whether the dispatcher returned it or recorded it in a failed callback receipt.
fn callback_denial(result: std::result::Result<IngressResult, Failure>) -> String {
    match result {
        Ok(accepted) => match accepted.dispatch.require_executed("fenced ingress") {
            Ok(_) => "callback executed".to_string(),
            Err(error) => error.to_string(),
        },
        Err(error) => format!("{error:?}"),
    }
}

/// One authority binding field and the edit that makes it differ from the admitted executable evidence.
type AuthorityMutation = (&'static str, fn(&mut molten::fabric_execution::ExecutionAuthorityFacts));

// r[verify molten.system_extension.independent_authority_and_output_gate]
#[test]
fn native_execution_admission_denies_authority_that_differs_from_the_admitted_grant() -> TestResult<()> {
    let mutations: [AuthorityMutation; 5] = [
        ("authority-ref", |authority| authority.authority_ref = HASH_F.to_string()),
        ("resource-grant-ref", |authority| authority.resource_grant_ref = HASH_A.to_string()),
        ("executable-identity-ref", |authority| authority.executable_identity_ref = HASH_F.to_string()),
        ("workspace-ref", |authority| authority.workspace_ref = HASH_F.to_string()),
        ("extension-id", |authority| authority.extension_id = "other-extension".to_string()),
    ];
    let base = Cohort::new()?;
    let mut control = base.install()?;
    control.start(START_TICK).or_fail("the unmodified grant starts")?;
    for (field, mutate) in mutations {
        let mut cohort = base.clone();
        cohort.journal = std::sync::Arc::new(std::sync::Mutex::new(Journal::empty()));
        mutate(&mut cohort.template.authority);
        let mut service = cohort.install()?;
        let failure = service.start(START_TICK).expect_err("mismatched authority must deny execution");
        let detail = format!("{failure:?}");
        assert!(detail.contains("AuthorityMismatch"), "{field}: {detail}");
        assert!(detail.contains(field), "{field}: {detail}");
        assert!(service.host().executor().observations().is_empty(), "{field}: no process may start");
    }
    Ok(())
}

// r[verify molten.system_extension.independent_authority_and_output_gate]
#[test]
fn native_execution_admission_denies_a_callback_generation_fenced_after_intent_commit() -> TestResult<()> {
    let values = FencingValuePort::default();
    let mut cohort = Cohort::new()?;
    cohort.values = shared_callback_value_port(values.clone());
    let mut service = cohort.install()?;
    service.start(START_TICK).or_fail("start fenced native service")?;
    let observations = service.host().executor().observations().len();
    *values.fence.lock().or_fail("arm generation fence")? = Some(service.host().executor().instance().clone());

    let detail = callback_denial(service.ingress(&ingress(GENERATION, cohort.admitted.manifest_ref())?, REQUEST_TICK));
    assert!(detail.contains("execution request denied"), "{detail}");
    assert!(detail.contains("StaleGeneration"), "{detail}");
    assert_eq!(service.host().executor().observations().len(), observations, "no process may start");
    Ok(())
}

// r[verify molten.system_extension.independent_authority_and_output_gate]
#[test]
fn journal_failure_after_a_routed_effect_is_reported_uncertain() -> TestResult<()> {
    let cohort = Cohort::new()?;
    let is_failing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let journal = std::sync::Arc::new(std::sync::Mutex::new(FailingJournal {
        inner: InMemory::empty(),
        is_failing: is_failing.clone(),
    }));
    let port = Port::new(cohort.execution_profile.clone(), Publisher::default()).or_fail("live execution adapter")?;
    let mut service = Supervisor::install(
        AdmissionSet {
            profile: cohort.native_profile.clone(),
            executable: cohort.executable.clone(),
            admitted: cohort.admitted.clone(),
        },
        port,
        journal,
        cohort.values.clone(),
        cohort.template.clone(),
    )
    .or_fail("install native service with failing journal")?;
    service.start(START_TICK).or_fail("start native service")?;
    let accepted = service
        .ingress(&ingress(GENERATION, cohort.admitted.manifest_ref())?, REQUEST_TICK)
        .or_fail("accepted native ingress")?;
    let (callback_receipt, _outcome) =
        accepted.dispatch.require_executed("accepted ingress").or_fail("accepted ingress callback")?;
    let mut effects = JournalBreakingEffectPort {
        inner: EffectPort::default(),
        is_failing,
    };

    let failure = service.route_effects(&callback_receipt, &mut effects).expect_err("unjournaled effect must fail");
    assert_eq!(effects.inner.routed, 1);
    let Failure::Host(detail) = &failure else {
        return Err(format!("expected classified host failure, got {failure:?}").into());
    };
    assert!(
        detail.starts_with(molten::error::FailureClass::UncertainExternalOutcome.as_str()),
        "post-effect journal failure must be uncertain, not storage: {detail}"
    );
    Ok(())
}
