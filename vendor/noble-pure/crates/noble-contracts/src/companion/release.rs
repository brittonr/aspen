//! Proof-required release accepts only applicable proved evidence. Artifact
//! correspondence independently checks the actual candidate and exact program.

pub(crate) fn select(
    engine: &crate::companion::Core,
    evidence: crate::companion::EvidenceId,
    subject: &crate::companion::Subject,
) -> Result<crate::companion::Release, crate::companion::Refusal> {
    let entry = attempt!(crate::companion::registry::application::proved(
        engine, evidence
    ));
    // Release is a decision about this subject, so the same complete
    // observation applicability that `bind` uses decides it; a digest or
    // cache match alone never does.
    attempt!(crate::companion::registry::application::matches_observation(&entry.subject, subject));
    Ok(crate::companion::Release {
        contract: entry.contract,
        evidence,
        statement: entry.statement,
        subject: entry.subject.identity,
        policy: engine.policy,
        ruleset: crate::companion::RULESET_V1,
    })
}

pub(crate) fn correspond(
    engine: &crate::companion::Core,
    evidence: crate::companion::EvidenceId,
    contract: crate::companion::ContractId,
) -> Result<(), crate::companion::Refusal> {
    let entry = attempt!(crate::companion::registry::application::proved(
        engine, evidence
    ));
    let selected = attempt!(engine.registry.contract(contract));
    if crate::companion::registry::Registry::stale(
        engine,
        crate::companion::registry::ContextSnapshot {
            policy: selected.policy,
            revision: selected.revision,
        },
    ) {
        return Err(crate::companion::Refusal::StaleContext);
    }
    if entry.contract != contract {
        return Err(crate::companion::Refusal::CorrespondenceMismatch);
    }
    Ok(())
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; bind_artifact checks current proved authority, closed quotation shape, actual kernel acceptance and exact normalized body before binding; malformed artifacts must return Refusal rather than panic."
)]
pub(crate) fn bind_artifact(
    engine: &crate::companion::Core,
    evidence: crate::companion::EvidenceId,
    submission: &noble_kernel::execution::Submission,
) -> Result<crate::companion::Subject, crate::companion::Refusal> {
    let entry = attempt!(crate::companion::registry::application::proved(
        engine, evidence
    ));
    let contract = attempt!(engine.registry.contract(entry.contract));
    let candidate = &submission.body.candidate;
    if !submission.definitions.is_empty()
        || !submission.body.texts.is_empty()
        || !is_quotation_root(candidate)
    {
        return Err(crate::companion::Refusal::CorrespondenceMismatch);
    }
    let environment = match noble_kernel::contracts::environment() {
        Ok(environment) => environment,
        Err(_) => return Err(crate::companion::Refusal::Internal),
    };
    let request = artifact_request(engine.limits, &submission.request, contract);
    attempt!(checked(&noble_kernel::acceptance::check(
        &environment,
        &request,
        candidate
    )));
    let events = attempt!(crate::companion::admit::program::canonical_op_events(
        candidate
    ));
    let last_index = match events.len().checked_sub(1) {
        Some(last_index) => last_index,
        None => return Err(crate::companion::Refusal::CorrespondenceMismatch),
    };
    if last_index < 1 || events[0] != (3, 0) || events[last_index] != (6, 0) {
        return Err(crate::companion::Refusal::CorrespondenceMismatch);
    }
    let observed = crate::companion::subject::observe(
        &events[1..last_index],
        crate::companion::InterfaceSignatures {
            input: entry.subject.input_signature,
            output: entry.subject.output_signature,
        },
    );
    match engine.bind(evidence, &observed) {
        Ok(subject) => Ok(subject),
        Err(_) => Err(crate::companion::Refusal::CorrespondenceMismatch),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; binding an ordinary checked program must reject external definitions, failed kernel admission, malformed event lowering, and any subject/interface mismatch before returning an applicable subject."
)]
pub(crate) fn bind_checked_program(
    engine: &crate::companion::Core,
    evidence: crate::companion::EvidenceId,
    submission: &noble_kernel::execution::Submission,
) -> Result<crate::companion::Subject, crate::companion::Refusal> {
    if !submission.definitions.is_empty() || !submission.body.texts.is_empty() {
        return Err(crate::companion::Refusal::CorrespondenceMismatch);
    }
    let environment = match noble_kernel::contracts::environment() {
        Ok(environment) => environment,
        Err(_) => return Err(crate::companion::Refusal::Internal),
    };
    let mut request = submission.request.clone();
    request.limits.bytes = request.limits.bytes.min(engine.limits.bytes);
    request.limits.nodes = request.limits.nodes.min(engine.limits.nodes);
    request.limits.depth = request.limits.depth.min(engine.limits.depth);
    request.limits.work = request.limits.work.min(engine.limits.work);
    attempt!(checked(&noble_kernel::acceptance::check(
        &environment,
        &request,
        &submission.body.candidate
    )));
    let events = attempt!(crate::companion::admit::program::canonical_op_events(
        &submission.body.candidate
    ));
    let observed = crate::companion::subject::observe(
        &events,
        crate::companion::InterfaceSignatures {
            input: crate::companion::interface_signature(&submission.request.expected.stack_in),
            output: crate::companion::interface_signature(&submission.request.expected.stack_out),
        },
    );
    engine.bind(evidence, &observed)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; is_quotation_root uses checked TryFrom conversion and Vec dereferencing for candidate arena lookup, neither stable const on the pinned compiler."
)]
fn is_quotation_root(candidate: &noble_kernel::untrusted::Candidate) -> bool {
    if candidate.body.len() != 1 {
        return false;
    }
    let root = match usize::try_from(candidate.body[0].0) {
        Ok(root) => root,
        Err(_) => return false,
    };
    match candidate.nodes.get(root) {
        Some(node) => match node {
            noble_kernel::untrusted::Node::Quotation { .. } => true,
            noble_kernel::untrusted::Node::Literal { .. }
            | noble_kernel::untrusted::Node::Invocation { .. }
            | noble_kernel::untrusted::Node::SignedSelf { .. }
            | noble_kernel::untrusted::Node::SlotInvoke { .. } => false,
        },
        None => false,
    }
}

