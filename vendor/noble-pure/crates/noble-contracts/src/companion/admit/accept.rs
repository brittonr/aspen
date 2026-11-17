const fn declaration_bytes(offer: &crate::companion::admit::EvidenceOffer) -> usize {
    match &offer.payload {
        crate::companion::admit::EvidencePayload::Declaration(bytes) => bytes.len(),
        crate::companion::admit::EvidencePayload::Resource(_)
        | crate::companion::admit::EvidencePayload::ServiceCapability(_) => 0,
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; eligible reads runtime Prepared accessors and uses non-const u32::try_from conversions before charging ingress work and checking ghost eligibility."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; eligible rejects impure payloads, unsupported semantics and unrepresentable or excessive work/bytes before ghost validation; untrusted offers must return typed refusals rather than panic."
)]
fn eligible(
    engine: &crate::companion::Core,
    prepared: &crate::Prepared,
    offer: &crate::companion::admit::EvidenceOffer,
) -> Result<(), crate::companion::Refusal> {
    attempt!(crate::companion::admit::check_evidence_purity(offer));
    if engine.semantic_revision != noble_kernel::untrusted::SEMANTIC_REVISION {
        return Err(crate::companion::Refusal::UnsupportedSemanticRevision);
    }
    let mut budget = crate::companion::Budget::new(engine.limits);
    let work = prepared
        .expressions()
        .len()
        .saturating_add(prepared.candidate().nodes.len())
        .saturating_add(declaration_bytes(offer))
        .saturating_add(1);
    let work = match u32::try_from(work) {
        Ok(work) => work,
        Err(_) => return Err(crate::companion::Refusal::ExhaustedRegistry),
    };
    attempt!(budget.charge(work, crate::companion::Refusal::ExhaustedRegistry,));
    let offered_bytes = match u32::try_from(declaration_bytes(offer)) {
        Ok(bytes) => bytes,
        Err(_) => return Err(crate::companion::Refusal::ExhaustedRegistry),
    };
    if engine.revision == u32::MAX || offered_bytes > engine.limits.bytes {
        return Err(crate::companion::Refusal::ExhaustedRegistry);
    }
    crate::companion::admit::check_ghost_eligibility(prepared)
}

/// Retain the exact consumer statement without creating evidence.
#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; registration is an explicit bounded mutation of the consumer-owned Core registry, never of Prepared or offered evidence."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; register preserves eligibility, program projection and registry failure order while retaining the exact Lean statement and interfaces; rejected offers and exhausted registries are typed failures."
)]
fn register(
    engine: &mut crate::companion::Core,
    prepared: &crate::Prepared,
    offer: &crate::companion::admit::EvidenceOffer,
) -> Result<(crate::companion::ContractId, u64), crate::companion::Refusal> {
    attempt!(eligible(engine, prepared, offer));
    let statement = crate::companion::statement_digest(prepared);
    let program = attempt!(crate::companion::admit::program::canonical_op_events(
        prepared.candidate()
    ));
    let template = crate::companion::admit::recognize::classify(prepared, statement, &program);
    let guards = crate::companion::guard::templates_of(prepared);
    let input_signature = crate::companion::admit::statement::interface_signature(
        &prepared.checked().interface.stack_in,
    );
    let output_signature = crate::companion::admit::statement::interface_signature(
        &prepared.checked().interface.stack_out,
    );
    let entry = crate::companion::registry::ContractEntry {
        statement,
        exact_statement: crate::export_lean(prepared),
        claim: template,
        policy: engine.policy,
        revision: engine.revision,
        guards,
        program,
        input_signature,
        output_signature,
        input: prepared.checked().interface.stack_in.clone(),
        output: prepared.checked().interface.stack_out.clone(),
    };
    let contract = attempt!(engine.registry.add_contract(entry));
    Ok((contract, statement))
}

/// Raw data alone never establishes proof authority.
#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; unproved registration is an explicit bounded Core state transition and never creates accepted evidence."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; unchecked propagates registration failures, returns Unknown for an empty declaration and ForgedStatus for raw proof bytes without minting evidence; arbitrary payloads must not trigger assertions."
)]
pub(crate) fn unchecked(
    engine: &mut crate::companion::Core,
    prepared: &crate::Prepared,
    offer: crate::companion::admit::EvidenceOffer,
) -> crate::companion::admit::Admission {
    let (contract, statement) = match register(engine, prepared, &offer) {
        Ok(binding) => binding,
        Err(refusal) => return registration_failure(refusal),
    };
    if declaration_bytes(&offer) == 0 {
        return crate::companion::admit::Admission {
            outcome: crate::companion::admit::Outcome::Unknown,
            contract: Some(contract),
            evidence: None,
            statement_digest: statement,
            refusals: alloc::vec::Vec::new(),
        };
    }
    crate::companion::admit::refused(
        crate::companion::admit::Outcome::Error,
        Some(contract),
        statement,
        crate::companion::Refusal::ForgedStatus,
    )
}

