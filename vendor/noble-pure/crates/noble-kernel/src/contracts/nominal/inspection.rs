//! Validate host-provided nominal payload patterns without executing them.

struct PatternLimits {
    remaining: u32,
    max_nodes: u32,
}

impl super::super::Env {
    pub(crate) fn valid_pattern(&self, pattern: &crate::shapes::Pattern, max_nodes: u32) -> bool {
        match pattern {
            crate::shapes::Pattern::Nominal(_, _)
            | crate::shapes::Pattern::GenericNominal(_, _, _)
            | crate::shapes::Pattern::Pair(_, _)
            | crate::shapes::Pattern::Sum(_, _)
            | crate::shapes::Pattern::List(_)
            | crate::shapes::Pattern::Program(_, _, _) => {}
            crate::shapes::Pattern::LiveRef(_, _, _) => return false,
            crate::shapes::Pattern::Resource(kind) => {
                return max_nodes > 0
                    && self.resource_kinds.contains(kind)
                    && !super::super::live::kind_is_slot_resource(self, *kind);
            }
            crate::shapes::Pattern::Unit
            | crate::shapes::Pattern::Bool
            | crate::shapes::Pattern::I64
            | crate::shapes::Pattern::Text
            | crate::shapes::Pattern::Syntax
            | crate::shapes::Pattern::Contract
            | crate::shapes::Pattern::Evidence
            | crate::shapes::Pattern::Certified
            | crate::shapes::Pattern::Var(_)
            | crate::shapes::Pattern::StackVar(_) => return max_nodes > 0,
        }
        let mut work: alloc::vec::Vec<&crate::shapes::Pattern> = alloc::vec::Vec::with_capacity(1);
        let mut visited = 0u32;
        work.push(pattern);
        let mut state = Some(work);
        let mut is_valid = true;
        while let Some(mut work) = state {
            state = match work.pop() {
                Some(next) if visited < max_nodes && work.len() < 512 => {
                    visited += 1;
                    match self.enqueue_pattern(
                        next,
                        work,
                        PatternLimits {
                            remaining: max_nodes.saturating_sub(visited),
                            max_nodes,
                        },
                    ) {
                        Some(updated) => Some(updated),
                        None => {
                            is_valid = false;
                            None
                        }
                    }
                }
                Some(_) => {
                    is_valid = false;
                    None
                }
                None => None,
            };
        }
        is_valid
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; Env::nominal reads a Vec through non-const indexing (E0277/E0658), and descriptor PartialEq is non-const (E0277/E0015) on both pinned Rust compilers; reassess when both become const-capable."
    )]
    fn nominal_shape_matches(
        &self,
        id: crate::types::NominalTypeId,
        shape: &crate::types::NominalShape,
    ) -> bool {
        matches!(self.nominal(id), Some(decl) if &decl.shape == shape)
    }

    fn enqueue_pattern<'a>(
        &self,
        pattern: &'a crate::shapes::Pattern,
        mut work: alloc::vec::Vec<&'a crate::shapes::Pattern>,
        limits: PatternLimits,
    ) -> Option<alloc::vec::Vec<&'a crate::shapes::Pattern>> {
        match pattern {
            crate::shapes::Pattern::Nominal(id, shape) => {
                if !super::r#type::walk::bounded_shape(shape, limits.remaining) {
                    return None;
                }
                let is_matches_shape = self.nominal_shape_matches(*id, shape.as_ref());
                if is_matches_shape {
                    Some(work)
                } else {
                    None
                }
            }
            crate::shapes::Pattern::GenericNominal(id, args, mapping) => {
                let decl = self.generic_variant(*id)?;
                if decl.payload_params != *mapping {
                    return None;
                }
                work.push(&args[0]);
                work.push(&args[1]);
                Some(work)
            }
            crate::shapes::Pattern::Resource(kind) => (self.resource_kinds.contains(kind)
                && !super::super::live::kind_is_slot_resource(self, *kind))
            .then_some(work),
            crate::shapes::Pattern::LiveRef(_, _, _) => None,
            crate::shapes::Pattern::Pair(left, right)
            | crate::shapes::Pattern::Sum(left, right) => {
                work.push(left);
                work.push(right);
                Some(work)
            }
            crate::shapes::Pattern::List(item) => {
                work.push(item);
                Some(work)
            }
            crate::shapes::Pattern::Program(input, output, effects) => {
                self.enqueue_program_pattern(input, output, effects, work, limits.max_nodes)
            }
            crate::shapes::Pattern::Unit
            | crate::shapes::Pattern::Bool
            | crate::shapes::Pattern::I64
            | crate::shapes::Pattern::Text
            | crate::shapes::Pattern::Syntax
            | crate::shapes::Pattern::Contract
            | crate::shapes::Pattern::Evidence
            | crate::shapes::Pattern::Certified
            | crate::shapes::Pattern::Var(_)
            | crate::shapes::Pattern::StackVar(_) => Some(work),
        }
    }

    fn enqueue_program_pattern<'a>(
        &self,
        input: &'a [crate::shapes::Pattern],
        output: &'a [crate::shapes::Pattern],
        effects: &[crate::shapes::EffectSlot],
        mut work: alloc::vec::Vec<&'a crate::shapes::Pattern>,
        max_nodes: u32,
    ) -> Option<alloc::vec::Vec<&'a crate::shapes::Pattern>> {
        let mut index = 0;
        let mut is_effect_set_known = true;
        while index < effects.len() && is_effect_set_known {
            if let crate::shapes::EffectSlot::Effect(id) = &effects[index] {
                is_effect_set_known = self.knows_effect(*id);
            }
            index += 1;
        }
        if !is_effect_set_known {
            return None;
        }
        let child_count = input.len().saturating_add(output.len());
        if u64::try_from(child_count).map_or(true, |count| count > u64::from(max_nodes)) {
            return None;
        }
        if work.len().checked_add(child_count).is_some() {
            work.reserve(child_count);
            work.extend(input.iter());
            work.extend(output.iter());
            Some(work)
        } else {
            None
        }
    }
}
