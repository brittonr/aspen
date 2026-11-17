//! Bounded concrete-type and embedded nominal-descriptor walks.

mod work;

struct Scope {
    max_nodes: u32,
    payload_only: bool,
    prior: usize,
    caller: Option<u64>,
}

impl super::super::super::Env {
    /// Exact validation of one untrusted type tree against environment schemas.
    /// Nominal nodes compare their *entire* embedded descriptor, not just id.
    pub fn valid_type(&self, ty: &crate::types::Ty, max_nodes: u32) -> bool {
        self.walk_type(
            ty,
            max_nodes,
            false,
            self.nominals.len(),
            self.caller_module,
        )
    }

    /// Check an operation payload exposed to a foreign caller. An exported
    /// opaque value is public as a value without exposing its private repr.
    pub fn public_payload(&self, ty: &crate::types::Ty, max_nodes: u32) -> bool {
        self.walk_type(ty, max_nodes, false, self.nominals.len(), None)
    }

    pub(crate) fn valid_nominal_payload(&self, ty: &crate::types::Ty, max_nodes: u32) -> bool {
        self.walk_type(ty, max_nodes, true, self.nominals.len(), None)
    }

    fn walk_type(
        &self,
        ty: &crate::types::Ty,
        max_nodes: u32,
        payload_only: bool,
        prior: usize,
        caller: Option<u64>,
    ) -> bool {
        match ty {
            crate::types::Ty::Unit
            | crate::types::Ty::Bool
            | crate::types::Ty::I64
            | crate::types::Ty::Text
            | crate::types::Ty::Syntax => return max_nodes > 0,
            crate::types::Ty::Resource(kind) => {
                return max_nodes > 0
                    && self.resource_kinds.contains(kind)
                    && (payload_only
                        || !super::super::super::live::kind_is_slot_resource(self, *kind));
            }
            crate::types::Ty::LiveRef(_, _, _) => return false,
            crate::types::Ty::Contract
            | crate::types::Ty::Evidence
            | crate::types::Ty::Certified => {
                return max_nodes > 0 && !payload_only;
            }
            _ => {}
        }
        let mut pending = alloc::vec::Vec::with_capacity(1);
        let scope = Scope {
            max_nodes,
            payload_only,
            prior,
            caller,
        };
        pending.push((ty, payload_only));
        let mut state = Some(work::State {
            pending,
            visited: 0,
        });
        let mut has_work = true;
        while has_work {
            state = match state {
                Some(current) if current.pending.is_empty() => {
                    has_work = false;
                    Some(current)
                }
                Some(current) => self.advance_checked_type(current, &scope),
                None => {
                    has_work = false;
                    None
                }
            };
        }
        state.is_some()
    }

    fn advance_checked_type<'a>(
        &self,
        mut state: work::State<'a>,
        scope: &Scope,
    ) -> Option<work::State<'a>> {
        let Some((next, payload_restricted)) = state.pending.pop() else {
            return Some(state);
        };
        if state.visited >= scope.max_nodes || state.pending.len() >= 512 {
            return None;
        }
        state.visited += 1;
        self.enqueue_checked_type(next, payload_restricted, state, scope)
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; State::with_pair/with_type/with_program mutate the pending Vec through non-const push/extend (E0658/E0015) on both pinned Rust compilers; reassess when those transitions become const-capable."
    )]
    fn enqueue_checked_type<'a>(
        &self,
        ty: &'a crate::types::Ty,
        payload_restricted: bool,
        state: work::State<'a>,
        scope: &Scope,
    ) -> Option<work::State<'a>> {
        match ty {
            crate::types::Ty::Nominal(id, shape) => {
                self.enqueue_known_nominal(*id, shape, payload_restricted, state, scope)
            }
            crate::types::Ty::GenericNominal(id, args, shape) => {
                self.enqueue_known_generic(*id, args, shape, payload_restricted, state, scope)
            }
            crate::types::Ty::Resource(kind) => (self.resource_kinds.contains(kind)
                && (payload_restricted
                    || !super::super::super::live::kind_is_slot_resource(self, *kind)))
            .then_some(state),
            crate::types::Ty::LiveRef(_, _, _) => None,
            crate::types::Ty::Pair(left, right) | crate::types::Ty::Sum(left, right) => {
                Some(state.with_pair(left, right, payload_restricted))
            }
            crate::types::Ty::List(item) => Some(state.with_type(item, payload_restricted)),
            crate::types::Ty::Program(input, output, effects) => {
                if !super::super::validation::valid_program_components(
                    self,
                    input,
                    output,
                    effects,
                    scope.max_nodes,
                ) {
                    return None;
                }
                let child_count = input.len().saturating_add(output.len());
                if state.pending.len().checked_add(child_count).is_some() {
                    Some(state.with_program(input, output))
                } else {
                    None
                }
            }
            crate::types::Ty::Unit
            | crate::types::Ty::Bool
            | crate::types::Ty::I64
            | crate::types::Ty::Text
            | crate::types::Ty::Syntax => Some(state),
            crate::types::Ty::Contract
            | crate::types::Ty::Evidence
            | crate::types::Ty::Certified
                if !payload_restricted =>
            {
                Some(state)
            }
            _ => None,
        }
    }

    fn enqueue_known_nominal<'a>(
        &self,
        id: crate::types::NominalTypeId,
        shape: &'a crate::types::NominalShape,
        payload_restricted: bool,
        state: work::State<'a>,
        scope: &Scope,
    ) -> Option<work::State<'a>> {
        if !bounded_shape(shape, scope.max_nodes.saturating_sub(state.visited)) {
            return None;
        }
        let mut index = 0;
        while index < scope.prior && self.nominals[index].id != id {
            index += 1;
        }
        if index == scope.prior {
            return None;
        }
        let decl = &self.nominals[index];
        let is_visible = scope.payload_only || decl.exported || scope.caller == Some(id.module);
        if &decl.shape != shape || !is_visible {
            return None;
        }
        if payload_restricted {
            return state.with_descriptor(shape, true);
        }
        Some(state)
    }

    #[expect(
        tigerstyle::too_many_parameters,
        reason = "Owner: noble-kernel; the parameter list is the kernel internal call shape; bundling it would change the extracted subject"
    )]
    fn enqueue_known_generic<'a>(
        &self,
        id: crate::types::NominalTypeId,
        args: &'a [crate::types::Ty; 2],
        shape: &'a crate::types::NominalShape,
        payload_restricted: bool,
        state: work::State<'a>,
        scope: &Scope,
    ) -> Option<work::State<'a>> {
        if !args[0].valid_generic_argument()
            || !args[1].valid_generic_argument()
            || !bounded_shape(shape, scope.max_nodes.saturating_sub(state.visited))
        {
            return None;
        }
        let decl = self.generic_variant(id)?;
        if !(scope.payload_only || decl.exported || scope.caller == Some(id.module))
            || !super::super::generic_descriptor_matches(decl, args, shape)
        {
            return None;
        }
        // Both ordered arguments are traversed in the same bounded walk,
        // rather than recursively restarting validation for nested families.
        if payload_restricted {
            return state.with_descriptor(shape, true);
        }
        Some(state.with_pair(&args[0], &args[1], false))
    }

    pub(crate) fn validate_decl(&self, index: usize, max_nodes: u32) -> bool {
        let decl = match self.nominals.get(index) {
            Some(decl) => decl,
            None => return false,
        };
        let mut prior = 0;
        while prior < index && self.nominals[prior].id != decl.id {
            prior += 1;
        }
        if prior < index || self.generic_variant(decl.id).is_some() {
            return false;
        }
        if self.live_slots && !self.live_resource_nominals.contains(&decl.id) {
            let is_owns_resource = match &decl.shape {
                crate::types::NominalShape::Opaque(representation) => !representation.is_data(),
                crate::types::NominalShape::Variant(left, right) => {
                    !left.is_data() || !right.is_data()
                }
            };
            if is_owns_resource {
                return false;
            }
        }
        if let crate::types::NominalShape::Opaque(ty) = &decl.shape {
            return self.walk_type(ty, max_nodes, true, index, None);
        }
        if let crate::types::NominalShape::Variant(left, right) = &decl.shape {
            return self.walk_type(left, max_nodes, true, index, None)
                && self.walk_type(right, max_nodes, true, index, None);
        }
        false
    }
}

