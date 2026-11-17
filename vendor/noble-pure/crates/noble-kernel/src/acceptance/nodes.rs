//! Node lookup and literal, invocation, and quotation folding.

mod visibility;

/// Fetch one node, rejecting a reference outside the finite arena.
///
/// The node returns owned: a reference into the candidate arena cannot be
/// carried across the machine state the extraction interpreter tracks. The
/// copy is structural, so the node's witness is measured first and refused
/// while it is still borrowed; a witness nested deeper than the local node cap
/// would otherwise overflow the stack inside the copy instead of rejecting.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; node_of checks index representability, arena membership and the witness copy bound and reports a typed diagnostic for each failure; hostile references must not trigger assertions."
)]
pub(super) fn node_of(
    candidate: &crate::untrusted::Candidate,
    node_id: crate::untrusted::NodeId,
    context: &super::parts::Ctx,
) -> Result<crate::untrusted::Node, super::Fail> {
    let index = match usize::try_from(node_id.0) {
        Ok(index) => index,
        Err(_) => {
            return Err(super::parts::invalid(
                context,
                super::parts::site(Some(node_id), None),
                alloc::vec::Vec::new(),
                alloc::vec::Vec::new(),
                crate::untrusted::Constraint::MalformedReference(node_id),
            ))
        }
    };
    match candidate.nodes.get(index) {
        Some(node) if is_within_copy_bound(node) => Ok(node.clone()),
        Some(_) => Err(super::parts::invalid_without_stacks(
            super::parts::site(
                Some(node_id),
                super::parts::definition_of(candidate, node_id),
            ),
            crate::untrusted::Constraint::InvalidType,
        )),
        None => Err(super::parts::invalid(
            context,
            super::parts::site(Some(node_id), None),
            alloc::vec::Vec::new(),
            alloc::vec::Vec::new(),
            crate::untrusted::Constraint::MalformedReference(node_id),
        )),
    }
}

/// Whether every type this node's witness would copy fits the copy cap.
///
/// `Ty::physical_nodes` walks borrowed nodes and fails closed past
/// `PHYSICAL_NODE_CAP`, so this reads the witness without copying it and
/// refuses exactly the shapes a structural copy could not survive. It is a
/// copy bound, not a typing bound: `Ty::size` keeps its declared meaning.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; is_within_copy_bound accepts every binding whose physical measurement is available and reports false for a witness past the copy cap; the measurement itself is the fail-closed bound."
)]
fn is_within_copy_bound(node: &crate::untrusted::Node) -> bool {
    let inst = match node {
        crate::untrusted::Node::Literal { inst, .. }
        | crate::untrusted::Node::Invocation { inst, .. }
        | crate::untrusted::Node::SignedSelf { inst, .. }
        | crate::untrusted::Node::Quotation { inst, .. } => inst,
        crate::untrusted::Node::SlotInvoke { .. } => return true,
    };
    let mut index = 0;
    let mut is_bounded = true;
    while index < inst.bindings.len() && is_bounded {
        is_bounded = match &inst.bindings[index] {
            crate::words::Binding::Value(ty) => ty.physical_nodes().is_some(),
            crate::words::Binding::Stack(stack) => stack_is_measurable(stack.as_slice()),
            crate::words::Binding::Effect(_) | crate::words::Binding::Ref(_) => true,
        };
        index += 1;
    }
    is_bounded
}

fn stack_is_measurable(stack: &[crate::types::Ty]) -> bool {
    let mut index = 0;
    let mut is_measurable = true;
    while index < stack.len() && is_measurable {
        is_measurable = stack[index].physical_nodes().is_some();
        index += 1;
    }
    is_measurable
}

/// Fold one literal or invocation node into its frame.
///
/// Returns the work charged for the node's instantiation, its witness
/// resolution and its join.
pub(super) fn fold_node(
    frame: super::Frame,
    node_id: crate::untrusted::NodeId,
    node: &crate::untrusted::Node,
    context: &super::parts::Ctx,
    fuel: u32,
) -> Result<
    (
        u32,
        super::Frame,
        crate::untrusted::Interface,
        Option<crate::untrusted::LiveSite>,
    ),
    super::Fail,
