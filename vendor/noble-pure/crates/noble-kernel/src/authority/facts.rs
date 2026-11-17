const MAX_REQUESTS: u32 = 4096;

impl super::Authority {
    /// Only the trusted shell constructs a table. The context must be unique,
    /// sources must designate authenticated boundaries, and all calls must be
    /// serialized with the shell's authority/quota synchronization protocol.
    pub fn new(profile: super::Profile) -> Result<Self, super::SetupError> {
        let limits = profile.limits;
        if limits.witnesses > super::MAX_RECORDS {
            return Err(super::SetupError::InvalidLimits);
        }
        if limits.attempts > super::MAX_RECORDS {
            return Err(super::SetupError::InvalidLimits);
        }
        if limits.requests == 0 {
            return Err(super::SetupError::InvalidLimits);
        }
        if limits.requests > MAX_REQUESTS {
            return Err(super::SetupError::InvalidLimits);
        }
        let requests = match usize::try_from(limits.requests) {
            Ok(requests) => requests,
            Err(_) => return Err(super::SetupError::InvalidLimits),
        };
        Ok(Self {
            profile,
            witnesses: alloc::vec::Vec::with_capacity(limits.witnesses),
            attempts: alloc::vec::Vec::with_capacity(limits.attempts),
            quotas: alloc::vec::Vec::with_capacity(limits.attempts),
            effects: alloc::vec::Vec::with_capacity(requests),
            counters: super::Counters {
                authorization_requests: 0,
                admission_requests: 0,
                witnesses_created: 0,
                witness_consumptions: 0,
                attempts_admitted: 0,
                protected_operations: 0,
                approved_observations: 0,
                successful_deliveries: 0,
            },
            invocation: super::InvocationOutcome::Pending,
            delivered: None,
        })
    }

