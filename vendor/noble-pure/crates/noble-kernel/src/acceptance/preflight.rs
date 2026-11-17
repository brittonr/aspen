//! Request preflight: format, limits, environment schemes, and effect
//! identities, plus the external-environment-data validation walks
//! (B-CHECK-02). Every check completes before the machine starts.

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; check_request preserves format, byte/node, scheme, dependency, schema, stack and effect rejection order; invalid requests are typed outcomes, not assertion failures."
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-maintainers; the B-CHECK-02 preflight and signed-owner stages stay in one ordered body so their rejection order remains reviewable in the extracted subject."
)]
pub(super) fn check_request(
    env: &crate::contracts::Env,
    request: &crate::untrusted::Request,
    candidate: &crate::untrusted::Candidate,
    signed_owner: Option<(crate::contracts::Definition, &crate::words::Inst)>,
) -> Result<u32, super::Fail> {
    if candidate.format != crate::untrusted::CANDIDATE_FORMAT
        || candidate.revision != crate::untrusted::SEMANTIC_REVISION
    {
        return Err(super::Fail::Unsupported(
            crate::untrusted::UnsupportedKind::FormatRevision,
        ));
    }
    if request.input_bytes > request.limits.bytes {
        return Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Bytes));
    }
    let node_count = match u64::try_from(candidate.nodes.len()) {
        Ok(count) => count,
        Err(_) => return Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Nodes)),
    };
    if node_count > u64::from(request.limits.nodes) {
        return Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Nodes));
    }
    // The validation stages and body consume one request-local work allowance.
    // Do not restart it when crossing a preflight or machine boundary.
    let mut remaining = attempt!(super::validate::dependencies(env, request.limits.work));
    let mut node_index = 0;
    let mut signed_self_scan: Result<(), super::Fail> = Ok(());
    while node_index < candidate.nodes.len() && signed_self_scan.is_ok() {
        match signed_self_check(env, &candidate.nodes[node_index], signed_owner, remaining) {
            Ok(next_remaining) => remaining = next_remaining,
            Err(failure) => signed_self_scan = Err(failure),
        }
        node_index += 1;
    }
    attempt!(signed_self_scan);
    let remaining = attempt!(super::validate::schemas(env, remaining));
    let mut remaining = attempt!(validate_nominals(env, request, remaining));
    attempt!(reject_ambient_emit(env, candidate));
    if !env.live_slots {
        let mut index = 0;
        let mut slot_scan: Result<(), super::Fail> = Ok(());
        while index < candidate.nodes.len() && slot_scan.is_ok() {
            slot_scan = reject_slot_invoke(&candidate.nodes[index]);
            index += 1;
        }
        attempt!(slot_scan);
    }
    attempt!(super::schemes::validate(env, request));
    if let Some((owner, owner_inst)) = signed_owner {
        let scheme = match env.scheme(owner) {
            Some(scheme) if env.kind(owner) == Some(crate::contracts::Behavior::Named) => scheme,
            Some(_) | None => {
                return Err(super::Fail::Unsupported(
                    crate::untrusted::UnsupportedKind::NodeForm,
                ))
            }
        };
        let context = super::parts::Ctx {
            request,
            env,
            signed_owner: Some(owner),
        };
        let at = super::parts::site(None, Some(owner));
        let (owner_cost, next_remaining) = attempt!(guard_inst(owner_inst, request, remaining));
        remaining = next_remaining;
        let (interface, _, spent) = attempt!(super::parts::instantiate::apply(
            scheme, owner_inst, None, at, &context, remaining,
        ));
        remaining = attempt!(super::parts::charge(
            remaining,
            attempt!(super::parts::scheme_cost(scheme))
                .saturating_add(attempt!(super::parts::join_cost(&interface)))
                .saturating_add(spent)
        ));
        if interface.stack_in != request.expected.stack_in
            || interface.stack_out != request.expected.stack_out
            || interface.effects != request.expected.allowed_effects
        {
            return Err(super::Fail::Unsupported(
                crate::untrusted::UnsupportedKind::NodeForm,
            ));
        }
        let owner = SignedOwner {
            scheme,
            inst: owner_inst,
            cost: owner_cost,
            at,
        };
        let mut site_index = 0;
        let mut site_scan: Result<(), super::Fail> = Ok(());
        while site_index < candidate.nodes.len() && site_scan.is_ok() {
            match signed_site_check(
                &owner,
                node_signed_self(&candidate.nodes[site_index]),
                &context,
                remaining,
            ) {
                Ok(next_remaining) => remaining = next_remaining,
                Err(failure) => site_scan = Err(failure),
            }
            site_index += 1;
        }
        attempt!(site_scan);
    }
    let context = super::parts::Ctx {
        request,
        env,
        signed_owner: signed_owner.map(|(owner, _)| owner),
    };
    attempt!(super::parts::limits_of_entry(
        &request.expected.stack_in,
        &context
    ));
    attempt!(super::parts::limits_of(
        &request.expected.stack_out,
        &context
    ));
    let unknown = super::parts::effects::first_unknown(
        request.expected.allowed_effects.as_slice(),
        &env.effects,
    );
    match unknown {
        Some(id) => Err(super::parts::invalid_without_stacks(
            super::parts::site(None, None),
            crate::untrusted::Constraint::UnknownEffect(id),
        )),
        None => Ok(remaining),
    }
}

