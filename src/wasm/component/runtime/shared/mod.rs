//! The product execution backend: Molten admission and receipts surround the
//! published shared execution shell. Product meaning stays outside the mechanism.

mod configuration;
mod hosted;
pub(crate) mod retained;

pub(crate) struct EngineAdapter {
    inner: hardened_wasmtime_shell::BoundedRuntime,
    profile_ref: String,
}

impl EngineAdapter {
    pub(crate) fn build(profile: &super::super::model::RuntimeProfile) -> super::super::model::Result<Self> {
        super::super::profile::validate_runtime(profile)?;
        let inner = configuration::build(profile)?;
        Ok(Self {
            inner,
            profile_ref: super::super::profile::r#ref(profile),
        })
    }

    /// The composition root must operate this exact engine's epoch service.
    pub(crate) fn epoch_service(&self) -> hardened_wasmtime_shell::EpochService {
        self.inner.epoch_service()
    }

    pub(crate) fn execute(
        &self,
        request: &super::shell::ExecutionRequest<'_>,
        deadline_ticks: u64,
    ) -> super::shell::ExecutionOutcome {
        self.execute_inner(request, deadline_ticks).unwrap_or_else(|denial| denied_outcome(request, denial))
    }

    fn execute_inner(
        &self,
        request: &super::shell::ExecutionRequest<'_>,
        deadline_ticks: u64,
    ) -> super::super::model::Result<super::shell::ExecutionOutcome> {
        let admitted = admit(request)?;
        let inspection = stage(request, &admitted, super::super::evidence::receipt::Stage::Inspection, Vec::new())?;
        let invocation = match self.prepare(request, deadline_ticks) {
            Ok(invocation) => invocation,
            Err(denial) => return denied(request, &admitted, denial, vec![inspection]),
        };
        let observation =
            invocation.run(super::super::admission::COMPONENT_INVOKE_EXPORT, &admitted.input_bytes, (), |_, _| {
                Err(hardened_wasmtime_shell::HostDenied)
            });
        finish(request, &admitted, vec![inspection], observation)
    }

    fn prepare(
        &self,
        request: &super::shell::ExecutionRequest<'_>,
        deadline_ticks: u64,
    ) -> super::super::model::Result<hardened_wasmtime_shell::ComponentInvocation> {
        if self.profile_ref != super::super::profile::r#ref(request.profile) {
            return Err(super::super::model::Denial::classified(
                super::super::model::DenialClass::ProfileDenial,
                "engine profile differs from admitted request",
            ));
        }
        if !request.facts.imports.is_empty() {
            return Err(super::super::model::Denial::new(
                "initial component runtime cohort has no admitted host linker bindings",
            ));
        }
        self.prepare_bound(request, deadline_ticks, &[])
    }

    fn prepare_bound(
        &self,
        request: &super::shell::ExecutionRequest<'_>,
        deadline_ticks: u64,
        descriptors: &[hardened_wasmtime_runtime::restricted::imports::Descriptor],
    ) -> super::super::model::Result<hardened_wasmtime_shell::ComponentInvocation> {
        if self.profile_ref != super::super::profile::r#ref(request.profile) {
            return Err(super::super::model::Denial::classified(
                super::super::model::DenialClass::ProfileDenial,
                "engine profile differs from admitted request",
            ));
        }
        let facts = configuration::facts(request);
        let service = self.epoch_service();
        hardened_wasmtime_shell::ComponentInvocation::prepare(&self.inner, hardened_wasmtime_shell::PrepareOptions {
            artifact: hardened_wasmtime_shell::Artifact {
                bytes: request.source.component_bytes(),
                facts: &facts,
            },
            descriptors,
            deadline: hardened_wasmtime_runtime::epoch::deadline::Budget {
                budget_ticks: deadline_ticks,
            },
            epoch_service: Some(&service),
        })
        .map_err(configuration::prepare_denial)
    }
}

pub(super) fn admit(
    request: &super::shell::ExecutionRequest<'_>,
) -> super::super::model::Result<super::shell::AdmittedRequest> {
    super::shell::admit_request(request)
}

pub(crate) fn denied_outcome(
    request: &super::shell::ExecutionRequest<'_>,
    denial: super::super::model::Denial,
) -> super::shell::ExecutionOutcome {
    super::denial::denied_outcome(request, denial)
}

fn stage(
    request: &super::shell::ExecutionRequest<'_>,
    admitted: &super::shell::AdmittedRequest,
    stage: super::super::evidence::receipt::Stage,
    parent_refs: Vec<String>,
) -> super::super::model::Result<super::super::evidence::receipt::ExecutionRecord> {
    super::shell::plan_receipt(request, super::shell::PlanReceiptInput {
        plan: &admitted.plan,
        stage,
        input_ref: None,
        output_ref: None,
        fuel_limit: None,
        fuel_remaining: None,
        parent_refs,
    })
}

fn denied(
    request: &super::shell::ExecutionRequest<'_>,
    admitted: &super::shell::AdmittedRequest,
    denial: super::super::model::Denial,
    receipts: Vec<super::super::evidence::receipt::ExecutionRecord>,
) -> super::super::model::Result<super::shell::ExecutionOutcome> {
    super::denial::plan_denied_outcome(request, &admitted.plan, denial, receipts, Some(admitted.input_ref.clone()))
}

