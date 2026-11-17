//! Rank-1 word schemes, finite instantiation, and interface substitution.
//!
//! An instantiation maps every declared variable to one binding of the
//! matching kind; substituting a scheme's patterns yields the concrete
//! interface exactly. Substitution lives in `subst` and is one bounded,
//! iterative post-order walk.

pub mod resolve;

pub mod subst;

mod segments;

pub(crate) mod bounds;

/// Index of a variable inside one scheme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Variable(pub u32);

/// What a scheme variable stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariableKind {
    /// A whole stack segment.
    Stack,
    /// One value type occupying one stack position.
    Value,
    /// An effect set.
    Effect,
}

/// A rank-1 scheme over the fixed variable kinds.
#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Clone, Debug)]
pub struct Scheme {
    /// Kinds of the scheme's variables, indexed by `Variable`.
    pub var_kinds: alloc::vec::Vec<VariableKind>,
    /// Required invocation stack pattern, bottom-first.
    pub stack_in: alloc::vec::Vec<crate::shapes::Pattern>,
    /// Result stack pattern, bottom-first.
    pub stack_out: alloc::vec::Vec<crate::shapes::Pattern>,
    /// Latent effect bound pattern.
    pub effects: alloc::vec::Vec<crate::shapes::EffectSlot>,
}

/// One variable's concrete value.
#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Clone, Debug)]
pub enum Binding {
    /// A stack segment for a stack variable.
    Stack(alloc::vec::Vec<crate::types::Ty>),
    /// A type for a value variable.
    Value(crate::types::Ty),
    /// An effect set for an effect variable.
    Effect(crate::types::EffSet),
    /// The witness of another variable of the same instantiation: a type
    /// equation between two variables. A chain of references must be finite
    /// and well founded; a cycle is rejected (B-CHECK-05).
    Ref(Variable),
}

/// A concrete instantiation: one binding per declared variable, in order.
#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Clone, Debug)]
pub struct Inst {
    /// One binding per entry of `Scheme::var_kinds`.
    pub bindings: alloc::vec::Vec<Binding>,
}

/// Why an instantiation does not fit its scheme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstError {
    /// A binding's kind does not match its declared kind.
    KindMismatch,
    /// A variable index exceeds the declared variable count.
    UnknownVariable,
    /// The instantiation carries a wrong number of bindings.
    ArityMismatch,
    /// A bound stack exceeds the declared stack-height limit.
    OversizedStack,
    /// A bound type exceeds the declared type-size limit.
    OversizedType,
    /// A bound effect set exceeds the environment's effect-identity count.
    OversizedEffects,
    /// A reference chain is cyclic: it returns to a variable already on
    /// the chain.
    CyclicWitness,
    /// A reference-resolution walk exceeded its declared work limit.
    WalkExhausted,
}

fn slot(var: Variable) -> Option<usize> {
    usize::try_from(var.0).ok()
}

impl Inst {
    /// The stack bound to a stack variable, if the binding has that kind.
    pub fn stack(&self, var: Variable) -> Option<&[crate::types::Ty]> {
        match slot(var) {
            Some(index) => match self.bindings.get(index) {
                Some(Binding::Stack(segment)) => Some(segment.as_slice()),
                Some(Binding::Value(_))
                | Some(Binding::Effect(_))
                | Some(Binding::Ref(_))
                | None => None,
            },
            None => None,
        }
    }

    /// The type bound to a value variable, if the binding has that kind.
    pub fn value(&self, var: Variable) -> Option<&crate::types::Ty> {
        match slot(var) {
            Some(index) => match self.bindings.get(index) {
                Some(Binding::Value(ty)) => Some(ty),
                Some(Binding::Stack(_))
                | Some(Binding::Effect(_))
                | Some(Binding::Ref(_))
                | None => None,
            },
            None => None,
        }
    }

    /// The effect set bound to an effect variable, if the binding has that kind.
    pub fn effects(&self, var: Variable) -> Option<&crate::types::EffSet> {
        match slot(var) {
            Some(index) => match self.bindings.get(index) {
                Some(Binding::Effect(set)) => Some(set),
                Some(Binding::Stack(_))
                | Some(Binding::Value(_))
                | Some(Binding::Ref(_))
                | None => None,
            },
            None => None,
        }
    }
}