/// The signed-self node of `node`, or `None`: the loop-free helper shape the
/// extraction tool needs, since a return inside a loop body is not extractable.
const fn node_signed_self(node: &crate::untrusted::Node) -> Option<&crate::words::Inst> {
    match node {
        crate::untrusted::Node::SignedSelf { inst, .. } => Some(inst),
        _ => None,
    }
}

/// Validate one signed-self node against the request-local work allowance. No
/// loop appears in this body, so its early returns stay extractable.
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; signed-self validation reads the runtime definition-owner and behavior tables before returning the typed owner refusal, so the helper cannot be const."
)]
fn signed_self_check(
    env: &crate::contracts::Env,
    node: &crate::untrusted::Node,
    signed_owner: Option<(crate::contracts::Definition, &crate::words::Inst)>,
    remaining: u32,
) -> Result<u32, super::Fail> {
    let crate::untrusted::Node::SignedSelf { def, .. } = node else {
        return Ok(remaining);
    };
    let remaining = attempt!(super::parts::charge(remaining, 1));
    let owner = usize::try_from(def.0)
        .ok()
        .and_then(|index| env.definition_owners.get(index));
    let is_valid = signed_owner.map(|(owner, _)| owner) == Some(*def)
        && env.kind(*def) == Some(crate::contracts::Behavior::Named)
        && env.caller_module.is_some()
        && owner == Some(&env.caller_module);
    if !is_valid {
        return Err(super::Fail::Unsupported(
            crate::untrusted::UnsupportedKind::NodeForm,
        ));
    }
    Ok(remaining)
}

/// Reject one live-slot invocation node outside the live-slot profile.
const fn reject_slot_invoke(node: &crate::untrusted::Node) -> Result<(), super::Fail> {
    if matches!(node, crate::untrusted::Node::SlotInvoke { .. }) {
        return Err(super::Fail::Unsupported(
            crate::untrusted::UnsupportedKind::NodeForm,
        ));
    }
    Ok(())
}

/// The owner-side values reused by every lexical signed site.
struct SignedOwner<'a> {
    scheme: &'a crate::words::Scheme,
    inst: &'a crate::words::Inst,
    cost: u32,
    at: super::parts::Site,
}

/// Each site matches the independently supplied owner-body concrete witness,
/// even when there is only one quoted site. Public structural results are not
/// source authority; only sealed source ingress authenticates that witness.
/// Charge and compare one signed site against the owner instance. Loop-free so
/// its early returns stay extractable.
fn signed_site_check(
    owner: &SignedOwner<'_>,
    site: Option<&crate::words::Inst>,
    context: &super::parts::Ctx,
    remaining: u32,
) -> Result<u32, super::Fail> {
    let Some(inst) = site else {
        return Ok(remaining);
    };
    let (site_cost, remaining) = attempt!(guard_inst(inst, context.request, remaining));
    let remaining = attempt!(charge_signed_site(owner, inst, context, remaining));
    let remaining = attempt!(super::parts::charge(
        remaining,
        owner.cost.saturating_add(site_cost)
    ));
    if !same_owner_bindings(owner.inst, inst) {
        return Err(super::Fail::Unsupported(
            crate::untrusted::UnsupportedKind::NodeForm,
        ));
    }
    Ok(remaining)
}