/// Bound the entire supplied descriptor before structural equality can walk it.
pub(in super::super) fn bounded_shape(shape: &crate::types::NominalShape, max_nodes: u32) -> bool {
    let Some(initial) = work::State::for_descriptor(shape) else {
        return false;
    };
    let mut state = Some(initial);
    let mut is_bounded = true;
    while let Some(current) = state {
        state = match bounded_step(current, max_nodes) {
            ShapeStep::Continue(updated) => Some(updated),
            ShapeStep::Done => None,
            _ => {
                is_bounded = false;
                None
            }
        };
    }
    is_bounded
}

enum ShapeStep<'a> {
    Continue(work::State<'a>),
    Done,
    Failed,
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; both pinned Rust compilers reject pending Vec::pop (E0015) before charged shape traversal; reassess when this work-queue transition becomes const-capable."
)]
fn bounded_step<'a>(mut state: work::State<'a>, max_nodes: u32) -> ShapeStep<'a> {
    let Some((ty, _)) = state.pending.pop() else {
        return ShapeStep::Done;
    };
    if state.visited >= max_nodes || state.pending.len() >= 512 {
        return ShapeStep::Failed;
    }
    state.visited += 1;
    if !program_children_fit(ty, max_nodes) {
        return ShapeStep::Failed;
    }
    match enqueue_shape(ty, state) {
        Some(updated) => ShapeStep::Continue(updated),
        None => ShapeStep::Failed,
    }
}

const fn program_children_fit(ty: &crate::types::Ty, max_nodes: u32) -> bool {
    if let crate::types::Ty::Program(input, output, _) = ty {
        let child_count = input.len().saturating_add(output.len());
        return (child_count as u128) <= (max_nodes as u128);
    }
    true
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; State::with_type/with_pair/with_program/with_descriptor mutate the pending Vec via non-const push/extend (E0658/E0015) under both pinned Rust compilers; reassess when those transitions become const-capable."
)]
fn enqueue_shape<'a>(ty: &'a crate::types::Ty, state: work::State<'a>) -> Option<work::State<'a>> {
    match ty {
        crate::types::Ty::Pair(left, right) | crate::types::Ty::Sum(left, right) => {
            Some(state.with_pair(left, right, false))
        }
        crate::types::Ty::List(item) => Some(state.with_type(item, false)),
        crate::types::Ty::Program(input, output, _) => Some(state.with_program(input, output)),
        crate::types::Ty::Nominal(_, nested) => state.with_descriptor(nested, false),
        crate::types::Ty::GenericNominal(_, args, nested) => state.with_generic(args, nested),
        _ => is_shape_leaf(ty).then_some(state),
    }
}

const fn is_shape_leaf(ty: &crate::types::Ty) -> bool {
    matches!(
        ty,
        crate::types::Ty::Unit
            | crate::types::Ty::Bool
            | crate::types::Ty::I64
            | crate::types::Ty::Text
            | crate::types::Ty::Syntax
            | crate::types::Ty::Contract
            | crate::types::Ty::Evidence
            | crate::types::Ty::Certified
            | crate::types::Ty::Resource(_)
    )
}