> {
    match node {
        crate::untrusted::Node::Literal { lit, inst } => {
            let scheme = super::parts::instantiate::literal_scheme(*lit);
            let at = super::parts::site(Some(node_id), None);
            let cost = attempt!(super::parts::scheme_cost(&scheme));
            let (interface, _resolved, spent) = attempt!(super::parts::instantiate::apply(
                &scheme, inst, None, at, context, fuel
            ));
            let cost = cost
                .saturating_add(attempt!(super::parts::join_cost(&interface)))
                .saturating_add(spent);
            let joined = attempt!(super::parts::join(frame, &interface, at, context));
            Ok((cost, joined, interface, None))
        }
        crate::untrusted::Node::Invocation { def, inst }
        | crate::untrusted::Node::SignedSelf { def, inst } => {
            if matches!(node, crate::untrusted::Node::SignedSelf { .. })
                && context.signed_owner != Some(*def)
            {
                return Err(super::Fail::Unsupported(
                    crate::untrusted::UnsupportedKind::NodeForm,
                ));
            }
            attempt!(visibility::check(*def, node_id, context));
            let scheme = match context.env.scheme(*def) {
                Some(scheme) => scheme.clone(),
                None => {
                    return Err(super::parts::invalid(
                        context,
                        super::parts::site(Some(node_id), Some(*def)),
                        alloc::vec::Vec::new(),
                        alloc::vec::Vec::new(),
                        crate::untrusted::Constraint::UnknownDefinition(*def),
                    ))
                }
            };
            let at = super::parts::site(Some(node_id), Some(*def));
            let data_var = super::parts::instantiate::data_slot(context.env.kind(*def));
            let cost = attempt!(super::parts::scheme_cost(&scheme));
            let (interface, _resolved, spent) = attempt!(super::parts::instantiate::apply(
                &scheme, inst, data_var, at, context, fuel
            ));
            let cost = cost
                .saturating_add(attempt!(super::parts::join_cost(&interface)))
                .saturating_add(spent);
            let joined = attempt!(super::parts::join(frame, &interface, at, context));
            Ok((cost, joined, interface, None))
        }
        crate::untrusted::Node::SlotInvoke {
            ref_ordinal,
            site_id,
            forwarded_ref_ordinals,
        } => fold_slot(
            frame,
            node_id,
            *ref_ordinal,
            *site_id,
            forwarded_ref_ordinals,
            context,
        ),
        crate::untrusted::Node::Quotation { .. } => Err(super::Fail::Internal),
    }
}

#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-kernel; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-kernel; the invariants of this linear dispatcher are enforced by its callers and the kernel test suite"
)]
#[expect(
    tigerstyle::too_many_parameters,
    reason = "Owner: noble-kernel; the parameter list is the kernel internal call shape; bundling it would change the extracted subject"
)]
#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-kernel; the same-typed parameters are positional by contract and named at every call site"
)]
/// Derive a live invocation solely from the host-owned root input and the
/// checked forwarding positions. A raw ordinal is never type authority.
fn fold_slot(
    frame: super::Frame,
    node_id: crate::untrusted::NodeId,
    ref_ordinal: u32,
    site_id: u32,
    forwarded: &[u32],
    context: &super::parts::Ctx,
) -> Result<
    (
        u32,
        super::Frame,
        crate::untrusted::Interface,
        Option<crate::untrusted::LiveSite>,
    ),
    super::Fail,
