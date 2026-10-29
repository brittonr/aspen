use super::ComponentCallbackHostFactory;

const MAX_CALLBACK_OBSERVATIONS: usize = 128;
const MAX_CALLBACK_VALUES: u64 = 128;

struct PendingOutcome {
    invocation: crate::system_extension::CallbackInvocation,
    outcome: crate::system_extension::MaterializedCallbackOutcome,
    observation_index: usize,
}

/// Real Component Model executor for an ordinary manifest-installed Dispatcher.
///
/// No alternate Wasmtime engine/linker, process probe, loose artifact source, or
/// replay-to-live fallback is available. Unadmitted outcomes never change state
/// and are not published. Host effects retain their separate Session admission
/// and recording, even if later guest execution or outcome admission fails.
pub struct Executor<F: ComponentCallbackHostFactory> {
    installation: super::Installation,
    engine: crate::wasm_component::Engine,
    values: crate::system_extension::SharedCallbackValuePort,
    hosts: F,
    context: crate::system_extension::CallbackContext,
    last_invocation: Option<crate::system_extension::CallbackInvocation>,
    pending: Option<PendingOutcome>,
    observations: Vec<super::HostedObservation>,
    publication_fence: Option<usize>,
}

impl<F: ComponentCallbackHostFactory> Executor<F> {
    pub fn new(
        installation: super::Installation,
        values: crate::system_extension::SharedCallbackValuePort,
        hosts: F,
    ) -> crate::wasm_component::Result<Self> {
        let context = installation.admit()?;
        let engine = crate::wasm_component::Engine::build(&installation.profile)?;
        Ok(Self {
            installation,
            engine,
            values,
            hosts,
            context,
            last_invocation: None,
            pending: None,
            observations: Vec::with_capacity(MAX_CALLBACK_OBSERVATIONS),
            publication_fence: None,
        })
    }

    pub fn observations(&self) -> &[super::HostedObservation] {
        &self.observations
    }

    pub fn values(&self) -> &crate::system_extension::SharedCallbackValuePort {
        &self.values
    }

    pub fn state_ref(&self) -> Option<&str> {
        self.context.state_ref.as_deref()
    }

    pub fn installation(&self) -> &super::Installation {
        &self.installation
    }

    fn invoke_component(
        &mut self,
        invocation: &crate::system_extension::CallbackInvocation,
    ) -> Result<crate::system_extension::CallbackOutcome, String> {
        self.admit_invocation(invocation)?;
        // Any earlier candidate was denied by Dispatcher and cannot be committed.
        self.pending = None;
        let maximum = self.installation.profile.resources.max_hostcall_bytes;
        let inputs = self.materialize_inputs(invocation, maximum)?;
        let envelope = crate::system_extension::canonical_callback_envelope(&self.context, invocation, &inputs)
            .map_err(|error| format!("component callback input: {error}"))?;
        crate::system_extension::decode_callback_envelope(&envelope.bytes, maximum, maximum, MAX_CALLBACK_VALUES)
            .map_err(|error| format!("component callback envelope admission: {error}"))?;
        let binding = self
            .hosts
            .create(&self.context, invocation)
            .map_err(|error| format!("component callback host admission: {error:?}"))?;
        self.last_invocation = Some(invocation.clone());
        let request = crate::wasm_component::ExecutionRequest {
            profile: &self.installation.profile,
            requested_profile: crate::wasm_component::RequestedExecutionProfile::ComponentV1,
            evidence_scope: crate::wasm_component::EvidenceScope::Production,
            source: self
                .installation
                .materialization
                .source(&self.installation.envelope, crate::wasm_component::Consumer::SystemExtension),
            facts: &self.installation.facts,
            import_manifest: &self.installation.import_manifest,
            import_grants: &binding.import_grants,
            input: &envelope.value,
        };
        let mut execution = self.engine.execute_hosted(&request, binding.host);
        if execution.is_pass()
            && binding.is_replay
            && let Err(denial) = binding.recording.ensure_replay_complete()
        {
            execution = crate::wasm_component::runtime::denial::deny_completed_execution(execution, denial);
        }
        let host_records = binding.recording.snapshot();
        let is_uncertain = host_records.as_ref().map_or(true, |records| records.has_uncertain_publication());
        if execution.is_pass() && is_uncertain {
            execution = crate::wasm_component::runtime::denial::deny_completed_execution(
                execution,
                crate::wasm_component::Denial::new(
                    "component host publication acceptance is unresolved; reconciliation required",
                ),
            );
        }
        let observation_index = self.observations.len();
        if is_uncertain {
            self.publication_fence = Some(observation_index);
        }
        self.observations.push(super::HostedObservation {
            envelope,
            execution,
            host_records,
            materialized_outcome: None,
            publication_refs: Vec::with_capacity(super::publication::MAX_CALLBACK_PUBLICATIONS),
            publications: Vec::with_capacity(super::publication::MAX_CALLBACK_PUBLICATIONS),
            is_publication_uncertain: is_uncertain,
            is_committed: false,
        });
        let observation = &mut self.observations[observation_index];
        if !observation.execution.is_pass() {
            return Err(format!(
                "component execution {:?}: {:?}",
                observation.execution.decision, observation.execution.diagnostics,
            ));
        }
        let output = observation
            .execution
            .output
            .as_ref()
            .ok_or_else(|| "passing component execution has no canonical output".to_string())?;
        let bytes = crate::preserves_rail::canonical_bytes(output)
            .map_err(|error| format!("component callback output bytes: {error}"))?;
        let maximum = self.installation.profile.resources.max_result_bytes;
        let outcome = crate::system_extension::decode_callback_outcome(&bytes, maximum, maximum, MAX_CALLBACK_VALUES)
            .map_err(|error| format!("component callback output admission: {error}"))?;
        let projected = outcome.project();
        observation.materialized_outcome = Some(outcome.clone());
        self.pending = Some(PendingOutcome {
            invocation: invocation.clone(),
            outcome,
            observation_index,
        });
        Ok(projected)
    }