    /// The shell acquires fresh facts at this point and must include previous
    /// admissions in the new quota snapshot. Time cannot move backwards and a
    /// sequence is never reused. Advancing does not undo any spent witness.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; the explicit owned synchronization transition checks sequence and time monotonicity before replacing the local checkpoint; it never reads ambient time or changes external facts."
    )]
    pub fn advance_from_trusted_host(
        &mut self,
        checkpoint: super::Checkpoint,
    ) -> Result<(), super::SetupError> {
        if checkpoint.sequence <= self.profile.checkpoint.sequence {
            return Err(super::SetupError::StaleCheckpoint);
        }
        if checkpoint.now < self.profile.checkpoint.now {
            return Err(super::SetupError::StaleCheckpoint);
        }
        self.profile.checkpoint = checkpoint;
        Ok(())
    }

    /// A deterministic policy decision is data, not a witness constructor.
    pub fn decide(&self, plan: &super::Plan, facts: &super::CurrentFacts) -> super::Decision {
        match self.validate_facts(plan, facts) {
            Ok(()) => super::Decision::Allow,
            Err(denial) => super::Decision::Deny(denial),
        }
    }

    /// TRUST PREMISE: `facts` came from the profile's authenticated authority
    /// boundary, independently of guest plans/allow records/digests. The shell
    /// must never expose this route as a constructor accepting guest facts.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; authorization checks current facts and reserved witness capacity before committing a move-only witness and counters to the exclusively owned table; denial creates no authority."
    )]
    pub fn authorize_from_trusted_host(
        &mut self,
        plan: super::Plan,
        facts: &super::CurrentFacts,
    ) -> super::Authorization {
        if !self.request_available() {
            return super::Authorization::Preflight(self.preflight(
                plan,
                super::PreflightFailure::RequestCapacity,
                super::ReceiptScope::AuthorizationBoundary,
            ));
        }
        self.counters.authorization_requests += 1;
        match self.validate_facts(&plan, facts) {
            Ok(()) => (),
            Err(denial) => {
                return super::Authorization::Denied(self.deny(
                    plan,
                    denial,
                    super::ReceiptScope::AuthorizationBoundary,
                ))
            }
        }
        if self.witnesses.len() >= self.profile.limits.witnesses {
            return super::Authorization::Preflight(self.preflight(
                plan,
                super::PreflightFailure::WitnessCapacity,
                super::ReceiptScope::AuthorizationBoundary,
            ));
        }
        let created = match self.counters.witnesses_created.checked_add(1) {
            Some(created) => created,
            None => {
                return super::Authorization::Preflight(self.preflight(
                    plan,
                    super::PreflightFailure::WitnessCapacity,
                    super::ReceiptScope::AuthorizationBoundary,
                ))
            }
        };
        let claim = super::WitnessClaim {
            owner: self.profile.owner,
            slot: self.witnesses.len(),
            generation: u64::from(created),
            kind: self.profile.witness_kind,
            rights: super::EXECUTE_RIGHT,
            scope: plan.description.constraints.scope,
        };
        self.witnesses.push(super::WitnessRecord {
            claim,
            plan,
            state: super::WitnessState::Live,
        });
        self.counters.witnesses_created = created;
        super::Authorization::Authorized(super::Witness { claim })
    }

    pub(super) fn request_available(&self) -> bool {
        match self
            .counters
            .authorization_requests
            .checked_add(self.counters.admission_requests)
        {
            Some(requests) => requests < self.profile.limits.requests,
            None => false,
        }
    }

    pub(super) fn validate_facts(
        &self,
        plan: &super::Plan,
        facts: &super::CurrentFacts,
    ) -> Result<(), super::Denial> {
        if self.invocation != super::InvocationOutcome::Pending {
            return Err(super::Denial::InvocationFinished);
        }
        if plan.description.owner != self.profile.owner {
            return Err(super::Denial::WrongOwnerContext);
        }
        if facts.source != self.profile.authority_source {
            return Err(super::Denial::WrongAuthoritySource);
        }
        if facts.checkpoint != self.profile.checkpoint {
            return Err(super::Denial::StaleFacts);
        }
        match self.validate_grant(plan, facts) {
            Ok(()) => (),
            Err(denial) => return Err(denial),
        }
        match current_status(plan, facts) {
            Ok(()) => (),
            Err(denial) => return Err(denial),
        }
        self.validate_quota(plan, facts)
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; exact grant binding uses runtime structural Plan equality, including bounded Vec argument and contract bytes; derived PartialEq is not const."
    )]
    fn validate_grant(
        &self,
        plan: &super::Plan,
        facts: &super::CurrentFacts,
    ) -> Result<(), super::Denial> {
        let grant = match &facts.grant {
            Some(grant) => grant,
            None => return Err(super::Denial::MissingFacts),
        };
        if grant.kind != self.profile.witness_kind {
            return Err(super::Denial::WrongResourceKind);
        }
        if grant.rights & super::EXECUTE_RIGHT == 0 {
            return Err(super::Denial::MissingRights);
        }
        if grant.plan != *plan {
            return Err(super::Denial::ChangedPlan);
        }
        Ok(())
    }

    pub(super) fn quota_position(&self, plan: &super::Plan) -> Option<usize> {
        let mut index = 0;
        while index < self.quotas.len() {
            let record = &self.quotas[index];
            if record.quota == plan.description.constraints.quota
                && record.checkpoint == self.profile.checkpoint
            {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; quota validation reads runtime Vec records at the current checkpoint; Vec access and derived checkpoint equality are not const."
    )]
    fn validate_quota(
        &self,
        plan: &super::Plan,
        facts: &super::CurrentFacts,
    ) -> Result<(), super::Denial> {
        let available = match facts.quota_remaining {
            Some(available) => available,
            None => return Err(super::Denial::MissingFacts),
        };
        let spent = match self.quota_position(plan) {
            Some(index) => {
                let record = &self.quotas[index];
                if record.available != available {
                    return Err(super::Denial::InconsistentQuota);
                }
                record.spent
            }
            None => 0,
        };
        match available.checked_sub(spent) {
            Some(remaining) if remaining >= plan.description.constraints.units => Ok(()),
            Some(_) | None => Err(super::Denial::QuotaExhausted),
        }
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; credentials, revocation, policy and validity are untrusted host-boundary facts with distinct denial results; assertions would replace specified fail-closed outcomes with panics."
)]
const fn current_status(
    plan: &super::Plan,
    facts: &super::CurrentFacts,
) -> Result<(), super::Denial> {
    match facts.credentials_valid {
        Some(true) => (),
        Some(false) => return Err(super::Denial::InvalidCredentials),
        None => return Err(super::Denial::MissingFacts),
    }
    match facts.revoked {
        Some(false) => (),
        Some(true) => return Err(super::Denial::Revoked),
        None => return Err(super::Denial::MissingFacts),
    }
    match facts.policy {
        Some(policy) if policy.0 == plan.description.policy.0 => (),
        Some(_) => return Err(super::Denial::ChangedPolicy),
        None => return Err(super::Denial::MissingFacts),
    }
    if facts.checkpoint.now < plan.description.constraints.not_before {
        return Err(super::Denial::NotYetValid);
    }
    if facts.checkpoint.now >= plan.description.constraints.expires_at {
        return Err(super::Denial::Expired);
    }
    Ok(())
}