/// Instantiate and charge one signed site, then check its binding cardinality
/// before the caller charges the exact element-wise comparison.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; untrusted signed-site witnesses are validated by instantiation, bounded scheme/interface charging, and a typed cardinality refusal in that order; assertion panics would replace required rejection outcomes."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; signed-site instantiation and scheme/interface cost checks are runtime operations over the supplied environment."
)]
fn charge_signed_site(
    owner: &SignedOwner<'_>,
    inst: &crate::words::Inst,
    context: &super::parts::Ctx,
    remaining: u32,
) -> Result<u32, super::Fail> {
    let (joined, _, spent) = attempt!(super::parts::instantiate::apply(
        owner.scheme,
        inst,
        None,
        owner.at,
        context,
        remaining,
    ));
    let remaining = attempt!(super::parts::charge(
        remaining,
        attempt!(super::parts::scheme_cost(owner.scheme))
            .saturating_add(attempt!(super::parts::join_cost(&joined)))
            .saturating_add(spent),
    ));
    if owner.inst.bindings.len() != inst.bindings.len() {
        return Err(super::Fail::Unsupported(
            crate::untrusted::UnsupportedKind::NodeForm,
        ));
    }
    Ok(remaining)
}

/// Compare both binding lists element-wise without returning from inside a nested
/// loop: the extraction tool (Aeneas) rejects returns inside nested loops, and the
/// agreement predicate leaves every examined pair identical.
fn same_owner_bindings(owner_inst: &crate::words::Inst, inst: &crate::words::Inst) -> bool {
    let mut index = 0usize;
    let mut is_agree = true;
    while index < owner_inst.bindings.len() && is_agree {
        if !same_owner_binding(&owner_inst.bindings[index], &inst.bindings[index]) {
            is_agree = false;
        }
        index += 1;
    }
    is_agree
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; exact binding equality delegates to the runtime PartialEq implementations for Ty and EffSet, which are not const."
)]
fn same_owner_binding(left: &crate::words::Binding, right: &crate::words::Binding) -> bool {
    match (left, right) {
        (crate::words::Binding::Stack(a), crate::words::Binding::Stack(b)) => a == b,
        (crate::words::Binding::Value(a), crate::words::Binding::Value(b)) => a == b,
        (crate::words::Binding::Effect(a), crate::words::Binding::Effect(b)) => a == b,
        (crate::words::Binding::Ref(a), crate::words::Binding::Ref(b)) => a == b,
        _ => false,
    }
}

/// Borrowed, metered walk BEFORE any scheme instantiation can clone a caller's
/// potentially enormous witness. Each queued child is bounded and charged
/// before insertion; the later equality walk is charged once more.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the bounded binding walk preserves the request-local charge order and reports typed exhaustion rather than assertion failures."
)]
/// Charge one instance binding and its nested types. Loop-free, so its early
/// returns stay extractable by Aeneas.
fn guard_one_binding(
    binding: &crate::words::Binding,
    request: &crate::untrusted::Request,
    remaining: u32,
) -> Result<(u32, u32), super::Fail> {
    let remaining = attempt!(super::parts::charge(remaining, 1));
    match binding {
        crate::words::Binding::Stack(stack) => {
            if u32::try_from(stack.len())
                .ok()
                .is_none_or(|len| len > request.limits.stack_height)
            {
                return Err(super::Fail::Exhausted(
                    crate::untrusted::LimitKind::StackHeight,
                ));
            }
            let (charged, remaining) =
                attempt!(guard_stack_entries(stack.as_slice(), request, remaining,));
            Ok((charged.saturating_add(1), remaining))
        }
        crate::words::Binding::Value(ty) => {
            let (charged, remaining) = attempt!(guard_ty(ty, request, remaining));
            Ok((charged.saturating_add(1), remaining))
        }
        crate::words::Binding::Effect(effects) => {
            let effect_count = u32::try_from(effects.as_slice().len())
                .map_err(|_| super::Fail::Exhausted(crate::untrusted::LimitKind::Work))?;
            let remaining = attempt!(super::parts::charge(remaining, effect_count));
            Ok((effect_count.saturating_add(1), remaining))
        }
        crate::words::Binding::Ref(_) => Ok((1, remaining)),
    }
}

/// Guard every type of one stack binding, accumulating the charge without
/// returning from inside the loop.
fn guard_stack_entries(
    stack: &[crate::types::Ty],
    request: &crate::untrusted::Request,
    remaining: u32,
) -> Result<(u32, u32), super::Fail> {
    let mut index = 0;
    let mut cost = 0u32;
    let mut remaining = remaining;
    let mut walked: Result<(), super::Fail> = Ok(());
    while index < stack.len() && walked.is_ok() {
        match guard_ty(&stack[index], request, remaining) {
            Ok((charged, next_remaining)) => {
                cost = cost.saturating_add(charged);
                remaining = next_remaining;
            }
            Err(failure) => walked = Err(failure),
        }
        index += 1;
    }
    attempt!(walked);
    Ok((cost, remaining))
}

