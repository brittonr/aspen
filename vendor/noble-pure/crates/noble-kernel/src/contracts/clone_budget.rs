//! Borrowed, bounded estimate of an owned calculator environment clone.
//!
//! Each node is counted at every occurrence: boxed nominal payloads and
//! program stacks cannot hide behind a small top-level scheme count.

/// Calculator source and compiler environments retain at most 896 definition
/// rows. This is independent of the reachable named-body bound.
pub const CALCULATOR_ENVIRONMENT_ROW_LIMIT: usize = 896;
/// One selected clone may reserve at most 16 MiB, including conservative
/// payload rounding and per-allocation overhead. The observed live ASCII
/// clone reserves under 9 MiB; this leaves room for the remaining closure
/// without admitting unbounded nested nominal and program payloads.
pub const CALCULATOR_ENV_CLONE_BYTES_LIMIT: u64 = 1u64 << 24;
/// Aggregate selected copies reserve at most one GiB, leaving the other half
/// of the compiler's two-billion-Work ceiling for actual checking and
/// lowering. The real 59-definition ASCII root reserves ~501 MiB.
pub const CALCULATOR_ENV_CLONE_FANOUT_LIMIT: u64 = 1u64 << 30;
const DEPTH_LIMIT: usize = 64;
const NODE_LIMIT: usize = 512;

/// Payload bytes and allocator-rounded reservation for one complete clone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    tigerstyle::path_segment_repetition,
    reason = "public kernel budget type; renaming it would change the extracted subject for a path-discipline lint"
)]
pub struct CloneBudget {
    pub bytes: u64,
    pub allocations: u64,
    pub reserved_bytes: u64,
}

struct Counter {
    bytes: u64,
    allocations: u64,
}

#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "the bounded accounting walker mutates only its private counter and per-subtree node budget"
)]
impl Counter {
    fn bytes(&mut self, bytes: u64) -> Option<()> {
        self.bytes = self.bytes.checked_add(bytes)?;
        (self.bytes <= CALCULATOR_ENV_CLONE_BYTES_LIMIT).then_some(())
    }

    fn allocation(&mut self) -> Option<()> {
        self.allocations = self.allocations.checked_add(1)?;
        Some(())
    }

    fn vector<T>(&mut self, len: usize) -> Option<()> {
        self.bytes(
            u64::try_from(len)
                .ok()?
                .checked_mul(u64::try_from(core::mem::size_of::<T>()).ok()?)?,
        )?;
        if len != 0 {
            self.allocation()?;
        }
        Some(())
    }

    fn node<T>(&mut self) -> Option<()> {
        self.bytes(u64::try_from(core::mem::size_of::<T>()).ok()?)
    }

    fn boxed_pattern(
        &mut self,
        value: &crate::shapes::Pattern,
        depth: usize,
        nodes: &mut usize,
    ) -> Option<()> {
        self.allocation()?;
        self.pattern(value, depth, nodes)
    }

