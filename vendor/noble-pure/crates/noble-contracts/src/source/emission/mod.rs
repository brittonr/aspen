#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; emission mutates only consumed inference state, a fresh environment, bounded assembly/materialization worklists, and the preparation-owned meter; no committed namespace or borrowed source/type is modified, and kernel admission never executes a body."
)]

mod contracts;
mod identity;
mod interfaces;
mod materialization;

#[derive(Clone, Copy)]
struct Admission<'a> {
    input_bytes: u32,
    limits: noble_kernel::untrusted::Limits,
    environment: &'a noble_kernel::contracts::Env,
    linked_slots: &'a [u32],
}

struct Assembly {
    definition_base: usize,
    definitions: alloc::vec::Vec<noble_kernel::execution::Definition>,
    root: Option<(
        noble_kernel::execution::Body,
        noble_kernel::untrusted::Request,
        noble_kernel::untrusted::Checked,
    )>,
}

struct CheckedBody {
    body: noble_kernel::execution::Body,
    request: noble_kernel::untrusted::Request,
    checked: noble_kernel::untrusted::Checked,
    identity: Option<u64>,
    source_identity: Option<u64>,
    owner_slot: Option<u32>,
    owner_inst: Option<noble_kernel::words::Inst>,
    span: crate::Span,
}

