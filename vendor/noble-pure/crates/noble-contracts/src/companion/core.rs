mod context;
pub(crate) mod refusal;

#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; mutating methods are explicit consumer-owned Core registry/context transitions, never mutations of borrowed Prepared or guest subject values; proof checking remains the declared trusted external boundary"
)]
impl crate::companion::Core {
    /// Register an unproved contract or refuse unchecked evidence. Raw bytes
    /// and serialized status never create evidence. Actual proof admission
    /// uses a request and complete observation from the trusted host checker.
    pub fn admit(
        &mut self,
        prepared: &crate::Prepared,
        offer: crate::companion::admit::EvidenceOffer,
    ) -> crate::companion::admit::Admission {
        crate::companion::admit::accept::unchecked(self, prepared, offer)
    }

    /// Retain an exact request without doing host IO or invoking a callback.
    /// Refused and empty offers return their ordinary unproved admission.
    pub fn begin_admission<'a>(
        &mut self,
        expected: &'a crate::Prepared,
        offer: crate::companion::admit::EvidenceOffer,
    ) -> Result<crate::companion::admit::AdmissionRequest<'a>, crate::companion::admit::Admission>
    {
        crate::companion::admit::accept::begin(self, expected, offer)
    }

    /// Decide from complete authentic host observations, not an effectful
    /// verifier. Changed context or exact input bindings refuse completion.
    pub fn complete_admission(
        &mut self,
        request: crate::companion::admit::AdmissionRequest<'_>,
        observation: crate::companion::admit::CheckObservation,
    ) -> crate::companion::admit::Admission {
        crate::companion::admit::observation::complete(self, request, observation)
    }

    /// Replay a finite, acyclic derivation against the registry.
    pub fn replay(
        &mut self,
        derivation: &crate::companion::rules::Derivation,
    ) -> Result<crate::companion::registry::EvidenceId, crate::companion::Refusal> {
        crate::companion::rules::replay(self, derivation)
    }

    /// Certified composition of two template-representable premises. The
    /// prechecked intermediate implication is supplied internally; the
    /// observed subject must be the plain-compose concatenation of the
    /// premise subjects.
    pub fn derive_compose(
        &mut self,
        left: crate::companion::registry::EvidenceId,
        right: crate::companion::registry::EvidenceId,
        subject: &crate::companion::Subject,
    ) -> Result<crate::companion::rules::Derived, crate::companion::Refusal> {
        crate::companion::rules::derive::compose(self, left, right, subject)
    }

    /// Bind a runtime capture into a returned program family.
    pub fn instantiate(
        &mut self,
        family: crate::companion::registry::EvidenceId,
        capture: i64,
        subject: &crate::companion::Subject,
    ) -> Result<crate::companion::rules::Derived, crate::companion::Refusal> {
        crate::companion::rules::derive::instantiate(self, family, capture, subject)
    }

    /// Check that an evidence entry applies to exactly this subject under the
    /// current policy. Returns the contract-bound canonical subject (whose
    /// identity is the retained statement digest) so a certified value can
    /// carry the contract's identity rather than the raw observation's.
    pub fn bind(
        &self,
        evidence: crate::companion::registry::EvidenceId,
        subject: &crate::companion::Subject,
    ) -> Result<crate::companion::Subject, crate::companion::Refusal> {
        crate::companion::registry::application::bind(self, evidence, subject)
    }

    /// Bind exact proved evidence to an independently checked ordinary source
    /// program. The supplied submission is rechecked under its frozen request,
    /// and its canonical operation subject must match the evidence.
    pub fn bind_checked_program(
        &self,
        evidence: crate::companion::registry::EvidenceId,
        submission: &noble_kernel::execution::Submission,
    ) -> Result<crate::companion::Subject, crate::companion::Refusal> {
        crate::companion::release::bind_checked_program(self, evidence, submission)
    }

    /// The derivation rule that minted an evidence entry, if any.
    pub fn evidence_rule(
        &self,
        evidence: crate::companion::registry::EvidenceId,
    ) -> Result<Option<crate::companion::rules::RuleId>, crate::companion::Refusal> {
        let entry = attempt!(self.registry.evidence(evidence));
        Ok(entry.rule)
    }

    /// The recomputed claim of a current proved evidence entry.
    pub fn claim(
        &self,
        evidence: crate::companion::registry::EvidenceId,
    ) -> Result<crate::companion::admit::ClaimTemplate, crate::companion::Refusal> {
        let entry = attempt!(crate::companion::registry::application::proved(
            self, evidence
        ));
        Ok(entry.template)
    }

    /// Fold complete observed recipe events into a subject identity. Total and
    /// bounded: at most [`crate::companion::subject::OBSERVATION_CAP`] events are folded.
    pub fn observe(
        events: &[(u32, u64)],
        signatures: crate::companion::InterfaceSignatures,
    ) -> crate::companion::Subject {
        crate::companion::subject::observe(events, signatures)
    }

    /// Observe and additionally cache the (bounded) events in the registry, so
    /// later composition can be checked without re-running the guest.
    pub fn observe_and_register(
        &mut self,
        events: &[(u32, u64)],
        signatures: crate::companion::InterfaceSignatures,
    ) -> Result<crate::companion::Subject, crate::companion::Refusal> {
        let subject = crate::companion::subject::observe(events, signatures);
        attempt!(self.registry.record_observation(&subject, events));
        Ok(subject)
    }

    /// The cached bounded observation events for a subject identity, if the
    /// subject was registered with `observe_and_register`.
    pub fn cached_events(&self, subject: crate::companion::SubjectDigest) -> Option<&[(u32, u64)]> {
        self.registry.cached_events(subject)
    }

    /// The plain-compose observation relation. Returns `UnresolvedImplication`
    /// when the intermediate implication premise is absent, i.e. when the left
    /// output interface does not join the right input interface.
    pub fn compose_subject(
        left: &crate::companion::Subject,
        right: &crate::companion::Subject,
    ) -> Result<crate::companion::Subject, crate::companion::Refusal> {
        crate::companion::compose_subject(left, right)
    }

    /// The finite prechecked guard templates whose predicate shape the
    /// contract's precondition matches.
    pub fn guard_templates(
        &self,
        contract: crate::companion::registry::ContractId,
    ) -> Result<alloc::vec::Vec<crate::companion::GuardTemplate>, crate::companion::Refusal> {
        crate::companion::guard::templates(self, contract)
    }

    /// Guards of the current evidence's actual conclusion descriptor, never
    /// the originating builder's or composition premise's precondition.
    pub fn evidence_guard_templates(
        &self,
        evidence: crate::companion::registry::EvidenceId,
    ) -> Result<alloc::vec::Vec<crate::companion::GuardTemplate>, crate::companion::Refusal> {
        let entry = attempt!(crate::companion::registry::application::proved(
            self, evidence
        ));
        crate::companion::guard::templates(self, entry.contract)
    }

    /// Render the exact wrapper program text for a guard template around the
    /// supplied subject program text.
    pub fn wrapper_source(
        &self,
        template: crate::companion::GuardTemplate,
        subject: &str,
    ) -> alloc::string::String {
        crate::companion::guard::wrapper_source(self, template, subject)
    }

    /// Wrap an existing `[I64, Program]` pair without rebuilding its subject.
    pub fn invocation_wrapper(
        &self,
        template: crate::companion::GuardTemplate,
    ) -> alloc::string::String {
        crate::companion::guard::invocation_wrapper(template)
    }

    /// Proof-required build release (VC-TOOL-04): only an applicable accepted
    /// `proved` claim for the exact subject/claim/context/policy releases. The
    /// caller supplies the observed subject; a claim about any other subject
    /// refuses with `MismatchedSubject` rather than releasing it.
    pub fn release(
        &self,
        evidence: crate::companion::registry::EvidenceId,
        subject: &crate::companion::Subject,
    ) -> Result<crate::companion::Release, crate::companion::Refusal> {
        crate::companion::release::select(self, evidence, subject)
    }

    /// Check that an evidence entry corresponds to a selected contract
    /// (CONTRACT-13): a same-interface but different subject refuses with
    /// `CorrespondenceMismatch`.
    pub fn correspond(
        &self,
        evidence: crate::companion::registry::EvidenceId,
        contract: crate::companion::registry::ContractId,
    ) -> Result<(), crate::companion::Refusal> {
        crate::companion::release::correspond(self, evidence, contract)
    }

    /// Independently check a closed single-quotation artifact submission and
    /// bind its exact quotation body and interface to the accepted evidence.
    pub fn bind_artifact(
        &self,
        evidence: crate::companion::registry::EvidenceId,
        submission: &noble_kernel::execution::Submission,
    ) -> Result<crate::companion::Subject, crate::companion::Refusal> {
        crate::companion::release::bind_artifact(self, evidence, submission)
    }

    /// The contract program's retained canonical op-event list. `bind`
    /// compares observed recipe events against exactly this list; the driver
    /// can use it to predict applicability.
    pub fn contract_program(
        &self,
        contract: crate::companion::registry::ContractId,
    ) -> Result<&[(u32, u64)], crate::companion::Refusal> {
        let entry = attempt!(self.registry.contract(contract));
        Ok(&entry.program)
    }
}