    #[expect(
        tigerstyle::no_recursion,
        reason = "depth and node budgets are per subtree, so the walk is deliberately recursive; a worklist rewrite would change the accounting subject"
    )]
    fn pattern(
        &mut self,
        value: &crate::shapes::Pattern,
        depth: usize,
        nodes: &mut usize,
    ) -> Option<()> {
        if depth >= DEPTH_LIMIT || *nodes >= NODE_LIMIT {
            return None;
        }
        *nodes = nodes.saturating_add(1);
        self.node::<crate::shapes::Pattern>()?;
        match value {
            crate::shapes::Pattern::Pair(left, right)
            | crate::shapes::Pattern::Sum(left, right) => {
                self.boxed_pattern(left, depth.saturating_add(1), nodes)?;
                self.boxed_pattern(right, depth.saturating_add(1), nodes)?;
            }
            crate::shapes::Pattern::List(item) => {
                self.boxed_pattern(item, depth.saturating_add(1), nodes)?
            }
            crate::shapes::Pattern::Program(input, output, effects) => {
                if input.len() > 128 || output.len() > 128 || effects.len() > 8 {
                    return None;
                }
                self.node::<alloc::vec::Vec<crate::shapes::Pattern>>()?;
                self.node::<alloc::vec::Vec<crate::shapes::Pattern>>()?;
                self.node::<alloc::vec::Vec<crate::shapes::EffectSlot>>()?;
                self.allocations = self.allocations.checked_add(3)?;
                if !input.is_empty() {
                    self.allocation()?;
                }
                if !output.is_empty() {
                    self.allocation()?;
                }
                self.vector::<crate::shapes::EffectSlot>(effects.len())?;
                for item in input.iter().chain(output.iter()) {
                    self.pattern(item, depth.saturating_add(1), nodes)?;
                }
            }
            crate::shapes::Pattern::Nominal(_, shape) => {
                self.allocation()?;
                self.node::<crate::types::NominalShape>()?;
                self.nominal(shape, 0, &mut 0)?;
            }
            crate::shapes::Pattern::GenericNominal(_, arguments, _) => {
                self.allocation()?;
                self.pattern(&arguments[0], depth.saturating_add(1), nodes)?;
                self.pattern(&arguments[1], depth.saturating_add(1), nodes)?;
            }
            // Calculator environments never admit borrowed live references.
            crate::shapes::Pattern::LiveRef(..) => return None,
            crate::shapes::Pattern::Unit
            | crate::shapes::Pattern::Bool
            | crate::shapes::Pattern::I64
            | crate::shapes::Pattern::Text
            | crate::shapes::Pattern::Syntax
            | crate::shapes::Pattern::Contract
            | crate::shapes::Pattern::Evidence
            | crate::shapes::Pattern::Certified
            | crate::shapes::Pattern::Resource(_)
            | crate::shapes::Pattern::Var(_)
            | crate::shapes::Pattern::StackVar(_) => {}
        }
        Some(())
    }

    fn boxed_type(
        &mut self,
        value: &crate::types::Ty,
        depth: usize,
        nodes: &mut usize,
    ) -> Option<()> {
        self.allocation()?;
        self.ty(value, depth, nodes)
    }

    fn nominal(
        &mut self,
        shape: &crate::types::NominalShape,
        depth: usize,
        nodes: &mut usize,
    ) -> Option<()> {
        match shape {
            crate::types::NominalShape::Opaque(value) => self.boxed_type(value, depth, nodes)?,
            crate::types::NominalShape::Variant(left, right) => {
                self.boxed_type(left, depth, nodes)?;
                self.boxed_type(right, depth, nodes)?;
            }
        }
        Some(())
    }

    #[expect(
        tigerstyle::no_recursion,
        reason = "depth and node budgets are per subtree, so the walk is deliberately recursive; a worklist rewrite would change the accounting subject"
    )]
    fn ty(&mut self, value: &crate::types::Ty, depth: usize, nodes: &mut usize) -> Option<()> {
        if depth >= DEPTH_LIMIT || *nodes >= NODE_LIMIT {
            return None;
        }
        *nodes = nodes.saturating_add(1);
        self.node::<crate::types::Ty>()?;
        match value {
            crate::types::Ty::Pair(left, right) | crate::types::Ty::Sum(left, right) => {
                self.boxed_type(left, depth.saturating_add(1), nodes)?;
                self.boxed_type(right, depth.saturating_add(1), nodes)?;
            }
            crate::types::Ty::List(item) => {
                self.boxed_type(item, depth.saturating_add(1), nodes)?
            }
            crate::types::Ty::Nominal(_, shape) => {
                self.allocation()?;
                self.node::<crate::types::NominalShape>()?;
                self.nominal(shape, depth.saturating_add(1), nodes)?;
            }
            crate::types::Ty::GenericNominal(_, arguments, shape) => {
                if matches!(shape.as_ref(), crate::types::NominalShape::Opaque(_)) {
                    return None;
                }
                self.allocation()?;
                self.ty(&arguments[0], depth.saturating_add(1), nodes)?;
                self.ty(&arguments[1], depth.saturating_add(1), nodes)?;
                self.allocation()?;
                self.node::<crate::types::NominalShape>()?;
                self.nominal(shape, depth.saturating_add(1), nodes)?;
            }
            crate::types::Ty::Program(input, output, effects) => {
                if input.len() > 128 || output.len() > 128 || effects.as_slice().len() > 2 {
                    return None;
                }
                self.node::<alloc::vec::Vec<crate::types::Ty>>()?;
                self.node::<alloc::vec::Vec<crate::types::Ty>>()?;
                self.allocations = self.allocations.checked_add(2)?;
                if !input.is_empty() {
                    self.allocation()?;
                }
                if !output.is_empty() {
                    self.allocation()?;
                }
                self.vector::<crate::types::EffId>(effects.as_slice().len())?;
                for item in input.iter().chain(output.iter()) {
                    self.ty(item, depth.saturating_add(1), nodes)?;
                }
            }
            crate::types::Ty::LiveRef(..) => return None,
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
        Some(())
    }

    fn scheme(&mut self, scheme: &crate::words::Scheme) -> Option<()> {
        let has_too_many_kinds = scheme.var_kinds.len() > 8;
        let has_too_many_effects = scheme.effects.len() > 8;
        let has_too_many_stacks = scheme.stack_in.len() > 128 || scheme.stack_out.len() > 128;
        if has_too_many_kinds || has_too_many_effects || has_too_many_stacks {
            return None;
        }
        self.vector::<crate::words::VariableKind>(scheme.var_kinds.len())?;
        self.vector::<crate::shapes::EffectSlot>(scheme.effects.len())?;
        if !scheme.stack_in.is_empty() {
            self.allocation()?;
        }
        if !scheme.stack_out.is_empty() {
            self.allocation()?;
        }
        for value in scheme.stack_in.iter().chain(scheme.stack_out.iter()) {
            self.pattern(value, 0, &mut 0)?;
        }
        Some(())
    }
}

