pub struct Executor {
    pub(in crate::mesh) pending: Option<crate::system_extension::TypedEffectRequest>,
    pub(super) state_ref: String,
}

/// Creates an absent host only; the owner must activate or restore before service startup.
pub fn create_host(
    manifest: &crate::mesh::admission::Manifest,
) -> crate::error::Result<crate::system_extension::Dispatcher<Executor>> {
    crate::system_extension::Dispatcher::new(manifest.extension().clone(), Executor {
        pending: None,
        state_ref: manifest.manifest_ref().to_string(),
    })
}

impl crate::system_extension::SystemExtensionExecutor for Executor {
    fn execution_profile(&self) -> crate::system_extension::ExecutionProfile {
        crate::system_extension::ExecutionProfile::InProcessNative
    }
    fn failure_class(&self) -> crate::system_extension::FailureClass {
        crate::system_extension::FailureClass::Fatal
    }
    fn invoke(
        &mut self,
        invocation: &crate::system_extension::CallbackInvocation,
    ) -> std::result::Result<crate::system_extension::CallbackOutcome, String> {
        let mut effects = Vec::new();
        if invocation.callback == crate::system_extension::CallbackKind::Request {
            let mut effect =
                self.pending.as_ref().ok_or_else(|| "no receiver-owned federation plan".to_string())?.clone();
            if invocation.payload_ref.as_deref() != Some(effect.request_ref.as_str()) {
                return Err("federation callback payload differs from receiver plan".to_string());
            }
            effect.generation = invocation.generation;
            effect.incarnation = invocation.incarnation;
            effects.push(effect);
        }
        let value = crate::preserves_rail::record("federation-callback-state-v1", vec![
            crate::preserves_rail::string(&self.state_ref),
            crate::preserves_rail::u64_value(invocation.generation),
            crate::preserves_rail::u64_value(invocation.incarnation),
            crate::preserves_rail::u64_value(invocation.sequence),
        ]);
        let state_ref = crate::preserves_rail::canonical_hash(&value).map_err(|error| error.to_string())?;
        Ok(crate::system_extension::CallbackOutcome {
            output_refs: Vec::new(),
            effects,
            state_ref: Some(state_ref.clone()),
            checkpoint_ref: Some(state_ref),
            health: crate::system_extension::HealthState::Healthy,
        })
    }
    fn commit_admitted_outcome(
        &mut self,
        invocation: &crate::system_extension::CallbackInvocation,
        outcome: &crate::system_extension::CallbackOutcome,
    ) -> std::result::Result<(), String> {
        self.state_ref = outcome.state_ref.as_ref().ok_or_else(|| "missing federation state ref".to_string())?.clone();
        if invocation.callback == crate::system_extension::CallbackKind::Request {
            self.pending = None;
        }
        Ok(())
    }
}
