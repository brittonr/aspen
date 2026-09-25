
impl FabricEffectPort for InvalidEffectPort {
    fn route(
        &mut self,
        _binding: &molten::fabric::CanonicalFabricPortBinding,
        _effect: &TypedEffectRequest,
    ) -> molten::fabric::FabricPortResult<PortEffectOutput> {
        self.routed = self
            .routed
            .checked_add(1)
            .ok_or_else(|| molten::fabric::FabricPortError::malformed("invalid effect route count overflow"))?;
        let (output_ref, materialized_output) = match self.kind {
            InvalidEffectOutput::Missing => (HASH_E.to_string(), None),
            InvalidEffectOutput::IdentityMismatch => (
                HASH_E.to_string(),
                Some(NativeCallbackValue {
                    value_ref: HASH_E.to_string(),
                    bytes: b"identity-mismatch".to_vec(),
                }),
            ),
            InvalidEffectOutput::Oversized => {
                let bytes = vec![0; OVERSIZED_VALUE_BYTES];
                let value_ref = molten::preserves_rail::content_ref_from_bytes(&bytes);
                (value_ref.clone(), Some(NativeCallbackValue { value_ref, bytes }))
            }
        };
        Ok(PortEffectOutput {
            output_schema_ref: EFFECT_OUTPUT_SCHEMA.to_string(),
            output_ref,
            materialized_output,
        })
    }
}

// r[verify molten.system_extension.native_host.effect_completion_value.rejected]
#[test]
fn materializing_native_host_rejects_missing_mismatched_and_oversized_effect_values_without_retry() -> TestResult<()> {
    assert_eq!(u64::try_from(MAX_VALUE_BYTES_USIZE), Ok(MAX_VALUE_BYTES));
    for kind in [
        InvalidEffectOutput::Missing,
        InvalidEffectOutput::IdentityMismatch,
        InvalidEffectOutput::Oversized,
    ] {
        let cohort = Cohort::new()?;
        let mut service = cohort.install()?;
        service.start(START_TICK).or_fail("start native service")?;
        let accepted = {
            let mut client = NativeServiceClient::new(&mut service);
            client
                .submit(&ingress(GENERATION, cohort.admitted.manifest_ref())?, REQUEST_TICK)
                .or_fail("accepted native ingress")?
        };
        let (callback_receipt, _outcome) =
            accepted.dispatch.require_executed("accepted ingress").or_fail("accepted ingress callback")?;
        let callback_observations = service.host().executor().observations().len();
        let mut effects = InvalidEffectPort { kind, routed: 0 };

        assert!(service.route_effects(&callback_receipt, &mut effects).is_err());
        assert_eq!(effects.routed, 1);
        assert_eq!(service.host().executor().observations().len(), callback_observations);
        let instance = service.instance().or_fail("terminal invalid effect instance")?;
        assert!(instance.completed_operations.iter().any(|operation| {
            operation.kind == NativeOperationKind::Effect
                && operation.state == NativeOperationState::Terminal
                && !operation.is_retry_permitted
        }));
    }
    Ok(())
}

#[derive(Clone, Default)]
struct ControlledValuePort {
    inner: std::sync::Arc<std::sync::Mutex<InMemoryNativeCallbackValuePort>>,
    failure: std::sync::Arc<std::sync::Mutex<Option<(usize, NativeValuePortFailureKind)>>>,
}

impl ControlledValuePort {
    fn fail_next(&self, kind: NativeValuePortFailureKind) -> TestResult<()> {
        self.fail_after(0, kind)
    }

    fn fail_after(&self, successful_publications: usize, kind: NativeValuePortFailureKind) -> TestResult<()> {
        *self.failure.lock().or_fail("controlled value failure")? = Some((successful_publications, kind));
        Ok(())
    }
}

impl NativeCallbackValuePort for ControlledValuePort {
    fn materialize(
        &mut self,
        value_ref: &str,
        maximum_bytes: u64,
    ) -> std::result::Result<NativeCallbackValue, NativeValuePortFailure> {
        self.inner
            .lock()
            .map_err(|_| {
                NativeValuePortFailure::new(
                    NativeValuePortFailureKind::RejectedBeforeAcceptance,
                    "controlled value port lock is unavailable",
                )
            })?
            .materialize(value_ref, maximum_bytes)
    }

    fn publish(
        &mut self,
        value: &NativeCallbackValue,
        maximum_bytes: u64,
    ) -> std::result::Result<NativeValuePublicationReceipt, NativeValuePortFailure> {
        let failure = {
            let mut failure = self.failure.lock().map_err(|_| {
                NativeValuePortFailure::new(
                    NativeValuePortFailureKind::RejectedBeforeAcceptance,
                    "controlled value failure lock is unavailable",
                )
            })?;
            match *failure {
                Some((0, kind)) => {
                    *failure = None;
                    Some(kind)
                }
                Some((remaining, kind)) => {
                    *failure = Some((remaining - 1, kind));
                    None
                }
                None => None,
            }
        };
        let mut inner = self.inner.lock().map_err(|_| {
            NativeValuePortFailure::new(
                NativeValuePortFailureKind::RejectedBeforeAcceptance,
                "controlled value port lock is unavailable",
            )
        })?;
        if let Some(kind) = failure {
            inner.fail_next_publication(kind);
        }
        inner.publish(value, maximum_bytes)
    }
}

// r[verify molten.system_extension.native_host.execution]
// r[verify molten.system_extension.native_host.validation]
#[test]
fn native_executor_fails_closed_for_malformed_nonzero_timeout_flood_spawn_and_cancellation() -> TestResult<()> {
    let shell = std::path::PathBuf::from("/bin/sh");
    for (script, timeout_ms, output_bytes) in [
        ("printf 'malformed'", NORMAL_TIMEOUT_MS, FULL_OUTPUT_BYTES),
        ("exit 7", NORMAL_TIMEOUT_MS, FULL_OUTPUT_BYTES),
        ("while :; do :; done", SHORT_TIMEOUT_MS, FULL_OUTPUT_BYTES),
        ("printf 'output-flood'", NORMAL_TIMEOUT_MS, SMALL_OUTPUT_BYTES),
    ] {
        let mut cohort = Cohort::new()?;
        cohort.replace_program(shell.clone(), vec!["-c".to_string(), script.to_string()], timeout_ms, output_bytes);
        let mut service = cohort.install()?;
        assert!(service.start(START_TICK).is_err());
        assert!(!service.host().executor().observations().is_empty());
    }

    let mut missing = Cohort::new()?;
    missing.replace_program(
        std::path::PathBuf::from("/definitely/missing/native-extension"),
        Vec::new(),
        NORMAL_TIMEOUT_MS,
        FULL_OUTPUT_BYTES,
    );
    let mut missing_service = missing.install()?;
    assert!(missing_service.start(START_TICK).is_err());

    let cancellation = Cohort::new()?;
    let mut cancelled_service = cancellation.install()?;
    cancelled_service
        .host()
        .executor()
        .cancellation_handle()
        .store(true, std::sync::atomic::Ordering::Release);
    assert!(cancelled_service.start(START_TICK).is_err());
    assert_eq!(
        cancelled_service.host().executor().observations()[0].lifecycle,
        molten::fabric_execution::ExecutionLifecycleState::Cancelled
    );
    Ok(())
}