#[expect(
    clippy::vec_init_then_push,
    reason = "Owner: noble-maintainers; explicit owned push preserves the pinned extractor's Vec representation."
)]
fn artifact_request(
    limits: crate::Limits,
    request: &noble_kernel::untrusted::Request,
    contract: &crate::companion::registry::ContractEntry,
) -> noble_kernel::untrusted::Request {
    let mut request = request.clone();
    let mut stack_out = alloc::vec::Vec::with_capacity(1);
    stack_out.push(noble_kernel::types::Ty::program(
        contract.input.clone(),
        contract.output.clone(),
        noble_kernel::types::EffSet::empty(),
    ));
    request.expected = noble_kernel::untrusted::Expected {
        stack_in: alloc::vec::Vec::new(),
        stack_out,
        allowed_effects: noble_kernel::types::EffSet::empty(),
    };
    request.limits.work = request.limits.work.min(limits.work);
    request.limits.nodes = request.limits.nodes.min(limits.nodes);
    request.limits.depth = request.limits.depth.min(limits.depth);
    request.limits.bytes = request.limits.bytes.min(limits.bytes);
    request
}

const fn checked(
    outcome: &noble_kernel::untrusted::Outcome,
) -> Result<(), crate::companion::Refusal> {
    match outcome {
        noble_kernel::untrusted::Outcome::Accepted(_) => Ok(()),
        noble_kernel::untrusted::Outcome::Exhausted(_) => {
            Err(crate::companion::Refusal::ExhaustedReplay)
        }
        noble_kernel::untrusted::Outcome::Invalid(_)
        | noble_kernel::untrusted::Outcome::Unsupported(_)
        | noble_kernel::untrusted::Outcome::InternalFailure => {
            Err(crate::companion::Refusal::CorrespondenceMismatch)
        }
    }
}
