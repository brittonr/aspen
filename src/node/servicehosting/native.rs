mod effects;

/// Runtime adapter over actual admitted, journal-backed native supervisors.
pub struct Runtime<P, J>
where
    P: crate::fabric_execution::ExecutionFabricPort,
    J: crate::system_extension::NativeHostJournal,
{
    services: std::collections::BTreeMap<String, crate::system_extension::Supervisor<P, J>>,
    replacements: std::collections::BTreeMap<String, crate::system_extension::Replacement<P>>,
    routers: std::collections::BTreeMap<String, Box<dyn super::NativeEffectRouter>>,
}

impl<P, J> Runtime<P, J>
where
    P: crate::fabric_execution::ExecutionFabricPort,
    J: crate::system_extension::NativeHostJournal,
{
    pub fn new(
        services: std::collections::BTreeMap<String, crate::system_extension::Supervisor<P, J>>,
    ) -> crate::error::Result<Self> {
        if services.len() > super::MAX_HOST_MEMBERS {
            return Err(super::invalid("native hosting member capacity exceeded"));
        }
        for (member, service) in &services {
            if service.instance().map_err(supervisor_error)?.service_id != *member {
                return Err(super::invalid("native hosting map key differs from admitted service identity"));
            }
        }
        Ok(Self {
            services,
            replacements: std::collections::BTreeMap::new(),
            routers: std::collections::BTreeMap::new(),
        })
    }

    /// The returned reference binds exact manifest, executable and admitted host profile.
    pub fn register_replacement(
        &mut self,
        replacement: crate::system_extension::Replacement<P>,
    ) -> crate::error::Result<String> {
        if self.replacements.len() >= super::MAX_HOST_MEMBERS {
            return Err(super::invalid("native hosting replacement capacity exceeded"));
        }
        let reference =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("host-native-replacement-v1", vec![
                crate::preserves_rail::string(replacement.admission.admitted.manifest_ref()),
                crate::preserves_rail::string(&replacement.admission.executable.executable.executable_ref),
                crate::preserves_rail::string(&replacement.admission.profile.profile.profile_ref),
            ]))?;
        if self.replacements.contains_key(&reference) {
            return Err(super::invalid("duplicate native replacement"));
        }
        self.replacements.insert(reference.clone(), replacement);
        Ok(reference)
    }
}

impl<P, J> super::RuntimePort for Runtime<P, J>
where
    P: crate::fabric_execution::ExecutionFabricPort,
    J: crate::system_extension::NativeHostJournal,
{
    fn instance(&self, member: &str) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        let service = self.services.get(member).ok_or_else(|| super::invalid("unknown native hosted member"))?;
        let current = service.instance().map_err(supervisor_error)?;
        let journal = service
            .host()
            .executor()
            .journal()
            .lock()
            .map_err(|_| super::invalid("native hosting journal lock poisoned"))?;
        let observed = journal
            .latest_instance(&current.instance_id)
            .map_err(|error| super::invalid(format!("native hosting journal observation: {error:?}")))?
            .ok_or_else(|| super::invalid("native hosting instance has no durable journal observation"))?;
        if current != observed {
            return Err(super::invalid("native runtime differs from its durable journal"));
        }
        Ok(observed)
    }

    fn apply(
        &mut self,
        member: &str,
        action: &super::RuntimeAction,
        logical_tick: u64,
    ) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        let service = self.services.get_mut(member).ok_or_else(|| super::invalid("unknown native hosted member"))?;
        match action {
            super::RuntimeAction::Start => {
                service.start(logical_tick).map_err(supervisor_error)?;
            }
            super::RuntimeAction::Stop => {
                if service.instance().map_err(supervisor_error)?.lifecycle.phase
                    != crate::system_extension::LifecyclePhase::Stopped
                {
                    service.stop(logical_tick).map_err(supervisor_error)?;
                }
            }
            super::RuntimeAction::Fence => {
                if service.instance().map_err(supervisor_error)?.lifecycle.phase
                    != crate::system_extension::LifecyclePhase::Failed
                {
                    service.observe_process_loss().map_err(supervisor_error)?;
                }
            }
            super::RuntimeAction::Restart => {
                service.restart(logical_tick).map_err(supervisor_error)?;
            }
            super::RuntimeAction::Drain => {
                service.drain(logical_tick).map_err(supervisor_error)?;
            }
            super::RuntimeAction::Checkpoint => {
                service.checkpoint(logical_tick).map_err(supervisor_error)?;
            }
            super::RuntimeAction::Upgrade { admission_ref } => {
                let replacement = self
                    .replacements
                    .remove(admission_ref)
                    .ok_or_else(|| super::invalid("native replacement artifact has not been admitted and loaded"))?;
                service.upgrade(replacement, logical_tick).map_err(supervisor_error)?;
            }
        }
        service.instance().map_err(supervisor_error)
    }

    fn deliver(
        &mut self,
        member: &str,
        ingress: &crate::system_extension::NativeIngressEnvelope,
        logical_tick: u64,
    ) -> crate::error::Result<super::RuntimeDelivery> {
        self.deliver_routed(member, ingress, logical_tick)
    }

    fn migration(&mut self, member: &str, migration_ref: &str) -> crate::error::Result<preserves::IOValue> {
        let current = self.instance(member)?;
        if !current.evidence_refs.iter().any(|reference| reference == migration_ref) {
            return Err(super::invalid("migration receipt is absent from the native durable instance"));
        }
        let service = self.services.get(member).ok_or_else(|| super::invalid("unknown native hosted member"))?;
        let value = service
            .host()
            .executor()
            .values()
            .lock()
            .map_err(|_| super::invalid("native migration value port lock poisoned"))?
            .materialize(migration_ref, super::MAX_HOST_PAYLOAD_BYTES as u64)
            .map_err(|error| super::invalid(format!("native migration evidence unavailable: {error:?}")))?;
        crate::system_extension::admit_callback_value(&value, super::MAX_HOST_PAYLOAD_BYTES as u64)
            .map_err(|error| super::invalid(format!("native migration identity denied: {error:?}")))?;
        let value = crate::preserves_rail::strict_canonical_decode(&value.bytes)?.value;
        if crate::preserves_rail::canonical_hash(&value)? != migration_ref {
            return Err(super::invalid("native migration receipt bytes do not match their reference"));
        }
        let fields = crate::preserves_rail::simple_record_fields(&value, "system-extension-state-migration-v1", 11)?;
        let target = crate::preserves_rail::value_to_iovalue(&fields[7]);
        let target = crate::preserves_rail::simple_record_fields(&target, "target-schema", 1)?;
        if crate::preserves_rail::required_string_field(&target[0], "target schema")?
            != service.host().manifest().manifest().state_schema
        {
            return Err(super::invalid("migration schema label differs from the executing admitted manifest"));
        }
        Ok(value)
    }
}

fn supervisor_error(error: crate::system_extension::Failure) -> crate::error::Failure {
    super::invalid(format!("native hosting lifecycle failed or remains uncertain: {error:?}"))
}
