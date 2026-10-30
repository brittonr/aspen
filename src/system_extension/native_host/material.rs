/// Exact admitted replacement material. Construction grants no execution authority.
pub struct Replacement<P> {
    pub admission: AdmissionSet,
    pub port: P,
    pub values: SharedCallbackValuePort,
    pub template: ExecutionTemplate,
}

impl<P, J> Supervisor<P, J>
where
    P: crate::fabric_execution::ExecutionFabricPort,
    J: NativeHostJournal,
{
    // r[impl molten.system_extension.node_hosting]
    pub fn upgrade(
        &mut self,
        replacement: Replacement<P>,
        logical_tick: u64,
    ) -> std::result::Result<GenerationReplacementArtifacts, Failure> {
        let checkpoint = self.instance()?.checkpoint_ref.ok_or(Failure::MissingCheckpoint)?;
        self.preflight_replacement(&replacement, &checkpoint)?;
        let mut next = self.instance()?;
        let generation = next.lifecycle.generation.checked_add(1)
            .ok_or_else(|| Failure::Host("native replacement generation overflow".to_string()))?;
        let event = LifecycleEvent {
            kind: LifecycleEventKind::BeginUpgrade,
            generation: next.lifecycle.generation,
            incarnation: next.lifecycle.incarnation,
            next_generation: Some(generation),
            checkpoint_ref: None,
            failure_class: None,
        };
        next.lifecycle = plan_lifecycle_transition(
            self.host.state(), &event, self.host.usage(),
            self.host.manifest().manifest().resources.max_restart_attempts,
        ).map_err(|issues| Failure::Host(format!("native replacement lifecycle denied: {issues:?}")))?;
        next.manifest_ref = replacement.admission.admitted.manifest_ref().to_string();
        next.executable_ref = replacement.admission.executable.executable.executable_ref.clone();
        next.profile_ref = replacement.admission.profile.profile.profile_ref.clone();
        next.state_schema_ref = replacement.admission.executable.executable.state_schema_ref.clone();
        next.is_accepting_ingress = false;
        let migration = canonical_state_migration_receipt(StateMigrationReceiptInput {
            operation: MigrationOperation::Upgrade,
            extension_id: &replacement.admission.admitted.manifest().extension_id,
            service_id: &replacement.admission.admitted.manifest().service_id,
            previous_manifest_ref: self.host.manifest().manifest_ref(),
            next_manifest_ref: replacement.admission.admitted.manifest_ref(),
            source_schema: &self.host.manifest().manifest().state_schema,
            target_schema: &replacement.admission.admitted.manifest().state_schema,
            checkpoint_ref: &checkpoint, generation,
        })?;
        let mut executor = Process::new(
            replacement.port, self.journal.clone(), replacement.values,
            self.instance.clone(), replacement.template,
        ).map_err(Failure::Executor)?;
        // Commit the target fence before its materialized Recover callback can run.
        self.replace_instance(next)?;
        self.profile = replacement.admission.profile;
        self.executable = replacement.admission.executable;
        self.publish_migration(&migration, &mut executor)?;
        let result = self.host.upgrade(replacement.admission.admitted, executor, &checkpoint, logical_tick);
        let artifacts = match result {
            Ok(artifacts) => artifacts,
            Err(error) => {
                if self.host.state().generation == generation { self.sync_instance(false)?; }
                return Err(Failure::from(error));
            }
        };
        self.sync_instance(true)?;
        Ok(artifacts)
    }

    pub fn observe_process_loss(&mut self) -> std::result::Result<LifecycleReceipt, Failure> {
        self.set_ingress(false)?;
        let receipt = self.host.observe_host_loss()?;
        self.sync_instance(false)?;
        Ok(receipt)
    }

    fn publish_migration(&mut self, migration: &StateMigrationReceipt, executor: &mut Process<P, J>) -> std::result::Result<(), Failure> {
        let bytes = crate::preserves_rail::canonical_bytes(&migration.value)?;
        executor.publish_external_value(
            &migration.receipt_ref, "generation-migration-receipt",
            &NativeCallbackValue { value_ref: migration.receipt_ref.clone(), bytes },
        ).map_err(Failure::Executor)?;
        let mut current = self.instance()?;
        current.evidence_refs.push(migration.receipt_ref.clone());
        current.evidence_refs.sort();
        current.evidence_refs.dedup();
        self.replace_instance(current)
    }

    fn preflight_replacement(
        &self,
        replacement: &Replacement<P>,
        checkpoint: &str,
    ) -> std::result::Result<(), Failure> {
        let current = self.host.manifest().manifest();
        let next = replacement.admission.admitted.manifest();
        if current.extension_id != next.extension_id || current.service_id != next.service_id
            || next.execution_profile != ExecutionProfile::NativeProcess
            || !next.declares_callback(CallbackKind::Recover)
            || self.host.state().checkpoint_ref.as_deref() != Some(checkpoint)
            || replacement.admission.admitted.manifest_ref()
                != replacement.admission.executable.executable.manifest_ref
            || replacement.admission.profile != replacement.template.host_profile
            || replacement.admission.executable != replacement.template.executable
            || replacement.admission.admitted.manifest_ref() != replacement.template.admitted.manifest_ref()
        {
            return Err(Failure::Host("native replacement admission does not match current identity or exact template".to_string()));
        }
        plan_state_migration(next, &current.state_schema, &next.state_schema)
            .map_err(|issues| Failure::Host(format!("native replacement migration denied: {issues:?}")))?;
        Ok(())
    }
}

impl<P, J> Supervisor<P, J>
where
    P: crate::fabric_execution::ExecutionFabricPort,
    J: NativeHostJournal,
{
    fn prepare_restart(&mut self) -> std::result::Result<(), Failure> {
        let maximum = self.host.manifest().manifest().resources.max_restart_attempts;
        let state = self.host.state();
        let event = LifecycleEvent::simple(LifecycleEventKind::BeginRestart, state.generation, state.incarnation);
        let restarting = plan_lifecycle_transition(state, &event, self.host.usage(), maximum)
            .map_err(|issues| Failure::Host(format!("native restart fence denied: {issues:?}")))?;
        let kind = if self.host.manifest().manifest().declares_callback(CallbackKind::Recover) {
            if state.checkpoint_ref.is_none() { return Err(Failure::MissingCheckpoint); }
            LifecycleEventKind::BeginRecovery
        } else { LifecycleEventKind::BeginStart };
        let event = LifecycleEvent::simple(kind, restarting.generation, restarting.incarnation);
        let planned = plan_lifecycle_transition(&restarting, &event, self.host.usage(), maximum)
            .map_err(|issues| Failure::Host(format!("native recovery fence denied: {issues:?}")))?;
        let mut next = self.instance()?;
        next.lifecycle = planned;
        next.is_accepting_ingress = false;
        self.replace_instance(next)
    }
}