> {
    let at = super::parts::site(Some(node_id), None);
    if !context.env.live_slots || frame.depth != 0 {
        return Err(super::parts::invalid_without_stacks(
            at,
            crate::untrusted::Constraint::BorrowProvenance,
        ));
    }
    if site_id != node_id.0 {
        return Err(super::parts::invalid_without_stacks(
            at,
            crate::untrusted::Constraint::SlotSite,
        ));
    }
    let root_index = match usize::try_from(ref_ordinal) {
        Ok(index) => index,
        Err(_) => {
            return Err(super::parts::invalid_without_stacks(
                at,
                crate::untrusted::Constraint::BorrowProvenance,
            ));
        }
    };
    let Some(root) = frame.borrowed.get(root_index) else {
        return Err(super::parts::invalid_without_stacks(
            at,
            crate::untrusted::Constraint::BorrowProvenance,
        ));
    };
    let root_position = root.logical_position;
    let crate::types::Ty::LiveRef(stack_in, stack_out, ceiling) = &root.ty else {
        return Err(super::Fail::Internal);
    };
    let mut value_in = alloc::vec::Vec::with_capacity(stack_in.len());
    let mut next_forward = 0usize;
    let mut formal_index = 0;
    // The single-pass fold records its first refusal instead of returning from
    // inside the loop: the extraction tool (Aeneas) rejects early returns inside
    // loops, and the recorded refusal leaves the walked prefix identical.
    let mut refusal: Option<super::Fail> = None;
    while formal_index < stack_in.len() && refusal.is_none() {
        let formal = &stack_in[formal_index];
        match forwarded_ref_matches(formal, forwarded, next_forward, at, context) {
            Ok(true) => next_forward += 1,
            Ok(false) => value_in.push(formal.clone()),
            Err(failure) => refusal = Some(failure),
        }
        formal_index += 1;
    }
    if let Some(refusal) = refusal {
        return Err(refusal);
    }
    if next_forward != forwarded.len() {
        return Err(super::parts::invalid_without_stacks(
            at,
            crate::untrusted::Constraint::BorrowProvenance,
        ));
    }
    let interface = crate::untrusted::Interface {
        stack_in: value_in,
        stack_out: stack_out.as_ref().clone(),
        effects: ceiling.with_id(crate::contracts::LIVE_DISPATCH),
    };
    let extra = match u32::try_from(stack_in.len().saturating_add(forwarded.len())) {
        Ok(extra) => extra,
        Err(_) => return Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Work)),
    };
    let cost = match attempt!(super::parts::join_cost(&interface)).checked_add(extra) {
        Some(cost) => cost,
        None => return Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Work)),
    };
    let root_logical_input_position = match u32::try_from(root_position) {
        Ok(position) => position,
        Err(_) => {
            return Err(super::Fail::Exhausted(
                crate::untrusted::LimitKind::StackHeight,
            ))
        }
    };
    let joined = attempt!(super::parts::join(frame, &interface, at, context));
    let Some(super::BorrowedInput {
        ty: crate::types::Ty::LiveRef(site_in, site_out, site_ceiling),
        ..
    }) = joined.borrowed.get(root_index)
    else {
        return Err(super::Fail::Internal);
    };
    let site = crate::untrusted::LiveSite {
        site_id,
        ref_ordinal,
        root_logical_input_position,
        forwarded_ref_ordinals: forwarded.to_vec(),
        stack_in: site_in.as_ref().clone(),
        stack_out: site_out.as_ref().clone(),
        allowed_effects: site_ceiling.clone(),
    };
    Ok((cost, joined, interface, Some(site)))
}

/// Check one formal borrowed reference against the host-owned logical input.
/// The loop caller records the first refusal; this helper stays loop-free so
/// its early returns remain extractable.
fn forwarded_ref_matches(
    formal: &crate::types::Ty,
    forwarded: &[u32],
    next_forward: usize,
    at: super::parts::Site,
    context: &super::parts::Ctx,
) -> Result<bool, super::Fail> {
    if !matches!(formal, crate::types::Ty::LiveRef(_, _, _)) {
        return Ok(false);
    }
    let logical_position = attempt!(forwarded_position(forwarded, next_forward, at));
    attempt!(validate_forwarded_ref(
        formal,
        logical_position,
        at,
        context
    ));
    Ok(true)
}

