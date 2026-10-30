use super::*;

pub struct Application {
    pub(super) store: Database,
    pub(super) state: State,
    pub(super) reference: StateRef,
    pub(super) values: Box<dyn crate::system_extension::NativeCallbackValuePort>,
}

impl Application {
    pub fn open(
        store: Database,
        values: Box<dyn crate::system_extension::NativeCallbackValuePort>,
    ) -> crate::error::Result<Self> {
        let state = store.load()?.ok_or_else(|| denied(Denial::Missing))?;
        let reference = validated_state_ref(&state)?;
        Ok(Self {
            store,
            state,
            reference,
            values,
        })
    }
    pub const fn state(&self) -> &State {
        &self.state
    }
    pub const fn state_ref(&self) -> &StateRef {
        &self.reference
    }

    pub fn reload_durable(&mut self) -> crate::error::Result<()> {
        let state = self.store.load()?.ok_or_else(|| denied(Denial::Missing))?;
        let reference = validated_state_ref(&state)?;
        self.state = state;
        self.reference = reference;
        Ok(())
    }

    pub fn inject_simulated_fault(
        &mut self,
        fault: crate::fabric_durability::SimulatedFault,
    ) -> crate::error::Result<()> {
        self.store.inject_simulated_fault(fault)
    }

    pub fn validate_staged(&self, command: &crate::fabric_consistency::raft::Command) -> crate::error::Result<()> {
        if command.command_schema_ref != command_schema_ref()? {
            return Err(denied(Denial::Domain));
        }
        let value = self.store.command(&command.command_ref)?;
        let (request, decisions) = command::parse_proposal(&value)?;
        if request.reference.as_str() != command.request_ref {
            return Err(denied(Denial::Domain));
        }
        preview_mutation(&self.state, &self.reference, &request, &decisions).map_err(denied).map(|_| ())
    }

    pub(super) fn materialize_command(&mut self, reference: &str) -> crate::error::Result<preserves::IOValue> {
        if self.store.has_command(reference)? {
            return self.store.command(reference);
        }
        let materialized =
            self.values.materialize(reference, MAX_RECORD_BYTES).map_err(|_| denied(Denial::Uncertain))?;
        if materialized.value_ref != reference
            || crate::preserves_rail::content_ref_from_bytes(&materialized.bytes) != reference
        {
            return Err(denied(Denial::Domain));
        }
        let value = crate::preserves_rail::parse_canonical_bytes(&materialized.bytes)?;
        command::parse_proposal(&value)?;
        self.store.stage(&value)?;
        Ok(value)
    }

    pub(super) fn publish_value(&mut self, value: &preserves::IOValue) -> crate::error::Result<()> {
        let bytes = crate::preserves_rail::canonical_bytes(value)?;
        let value_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
        self.values
            .publish(&crate::system_extension::NativeCallbackValue { value_ref, bytes }, MAX_RECORD_BYTES)
            .map_err(|_| denied(Denial::Uncertain))?;
        Ok(())
    }

    /// This is the admission boundary, not an arbitrary command import. Each
    /// replica stages the exact request through its admitted authority services.
    // r[impl molten.collaboration_scope.authority_snapshot]
    pub fn stage_mutation(
        &mut self,
        input: MutationInput<'_>,
    ) -> crate::error::Result<crate::fabric_consistency::raft::Command> {
        self.store.ensure_serving()?;
        let MutationInput {
            request,
            current,
            policies,
            credential,
            services,
        } = input;
        if current.state != self.reference || current.currentness != request.currentness {
            return Err(denied(Denial::Stale));
        }
        if mutation_request_ref(request)? != request.reference {
            return Err(denied(Denial::Domain));
        }
        let required = mutation_policies(&self.state, &request.mutation).map_err(denied)?;
        if required.len() != policies.len()
            || required.iter().any(|policy| !policies.iter().any(|input| &input.reference == policy))
        {
            return Err(denied(Denial::Authority));
        }
        let resource = mutation_resource(&request.mutation)?;
        let input = DecisionInput {
            request: &request.reference,
            subject: &request.actor,
            resource: &resource,
            ability: request.mutation.ability(),
            currentness: &request.currentness,
            consent: None,
            policies,
        };
        let decisions = verify_decisions(&input, credential, services)?;
        preview_mutation(&self.state, &self.reference, request, &decisions).map_err(denied)?;
        let value = command::proposal_value(request, &decisions);
        let command_ref = self.store.stage(&value)?;
        let bytes = crate::preserves_rail::canonical_bytes(&value)?;
        self.values
            .publish(
                &crate::system_extension::NativeCallbackValue {
                    value_ref: command_ref.clone(),
                    bytes,
                },
                MAX_RECORD_BYTES,
            )
            .map_err(|_| denied(Denial::Uncertain))?;
        Ok(crate::fabric_consistency::raft::Command {
            request_ref: request.reference.as_str().to_string(),
            command_ref,
            command_schema_ref: command_schema_ref()?,
        })
    }