fn guard_inst(
    inst: &crate::words::Inst,
    request: &crate::untrusted::Request,
    remaining: u32,
) -> Result<(u32, u32), super::Fail> {
    let mut index = 0;
    let mut cost = 0u32;
    let mut remaining = remaining;
    let mut walked: Result<(), super::Fail> = Ok(());
    while index < inst.bindings.len() && walked.is_ok() {
        match guard_one_binding(&inst.bindings[index], request, remaining) {
            Ok((charged, next_remaining)) => {
                cost = cost.saturating_add(charged);
                remaining = next_remaining;
            }
            Err(failure) => walked = Err(failure),
        }
        index += 1;
    }
    attempt!(walked);
    Ok((cost, remaining))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the bounded type walk preserves charge-before-queue order and reports typed type-size/work exhaustion rather than assertion failures."
)]
fn guard_ty(
    ty: &crate::types::Ty,
    request: &crate::untrusted::Request,
    remaining: u32,
) -> Result<(u32, u32), super::Fail> {
    let mut pending = alloc::vec![ty];
    let mut visited = 0u32;
    let mut reserved = 0u32;
    let mut remaining = remaining;
    while let Some(next) = pending.pop() {
        remaining = attempt!(super::parts::charge(remaining, 1));
        visited = visited.saturating_add(1);
        if visited > request.limits.type_size {
            return Err(super::Fail::Exhausted(
                crate::untrusted::LimitKind::TypeSize,
            ));
        }
        match next {
            crate::types::Ty::Pair(a, b) | crate::types::Ty::Sum(a, b) => {
                pending.push(a);
                pending.push(b);
            }
            crate::types::Ty::List(item) => pending.push(item),
            crate::types::Ty::Program(input, output, effects)
            | crate::types::Ty::LiveRef(input, output, effects) => {
                let children = input.len().saturating_add(output.len());
                if u32::try_from(children)
                    .ok()
                    .is_none_or(|len| len > request.limits.type_size)
                {
                    return Err(super::Fail::Exhausted(
                        crate::untrusted::LimitKind::TypeSize,
                    ));
                }
                let count = u32::try_from(children.saturating_add(effects.as_slice().len()))
                    .map_err(|_| super::Fail::Exhausted(crate::untrusted::LimitKind::Work))?;
                remaining = attempt!(super::parts::charge(remaining, count));
                reserved = reserved.saturating_add(count);
                for child in input.iter().chain(output.iter()) {
                    pending.push(child);
                }
            }
            crate::types::Ty::Nominal(_, shape) => match &**shape {
                crate::types::NominalShape::Opaque(value) => pending.push(value),
                crate::types::NominalShape::Variant(a, b) => {
                    pending.push(a);
                    pending.push(b);
                }
            },
            crate::types::Ty::GenericNominal(_, args, shape) => {
                pending.push(&args[0]);
                pending.push(&args[1]);
                match &**shape {
                    crate::types::NominalShape::Opaque(value) => pending.push(value),
                    crate::types::NominalShape::Variant(a, b) => {
                        pending.push(a);
                        pending.push(b);
                    }
                }
            }
            crate::types::Ty::Unit
            | crate::types::Ty::Bool
            | crate::types::Ty::I64
            | crate::types::Ty::Text
            | crate::types::Ty::Syntax
            | crate::types::Ty::Contract
            | crate::types::Ty::Evidence
            | crate::types::Ty::Certified
            | crate::types::Ty::Resource(_) => {}
        }
    }
    Ok((visited.saturating_add(reserved), remaining))
}

/// No candidate arena node, including an unreachable one, may smuggle the
/// historical fixed host operation outside the explicitly selected test-host
/// profile. Runtime authorization remains the independent host's obligation.
fn reject_ambient_emit(
    env: &crate::contracts::Env,
    candidate: &crate::untrusted::Candidate,
) -> Result<(), super::Fail> {
    if env.ambient_test_emit_visible() {
        return Ok(());
    }
    let Some((index, def)) = ambient_emit(env, candidate) else {
        return Ok(());
    };
    let Ok(index) = u32::try_from(index) else {
        return Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Nodes));
    };
    let node_id = crate::untrusted::NodeId(index);
    Err(super::parts::invalid_without_stacks(
        super::parts::site(Some(node_id), Some(def)),
        crate::untrusted::Constraint::PrivateDefinition(def),
    ))
}

