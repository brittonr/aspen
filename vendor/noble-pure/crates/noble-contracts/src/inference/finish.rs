impl super::materialize::State {
    pub(super) fn finish(
        mut self,
        term: super::Term,
        effects: noble_kernel::types::EffSet,
        generics: &[noble_kernel::contracts::GenericVariantDecl],
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<Self, crate::Diagnostic> {
        let b = attempt!(self.pop(span));
        let material = match term {
            super::Term::List(_) => list(b, span),
            super::Term::Pair(_, _) | super::Term::Sum(_, _) => {
                let a = attempt!(self.pop(span));
                pair(matches!(term, super::Term::Pair(_, _)), a, b, span)
            }
            super::Term::Program(_, _) => {
                let a = attempt!(self.pop(span));
                program(a, b, effects, span)
            }
            super::Term::GenericNominal(id, _, _) => {
                let a = attempt!(self.pop(span));
                generic_nominal(id, a, b, generics, span, meter)
            }
            super::Term::Push(_, _) => {
                let a = attempt!(self.pop(span));
                push(a, b, span)
            }
            super::Term::Hole(_)
            | super::Term::RigidValue(_)
            | super::Term::RigidStack(_)
            | super::Term::Link(_)
            | super::Term::Unit
            | super::Term::Bool
            | super::Term::I64
            | super::Term::Text
            | super::Term::Syntax
            | super::Term::Contract
            | super::Term::Evidence
            | super::Term::Certified
            | super::Term::Resource(_)
            | super::Term::Nominal(_)
            | super::Term::Empty => Err(crate::internal(span)),
        };
        self.values.push(attempt!(material));
        Ok(self)
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::too_many_parameters,
    reason = "Owner: noble-contracts; the parameter list is the internal call shape; bundling it would change the extracted subject"
)]
/// Build one generic instance after both of its limits are known to hold.
///
/// The instance owns its two ordered arguments *and* a canonical descriptor
/// holding a second copy of the selected payload arguments. The descriptor
/// copies are charged to the work meter and the resulting owned node count is
/// checked against `TYPE_NODE_CAP` before anything is cloned, so nesting
/// cannot double host allocation per level beneath a linear logical size.
fn generic_nominal(
    id: noble_kernel::types::NominalTypeId,
    first: super::materialize::Material,
    second: super::materialize::Material,
    generics: &[noble_kernel::contracts::GenericVariantDecl],
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<super::materialize::Material, crate::Diagnostic> {
    let Some(declaration) = generics.iter().find(|decl| decl.id == id) else {
        return Err(crate::internal(span));
    };
    let (
        super::materialize::Material::Value(a, a_extent),
        super::materialize::Material::Value(b, b_extent),
    ) = (first, second)
    else {
        return Err(crate::internal(span));
    };
    let extents = [a_extent, b_extent];
    let (Some(left), Some(right)) = (
        extents.get(usize::from(declaration.payload_params[0])),
        extents.get(usize::from(declaration.payload_params[1])),
    ) else {
        return Err(crate::internal(span));
    };
    let constructors = attempt!(type_size(
        a_extent
            .constructors
            .saturating_add(b_extent.constructors)
            .saturating_add(1),
        span
    ));
    let copied = attempt!(type_nodes(left.nodes, right.nodes, span));
    let retained = attempt!(type_nodes(a_extent.nodes, b_extent.nodes, span));
    let nodes = attempt!(type_nodes(
        attempt!(type_nodes(retained, copied, span)),
        1,
        span
    ));
    attempt!(meter.charge(copied, span));
    let arguments = [a, b];
    let shape = noble_kernel::types::NominalShape::Variant(
        alloc::boxed::Box::new(arguments[usize::from(declaration.payload_params[0])].clone()),
        alloc::boxed::Box::new(arguments[usize::from(declaration.payload_params[1])].clone()),
    );
    Ok(super::materialize::Material::Value(
        noble_kernel::types::Ty::GenericNominal(
            id,
            alloc::boxed::Box::new(arguments),
            alloc::boxed::Box::new(shape),
        ),
        super::materialize::Extent {
            constructors,
            nodes,
        },
    ))
}

fn list(
    item: super::materialize::Material,
    span: crate::Span,
) -> Result<super::materialize::Material, crate::Diagnostic> {
    match item {
        super::materialize::Material::Value(ty, extent) => Ok(super::materialize::Material::Value(
            noble_kernel::types::Ty::List(alloc::boxed::Box::new(ty)),
            super::materialize::Extent {
                constructors: attempt!(type_size(extent.constructors.saturating_add(1), span)),
                nodes: attempt!(type_nodes(extent.nodes, 1, span)),
            },
        )),
        super::materialize::Material::Stack(_, _) => Err(crate::internal(span)),
    }
}

fn pair(
    is_pair: bool,
    a: super::materialize::Material,
    b: super::materialize::Material,
    span: crate::Span,
) -> Result<super::materialize::Material, crate::Diagnostic> {
    match (a, b) {
        (
            super::materialize::Material::Value(a, sa),
            super::materialize::Material::Value(b, sb),
        ) => {
            let extent = attempt!(joined(sa, sb, span));
            let ty = if is_pair {
                noble_kernel::types::Ty::Pair(alloc::boxed::Box::new(a), alloc::boxed::Box::new(b))
            } else {
                noble_kernel::types::Ty::Sum(alloc::boxed::Box::new(a), alloc::boxed::Box::new(b))
            };
            Ok(super::materialize::Material::Value(ty, extent))
        }
        (super::materialize::Material::Stack(_, _), _)
        | (super::materialize::Material::Value(_, _), super::materialize::Material::Stack(_, _)) => {
            Err(crate::internal(span))
        }
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; Ty::program owns its allocated input/output stacks and constructor-limit errors allocate diagnostics."
)]
fn program(
    inputs: super::materialize::Material,
    outputs: super::materialize::Material,
    effects: noble_kernel::types::EffSet,
    span: crate::Span,
) -> Result<super::materialize::Material, crate::Diagnostic> {
    match (inputs, outputs) {
        (
            super::materialize::Material::Stack(a, sa),
            super::materialize::Material::Stack(b, sb),
        ) => Ok(super::materialize::Material::Value(
            noble_kernel::types::Ty::program(a, b, effects),
            attempt!(joined(sa, sb, span)),
        )),
        (super::materialize::Material::Value(_, _), _)
        | (super::materialize::Material::Stack(_, _), super::materialize::Material::Value(_, _)) => {
            Err(crate::internal(span))
        }
    }
}

/// Extent of a value built from one new constructor over two children.
fn joined(
    a: super::materialize::Extent,
    b: super::materialize::Extent,
    span: crate::Span,
) -> Result<super::materialize::Extent, crate::Diagnostic> {
    Ok(super::materialize::Extent {
        constructors: attempt!(type_size(
            a.constructors
                .saturating_add(b.constructors)
                .saturating_add(1),
            span
        )),
        nodes: attempt!(type_nodes(
            attempt!(type_nodes(a.nodes, b.nodes, span)),
            1,
            span
        )),
    })
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; pushing a materialized value grows a Vec and stack-cap errors construct diagnostics."
)]
fn push(
    stack: super::materialize::Material,
    value: super::materialize::Material,
    span: crate::Span,
) -> Result<super::materialize::Material, crate::Diagnostic> {
    match (stack, value) {
        (
            super::materialize::Material::Stack(mut stack, sa),
            super::materialize::Material::Value(ty, sb),
        ) => {
            if stack.len() >= attempt!(crate::offset(super::STACK_CAP, span)) {
                return Err(crate::Diagnostic::new(
                    crate::DiagnosticKind::Exhausted,
                    span,
                    "stack height limit (256 values) exceeded",
                ));
            }
            // A stack holds at most STACK_CAP values, each within
            // TYPE_NODE_CAP, so its owned node total is bounded without a
            // separate cap; any enclosing program type is checked again.
            let nodes = match sa.nodes.checked_add(sb.nodes) {
                Some(nodes) => nodes,
                None => return Err(node_limit(span)),
            };
            stack.push(ty);
            Ok(super::materialize::Material::Stack(
                stack,
                super::materialize::Extent {
                    constructors: sa.constructors.saturating_add(sb.constructors),
                    nodes,
                },
            ))
        }
        (super::materialize::Material::Value(_, _), _)
        | (super::materialize::Material::Stack(_, _), super::materialize::Material::Stack(_, _)) => {
            Err(crate::internal(span))
        }
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
/// Count, and charge one work unit per node for, the owned `Ty` nodes of a
/// registered nominal descriptor before a materialized occurrence clones it.
/// Nodes are counted as they are scheduled, so the worklist stays within
/// `TYPE_NODE_CAP`.
pub(super) fn reserve_descriptor(
    shape: &noble_kernel::types::NominalShape,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<u32, crate::Diagnostic> {
    let mut walk = Reservation {
        pending: alloc::vec::Vec::with_capacity(2),
        nodes: 0,
    };
    attempt!(walk.descriptor(shape, span, meter));
    while let Some(ty) = walk.pending.pop() {
        match ty {
            noble_kernel::types::Ty::Pair(left, right)
            | noble_kernel::types::Ty::Sum(left, right) => {
                attempt!(walk.enter(left, span, meter));
                attempt!(walk.enter(right, span, meter));
            }
            noble_kernel::types::Ty::List(item) => attempt!(walk.enter(item, span, meter)),
            noble_kernel::types::Ty::Program(input, output, _)
            | noble_kernel::types::Ty::LiveRef(input, output, _) => {
                for item in input.iter().chain(output.iter()) {
                    attempt!(walk.enter(item, span, meter));
                }
            }
            noble_kernel::types::Ty::Nominal(_, nested) => {
                attempt!(walk.descriptor(nested, span, meter));
            }
            noble_kernel::types::Ty::GenericNominal(_, arguments, nested) => {
                attempt!(walk.enter(&arguments[0], span, meter));
                attempt!(walk.enter(&arguments[1], span, meter));
                attempt!(walk.descriptor(nested, span, meter));
            }
            noble_kernel::types::Ty::Unit
            | noble_kernel::types::Ty::Bool
            | noble_kernel::types::Ty::I64
            | noble_kernel::types::Ty::Text
            | noble_kernel::types::Ty::Syntax
            | noble_kernel::types::Ty::Contract
            | noble_kernel::types::Ty::Evidence
            | noble_kernel::types::Ty::Certified
            | noble_kernel::types::Ty::Resource(_) => {}
        }
    }
    Ok(walk.nodes)
}

struct Reservation<'a> {
    pending: alloc::vec::Vec<&'a noble_kernel::types::Ty>,
    nodes: u32,
}

impl<'a> Reservation<'a> {
    fn enter(
        &mut self,
        ty: &'a noble_kernel::types::Ty,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        attempt!(meter.charge(1, span));
        self.nodes = attempt!(type_nodes(self.nodes, 1, span));
        self.pending.push(ty);
        Ok(())
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
    )]
    fn descriptor(
        &mut self,
        shape: &'a noble_kernel::types::NominalShape,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        match shape {
            noble_kernel::types::NominalShape::Opaque(representation) => {
                self.enter(representation, span, meter)
            }
            noble_kernel::types::NominalShape::Variant(left, right) => {
                attempt!(self.enter(left, span, meter));
                self.enter(right, span, meter)
            }
        }
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; constructor-limit failure constructs an owned diagnostic String."
)]
fn type_size(constructors: u32, span: crate::Span) -> Result<u32, crate::Diagnostic> {
    if constructors > crate::syntax::TYPE_CAP {
        Err(crate::Diagnostic::new(
            crate::DiagnosticKind::Exhausted,
            span,
            "inferred type exceeds the 256-constructor limit",
        ))
    } else {
        Ok(constructors)
    }
}

#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-contracts; the same-typed parameters are positional by contract and named at every call site"
)]
/// Checked sum of owned `Ty` node counts, refused above `TYPE_NODE_CAP`.
/// This physical bound is independent of the logical constructor limit.
pub(super) fn type_nodes(
    left: u32,
    right: u32,
    span: crate::Span,
) -> Result<u32, crate::Diagnostic> {
    match left.checked_add(right) {
        Some(nodes) if nodes <= super::TYPE_NODE_CAP => Ok(nodes),
        Some(_) | None => Err(node_limit(span)),
    }
}