impl super::Env {
    /// Count one clone without cloning input or allocating. Twice the
    /// requested payload plus 64 bytes per allocation is a conservative
    /// admission estimate for allocator rounding, not a claim to cap RSS
    /// from the entire process. Every addition fails closed before source
    /// or kernel owns another Env.
    pub fn calculator_clone_budget(&self) -> Option<CloneBudget> {
        if !self.calculator || !self.schemas.is_empty() || !self.bound_adapters.is_empty() {
            return None;
        }
        let mut cost = Counter {
            bytes: 0,
            allocations: 0,
        };
        cost.node::<super::Env>()?;
        cost.vector::<crate::words::Scheme>(self.defs.len())?;
        cost.vector::<super::Behavior>(self.kinds.len())?;
        cost.vector::<alloc::vec::Vec<super::Definition>>(self.deps.len())?;
        cost.vector::<super::NominalDecl>(self.nominals.len())?;
        cost.vector::<super::GenericVariantDecl>(self.generic_variants.len())?;
        cost.vector::<crate::types::ResourceKind>(self.resource_kinds.len())?;
        cost.vector::<Option<u64>>(self.definition_owners.len())?;
        cost.vector::<crate::types::EffId>(self.effects.len())?;
        // Accumulate the first budget refusal instead of returning from inside a
        // loop: the extraction tool (Aeneas) rejects returns inside loop bodies,
        // and a refused element stops the walk exactly as `?` did.
        let mut index = 0;
        let mut walked: Option<()> = Some(());
        while index < self.deps.len() && walked.is_some() {
            walked = cost.vector::<super::Definition>(self.deps[index].len());
            index += 1;
        }
        walked?;
        let mut index = 0;
        let mut walked: Option<()> = Some(());
        while index < self.defs.len() && walked.is_some() {
            walked = cost.scheme(&self.defs[index]);
            index += 1;
        }
        walked?;
        let mut index = 0;
        let mut walked: Option<()> = Some(());
        while index < self.nominals.len() && walked.is_some() {
            walked = cost.nominal(&self.nominals[index].shape, 0, &mut 0);
            index += 1;
        }
        walked?;
        let reserved_bytes = cost
            .bytes
            .checked_mul(2)?
            .checked_add(cost.allocations.checked_mul(64)?)?;
        (reserved_bytes <= CALCULATOR_ENV_CLONE_BYTES_LIMIT).then_some(CloneBudget {
            bytes: cost.bytes,
            allocations: cost.allocations,
            reserved_bytes,
        })
    }
}