/// Find the first forbidden invocation, including unreachable arena nodes.
fn ambient_emit(
    env: &crate::contracts::Env,
    candidate: &crate::untrusted::Candidate,
) -> Option<(usize, crate::contracts::Definition)> {
    let mut index = 0;
    let mut first = None;
    while index < candidate.nodes.len() {
        if let crate::untrusted::Node::Invocation { def, .. } = &candidate.nodes[index] {
            if env.kind(*def) == Some(crate::contracts::Behavior::TestEmit) {
                first = Some((index, *def));
                break;
            }
        }
        index += 1;
    }
    first
}

fn validate_nominals(
    env: &crate::contracts::Env,
    request: &crate::untrusted::Request,
    remaining: u32,
) -> Result<u32, super::Fail> {
    let remaining = attempt!(nominal_budget(env, request, remaining));
    if let Some(failure) = first_nominal_failure(env, &request.limits) {
        return Err(nominal_failure(failure));
    }
    if !env.valid_generic_declarations() {
        return Err(nominal_failure(NominalFailure {
            is_invalid_type: true,
        }));
    }
    if !env.validate_contracts() {
        return Err(super::parts::invalid_without_stacks(
            super::parts::site(None, None),
            crate::untrusted::Constraint::InvalidContract,
        ));
    }
    Ok(remaining)
}

/// Preserve the first invalid declaration before checking contract coherence.
struct NominalFailure {
    is_invalid_type: bool,
}

const fn nominal_failure(failure: NominalFailure) -> super::Fail {
    if failure.is_invalid_type {
        return super::parts::invalid_without_stacks(
            super::parts::site(None, None),
            crate::untrusted::Constraint::InvalidType,
        );
    }
    super::Fail::Exhausted(crate::untrusted::LimitKind::TypeSize)
}

fn first_nominal_failure(
    env: &crate::contracts::Env,
    limits: &crate::untrusted::Limits,
) -> Option<NominalFailure> {
    let mut index = 0;
    let mut first = None;
    while index < env.nominals.len() && first.is_none() {
        first = nominal_failure_at(env, limits, index);
        index += 1;
    }
    first
}

/// New definitions keep their separate node bound; their work charge is
/// carried into the nominal scan and the candidate body.
fn nominal_budget(
    env: &crate::contracts::Env,
    request: &crate::untrusted::Request,
    remaining: u32,
) -> Result<u32, super::Fail> {
    let remaining = if env.declared_modules {
        attempt!(declared_definition_budget(env, request, remaining))
    } else {
        remaining
    };
    let total = env
        .nominals
        .len()
        .saturating_add(env.generic_variants.len())
        .saturating_add(env.bound_adapters.len());
    let Ok(count) = u64::try_from(total) else {
        return Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Work));
    };
    let Ok(cost) = u32::try_from(count) else {
        return Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Work));
    };
    super::parts::charge(remaining, cost)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "reads env.defs, the runtime definition table, which is not const"
)]
fn declared_definition_budget(
    env: &crate::contracts::Env,
    request: &crate::untrusted::Request,
    remaining: u32,
) -> Result<u32, super::Fail> {
    let Ok(additions) = u64::try_from(env.defs.len().saturating_sub(23)) else {
        return Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Nodes));
    };
    if additions > u64::from(request.limits.nodes) {
        return Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Nodes));
    }
    let Ok(cost) = u32::try_from(additions) else {
        return Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Work));
    };
    super::parts::charge(remaining, cost)
}

/// Validate one declaration's representation before checking its size.
fn nominal_failure_at(
    env: &crate::contracts::Env,
    limits: &crate::untrusted::Limits,
    index: usize,
) -> Option<NominalFailure> {
    if !env.validate_decl(index, 512) {
        return Some(NominalFailure {
            is_invalid_type: true,
        });
    }
    if !nominal_size_within_limit(nominal_payload_size(&env.nominals[index].shape), limits) {
        return Some(NominalFailure {
            is_invalid_type: false,
        });
    }
    None
}

fn nominal_payload_size(shape: &crate::types::NominalShape) -> Option<u32> {
    if let crate::types::NominalShape::Opaque(representation) = shape {
        return representation.size();
    }
    if let crate::types::NominalShape::Variant(left, right) = shape {
        return left
            .size()
            .zip(right.size())
            .and_then(|(left, right)| left.checked_add(right));
    }
    None
}

/// A nominal descriptor adds one node for its wrapper, even when the payload
/// cannot be represented or exceeds the request's structural type bound.
const fn nominal_size_within_limit(
    payload_node_count: Option<u32>,
    limits: &crate::untrusted::Limits,
) -> bool {
    let Some(payload_node_count) = payload_node_count else {
        return false;
    };
    let Some(total_node_count) = payload_node_count.checked_add(1) else {
        return false;
    };
    total_node_count <= limits.type_size
}