fn forwarded_position(
    forwarded: &[u32],
    next_forward: usize,
    at: super::parts::Site,
) -> Result<usize, super::Fail> {
    let Some(position) = forwarded.get(next_forward) else {
        return Err(super::parts::invalid_without_stacks(
            at,
            crate::untrusted::Constraint::BorrowProvenance,
        ));
    };
    let Ok(logical_position) = usize::try_from(*position) else {
        return Err(super::parts::invalid_without_stacks(
            at,
            crate::untrusted::Constraint::BorrowProvenance,
        ));
    };
    Ok(logical_position)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; exact borrowed type comparison calls non-const Ty::PartialEq and rejection constructs owned stack diagnostics."
)]
fn validate_forwarded_ref(
    formal: &crate::types::Ty,
    logical_position: usize,
    at: super::parts::Site,
    context: &super::parts::Ctx,
) -> Result<(), super::Fail> {
    let logical = &context.request.expected.stack_in;
    let Some(actual @ crate::types::Ty::LiveRef(_, _, _)) = logical.get(logical_position) else {
        return Err(super::parts::invalid_without_stacks(
            at,
            crate::untrusted::Constraint::BorrowProvenance,
        ));
    };
    if actual != formal {
        return Err(super::parts::invalid(
            context,
            at,
            alloc::vec![formal.clone()],
            alloc::vec![actual.clone()],
            crate::untrusted::Constraint::StackJoin,
        ));
    }
    Ok(())
}

/// Validate one quotation node and build its parent and child frames.
///
/// Returns the work charged for the node's instantiation and its witness
/// resolution.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; open_quotation validates the witness and depth before reading its three required bindings, with typed rejection or Internal failure instead of panic paths."
)]
pub(super) fn open_quotation(
    frame: super::Frame,
    node_id: crate::untrusted::NodeId,
    node: &crate::untrusted::Node,
    context: &super::parts::Ctx,
    fuel: u32,
) -> Result<(u32, super::Frame, super::Frame), super::Fail> {
    let (body, inst) = match node {
        crate::untrusted::Node::Quotation { body, inst } => (body, inst),
        crate::untrusted::Node::Literal { .. }
        | crate::untrusted::Node::Invocation { .. }
        | crate::untrusted::Node::SignedSelf { .. }
        | crate::untrusted::Node::SlotInvoke { .. } => return Err(super::Fail::Internal),
    };
    let scheme = super::parts::instantiate::quotation_scheme();
    let cost = attempt!(super::parts::scheme_cost(&scheme));
    // The child frame opens from the resolved witness: a reference binding
    // (for example a result stack stated as the input stack) has no direct
    // stack of its own.
    let (_, resolved, spent) = attempt!(super::parts::instantiate::apply(
        &scheme,
        inst,
        None,
        super::parts::site(Some(node_id), None),
        context,
        fuel,
    ));
    let cost = cost.saturating_add(spent);
    if frame.depth.saturating_add(1) > context.request.limits.depth {
        return Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Depth));
    }
    let start = match resolved.stack(crate::words::Variable(1)) {
        Some(segment) => segment.to_vec(),
        None => return Err(super::Fail::Internal),
    };
    let claimed_out = match resolved.stack(crate::words::Variable(2)) {
        Some(segment) => segment.to_vec(),
        None => return Err(super::Fail::Internal),
    };
    let claimed_effects = match resolved.effects(crate::words::Variable(3)) {
        Some(set) => set.clone(),
        None => return Err(super::Fail::Internal),
    };
    let depth = frame.depth.saturating_add(1);
    let child = super::Frame {
        depth,
        origin: Some(node_id),
        body: body.clone(),
        index: 0,
        stack: start,
        borrowed: alloc::vec::Vec::new(),
        effects: crate::types::EffSet::empty(),
        claimed_out,
        claimed_effects,
    };
    Ok((cost, frame, child))
}