fn node_limit(span: crate::Span) -> crate::Diagnostic {
    crate::Diagnostic::new(
        crate::DiagnosticKind::Exhausted,
        span,
        "inferred type representation exceeds the 4096-node limit",
    )
}

#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
)]
#[cfg(test)]
mod tests {
    use super::super::materialize::Material;
    #[expect(
        tigerstyle::non_trait_imports,
        reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
    )]
    use super::super::{Arena, Term};
    #[expect(
        tigerstyle::non_trait_imports,
        reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
    )]
    use noble_kernel::types::{NominalShape, NominalTypeId, Ty};

    const SPAN: crate::Span = crate::Span { start: 0, end: 0 };
    const FAMILY: NominalTypeId = NominalTypeId {
        module: 1,
        ordinal: 0,
    };
    const ORDERS: [[u8; 2]; 2] = [[0, 1], [1, 0]];

    #[expect(
        tigerstyle::assertion_density,
        reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
    )]
    #[expect(
        tigerstyle::no_recursion,
        reason = "Owner: noble-contracts; the walk is bounded by the type-node budget it charges, and the recursive form states the traversal directly"
    )]
    /// Every owned `Ty` node, including retained generic arguments and their
    /// separately boxed descriptor copies.
    fn owned_nodes(ty: &Ty) -> u32 {
        let shape = |shape: &NominalShape| match shape {
            NominalShape::Opaque(inner) => owned_nodes(inner),
            NominalShape::Variant(left, right) => {
                (owned_nodes(left)).saturating_add(owned_nodes(right))
            }
        };
        1u32.saturating_add(match ty {
            Ty::Pair(left, right) | Ty::Sum(left, right) => {
                (owned_nodes(left)).saturating_add(owned_nodes(right))
            }
            Ty::List(item) => owned_nodes(item),
            Ty::Program(input, output, _) | Ty::LiveRef(input, output, _) => {
                input.iter().chain(output.iter()).map(owned_nodes).sum()
            }
            Ty::Nominal(_, descriptor) => shape(descriptor),
            Ty::GenericNominal(_, arguments, descriptor) => (owned_nodes(&arguments[0]))
                .saturating_add(owned_nodes(&arguments[1]))
                .saturating_add(shape(descriptor)),
            _ => 0,
        })
    }

    /// `T0 = I64`, `T(n+1) = G<Tn, I64>` as a small acyclic symbolic arena.
    fn nested(depth: u32, payload_params: [u8; 2], meter: &mut crate::Meter) -> (Arena, u32) {
        let mut arena = Arena::new();
        arena
            .generic_nominals
            .push(noble_kernel::contracts::GenericVariantDecl {
                id: FAMILY,
                payload_params,
                exported: true,
                public: [true, true],
            });
        let leaf = arena.add(Term::I64, SPAN, meter).expect("leaf fits");
        let mut root = leaf;
        for _ in 0..depth {
            root = arena
                .add(Term::GenericNominal(FAMILY, root, leaf), SPAN, meter)
                .expect("symbolic arena fits");
        }
        (arena, root)
    }

    struct Materialized {
        constructors: u32,
        accounted: u32,
        owned: u32,
        charged: u32,
    }

    fn materialize(
        depth: u32,
        payload_params: [u8; 2],
        limits: crate::Limits,
    ) -> Result<Materialized, crate::Diagnostic> {
        let mut meter = crate::Meter::new(limits);
        let (arena, root) = nested(depth, payload_params, &mut meter);
        let before = meter.work;
        let material = arena.materialize(root, SPAN, &mut meter)?;
        let Material::Value(ty, extent) = material else {
            panic!("a generic instance materializes as a value");
        };
        Ok(Materialized {
            constructors: extent.constructors,
            accounted: extent.nodes,
            owned: owned_nodes(&ty),
            charged: (before).saturating_sub(meter.work),
        })
    }

    #[test]
    fn nested_generic_descriptor_copies_are_counted_and_charged_before_cloning() {
        for payload_params in ORDERS {
            for depth in [1, 2, 4, 6, 8, 10] {
                let built = materialize(depth, payload_params, crate::Limits::default())
                    .expect("depth within the representation bound");
                // Logical size grows linearly; owned nodes double per level.
                assert_eq!(
                    built.constructors,
                    2u32.saturating_mul(depth).saturating_add(1)
                );
                assert_eq!(
                    built.owned,
                    4u32.saturating_mul(1u32 << depth).saturating_sub(3)
                );
                assert_eq!(
                    built.accounted, built.owned,
                    "{payload_params:?} depth {depth}"
                );
                assert!(
                    built.charged >= built.owned,
                    "{payload_params:?} depth {depth}: {} owned nodes for {} work",
                    built.owned,
                    built.charged
                );
            }
        }
    }

    #[test]
    fn generic_descriptor_node_cap_refuses_before_cloning() {
        for payload_params in ORDERS {
            let largest = materialize(10, payload_params, crate::Limits::default())
                .expect("4093 owned nodes fit the representation bound");
            assert!(largest.owned <= super::super::TYPE_NODE_CAP);
            let refused = materialize(11, payload_params, crate::Limits::default())
                .err()
                .expect("8189 owned nodes exceed the representation bound");
            assert_eq!(refused.kind, crate::DiagnosticKind::Exhausted);
            assert_eq!(
                refused.message,
                "inferred type representation exceeds the 4096-node limit"
            );
        }
    }

    #[test]
    fn fixed_work_budget_bounds_descriptor_allocation() {
        let limits = crate::Limits {
            bytes: 65_536,
            nodes: 256,
            depth: 64,
            work: 256,
        };
        for payload_params in ORDERS {
            let mut accepted = 0;
            for depth in 0..=12 {
                match materialize(depth, payload_params, limits) {
                    Ok(built) => {
                        assert!(built.owned <= limits.work);
                        accepted = depth;
                    }
                    Err(refused) => {
                        assert_eq!(refused.kind, crate::DiagnosticKind::Exhausted);
                        assert_eq!(refused.message, "frontend work limit exceeded");
                    }
                }
            }
            // Formerly depth 12 (16381 owned nodes) materialized for 62 work.
            assert!(accepted < 6, "{payload_params:?} accepted depth {accepted}");
        }
    }
}