/// Create an opaque request after deterministic checks; no callback or IO occurs.
#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; request creation only retains a bounded contract in the consumer-owned registry; borrowed Prepared and offered source remain immutable."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; begin preserves empty-offer outcomes, registration refusals and class/refutation rejection before forming a request; malformed offers are ordinary admission outcomes, not panic conditions."
)]
pub(crate) fn begin<'a>(
    engine: &mut crate::companion::Core,
    expected: &'a crate::Prepared,
    offer: crate::companion::admit::EvidenceOffer,
) -> Result<crate::companion::admit::AdmissionRequest<'a>, crate::companion::admit::Admission> {
    if declaration_bytes(&offer) == 0 {
        return Err(unchecked(engine, expected, offer));
    }
    let (contract, statement) = match register(engine, expected, &offer) {
        Ok(binding) => binding,
        Err(refusal) => return Err(registration_failure(refusal)),
    };
    if let Err(refusal) = check_class(&offer) {
        let outcome = if refusal == crate::companion::Refusal::UnsupportedEvidenceClass {
            crate::companion::admit::Outcome::Unsupported
        } else {
            crate::companion::admit::Outcome::Error
        };
        return Err(crate::companion::admit::refused(
            outcome,
            Some(contract),
            statement,
            refusal,
        ));
    }
    Ok(crate::companion::admit::AdmissionRequest {
        expected,
        offer,
        contract,
        statement,
        policy: engine.policy,
        revision: engine.revision,
    })
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; check_class uses EvidenceClass's non-const derived PartialEq to bind refutation polarity after rejecting unsupported classes."
)]
fn check_class(
    offer: &crate::companion::admit::EvidenceOffer,
) -> Result<(), crate::companion::Refusal> {
    let is_supported = match offer.class {
        crate::companion::EvidenceClass::LeanExact
        | crate::companion::EvidenceClass::LeanRefutation => true,
        crate::companion::EvidenceClass::Replay | crate::companion::EvidenceClass::Assumption => {
            false
        }
    };
    if !is_supported {
        return Err(crate::companion::Refusal::UnsupportedEvidenceClass);
    }
    if offer.refutation != (offer.class == crate::companion::EvidenceClass::LeanRefutation) {
        return Err(crate::companion::Refusal::WrongPremiseClass);
    }
    Ok(())
}

#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; this private continuation retains an independently checked result in the operation-owned Core registry."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; mint checks retained-contract lookup and bounded evidence insertion before exposing an EvidenceId; registry failures return Error with the actual refusal rather than asserting successful retention."
)]
pub(super) fn mint(
    engine: &mut crate::companion::Core,
    contract: crate::companion::ContractId,
    statement: u64,
    offer: &crate::companion::admit::EvidenceOffer,
) -> crate::companion::admit::Admission {
    let outcome = checked_outcome(offer);
    let retained = match engine.registry.contract(contract) {
        Ok(retained) => retained,
        Err(refusal) => {
            return crate::companion::admit::refused(
                crate::companion::admit::Outcome::Error,
                Some(contract),
                statement,
                refusal,
            )
        }
    };
    let entry = crate::companion::registry::EvidenceEntry {
        contract,
        statement,
        template: retained.claim,
        subject: interface_subject(retained),
        class: offer.class,
        outcome,
        policy: engine.policy,
        revision: engine.revision,
        premises: alloc::vec::Vec::new(),
        rule: None,
    };
    let evidence = match engine.registry.add_evidence(entry) {
        Ok(id) => id,
        Err(refusal) => {
            return crate::companion::admit::refused(
                crate::companion::admit::Outcome::Error,
                Some(contract),
                statement,
                refusal,
            )
        }
    };
    crate::companion::admit::Admission {
        outcome,
        contract: Some(contract),
        evidence: Some(evidence),
        statement_digest: statement,
        refusals: alloc::vec::Vec::new(),
    }
}

const fn checked_outcome(
    offer: &crate::companion::admit::EvidenceOffer,
) -> crate::companion::admit::Outcome {
    if offer.refutation {
        crate::companion::admit::Outcome::Disproved
    } else {
        crate::companion::admit::Outcome::Proved
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; registration_failure uses Refusal's non-const derived equality to select Unsupported, then allocates the owned refusal vector through refused."
)]
fn registration_failure(refusal: crate::companion::Refusal) -> crate::companion::admit::Admission {
    let outcome = if refusal == crate::companion::Refusal::UnsupportedSemanticRevision {
        crate::companion::admit::Outcome::Unsupported
    } else {
        crate::companion::admit::Outcome::Error
    };
    crate::companion::admit::refused(outcome, None, 0, refusal)
}

/// Use the already retained exact descriptor, not a second template/program
/// reconstruction. Digest equality alone is never used for applicability.
fn interface_subject(
    contract: &crate::companion::registry::ContractEntry,
) -> crate::companion::Subject {
    crate::companion::Subject {
        identity: crate::companion::SubjectDigest(contract.statement),
        input_signature: contract.input_signature,
        output_signature: contract.output_signature,
        captures: alloc::vec::Vec::new(),
        events: contract.program.clone(),
    }
}
