//! Bounded ordinary Component Model actor installation and invocation.
//!
//! This is not the addressable-actor sleep/wake lifecycle or the legacy core
//! module harness. Every turn instantiates the retained portable component using
//! the shared admitted engine and canonical product input.

pub mod host;

/// A genuine immutable Mantle snapshot plus its separate current admission
/// envelope. There is deliberately no loose-byte or synthetic-bundle variant.
pub struct ActorInstallInput {
    pub profile: super::RuntimeProfile,
    pub materialization: std::sync::Arc<super::VerifiedBundle>,
    pub envelope: super::AdmissionEnvelope,
    pub facts: super::ArtifactFacts,
    pub import_manifest: super::DeclaredManifest,
    pub import_grants: Vec<super::ImportGrant>,
}

/// Current authority grants are provided for each turn, never inferred from
/// artifact possession or copied from a previous invocation.
pub struct ActorInvocationInput<'a> {
    pub input: &'a preserves::IOValue,
    pub requested_profile: super::RequestedExecutionProfile,
    pub import_grants: &'a [super::ImportGrant],
}

pub struct ActorInvocation {
    pub outcome: super::ExecutionOutcome,
    pub host_recording: host::Recording,
}

/// Installation keeps the exact verified bytes, measured facts and declared
/// import surface together. None can be replaced after admission.
pub struct Actor {
    engine: super::Engine,
    profile: super::RuntimeProfile,
    materialization: std::sync::Arc<super::VerifiedBundle>,
    envelope: super::AdmissionEnvelope,
    facts: super::ArtifactFacts,
    import_manifest: super::DeclaredManifest,
}

impl Actor {
    // r[impl molten.wasm_component_adoption.boundary]
    pub fn install(input: ActorInstallInput) -> super::Result<Self> {
        super::validate_runtime(&input.profile)?;
        super::admit_requested_profile(&input.profile, super::RequestedExecutionProfile::ComponentV1)?;
        super::classify_for_profile(
            super::RequestedExecutionProfile::ComponentV1,
            input.materialization.component_bytes(),
        )?;
        let source = input.materialization.source(&input.envelope, super::Consumer::Actor);
        let admission = super::verify(&input.profile, super::EvidenceScope::Production, source)?;
        super::admission::inspection::verify_artifact_facts(source.component_bytes(), &input.facts)?;
        let observation = super::observe_artifact_surface(
            &input.facts.declared_world,
            &admission.component_ref,
            source.component_bytes(),
        )?;
        let verdict =
            super::admit_declared(&input.import_manifest, &super::declared_world(&input.profile), &observation)?;
        if !verdict.is_admitted {
            return Err(super::Denial::from_blockers(verdict.blockers));
        }
        super::plan_execution(&input.profile, admission, &input.import_manifest, &input.facts, &input.import_grants)?;
        let engine = super::Engine::build(&input.profile)?;
        Ok(Self {
            engine,
            profile: input.profile,
            materialization: input.materialization,
            envelope: input.envelope,
            facts: input.facts,
            import_manifest: input.import_manifest,
        })
    }

    pub fn profile(&self) -> &super::RuntimeProfile {
        &self.profile
    }

    pub fn materialization(&self) -> &super::VerifiedBundle {
        &self.materialization
    }

    pub fn facts(&self) -> &super::ArtifactFacts {
        &self.facts
    }

    /// Pure turns still pass the complete request admission pipeline; an effect
    /// import cannot acquire a host by taking this path.
    pub fn invoke(&self, input: ActorInvocationInput<'_>) -> ActorInvocation {
        ActorInvocation {
            outcome: self.engine.execute(&self.request(&input)),
            host_recording: host::Recording::empty(),
        }
    }

    pub fn invoke_live<C, V>(
        &self,
        input: ActorInvocationInput<'_>,
        live_input: host::LiveInput<C, V>,
    ) -> super::Result<ActorInvocation>
    where
        C: host::CurrentContext + 'static,
        V: crate::system_extension::NativeCallbackValuePort + 'static,
    {
        let (effect_host, host_recording) = host::Live::new(
            live_input.scope,
            live_input.generation,
            live_input.incarnation,
            live_input.current,
            live_input.values,
        )?;
        Ok(ActorInvocation {
            outcome: self.engine.execute_hosted(&self.request(&input), effect_host),
            host_recording,
        })
    }

    /// Offline replay accepts no live services. Missing, changed and extra host
    /// boundaries deny rather than substituting a current materialization.
    pub fn replay(
        &self,
        input: ActorInvocationInput<'_>,
        scope: super::host::Scope,
        generation: u64,
        incarnation: u64,
        effects: Vec<host::RecordedEffect>,
    ) -> super::Result<ActorInvocation> {
        let (effect_host, host_recording) = host::Replay::new(scope, generation, incarnation, effects)?;
        let request = self.request(&input);
        let mut outcome = self.engine.execute_hosted(&request, effect_host);
        if outcome.is_pass()
            && let Err(denial) = host_recording.ensure_replay_complete()
        {
            outcome = super::runtime::denial::deny_completed_execution(outcome, denial);
        }
        Ok(ActorInvocation {
            outcome,
            host_recording,
        })
    }

    fn request<'a>(&'a self, input: &'a ActorInvocationInput<'a>) -> super::ExecutionRequest<'a> {
        super::ExecutionRequest {
            profile: &self.profile,
            requested_profile: input.requested_profile,
            evidence_scope: super::EvidenceScope::Production,
            source: self.materialization.source(&self.envelope, super::Consumer::Actor),
            facts: &self.facts,
            import_manifest: &self.import_manifest,
            import_grants: input.import_grants,
            input: input.input,
        }
    }
}
