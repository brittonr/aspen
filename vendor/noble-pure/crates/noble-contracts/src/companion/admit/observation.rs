//! The deterministic core consumes complete observations; host checking happens
//! outside it. Authentic independent-check observations are an explicit premise.

impl<'a> crate::companion::admit::AdmissionRequest<'a> {
    pub fn expected(&self) -> &'a crate::Prepared {
        self.expected
    }

    pub fn offer(&self) -> &crate::companion::admit::EvidenceOffer {
        &self.offer
    }
}

impl crate::companion::admit::CheckObservation {
    /// Normalize an actual trusted-host independent check of these exact inputs.
    /// A successful class is justified only by the complete consumer check of
    /// declaration/type/axioms, never by producer status or process exit alone.
    pub fn new(
        checked_statement: alloc::string::String,
        checked_source: alloc::vec::Vec<u8>,
        result: Result<crate::companion::EvidenceClass, crate::companion::Refusal>,
    ) -> Self {
        Self {
            checked_statement,
            checked_source,
            result,
        }
    }
}

#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; completion is an explicit bounded registry transition after immutable request/observation validation; no host callback or IO occurs."
)]
pub(crate) fn complete(
    engine: &mut crate::companion::Core,
    request: crate::companion::admit::AdmissionRequest<'_>,
    observation: crate::companion::admit::CheckObservation,
) -> crate::companion::admit::Admission {
    if let Err(refusal) = applicable(engine, &request, &observation) {
        return crate::companion::admit::refused(
            crate::companion::Outcome::Error,
            Some(request.contract),
            request.statement,
            refusal,
        );
    }
    crate::companion::admit::accept::mint(
        engine,
        request.contract,
        request.statement,
        &request.offer,
    )
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; applicable performs runtime registry lookup, derived EvidenceClass equality and String/Vec equality to bind the observation to the exact retained statement and offered source."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; applicable rejects stale contexts, propagates negative observations, charges comparison work and checks class/statement/source in order; invalid observations must return typed refusals without asserting authenticity."
)]
fn applicable(
    engine: &crate::companion::Core,
    request: &crate::companion::admit::AdmissionRequest<'_>,
    observation: &crate::companion::admit::CheckObservation,
) -> Result<(), crate::companion::Refusal> {
    let requested_context = crate::companion::registry::ContextSnapshot {
        policy: request.policy,
        revision: request.revision,
    };
    if crate::companion::registry::Registry::stale(engine, requested_context) {
        return Err(crate::companion::Refusal::StaleContext);
    }
    let retained = attempt!(engine.registry.contract(request.contract));
    let retained_context = crate::companion::registry::ContextSnapshot {
        policy: retained.policy,
        revision: retained.revision,
    };
    if crate::companion::registry::Registry::stale(engine, retained_context) {
        return Err(crate::companion::Refusal::StaleContext);
    }
    // Negative observations publish no authority and retain their actual refusal.
    let class = attempt!(observation.result);
    attempt!(charge(engine.limits, observation));
    if class != request.offer.class {
        return Err(crate::companion::Refusal::WrongPremiseClass);
    }
    if request.statement != retained.statement
        || observation.checked_statement != retained.exact_statement
    {
        return Err(crate::companion::Refusal::MismatchedClaim);
    }
    match &request.offer.payload {
        crate::companion::EvidencePayload::Declaration(bytes) => {
            if observation.checked_source != *bytes {
                return Err(crate::companion::Refusal::MismatchedClaim);
            }
        }
        crate::companion::EvidencePayload::Resource(_)
        | crate::companion::EvidencePayload::ServiceCapability(_) => {
            return Err(crate::companion::Refusal::LiveCapabilityInEvidence);
        }
    }
    Ok(())
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; charge uses non-const u32::try_from conversions of the actual checked-source and statement lengths before checked work charging."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; charge checks length representability, each artifact's byte limit and sum overflow before budget subtraction; oversized checker observations return ExhaustedRegistry rather than panic."
)]
fn charge(
    limits: crate::Limits,
    observation: &crate::companion::admit::CheckObservation,
) -> Result<(), crate::companion::Refusal> {
    let source_bytes = match u32::try_from(observation.checked_source.len()) {
        Ok(count) => count,
        Err(_) => return Err(crate::companion::Refusal::ExhaustedRegistry),
    };
    let statement_bytes = match u32::try_from(observation.checked_statement.len()) {
        Ok(count) => count,
        Err(_) => return Err(crate::companion::Refusal::ExhaustedRegistry),
    };
    if source_bytes > limits.bytes || statement_bytes > limits.bytes {
        return Err(crate::companion::Refusal::ExhaustedRegistry);
    }
    let work = match source_bytes.checked_add(statement_bytes) {
        Some(work) => work,
        None => return Err(crate::companion::Refusal::ExhaustedRegistry),
    };
    let work = match work.checked_add(1) {
        Some(work) => work,
        None => return Err(crate::companion::Refusal::ExhaustedRegistry),
    };
    let mut budget = crate::companion::Budget::new(limits);
    budget.charge(work, crate::companion::Refusal::ExhaustedRegistry)
}