fn finish(
    request: &super::shell::ExecutionRequest<'_>,
    admitted: &super::shell::AdmittedRequest,
    mut receipts: Vec<super::super::evidence::receipt::ExecutionRecord>,
    observation: hardened_wasmtime_shell::InvocationOutcome,
) -> super::super::model::Result<super::shell::ExecutionOutcome> {
    if observation.phase == hardened_wasmtime_runtime::outcomes::InvocationPhase::Invocation {
        let parents = receipts.last().map(|receipt| receipt.receipt_ref.clone()).into_iter().collect();
        receipts.push(stage(request, admitted, super::super::evidence::receipt::Stage::Instantiation, parents)?);
    }
    if observation.class != hardened_wasmtime_runtime::outcomes::Class::Completed {
        return denied(request, admitted, observation_denial(&observation), receipts);
    }
    let (output, output_ref) = match super::shell::decode_output(request.profile, &observation.output) {
        Ok(decoded) => decoded,
        Err(denial) => return denied(request, admitted, denial, receipts),
    };
    let parents = receipts.last().map(|receipt| receipt.receipt_ref.clone()).into_iter().collect();
    receipts.push(super::shell::plan_receipt(request, super::shell::PlanReceiptInput {
        plan: &admitted.plan,
        stage: super::super::evidence::receipt::Stage::Execution,
        input_ref: Some(admitted.input_ref.clone()),
        output_ref: Some(output_ref),
        fuel_limit: Some(request.profile.resources.fuel),
        fuel_remaining: Some(observation.fuel_remaining),
        parent_refs: parents,
    })?);
    Ok(super::shell::ExecutionOutcome {
        decision: super::super::evidence::receipt::Decision::Pass,
        output: Some(output),
        receipts,
        diagnostics: Vec::new(),
    })
}

fn observation_denial(observation: &hardened_wasmtime_shell::InvocationOutcome) -> super::super::model::Denial {
    let class = if observation.phase == hardened_wasmtime_runtime::outcomes::InvocationPhase::Instantiation {
        super::super::model::DenialClass::ComponentInstantiationDenied
    } else if observation.phase == hardened_wasmtime_runtime::outcomes::InvocationPhase::Invocation
        && observation.fuel_remaining == 0
        && matches!(
            observation.class,
            hardened_wasmtime_runtime::outcomes::Class::MemoryLimitExceeded
                | hardened_wasmtime_runtime::outcomes::Class::GuestTrapped
                | hardened_wasmtime_runtime::outcomes::Class::GuestPointerRejected
                | hardened_wasmtime_runtime::outcomes::Class::RuntimeFailed
                | hardened_wasmtime_runtime::outcomes::Class::HostFailed
        )
    {
        // Only Wasmtime invocation errors consult remaining fuel; the stable class is fixed by the frozen
        // corpus.
        super::super::model::DenialClass::FuelExhausted
    } else {
        match observation.class {
            hardened_wasmtime_runtime::outcomes::Class::ProfileDenied => {
                super::super::model::DenialClass::ProfileDenial
            }
            hardened_wasmtime_runtime::outcomes::Class::ArtifactDenied => {
                super::super::model::DenialClass::ComponentAdmissionDenial
            }
            hardened_wasmtime_runtime::outcomes::Class::CompileFailed => {
                super::super::model::DenialClass::ComponentCompilationDenied
            }
            hardened_wasmtime_runtime::outcomes::Class::InstantiateFailed => {
                super::super::model::DenialClass::ComponentInstantiationDenied
            }
            hardened_wasmtime_runtime::outcomes::Class::FuelExhausted => {
                super::super::model::DenialClass::FuelExhausted
            }
            hardened_wasmtime_runtime::outcomes::Class::DeadlineExhausted => {
                super::super::model::DenialClass::DeadlineExceeded
            }
            hardened_wasmtime_runtime::outcomes::Class::GuestDenied => super::super::model::DenialClass::GuestDenial,
            hardened_wasmtime_runtime::outcomes::Class::InputLimitExceeded
            | hardened_wasmtime_runtime::outcomes::Class::OutputLimitExceeded
            | hardened_wasmtime_runtime::outcomes::Class::ConcurrencyExhausted => {
                super::super::model::DenialClass::ResourceDenial
            }
            hardened_wasmtime_runtime::outcomes::Class::MemoryLimitExceeded
            | hardened_wasmtime_runtime::outcomes::Class::GuestTrapped
            | hardened_wasmtime_runtime::outcomes::Class::GuestPointerRejected
            | hardened_wasmtime_runtime::outcomes::Class::RuntimeFailed
            | hardened_wasmtime_runtime::outcomes::Class::HostFailed
            | hardened_wasmtime_runtime::outcomes::Class::Completed => super::super::model::DenialClass::ComponentTrap,
        }
    };
    super::super::model::Denial::classified(
        class,
        format!("component {:?} denied as {}: {:?}", observation.phase, class.as_str(), observation.class),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn explicit_deadline_is_not_reclassified_as_trap_or_empty_fuel() {
        // r[verify molten.wasm_component_adoption.parity]
        for fuel_remaining in [0, 1] {
            let observation = hardened_wasmtime_shell::InvocationOutcome {
                class: hardened_wasmtime_runtime::outcomes::Class::DeadlineExhausted,
                phase: hardened_wasmtime_runtime::outcomes::InvocationPhase::Invocation,
                output: Vec::new(),
                fuel_remaining,
            };
            let denial = super::observation_denial(&observation);
            assert_eq!(denial.canonical_class(), "deadline-exceeded");
            assert_eq!(
                super::super::super::model::DenialClass::parse(denial.canonical_class()),
                Some(super::super::super::model::DenialClass::DeadlineExceeded),
            );
            let trapped = hardened_wasmtime_shell::InvocationOutcome {
                class: hardened_wasmtime_runtime::outcomes::Class::GuestTrapped,
                phase: observation.phase,
                output: Vec::new(),
                fuel_remaining,
            };
            let expected = if fuel_remaining == 0 {
                "fuel-exhausted"
            } else {
                "component-trap"
            };
            assert_eq!(super::observation_denial(&trapped).canonical_class(), expected);
        }
    }
}
