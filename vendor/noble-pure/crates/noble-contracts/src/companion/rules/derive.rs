#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; checked derivation transitions update only the Core-owned registry and fresh operation budget; immutable premises and observed subject are never mutated"
)]

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; composition requires two current proved increment premises, charges both complete subjects and compares the ordered composed observation before returning the wrapped increment; unsupported implications must refuse rather than panic."
)]
pub(super) fn composition(
    engine: &crate::companion::Core,
    left: crate::companion::registry::EvidenceId,
    right: crate::companion::registry::EvidenceId,
    subject: &crate::companion::Subject,
    budget: &mut crate::companion::Budget,
) -> Result<crate::companion::admit::ClaimTemplate, crate::companion::Refusal> {
    let left_entry = attempt!(crate::companion::registry::application::proved(
        engine, left
    ));
    let right_entry = attempt!(crate::companion::registry::application::proved(
        engine, right
    ));
    let a = match left_entry.template {
        crate::companion::admit::ClaimTemplate::IncrementBy(value) => value,
        crate::companion::admit::ClaimTemplate::Admitted(_)
        | crate::companion::admit::ClaimTemplate::GuardCorrespondence(_)
        | crate::companion::admit::ClaimTemplate::IncrementByCapture => {
            return Err(crate::companion::Refusal::UnresolvedImplication)
        }
    };
    let b = match right_entry.template {
        crate::companion::admit::ClaimTemplate::IncrementBy(value) => value,
        crate::companion::admit::ClaimTemplate::Admitted(_)
        | crate::companion::admit::ClaimTemplate::GuardCorrespondence(_)
        | crate::companion::admit::ClaimTemplate::IncrementByCapture => {
            return Err(crate::companion::Refusal::UnresolvedImplication)
        }
    };
    attempt!(crate::companion::rules::premises::subject_work(
        budget,
        &left_entry.subject
    ));
    attempt!(crate::companion::rules::premises::subject_work(
        budget,
        &right_entry.subject
    ));
    let expected = attempt!(crate::companion::compose_subject(
        &left_entry.subject,
        &right_entry.subject
    ));
    attempt!(crate::companion::registry::application::matches_observation(&expected, subject));
    Ok(crate::companion::admit::ClaimTemplate::IncrementBy(
        a.wrapping_add(b),
    ))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; instantiation checks the current proved capture family and the exact independently observed literal/add program before returning the concrete increment; wrong families or observations remain WrongInstantiation."
)]
pub(super) fn instantiation(
    engine: &crate::companion::Core,
    family: crate::companion::registry::EvidenceId,
    capture: i64,
    subject: &crate::companion::Subject,
) -> Result<crate::companion::admit::ClaimTemplate, crate::companion::Refusal> {
    let entry = attempt!(crate::companion::registry::application::proved(
        engine, family
    ));
    if entry.template != crate::companion::admit::ClaimTemplate::IncrementByCapture {
        return Err(crate::companion::Refusal::WrongInstantiation);
    }
    let value = u64::from_ne_bytes(capture.to_ne_bytes());
    let signature =
        crate::companion::admit::statement::interface_signature(&[noble_kernel::types::Ty::I64]);
    let expected = crate::companion::subject::observe(
        &[(1, value), (2, 4)],
        crate::companion::InterfaceSignatures {
            input: signature,
            output: signature,
        },
    );
    // The theorem is about the exact quote/[+]/compose family, not about
    // arbitrary programs that happen to mention its runtime capture.
    if crate::companion::registry::application::matches_observation(&expected, subject).is_err() {
        return Err(crate::companion::Refusal::WrongInstantiation);
    }
    Ok(crate::companion::admit::ClaimTemplate::IncrementBy(capture))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; insert retains the checked concrete conclusion, rejects duplicate exact evidence and propagates bounded registry exhaustion before returning its descriptor; these outcomes must not become assertion panics."
)]
pub(super) fn insert(
    engine: &mut crate::companion::Core,
    derivation: crate::companion::rules::Derivation,
    template: crate::companion::admit::ClaimTemplate,
) -> Result<crate::companion::rules::Derived, crate::companion::Refusal> {
    let contract = attempt!(conclusion(engine, &derivation, template));
    let statement = derivation.statement;
    if engine
        .registry
        .find_evidence(contract, statement, &derivation.subject)
        .is_some()
    {
        return Err(crate::companion::Refusal::DuplicateEntry);
    }
    let evidence =
        attempt!(engine
            .registry
            .add_evidence(crate::companion::registry::EvidenceEntry {
                contract,
                statement,
                template,
                subject: derivation.subject,
                class: crate::companion::registry::EvidenceClass::Replay,
                outcome: crate::companion::admit::Outcome::Proved,
                policy: engine.policy,
                revision: engine.revision,
                premises: derivation.premises,
                rule: Some(derivation.rule),
            }));
    Ok(crate::companion::rules::Derived {
        evidence,
        contract,
        statement_digest: statement,
    })
}