impl Assembly {
    fn install(
        &mut self,
        mut checked: CheckedBody,
        position: usize,
        downstream_work: u32,
    ) -> Result<(), super::Error> {
        match checked.identity {
            Some(identity) => {
                let definition = match crate::index(
                    position
                        .saturating_sub(1)
                        .saturating_add(self.definition_base),
                    checked.span,
                ) {
                    Ok(id) => noble_kernel::contracts::Definition(id),
                    Err(error) => return Err(super::Error::at(crate::source::Stage::Check, error)),
                };
                self.definitions.push(noble_kernel::execution::Definition {
                    definition,
                    identity,
                    body: checked.body,
                    expected: checked.request.expected,
                    source_identity: checked.source_identity.ok_or_else(|| {
                        super::Error::at(crate::source::Stage::Check, crate::internal(checked.span))
                    })?,
                    owner_slot: checked.owner_slot,
                    owner_inst: checked.owner_inst,
                });
            }
            None => {
                // Backend admission is an independent bounded stage, not one
                // more specialization sharing the frontend's reservation.
                checked.request.limits.work = downstream_work;
                self.root = Some((checked.body, checked.request, checked.checked));
            }
        }
        Ok(())
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; closing, concrete interface materialization, and independent body checks propagate typed failures; emission requires a checked root before returning a submission."
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
pub(super) fn emit(
    mut state: super::inference::State,
    mut environment: noble_kernel::contracts::Env,
    definitions: &[super::Named],
    input_bytes: usize,
    meter: &mut crate::Meter,
) -> Result<
    (
        noble_kernel::execution::Submission,
        noble_kernel::untrusted::Checked,
        alloc::vec::Vec<noble_kernel::types::Ty>,
    ),
    super::Error,
> {
    if let Err(error) = state.arena.close(state.span, meter) {
        return Err(super::Error::at(crate::source::Stage::Check, error));
    }
    let input_bytes = match crate::index(input_bytes, state.span) {
        Ok(bytes) => bytes,
        Err(error) => return Err(super::Error::at(crate::source::Stage::Check, error)),
    };
    let expected = match interfaces::collect(&state, &mut environment, definitions, meter) {
        Ok(expected) => expected,
        Err(error) => return Err(super::Error::at(crate::source::Stage::Check, error)),
    };
    if environment.calculator {
        if environment.defs.len() > noble_kernel::contracts::CALCULATOR_ENVIRONMENT_ROW_LIMIT {
            return Err(super::Error::at(
                crate::source::Stage::Check,
                super::exhausted(state.span, "calculator environment row limit exceeded"),
            ));
        }
        // Interfaces have now materialized every selected specialization.
        // Each body below independently clones this exact environment with
        // its own caller owner; count and charge the whole fanout before the
        // first checked body can own an Env.
        let budget = match environment.calculator_clone_budget() {
            Some(budget) => budget,
            None => {
                return Err(super::Error::at(
                    crate::source::Stage::Check,
                    super::exhausted(
                        state.span,
                        "calculator selected environment clone bound exceeded",
                    ),
                ))
            }
        };
        let copies = match u64::try_from(state.bodies.len()) {
            Ok(copies) => copies,
            Err(_) => {
                return Err(super::Error::at(
                    crate::source::Stage::Check,
                    super::exhausted(state.span, "calculator selected clone count overflow"),
                ))
            }
        };
        if budget
            .reserved_bytes
            .checked_mul(copies)
            .is_none_or(|total| total > noble_kernel::contracts::CALCULATOR_ENV_CLONE_FANOUT_LIMIT)
        {
            return Err(super::Error::at(
                crate::source::Stage::Check,
                super::exhausted(
                    state.span,
                    "calculator selected environment clone fanout exceeded",
                ),
            ));
        }
        let work = match budget
            .bytes
            .checked_mul(copies)
            .and_then(|bytes| u32::try_from(bytes).ok())
        {
            Some(work) => work,
            None => {
                return Err(super::Error::at(
                    crate::source::Stage::Check,
                    super::exhausted(
                        state.span,
                        "calculator selected environment clone work exceeded",
                    ),
                ))
            }
        };
        if let Err(error) = meter.charge(work, state.span) {
            return Err(super::Error::at(crate::source::Stage::Check, error));
        }
    }
    let span = state.span;
    let linked_slots = identity::selected_slots(&state, &environment);
    let limits = match allowance(state.bodies.len(), state.span, meter) {
        Ok(limits) => limits,
        Err(error) => return Err(super::Error::at(crate::source::Stage::Check, error)),
    };
    let admission = Admission {
        input_bytes,
        limits,
        environment: &environment,
        linked_slots: &linked_slots,
    };
    let assembly = attempt!(assemble(state, expected, admission, meter));
    let (body, request, checked) = match assembly.root {
        Some(root) => root,
        None => {
            return Err(super::Error::at(
                crate::source::Stage::Check,
                crate::internal(span),
            ))
        }
    };
    let output = request.expected.stack_out.clone();
    Ok((
        noble_kernel::execution::Submission {
            environment,
            definitions: assembly.definitions,
            body,
            request,
        },
        checked,
        output,
    ))
}

#[expect(
    clippy::while_let_on_iterator,
    reason = "Owner: noble-maintainers; the explicit owned iterator avoids the pinned architecture collector's UnsupportedExpansion for Desugaring(ForLoop); retain complete required compiler facts until that desugaring is supported."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; allowance uses checked division, every body must have its matching expected interface, and checked installation preserves the root's independent full downstream work limit."
)]
fn assemble(
    state: super::inference::State,
    expected: alloc::vec::Vec<noble_kernel::untrusted::Expected>,
    admission: Admission<'_>,
    meter: &mut crate::Meter,
) -> Result<Assembly, super::Error> {
    let mut assembly = Assembly {
        definition_base: state.definition_base,
        definitions: alloc::vec::Vec::with_capacity(state.bodies.len().saturating_sub(1)),
        root: None,
    };
    let mut expected = expected.into_iter();
    let arena = state.arena;
    let mut drafts = state.bodies.into_iter();
    let mut position = 0usize;
    let mut failure = None;
    while let Some(draft) = drafts.next() {
        let result = match checked(
            draft,
            expected.next(),
            &arena,
            admission,
            position,
            state.definition_base,
            meter,
        ) {
            Ok(body) => assembly.install(body, position, meter.limits.work),
            Err(problem) => Err(problem),
        };
        if let Err(problem) = result {
            failure = Some(problem);
            break;
        }
        position = position.saturating_add(1);
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(assembly),
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; allowance divides the current runtime meter remainder by each admitted body count, so it is request-state-dependent and cannot be a constant expression"
)]
fn allowance(
    body_count: usize,
    span: crate::Span,
    meter: &crate::Meter,
) -> Result<noble_kernel::untrusted::Limits, crate::Diagnostic> {
    // Bound aggregate kernel checker work to at most one quarter of the
    // remaining source meter; the exact work used is charged after each check.
    let body_count = attempt!(crate::index(body_count, span));
    let work = match body_count
        .checked_mul(4)
        .and_then(|count| meter.available().checked_div(count))
    {
        Some(allowance) => allowance,
        None => return Err(crate::internal(span)),
    };
    Ok(noble_kernel::untrusted::Limits {
        bytes: meter.limits.bytes,
        nodes: meter.limits.nodes,
        depth: meter.limits.depth,
        type_size: crate::syntax::TYPE_CAP,
        stack_height: crate::inference::STACK_CAP,
        work,
        diagnostics: 8,
    })
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; witness materialization, reserved checker work, and independent kernel checking are fallible admission checks, not conditions that may panic on source input."
)]
#[expect(
    tigerstyle::ambiguous_params,
    tigerstyle::function_length,
    tigerstyle::too_many_parameters,
    reason = "Owner: noble-contracts; the same-typed parameters are positional coordinates at each checked call site; a newtype would not change the call shape; the function keeps one ordered pass in a single reviewable unit; splitting it would separate each step from its bounds checks; the parameters are independent checked inputs and shared budget state threaded explicitly; an options struct would only rename the same borrowed fields."
)]
fn checked(
    draft: super::inference::Body,
    expected: Option<noble_kernel::untrusted::Expected>,
    arena: &crate::inference::Arena,
    admission: Admission<'_>,
    position: usize,
    definition_base: usize,
    meter: &mut crate::Meter,
) -> Result<CheckedBody, super::Error> {
    let expected = match expected {
        Some(expected) => expected,
        None => {
            return Err(super::Error::at(
                crate::source::Stage::Check,
                crate::internal(draft.span),
            ))
        }
    };
    let request = noble_kernel::untrusted::Request {
        input_bytes: admission.input_bytes,
        expected,
        limits: admission.limits,
    };
    let span = draft.span;
    let source_identity = draft.identity;
    let identity = source_identity
        .map(|id| identity::specialization(id, admission.environment, admission.linked_slots));
    let draft_owner = draft.owner;
    let owner_slot = draft.owner_slot;
    let owner_inst = if draft
        .nodes
        .iter()
        .any(|node| matches!(node.kind, super::inference::DraftKind::SignedSelf(_)))
    {
        let variables = draft.owner_variables.as_ref().ok_or_else(|| {
            super::Error::at(
                crate::source::Stage::Check,
                crate::invalid(span, "signed body has no owner binder"),
            )
        })?;
        Some(attempt!(arena
            .instantiation(variables, span, meter)
            .map_err(|problem| super::Error::at(
                crate::source::Stage::Check,
                problem
            ))))
    } else {
        None
    };
    let (body, spans) = match materialization::body(draft, arena, meter) {
        Ok(body) => body,
        Err(error) => return Err(super::Error::at(crate::source::Stage::Check, error)),
    };
    // Every concrete body derives independently: the root invocation contract
    // alone is not evidence for any definition's implementation.
    let mut caller_environment = admission.environment.clone();
    caller_environment.caller_module = draft_owner;
    let self_owner = if body
        .candidate
        .nodes
        .iter()
        .any(|node| matches!(node, noble_kernel::untrusted::Node::SignedSelf { .. }))
    {
        let Some(owner_offset) = position.checked_sub(1) else {
            return Err(super::Error::at(
                crate::source::Stage::Acceptance,
                crate::invalid(span, "signed self cannot appear in the executable root"),
            ));
        };
        let owner_index = definition_base
            .checked_add(owner_offset)
            .ok_or_else(|| super::Error::at(crate::source::Stage::Check, crate::internal(span)))?;
        Some(noble_kernel::contracts::Definition(attempt!(crate::index(
            owner_index,
            span
        )
        .map_err(|error| super::Error::at(
            crate::source::Stage::Check,
            error
        )))))
    } else {
        None
    };
    let (result, work_used) = match self_owner {
        Some(owner) => crate::program::check_signed_structure_with_work(
            &caller_environment,
            &body.candidate,
            &request,
            &spans,
            span,
            owner,
            owner_inst.as_ref().ok_or_else(|| {
                super::Error::at(crate::source::Stage::Check, crate::internal(span))
            })?,
        ),
        None => crate::program::check_with_work(
            &caller_environment,
            &body.candidate,
            &request,
            &spans,
            span,
        ),
    };
    if let Err(error) = meter.charge(work_used, span) {
        return Err(super::Error::at(crate::source::Stage::Acceptance, error));
    }
    let checked = match result {
        Ok(checked) => checked,
        Err(error) => return Err(super::Error::at(crate::source::Stage::Acceptance, error)),
    };
    Ok(CheckedBody {
        body,
        request,
        checked,
        identity,
        source_identity,
        owner_slot,
        owner_inst,
        span,
    })
}