impl Scheme {
    /// Validate that every variable use matches its declared kind.
    pub fn validate(&self) -> Result<(), crate::shapes::Defect> {
        crate::shapes::validate(
            &self.var_kinds,
            &self.stack_in,
            &self.stack_out,
            &self.effects,
        )
    }

    /// Check that an instantiation fits this scheme within the given bounds.
    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-maintainers; the public witness check takes named u32 entry-count and type-node limits, returning distinct OversizedStack/OversizedType errors; retaining these scalar bounds preserves the existing API."
    )]
    pub fn check_inst(
        &self,
        inst: &Inst,
        max_stack: u32,
        max_type: u32,
        max_effects: u64,
    ) -> Result<(), InstError> {
        if inst.bindings.len() != self.var_kinds.len() {
            return Err(InstError::ArityMismatch);
        }
        let mut binding_index = 0;
        let mut failure: Option<InstError> = None;
        while binding_index < inst.bindings.len() && binding_index < self.var_kinds.len() {
            let step = bounds::check_binding(
                &inst.bindings[binding_index],
                &self.var_kinds[binding_index],
                max_stack,
                max_type,
                max_effects,
            );
            match step {
                Ok(()) => binding_index += 1,
                Err(problem) => {
                    failure = Some(problem);
                    break;
                }
            }
        }
        match failure {
            Some(problem) => Err(problem),
            None => Ok(()),
        }
    }

    /// Substitute a stack pattern to a concrete stack.
    pub fn subst_stack(
        &self,
        parts: &[crate::shapes::Pattern],
        inst: &Inst,
    ) -> Result<alloc::vec::Vec<crate::types::Ty>, InstError> {
        let mut out: alloc::vec::Vec<crate::types::Ty> =
            alloc::vec::Vec::with_capacity(crate::capacity::at_least(parts.len(), 4));
        let mut part_index = 0;
        let mut failure: Option<InstError> = None;
        while part_index < parts.len() {
            let step = match parts[part_index].clone() {
                crate::shapes::Pattern::StackVar(var) => {
                    let segment: Option<alloc::vec::Vec<crate::types::Ty>> =
                        inst.stack(var).map(|found| found.to_vec());
                    match segment {
                        Some(owned) => {
                            out.extend_from_slice(&owned);
                            Ok(())
                        }
                        None => Err(InstError::UnknownVariable),
                    }
                }
                pattern => match self.subst_pattern(&pattern, inst) {
                    Ok(ty) => {
                        out.push(ty);
                        Ok(())
                    }
                    Err(problem) => Err(problem),
                },
            };
            match step {
                Ok(()) => part_index += 1,
                Err(problem) => {
                    failure = Some(problem);
                    break;
                }
            }
        }
        match failure {
            Some(problem) => Err(problem),
            None => Ok(out),
        }
    }

    /// Substitute an effect pattern to a concrete set.
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-maintainers; each EffectSlot constructor must explicitly contribute a concrete identity or a resolved effect binding; a new slot form must require an implemented substitution rule."
    )]
    pub fn subst_effects(
        &self,
        slots: &[crate::shapes::EffectSlot],
        inst: &Inst,
    ) -> Result<crate::types::EffSet, InstError> {
        let bound = slots.len().saturating_mul(4).saturating_add(4);
        let mut ids: alloc::vec::Vec<crate::types::EffId> = alloc::vec::Vec::with_capacity(bound);
        let mut slot_index = 0;
        let mut failure: Option<InstError> = None;
        while slot_index < slots.len() {
            let step = match slots[slot_index].clone() {
                crate::shapes::EffectSlot::Effect(id) => {
                    ids.push(id);
                    Ok(())
                }
                crate::shapes::EffectSlot::Var(var) => {
                    let bound: Option<alloc::vec::Vec<crate::types::EffId>> =
                        inst.effects(var).map(|set| set.as_slice().to_vec());
                    match bound {
                        Some(owned) => {
                            ids.extend_from_slice(&owned);
                            Ok(())
                        }
                        None => Err(InstError::UnknownVariable),
                    }
                }
            };
            match step {
                Ok(()) => slot_index += 1,
                Err(problem) => {
                    failure = Some(problem);
                    break;
                }
            }
        }
        match failure {
            Some(problem) => Err(problem),
            None => Ok(crate::types::EffSet::from_ids(&ids)),
        }
    }
}