#[expect(
    clippy::vec_init_then_push,
    reason = "Owner: noble-maintainers; explicit pushes preserve the pinned extractor's owned Vec representation."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; conclusion preserves transport-rule descriptors and constructs concrete compose/instantiate program, interface and context descriptors through bounded registry insertion, where exhaustion is a typed refusal."
)]
fn conclusion(
    engine: &mut crate::companion::Core,
    derivation: &crate::companion::rules::Derivation,
    template: crate::companion::admit::ClaimTemplate,
) -> Result<crate::companion::ContractId, crate::companion::Refusal> {
    if !matches!(
        derivation.rule,
        crate::companion::rules::RuleId::ComposeV1 | crate::companion::rules::RuleId::InstantiateV1
    ) {
        return Ok(derivation.contract);
    }
    let mut input = alloc::vec::Vec::with_capacity(1);
    input.push(noble_kernel::types::Ty::I64);
    let output = input.clone();
    engine
        .registry
        .add_contract(crate::companion::registry::ContractEntry {
            statement: derivation.statement,
            exact_statement: alloc::string::String::new(),
            claim: template,
            policy: engine.policy,
            revision: engine.revision,
            guards: alloc::vec::Vec::new(),
            program: crate::companion::registry::application::op_stream(&derivation.subject.events),
            input_signature: derivation.subject.input_signature,
            output_signature: derivation.subject.output_signature,
            input,
            output,
        })
}

/// Ordinary composition may reuse one proved proposition twice. The typed
/// helper constructs exactly two premises itself; guest replay also rejects
/// duplicate serialized premise references.
#[expect(
    clippy::vec_init_then_push,
    reason = "Owner: noble-maintainers; explicit pushes preserve the pinned extractor's owned alloc::vec::Vec representation."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; compose charges the offered subject, validates its two ordered premises and recomputes the exact composition before insertion; one theorem may serve twice and all invalid premises retain typed refusals."
)]
pub(crate) fn compose(
    engine: &mut crate::companion::Core,
    left: crate::companion::registry::EvidenceId,
    right: crate::companion::registry::EvidenceId,
    subject: &crate::companion::Subject,
) -> Result<crate::companion::rules::Derived, crate::companion::Refusal> {
    let mut budget = crate::companion::Budget::new(engine.limits);
    attempt!(crate::companion::rules::premises::subject_work(
        &mut budget,
        subject
    ));
    let mut premises = alloc::vec::Vec::with_capacity(2);
    premises.push(left);
    premises.push(right);
    attempt!(crate::companion::rules::premises::validate(
        engine,
        &premises,
        &mut budget,
        false
    ));
    let template = attempt!(composition(engine, left, right, subject, &mut budget));
    let contract = attempt!(crate::companion::registry::application::proved(
        engine, left
    ))
    .contract;
    let statement = template.digest();
    insert(
        engine,
        crate::companion::rules::Derivation {
            contract,
            statement,
            subject: subject.clone(),
            premises,
            rule: crate::companion::rules::RuleId::ComposeV1,
        },
        template,
    )
}

#[expect(
    clippy::vec_init_then_push,
    reason = "Owner: noble-maintainers; explicit pushes preserve the pinned extractor's owned alloc::vec::Vec representation."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; instantiate charges the offered subject, validates the single family premise and checks the concrete capture observation before insertion; failed bounds, family checks or registry capacity remain typed refusals."
)]
pub(crate) fn instantiate(
    engine: &mut crate::companion::Core,
    family: crate::companion::registry::EvidenceId,
    capture: i64,
    subject: &crate::companion::Subject,
) -> Result<crate::companion::rules::Derived, crate::companion::Refusal> {
    let mut budget = crate::companion::Budget::new(engine.limits);
    attempt!(crate::companion::rules::premises::subject_work(
        &mut budget,
        subject
    ));
    let mut premises = alloc::vec::Vec::with_capacity(1);
    premises.push(family);
    attempt!(crate::companion::rules::premises::validate(
        engine,
        &premises,
        &mut budget,
        true
    ));
    let template = attempt!(instantiation(engine, family, capture, subject));
    let contract = attempt!(crate::companion::registry::application::proved(
        engine, family
    ))
    .contract;
    let statement = template.digest();
    insert(
        engine,
        crate::companion::rules::Derivation {
            contract,
            statement,
            subject: subject.clone(),
            premises,
            rule: crate::companion::rules::RuleId::InstantiateV1,
        },
        template,
    )
}