    fn admit_invocation(&self, invocation: &crate::system_extension::CallbackInvocation) -> Result<(), String> {
        if self.publication_fence.is_some() {
            return Err("component callback publication is unresolved; reconciliation required before any invocation or restart".to_string());
        }
        if self.observations.len() >= MAX_CALLBACK_OBSERVATIONS {
            return Err("component callback observation capacity exhausted before execution".to_string());
        }
        if !self.installation.admitted.manifest().declares_callback(invocation.callback) {
            return Err("component invocation callback is not declared by the installed manifest".to_string());
        }
        if invocation.generation < self.installation.admitted.manifest().initial_generation {
            return Err("component invocation generation predates installation".to_string());
        }
        if invocation.deadline_tick < invocation.logical_tick {
            return Err("component invocation logical deadline is expired".to_string());
        }
        if let Some(previous) = &self.last_invocation
            && (invocation.generation < previous.generation
                || (invocation.generation == previous.generation && invocation.incarnation < previous.incarnation)
                || invocation.sequence <= previous.sequence)
        {
            return Err("component invocation generation, incarnation or sequence is stale".to_string());
        }
        Ok(())
    }

    fn materialize_inputs(
        &self,
        invocation: &crate::system_extension::CallbackInvocation,
        maximum: u64,
    ) -> Result<crate::system_extension::CallbackInputs, String> {
        let mut values = self.values.lock().map_err(|_| "component callback value port lock poisoned".to_string())?;
        let mut materialize = |reference: &str| {
            let value = values
                .materialize(reference, maximum)
                .map_err(|error| format!("component callback value materialization: {error:?}"))?;
            crate::system_extension::admit_callback_value(&value, maximum)
                .map_err(|error| format!("component callback value admission: {error:?}"))?;
            if value.value_ref != reference {
                return Err("component callback value port returned another object".to_string());
            }
            Ok(value)
        };
        let payload = invocation.payload_ref.as_deref().map(&mut materialize).transpose()?;
        let state = self.context.state_ref.as_deref().map(&mut materialize).transpose()?;
        Ok(crate::system_extension::CallbackInputs { payload, state })
    }

    fn commit_component(
        &mut self,
        invocation: &crate::system_extension::CallbackInvocation,
        outcome: &crate::system_extension::CallbackOutcome,
    ) -> Result<(), String> {
        let pending = self.pending.take().ok_or_else(|| "component callback has no pending outcome".to_string())?;
        if pending.invocation != *invocation || pending.outcome.project() != *outcome {
            return Err("component callback commit differs from the executed invocation or outcome".to_string());
        }
        let maximum = self.installation.profile.resources.max_result_bytes;
        let mut values = self.values.lock().map_err(|_| "component callback value port lock poisoned".to_string())?;
        let observation = &mut self.observations[pending.observation_index];
        let publication =
            super::publication::publish_outcome_values(&mut **values, &pending.outcome, maximum, observation);
        if observation.is_publication_uncertain {
            self.publication_fence = Some(pending.observation_index);
        }
        publication?;
        if let Some(state_ref) = &outcome.state_ref {
            self.context.state_ref = Some(state_ref.clone());
        }
        self.observations[pending.observation_index].is_committed = true;
        Ok(())
    }
}

impl<F: ComponentCallbackHostFactory> crate::system_extension::SystemExtensionExecutor for Executor<F> {
    fn execution_profile(&self) -> crate::system_extension::ExecutionProfile {
        crate::system_extension::ExecutionProfile::SandboxedComponent
    }

    fn failure_class(&self) -> crate::system_extension::FailureClass {
        if self.publication_fence.is_some() {
            crate::system_extension::FailureClass::Fatal
        } else {
            crate::system_extension::FailureClass::Retryable
        }
    }

    fn invoke(
        &mut self,
        invocation: &crate::system_extension::CallbackInvocation,
    ) -> Result<crate::system_extension::CallbackOutcome, String> {
        self.invoke_component(invocation)
    }

    fn commit_admitted_outcome(
        &mut self,
        invocation: &crate::system_extension::CallbackInvocation,
        outcome: &crate::system_extension::CallbackOutcome,
    ) -> Result<(), String> {
        self.commit_component(invocation, outcome)
    }
}

#[cfg(test)]
mod tests;