    /// No protected consumer effect is performed here. The consumer must obtain
    /// another current observation and exact authorization immediately before dispatch.
    pub fn admit_snapshot(&self, input: SnapshotInput<'_>) -> crate::error::Result<ScopeAdmissionSnapshot> {
        self.store.ensure_serving()?;
        let SnapshotInput {
            request,
            current,
            policies,
            credentials,
            services,
        } = input;
        if credentials.is_empty()
            || credentials.len() > MAX_MEMBERS
            || request.audience.len() > MAX_MEMBERS
            || policies.len() > 5
        {
            return Err(denied(Denial::Bound));
        }
        if current.state != self.reference
            || current.currentness != request.currentness
            || audience_request_ref(request)? != request.reference
        {
            return Err(denied(Denial::Stale));
        }
        let required = required_policies(&self.state, request).map_err(denied)?;
        if required.len() != policies.len()
            || required.iter().any(|policy| !policies.iter().any(|input| &input.reference == *policy))
            || credentials.len() != request.audience.len()
        {
            return Err(denied(Denial::Authority));
        }
        let mut decisions = Vec::with_capacity(credentials.len() * required.len());
        let mut subjects = std::collections::BTreeSet::new();
        for credential in credentials {
            let subject = subject_ref(credential.issuer, credential.holder)?;
            if !request.audience.contains(&subject) || !subjects.insert(subject.clone()) {
                return Err(denied(Denial::Authority));
            }
            decisions.extend(verify_decisions(
                &DecisionInput {
                    request: &request.reference,
                    subject: &subject,
                    resource: &request.resource,
                    ability: request.ability,
                    currentness: &request.currentness,
                    consent: request.consent.clone(),
                    policies,
                },
                credential,
                services,
            )?);
        }
        decisions.sort_by(|left, right| (&left.subject, &left.policy).cmp(&(&right.subject, &right.policy)));
        admit_audience(&self.state, &self.reference, request, &decisions).map_err(denied)
    }

    pub fn status(&self) -> preserves::IOValue {
        crate::preserves_rail::record("collaboration-operator-status-v1", vec![
            status_value(&self.state, &self.reference),
            crate::preserves_rail::string(if self.store.ensure_serving().is_ok() {
                "loaded-state-not-current-authority"
            } else {
                "storage-uncertain-no-admission"
            }),
        ])
    }
}

pub fn command_schema_ref() -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("collaboration-command-schema-v1", vec![
        crate::preserves_rail::string(SCHEMA),
    ]))
}

pub fn mutation_resource(mutation: &Mutation) -> crate::error::Result<molten_core::nominal::ResourceRef> {
    let reference = match mutation {
        Mutation::Create(scope) => scope.reference.as_str(),
        Mutation::PutMember { scope, .. }
        | Mutation::RemoveMember { scope, .. }
        | Mutation::ExpireMember { scope, .. } => scope.as_str(),
        Mutation::Bind(binding) => binding.resource.as_str(),
        Mutation::Move { resource, .. } => resource.as_str(),
    };
    molten_core::nominal::ResourceRef::new(reference).map_err(|_| denied(Denial::Domain))
}

pub struct MutationInput<'a> {
    pub request: &'a MutationRequest,
    pub current: CurrentState,
    pub policies: &'a [PolicyInput<'a>],
    pub credential: &'a HolderCredential<'a>,
    pub services: &'a basalt::ucan_enforcement::UcanEnforcementBoundaries<'a>,
}

pub struct SnapshotInput<'a> {
    pub request: &'a AudienceRequest,
    pub current: CurrentState,
    pub policies: &'a [PolicyInput<'a>],
    pub credentials: &'a [HolderCredential<'a>],
    pub services: &'a basalt::ucan_enforcement::UcanEnforcementBoundaries<'a>,
}
