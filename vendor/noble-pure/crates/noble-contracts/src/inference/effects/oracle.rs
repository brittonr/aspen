#![allow(
    tigerstyle::ambiguous_params,
    tigerstyle::compound_condition,
    tigerstyle::fragile_exhaustive_enum_match,
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-contracts-maintainers; this cfg(test) finite-model oracle is an independent reference implementation whose bounded assignments and exhaustive matches are the scenario under test."
)]
//! Independent finite-model oracle for the frontend effect solver.
//!
//! This module is compiled only for tests. The reference representation and
//! every expected result are independent of production narrowing, symbolic
//! propagation and `EffSet` helpers. Production is driven through its public
//! effect constructors and the complete final bound table is compared with the
//! reference greatest assignment.
//!
//! See `.cairn/changes/effect-oracle/design.md` for the selected semantics.

#![expect(
    tigerstyle::function_length,
    reason = "Owner: noble-maintainers; the test-only oracle keeps each reference and corpus function linear so the finite semantics stay reviewable in one place"
)]
#![expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; test-only reference checks assert each finite invariant directly and fail the lane on any mismatch"
)]
#![expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-maintainers; the test-only module imports the arena, effect, kernel set and diagnostic domain types directly for readability"
)]
use alloc::vec::Vec;

extern crate std;

use crate::inference::{Program, Term};
use crate::DiagnosticKind;
use noble_kernel::types::{EffId, EffSet};

use super::Effect;
use crate::inference::Arena;

const SPAN: crate::Span = crate::Span { start: 0, end: 0 };
const MAX_UNIVERSE_BITS: u32 = 6;
const MAX_REFERENCE_HOLES: usize = 8;
const MAX_REFERENCE_RIGID: u32 = 6;
const MAX_REFERENCE_NODES: usize = 64;
const MAX_REFERENCE_EQUATIONS: usize = 32;
const MAX_REFERENCE_GOALS: usize = 16;
const MAX_ASSIGNMENTS: u64 = 1 << 20;
const MAX_PASSES: u32 = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RefNode {
    Hole(usize),
    Constant(u64),
    Rigid(usize),
    Union(usize, usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct System {
    universe: u64,
    nodes: Vec<RefNode>,
    equations: Vec<(usize, usize)>,
    goals: Vec<(usize, usize)>,
    rigid: u32,
}

impl System {
    fn holes(&self) -> usize {
        let mut count = 0usize;
        let mut at = 0usize;
        while at < self.nodes.len() {
            if matches!(self.nodes[at], RefNode::Hole(_)) {
                count += 1;
            }
            at += 1;
        }
        count
    }

    fn has_rigid_node(&self) -> bool {
        let mut at = 0usize;
        while at < self.nodes.len() {
            if matches!(self.nodes[at], RefNode::Rigid(_)) {
                return true;
            }
            at += 1;
        }
        false
    }

    fn is_well_formed(&self) -> bool {
        if self.universe.count_ones() > MAX_UNIVERSE_BITS {
            return false;
        }
        let mut at = 0usize;
        while at < self.nodes.len() {
            match self.nodes[at] {
                RefNode::Hole(_) => {}
                RefNode::Constant(bits) => {
                    if bits & !self.universe != 0 {
                        return false;
                    }
                }
                RefNode::Rigid(identity) => {
                    if identity >= usize::try_from(self.rigid).unwrap_or(usize::MAX) {
                        return false;
                    }
                }
                RefNode::Union(left, right) => {
                    if left >= at || right >= at {
                        return false;
                    }
                }
            }
            at += 1;
        }
        let mut pair_at = 0usize;
        while pair_at < self.equations.len() {
            if !self.reference_in_range(self.equations[pair_at].0)
                || !self.reference_in_range(self.equations[pair_at].1)
            {
                return false;
            }
            pair_at += 1;
        }
        let mut goal_at = 0usize;
        while goal_at < self.goals.len() {
            if !self.reference_in_range(self.goals[goal_at].0)
                || !self.reference_in_range(self.goals[goal_at].1)
            {
                return false;
            }
            goal_at += 1;
        }
        true
    }

    fn reference_in_range(&self, id: usize) -> bool {
        id < self.nodes.len()
    }

    fn lane(&self) -> Lane {
        if !self.goals.is_empty() {
            return Lane::Rigid;
        }
        if self.has_rigid_node() {
            return Lane::Unsupported;
        }
        Lane::Concrete
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lane {
    Concrete,
    Rigid,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ConcreteReference {
    Satisfiable {
        assignment: Vec<u64>,
        solutions: u64,
    },
    Inconsistent,
    Exhausted,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RigidReference {
    Valid {
        valuations: u64,
        assignments: u64,
    },
    Counterexample {
        valuation: Vec<u64>,
        holes: Vec<u64>,
        body: usize,
        declared: usize,
    },
    Inconsistent,
    Exhausted,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Reference {
    Concrete(ConcreteReference),
    Rigid(RigidReference),
}

fn reference(system: &System) -> Reference {
    if !system.is_well_formed() {
        return Reference::Concrete(ConcreteReference::Unsupported);
    }
    if system.holes() > MAX_REFERENCE_HOLES
        || system.rigid > MAX_REFERENCE_RIGID
        || system.nodes.len() > MAX_REFERENCE_NODES
        || system.equations.len() > MAX_REFERENCE_EQUATIONS
        || system.goals.len() > MAX_REFERENCE_GOALS
    {
        return match system.lane() {
            Lane::Rigid => Reference::Rigid(RigidReference::Unsupported),
            Lane::Concrete | Lane::Unsupported => {
                Reference::Concrete(ConcreteReference::Unsupported)
            }
        };
    }
    match system.lane() {
        Lane::Concrete => Reference::Concrete(reference_concrete(system)),
        Lane::Rigid => Reference::Rigid(reference_rigid(system)),
        Lane::Unsupported => Reference::Concrete(ConcreteReference::Unsupported),
    }
}

fn subsets(universe: u64) -> Vec<u64> {
    let mut positions = Vec::with_capacity(usize::try_from(universe.count_ones()).unwrap_or(0));
    let mut bit = 0u32;
    while bit < 64 {
        if universe & (1u64 << bit) != 0 {
            positions.push(bit);
        }
        bit += 1;
    }
    let count = 1usize << positions.len();
    let mut values = Vec::with_capacity(count);
    let mut mask = 0usize;
    while mask < count {
        let mut value = 0u64;
        let mut at = 0usize;
        while at < positions.len() {
            if mask & (1usize << at) != 0 {
                value |= 1u64 << positions[at];
            }
            at += 1;
        }
        values.push(value);
        mask += 1;
    }
    values
}

fn assignment_count(domain: u64, slots: usize) -> Option<u64> {
    let mut total = 1u64;
    let mut at = 0usize;
    while at < slots {
        total = total.checked_mul(domain)?;
        at += 1;
    }
    Some(total)
}

fn decode(index: u64, domain: u64, slots: usize) -> Vec<usize> {
    let mut choices = Vec::with_capacity(slots);
    let mut remaining = index;
    let mut at = 0usize;
    while at < slots {
        let divisor = domain.max(1);
        choices.push(usize::try_from(remaining.checked_rem(divisor).unwrap_or(0)).unwrap_or(0));
        remaining = remaining.checked_div(divisor).unwrap_or(0);
        at += 1;
    }
    choices
}

fn derive(system: &System, holes: &[u64], rigid: &[u64]) -> Vec<u64> {
    let mut values = Vec::with_capacity(system.nodes.len());
    let mut at = 0usize;
    while at < system.nodes.len() {
        let value = match system.nodes[at] {
            RefNode::Hole(identity) => holes.get(identity).copied().unwrap_or(0),
            RefNode::Constant(bits) => bits,
            RefNode::Rigid(identity) => rigid.get(identity).copied().unwrap_or(0),
            RefNode::Union(left, right) => {
                values.get(left).copied().unwrap_or(0) | values.get(right).copied().unwrap_or(0)
            }
        };
        values.push(value);
        at += 1;
    }
    values
}

fn evaluate(
    system: &System,
    values: &[u64],
    hole_choices: &[usize],
    rigid_choices: &[usize],
) -> Vec<u64> {
    let mut holes = Vec::with_capacity(hole_choices.len());
    let mut at = 0usize;
    while at < hole_choices.len() {
        holes.push(values.get(hole_choices[at]).copied().unwrap_or(0));
        at += 1;
    }
    let mut rigid = Vec::with_capacity(rigid_choices.len());
    let mut at = 0usize;
    while at < rigid_choices.len() {
        rigid.push(values.get(rigid_choices[at]).copied().unwrap_or(0));
        at += 1;
    }
    derive(system, &holes, &rigid)
}

fn equations_hold(system: &System, values: &[u64]) -> bool {
    let mut at = 0usize;
    while at < system.equations.len() {
        let (left, right) = system.equations[at];
        if values.get(left).copied() != values.get(right).copied() {
            return false;
        }
        at += 1;
    }
    true
}

fn reference_concrete(system: &System) -> ConcreteReference {
    let values = subsets(system.universe);
    let domain = u64::try_from(values.len()).unwrap_or(0);
    let holes = system.holes();
    let Some(total) = assignment_count(domain, holes) else {
        return ConcreteReference::Exhausted;
    };
    if total > MAX_ASSIGNMENTS {
        return ConcreteReference::Exhausted;
    }
    let mut greatest = alloc::vec![0u64; holes];
    let mut solutions = 0u64;
    let mut index = 0u64;
    while index < total {
        let choices = decode(index, domain, holes);
        let assignment = evaluate(system, &values, &choices, &[]);
        if equations_hold(system, &assignment) {
            solutions += 1;
            let mut hole_at = 0usize;
            while hole_at < holes {
                greatest[hole_at] |= values.get(choices[hole_at]).copied().unwrap_or(0);
                hole_at += 1;
            }
        }
        index += 1;
    }
    if solutions == 0 {
        return ConcreteReference::Inconsistent;
    }
    ConcreteReference::Satisfiable {
        assignment: derive(system, &greatest, &[]),
        solutions,
    }
}

fn reference_rigid(system: &System) -> RigidReference {
    if system.goals.is_empty() {
        return RigidReference::Unsupported;
    }
    let values = subsets(system.universe);
    let domain = u64::try_from(values.len()).unwrap_or(0);
    let holes = system.holes();
    let rigid = usize::try_from(system.rigid).unwrap_or(usize::MAX);
    let Some(hole_total) = assignment_count(domain, holes) else {
        return RigidReference::Exhausted;
    };
    let Some(valuation_total) = assignment_count(domain, rigid) else {
        return RigidReference::Exhausted;
    };
    let Some(total) = valuation_total.checked_mul(hole_total) else {
        return RigidReference::Exhausted;
    };
    if total > MAX_ASSIGNMENTS {
        return RigidReference::Exhausted;
    }
    let mut valuations = 0u64;
    let mut assignments = 0u64;
    let mut witness: Option<(Vec<u64>, Vec<u64>, usize, usize)> = None;
    let mut valuation_index = 0u64;
    while valuation_index < valuation_total {
        let rigid_choices = decode(valuation_index, domain, rigid);
        let mut has_solution = false;
        let mut hole_index = 0u64;
        while hole_index < hole_total {
            let hole_choices = decode(hole_index, domain, holes);
            let assignment = evaluate(system, &values, &hole_choices, &rigid_choices);
            if equations_hold(system, &assignment) {
                has_solution = true;
                assignments += 1;
                let mut goal_at = 0usize;
                while goal_at < system.goals.len() {
                    let (body, declared) = system.goals[goal_at];
                    if witness.is_none()
                        && assignment.get(body).copied() != assignment.get(declared).copied()
                    {
                        let mut valuation = Vec::with_capacity(rigid_choices.len());
                        let mut at = 0usize;
                        while at < rigid_choices.len() {
                            valuation.push(values.get(rigid_choices[at]).copied().unwrap_or(0));
                            at += 1;
                        }
                        let mut hole_values = Vec::with_capacity(hole_choices.len());
                        let mut at = 0usize;
                        while at < hole_choices.len() {
                            hole_values.push(values.get(hole_choices[at]).copied().unwrap_or(0));
                            at += 1;
                        }
                        witness = Some((valuation, hole_values, body, declared));
                    }
                    goal_at += 1;
                }
            }
            hole_index += 1;
        }
        if !has_solution {
            return RigidReference::Inconsistent;
        }
        valuations += 1;
        valuation_index += 1;
    }
    if let Some((valuation, holes, body, declared)) = witness {
        return RigidReference::Counterexample {
            valuation,
            holes,
            body,
            declared,
        };
    }
    RigidReference::Valid {
        valuations,
        assignments,
    }
}

struct Built {
    arena: Arena,
    ids: Vec<u32>,
}

fn bits_to_ids(bits: u64) -> Vec<EffId> {
    let mut ids = Vec::with_capacity(usize::try_from(bits.count_ones()).unwrap_or(0));
    let mut bit = 0u32;
    while bit < 64 {
        if bits & (1u64 << bit) != 0 {
            ids.push(EffId(bit));
        }
        bit += 1;
    }
    ids
}

fn build(system: &System) -> Option<Built> {
    if !system.is_well_formed() {
        return None;
    }
    let mut arena = Arena::source(system.universe, false);
    let mut meter = crate::Meter::new(crate::Limits::default());
    let mut ids = Vec::with_capacity(system.nodes.len());
    let mut at = 0usize;
    while at < system.nodes.len() {
        let id = match system.nodes[at] {
            RefNode::Hole(_) => arena.effect_hole(SPAN, &mut meter).ok()?,
            RefNode::Constant(bits) => {
                let effects = bits_to_ids(bits);
                let set = EffSet::from_ids(&effects);
                arena.effect_constant(&set, SPAN, &mut meter).ok()?
            }
            RefNode::Rigid(identity) => arena
                .rigid_effect(u32::try_from(identity).ok()?, SPAN, &mut meter)
                .ok()?,
            RefNode::Union(left, right) => {
                let left = *ids.get(left)?;
                let right = *ids.get(right)?;
                arena.effect_union(left, right, SPAN, &mut meter).ok()?
            }
        };
        ids.push(id);
        at += 1;
    }
    let mut pair_at = 0usize;
    while pair_at < system.equations.len() {
        let (left, right) = system.equations[pair_at];
        arena
            .equate_effects(*ids.get(left)?, *ids.get(right)?, SPAN, &mut meter)
            .ok()?;
        pair_at += 1;
    }
    let mut goal_at = 0usize;
    while goal_at < system.goals.len() {
        let (body, declared) = system.goals[goal_at];
        arena
            .require_rigid_effect(*ids.get(body)?, *ids.get(declared)?, SPAN, &mut meter)
            .ok()?;
        goal_at += 1;
    }
    Some(Built { arena, ids })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProductionOutcome {
    Accepted,
    Invalid,
    Unsupported,
    Exhausted,
    Internal,
    BuildFailed,
}

struct ProductionRun {
    outcome: ProductionOutcome,
    bounds: Option<Vec<u64>>,
    ids: Vec<u32>,
}

fn run_production(system: &System, limits: crate::Limits) -> ProductionRun {
    let Some(mut built) = build(system) else {
        return ProductionRun {
            outcome: ProductionOutcome::BuildFailed,
            bounds: None,
            ids: Vec::new(),
        };
    };
    let mut meter = crate::Meter::new(limits);
    let outcome = match built.arena.solve_effects(SPAN, &mut meter) {
        Ok(()) => ProductionOutcome::Accepted,
        Err(diagnostic) => match diagnostic.kind {
            DiagnosticKind::Invalid => ProductionOutcome::Invalid,
            DiagnosticKind::Unsupported => ProductionOutcome::Unsupported,
            DiagnosticKind::Exhausted => ProductionOutcome::Exhausted,
            DiagnosticKind::Internal => ProductionOutcome::Internal,
        },
    };
    let bounds = if matches!(
        outcome,
        ProductionOutcome::Accepted | ProductionOutcome::Invalid
    ) {
        Some(built.arena.effect_bounds.clone())
    } else {
        None
    };
    ProductionRun {
        outcome,
        bounds,
        ids: built.ids,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    AgreeAccept,
    AgreeRefuse,
    ConservativeRefusal,
    WrongAssignment,
    UnsoundAccept,
    ProductionUnsupported,
    ProductionExhausted,
    ProductionInternal,
    OracleUnavailable,
    OracleExhausted,
    BuildFailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Analysis {
    class: Class,
    reference: Reference,
    wrong_node: Option<usize>,
    expected: u64,
    actual: u64,
}

fn compare_assignment(
    system: &System,
    ids: &[u32],
    bounds: &[u64],
    assignment: &[u64],
) -> Analysis {
    let mut at = 0usize;
    while at < system.nodes.len() {
        let Some(id) = ids.get(at).copied() else {
            return Analysis {
                class: Class::OracleUnavailable,
                reference: Reference::Concrete(ConcreteReference::Unsupported),
                wrong_node: None,
                expected: 0,
                actual: 0,
            };
        };
        let actual = bounds
            .get(usize::try_from(id).unwrap_or(usize::MAX))
            .copied();
        let expected = assignment.get(at).copied();
        if actual != expected {
            return Analysis {
                class: Class::WrongAssignment,
                reference: Reference::Concrete(ConcreteReference::Satisfiable {
                    assignment: assignment.to_vec(),
                    solutions: 0,
                }),
                wrong_node: Some(at),
                expected: expected.unwrap_or(0),
                actual: actual.unwrap_or(0),
            };
        }
        at += 1;
    }
    Analysis {
        class: Class::AgreeAccept,
        reference: Reference::Concrete(ConcreteReference::Satisfiable {
            assignment: assignment.to_vec(),
            solutions: 0,
        }),
        wrong_node: None,
        expected: 0,
        actual: 0,
    }
}

fn classify(
    system: &System,
    outcome: ProductionOutcome,
    bounds: Option<&[u64]>,
    ids: &[u32],
    reference: Reference,
) -> Analysis {
    if matches!(outcome, ProductionOutcome::BuildFailed) {
        return Analysis {
            class: Class::BuildFailed,
            reference,
            wrong_node: None,
            expected: 0,
            actual: 0,
        };
    }
    if matches!(outcome, ProductionOutcome::Internal) {
        return Analysis {
            class: Class::ProductionInternal,
            reference,
            wrong_node: None,
            expected: 0,
            actual: 0,
        };
    }
    if matches!(outcome, ProductionOutcome::Exhausted) {
        return Analysis {
            class: Class::ProductionExhausted,
            reference,
            wrong_node: None,
            expected: 0,
            actual: 0,
        };
    }
    if matches!(outcome, ProductionOutcome::Unsupported) {
        return Analysis {
            class: Class::ProductionUnsupported,
            reference,
            wrong_node: None,
            expected: 0,
            actual: 0,
        };
    }
    match &reference {
        Reference::Concrete(ConcreteReference::Exhausted)
        | Reference::Rigid(RigidReference::Exhausted) => {
            return Analysis {
                class: Class::OracleExhausted,
                reference,
                wrong_node: None,
                expected: 0,
                actual: 0,
            }
        }
        Reference::Concrete(ConcreteReference::Unsupported)
        | Reference::Rigid(RigidReference::Unsupported) => {
            return Analysis {
                class: Class::OracleUnavailable,
                reference,
                wrong_node: None,
                expected: 0,
                actual: 0,
            }
        }
        _ => {}
    }
    let is_accepted = matches!(outcome, ProductionOutcome::Accepted);
    match reference {
        Reference::Concrete(ConcreteReference::Satisfiable { assignment, .. }) => {
            if is_accepted {
                let Some(bounds) = bounds else {
                    return Analysis {
                        class: Class::OracleUnavailable,
                        reference: Reference::Concrete(ConcreteReference::Unsupported),
                        wrong_node: None,
                        expected: 0,
                        actual: 0,
                    };
                };
                let mut result = compare_assignment(system, ids, bounds, &assignment);
                result.reference = Reference::Concrete(ConcreteReference::Satisfiable {
                    assignment,
                    solutions: 0,
                });
                result
            } else {
                Analysis {
                    class: Class::ConservativeRefusal,
                    reference: Reference::Concrete(ConcreteReference::Satisfiable {
                        assignment,
                        solutions: 0,
                    }),
                    wrong_node: None,
                    expected: 0,
                    actual: 0,
                }
            }
        }
        Reference::Concrete(ConcreteReference::Inconsistent) => Analysis {
            class: if is_accepted {
                Class::UnsoundAccept
            } else {
                Class::AgreeRefuse
            },
            reference: Reference::Concrete(ConcreteReference::Inconsistent),
            wrong_node: None,
            expected: 0,
            actual: 0,
        },
        Reference::Rigid(RigidReference::Valid { .. }) => Analysis {
            class: if is_accepted {
                Class::AgreeAccept
            } else {
                Class::ConservativeRefusal
            },
            reference,
            wrong_node: None,
            expected: 0,
            actual: 0,
        },
        Reference::Rigid(RigidReference::Counterexample { .. })
        | Reference::Rigid(RigidReference::Inconsistent) => Analysis {
            class: if is_accepted {
                Class::UnsoundAccept
            } else {
                Class::AgreeRefuse
            },
            reference,
            wrong_node: None,
            expected: 0,
            actual: 0,
        },
        Reference::Concrete(ConcreteReference::Exhausted)
        | Reference::Concrete(ConcreteReference::Unsupported)
        | Reference::Rigid(RigidReference::Exhausted)
        | Reference::Rigid(RigidReference::Unsupported) => Analysis {
            class: Class::OracleUnavailable,
            reference,
            wrong_node: None,
            expected: 0,
            actual: 0,
        },
    }
}

fn analyze(system: &System) -> Analysis {
    analyze_with_limits(system, crate::Limits::default())
}

fn analyze_with_limits(system: &System, limits: crate::Limits) -> Analysis {
    let reference = reference(system);
    let production = run_production(system, limits);
    classify(
        system,
        production.outcome,
        production.bounds.as_deref(),
        &production.ids,
        reference,
    )
}

fn system(
    universe: u64,
    nodes: Vec<RefNode>,
    equations: Vec<(usize, usize)>,
    goals: Vec<(usize, usize)>,
    rigid: u32,
) -> System {
    System {
        universe,
        nodes,
        equations,
        goals,
        rigid,
    }
}

fn corpus() -> Vec<(&'static str, System, Class)> {
    alloc::vec![
        (
            "empty-universe-hole",
            system(0, alloc::vec![RefNode::Hole(0)], Vec::new(), Vec::new(), 0),
            Class::AgreeAccept,
        ),
        (
            "hole-host-equation",
            system(
                1,
                alloc::vec![RefNode::Hole(0), RefNode::Constant(1)],
                alloc::vec![(0, 1)],
                Vec::new(),
                0
            ),
            Class::AgreeAccept,
        ),
        (
            "contradictory-constants",
            system(
                1,
                alloc::vec![RefNode::Constant(1), RefNode::Constant(0)],
                alloc::vec![(0, 1)],
                Vec::new(),
                0
            ),
            Class::AgreeRefuse,
        ),
        (
            "nested-union",
            system(
                1,
                alloc::vec![
                    RefNode::Hole(0),
                    RefNode::Constant(0),
                    RefNode::Union(0, 1),
                    RefNode::Union(2, 0)
                ],
                Vec::new(),
                Vec::new(),
                0
            ),
            Class::AgreeAccept,
        ),
        (
            "neutral-equation",
            system(
                1,
                alloc::vec![RefNode::Hole(0), RefNode::Constant(0), RefNode::Union(0, 1)],
                alloc::vec![(0, 2)],
                Vec::new(),
                0
            ),
            Class::AgreeAccept,
        ),
        (
            "two-non-neutral",
            system(
                2,
                alloc::vec![
                    RefNode::Hole(0),
                    RefNode::Hole(1),
                    RefNode::Union(0, 1),
                    RefNode::Union(1, 0)
                ],
                alloc::vec![(2, 3)],
                Vec::new(),
                0
            ),
            Class::AgreeAccept,
        ),
        (
            "shared-hole-two-equations",
            system(
                3,
                alloc::vec![RefNode::Hole(0), RefNode::Constant(1), RefNode::Constant(2)],
                alloc::vec![(0, 1), (0, 2)],
                Vec::new(),
                0
            ),
            Class::AgreeRefuse,
        ),
        (
            "rigid-identity",
            system(
                1,
                alloc::vec![RefNode::Rigid(0)],
                Vec::new(),
                alloc::vec![(0, 0)],
                1
            ),
            Class::AgreeAccept,
        ),
        (
            "rigid-distinct",
            system(
                1,
                alloc::vec![RefNode::Rigid(0), RefNode::Rigid(1)],
                Vec::new(),
                alloc::vec![(0, 1)],
                2
            ),
            Class::AgreeRefuse,
        ),
        (
            "rigid-constant",
            system(
                1,
                alloc::vec![RefNode::Rigid(0), RefNode::Constant(1)],
                Vec::new(),
                alloc::vec![(0, 1)],
                1
            ),
            Class::AgreeRefuse,
        ),
        (
            "rigid-inconsistent-equations",
            system(
                1,
                alloc::vec![RefNode::Constant(1), RefNode::Constant(0)],
                alloc::vec![(0, 1)],
                alloc::vec![(0, 0)],
                0
            ),
            Class::AgreeRefuse,
        ),
        (
            "unconstrained-hole-goal",
            system(
                1,
                alloc::vec![RefNode::Hole(0), RefNode::Constant(0)],
                Vec::new(),
                alloc::vec![(0, 1)],
                0
            ),
            Class::AgreeRefuse,
        ),
        (
            "conservative-valid-goal",
            system(
                1,
                alloc::vec![RefNode::Hole(0), RefNode::Constant(0), RefNode::Union(0, 1)],
                Vec::new(),
                alloc::vec![(2, 0)],
                0
            ),
            Class::ConservativeRefusal,
        ),
        (
            "rigid-goal-through-equation",
            system(
                1,
                alloc::vec![RefNode::Hole(0), RefNode::Constant(0)],
                alloc::vec![(0, 1)],
                alloc::vec![(0, 1)],
                0
            ),
            Class::AgreeAccept,
        ),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mutant {
    ConstantNotExact,
    UnionChildNotNarrowed,
    EquationLeftOnly,
    RigidAsHole,
    SinglePass,
    HoleZero,
}

struct MutantRun {
    bounds: Option<Vec<u64>>,
    ids: Vec<u32>,
    built: bool,
}

fn bound(arena: &Arena, id: u32) -> u64 {
    arena
        .effect_bounds
        .get(usize::try_from(id).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or(0)
}

fn narrow(arena: &mut Arena, id: u32, mask: u64) -> bool {
    let Some(slot) = arena
        .effect_bounds
        .get_mut(usize::try_from(id).unwrap_or(usize::MAX))
    else {
        return false;
    };
    let next = *slot & mask;
    let is_changed = next != *slot;
    *slot = next;
    is_changed
}

fn mutant_pass(arena: &mut Arena, kind: Mutant) -> bool {
    let mut is_changed = false;
    let mut at = 0usize;
    while at < arena.effects.len() {
        let effect = arena.effects[at];
        if let Effect::Union(left, right) = effect {
            let upper = bound(arena, left) | bound(arena, right);
            if let Ok(id) = u32::try_from(at) {
                is_changed |= narrow(arena, id, upper);
            }
            if kind != Mutant::UnionChildNotNarrowed {
                is_changed |= narrow(arena, left, upper);
                is_changed |= narrow(arena, right, upper);
            }
        }
        at += 1;
    }
    let mut equation = 0usize;
    while equation < arena.effect_equations.len() {
        let (left, right) = arena.effect_equations[equation];
        let upper = bound(arena, left) & bound(arena, right);
        is_changed |= narrow(arena, left, upper);
        if kind != Mutant::EquationLeftOnly {
            is_changed |= narrow(arena, right, upper);
        }
        equation += 1;
    }
    is_changed
}

#[expect(
    tigerstyle::nested_conditionals,
    reason = "Owner: noble-maintainers; the test-only hole-zero mutation walks a bounded node list and updates only matching hole slots, keeping the faulty variant explicit"
)]
fn mutant_run(system: &System, kind: Mutant) -> MutantRun {
    let Some(mut built) = build(system) else {
        return MutantRun {
            bounds: None,
            ids: Vec::new(),
            built: false,
        };
    };
    if kind == Mutant::HoleZero {
        let mut at = 0usize;
        while at < system.nodes.len() {
            if matches!(system.nodes[at], RefNode::Hole(_)) {
                if let Some(&id) = built.ids.get(at) {
                    if let Some(slot) = built
                        .arena
                        .effect_bounds
                        .get_mut(usize::try_from(id).unwrap_or(usize::MAX))
                    {
                        *slot = 0;
                    }
                }
            }
            at += 1;
        }
    }
    let mut passes = 0u32;
    loop {
        let is_changed = mutant_pass(&mut built.arena, kind);
        passes += 1;
        if !is_changed || kind == Mutant::SinglePass || passes >= MAX_PASSES {
            break;
        }
    }
    let mut is_rejected = false;
    if kind != Mutant::ConstantNotExact {
        let mut at = 0usize;
        while at < built.arena.effects.len() {
            if let Effect::Constant(bits) = built.arena.effects[at] {
                if let Ok(id) = u32::try_from(at) {
                    if bound(&built.arena, id) != bits {
                        is_rejected = true;
                        break;
                    }
                }
            }
            at += 1;
        }
    }
    MutantRun {
        bounds: if is_rejected {
            None
        } else {
            Some(built.arena.effect_bounds.clone())
        },
        ids: built.ids,
        built: true,
    }
}

fn mutant_analyze(system: &System, kind: Mutant) -> Analysis {
    let reference = reference(system);
    let run = mutant_run(system, kind);
    if !run.built {
        return Analysis {
            class: Class::BuildFailed,
            reference,
            wrong_node: None,
            expected: 0,
            actual: 0,
        };
    }
    let outcome = if run.bounds.is_some() {
        ProductionOutcome::Accepted
    } else {
        ProductionOutcome::Invalid
    };
    classify(system, outcome, run.bounds.as_deref(), &run.ids, reference)
}

fn class_of(system: &System, mutant: Option<Mutant>) -> Class {
    match mutant {
        Some(kind) => mutant_analyze(system, kind).class,
        None => analyze(system).class,
    }
}

fn is_referenced(system: &System, index: usize) -> bool {
    let mut at = 0usize;
    while at < system.nodes.len() {
        if let RefNode::Union(left, right) = system.nodes[at] {
            if left == index || right == index {
                return true;
            }
        }
        at += 1;
    }
    let mut pair = 0usize;
    while pair < system.equations.len() {
        if system.equations[pair].0 == index || system.equations[pair].1 == index {
            return true;
        }
        pair += 1;
    }
    let mut goal = 0usize;
    while goal < system.goals.len() {
        if system.goals[goal].0 == index || system.goals[goal].1 == index {
            return true;
        }
        goal += 1;
    }
    false
}

fn remove_node(system: &System, index: usize) -> Option<System> {
    if is_referenced(system, index) {
        return None;
    }
    let mut candidate = system.clone();
    candidate.nodes.remove(index);
    let mut at = 0usize;
    while at < candidate.nodes.len() {
        if let RefNode::Union(left, right) = candidate.nodes[at] {
            let left = if left > index { left - 1 } else { left };
            let right = if right > index { right - 1 } else { right };
            candidate.nodes[at] = RefNode::Union(left, right);
        }
        at += 1;
    }
    let mut pair = 0usize;
    while pair < candidate.equations.len() {
        let left = if candidate.equations[pair].0 > index {
            candidate.equations[pair].0 - 1
        } else {
            candidate.equations[pair].0
        };
        let right = if candidate.equations[pair].1 > index {
            candidate.equations[pair].1 - 1
        } else {
            candidate.equations[pair].1
        };
        candidate.equations[pair] = (left, right);
        pair += 1;
    }
    let mut goal = 0usize;
    while goal < candidate.goals.len() {
        let left = if candidate.goals[goal].0 > index {
            candidate.goals[goal].0 - 1
        } else {
            candidate.goals[goal].0
        };
        let right = if candidate.goals[goal].1 > index {
            candidate.goals[goal].1 - 1
        } else {
            candidate.goals[goal].1
        };
        candidate.goals[goal] = (left, right);
        goal += 1;
    }
    Some(candidate)
}

fn shrink(system: &System, mutant: Option<Mutant>, class: Class) -> System {
    let mut current = system.clone();
    let mut attempts = 0u32;
    let mut steps = 0u32;
    loop {
        if attempts >= 256 || steps >= 64 {
            break;
        }
        let mut is_improved = false;
        let mut pair = current.equations.len();
        while pair > 0 {
            pair -= 1;
            if attempts >= 256 {
                break;
            }
            attempts += 1;
            let mut candidate = current.clone();
            candidate.equations.remove(pair);
            if candidate.is_well_formed() && class_of(&candidate, mutant) == class {
                current = candidate;
                steps += 1;
                is_improved = true;
                break;
            }
        }
        if is_improved {
            continue;
        }
        let mut goal = current.goals.len();
        while goal > 0 {
            goal -= 1;
            if attempts >= 256 {
                break;
            }
            attempts += 1;
            let mut candidate = current.clone();
            candidate.goals.remove(goal);
            if candidate.is_well_formed() && class_of(&candidate, mutant) == class {
                current = candidate;
                steps += 1;
                is_improved = true;
                break;
            }
        }
        if is_improved {
            continue;
        }
        let mut node = current.nodes.len();
        while node > 0 {
            node -= 1;
            if attempts >= 256 {
                break;
            }
            attempts += 1;
            if let Some(candidate) = remove_node(&current, node) {
                if candidate.is_well_formed() && class_of(&candidate, mutant) == class {
                    current = candidate;
                    steps += 1;
                    is_improved = true;
                    break;
                }
            }
        }
        if !is_improved {
            break;
        }
    }
    current
}

struct Rng {
    state: u64,
}

impl Rng {
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            0
        } else {
            self.next() % bound
        }
    }
}

fn seeded_system(seed: u64) -> System {
    let mut rng = Rng { state: seed };
    let bits = 1u32 + u32::try_from(rng.below(3)).unwrap_or(0);
    let universe = (1u64 << bits) - 1;
    let holes = 1usize + usize::try_from(rng.below(3)).unwrap_or(0);
    let rigid = u32::try_from(rng.below(3)).unwrap_or(0);
    let node_count = holes
        + usize::try_from(rigid).unwrap_or(0)
        + 1
        + usize::try_from(rng.below(6)).unwrap_or(0);
    let mut nodes = Vec::with_capacity(node_count);
    let mut hole = 0usize;
    while hole < holes {
        nodes.push(RefNode::Hole(hole));
        hole += 1;
    }
    let mut identity = 0usize;
    while identity < usize::try_from(rigid).unwrap_or(0) {
        nodes.push(RefNode::Rigid(identity));
        identity += 1;
    }
    while nodes.len() < node_count {
        if rng.below(2) == 0 {
            nodes.push(RefNode::Constant(rng.next() & universe));
        } else {
            let at = nodes.len();
            let left = usize::try_from(rng.below(u64::try_from(at).unwrap_or(1))).unwrap_or(0);
            let right = usize::try_from(rng.below(u64::try_from(at).unwrap_or(1))).unwrap_or(0);
            nodes.push(RefNode::Union(left, right));
        }
    }
    let equation_count = usize::try_from(rng.below(5)).unwrap_or(0);
    let mut equations = Vec::with_capacity(equation_count);
    let mut at = 0usize;
    while at < equation_count {
        let len = u64::try_from(nodes.len()).unwrap_or(1);
        equations.push((
            usize::try_from(rng.below(len)).unwrap_or(0),
            usize::try_from(rng.below(len)).unwrap_or(0),
        ));
        at += 1;
    }
    let goal_count = if rng.below(2) == 0 { 1 } else { 0 };
    let mut goals = Vec::with_capacity(goal_count);
    let mut at = 0usize;
    while at < goal_count {
        let len = u64::try_from(nodes.len()).unwrap_or(1);
        goals.push((
            usize::try_from(rng.below(len)).unwrap_or(0),
            usize::try_from(rng.below(len)).unwrap_or(0),
        ));
        at += 1;
    }
    if rigid > 0 && goals.is_empty() {
        goals.push((0, 0));
    }
    System {
        universe,
        nodes,
        equations,
        goals,
        rigid,
    }
}

fn node_choices(universe: u64, at: usize) -> Vec<RefNode> {
    let mut choices = Vec::with_capacity(2 + usize::from(universe != 0) + at.saturating_mul(at));
    choices.push(RefNode::Hole(0));
    choices.push(RefNode::Constant(0));
    if universe != 0 {
        choices.push(RefNode::Constant(universe));
    }
    let mut left = 0usize;
    while left < at {
        let mut right = 0usize;
        while right < at {
            choices.push(RefNode::Union(left, right));
            right += 1;
        }
        left += 1;
    }
    choices
}

fn normalize_holes(mut nodes: Vec<RefNode>) -> Vec<RefNode> {
    let mut next = 0usize;
    let mut at = 0usize;
    while at < nodes.len() {
        if let RefNode::Hole(_) = nodes[at] {
            nodes[at] = RefNode::Hole(next);
            next += 1;
        }
        at += 1;
    }
    nodes
}

fn push_equation_variants(cases: &mut Vec<System>, universe: u64, nodes: Vec<RefNode>) {
    let nodes = normalize_holes(nodes);
    cases.push(system(universe, nodes.clone(), Vec::new(), Vec::new(), 0));
    let mut left = 0usize;
    while left < nodes.len() {
        let mut right = 0usize;
        while right < nodes.len() {
            cases.push(system(
                universe,
                nodes.clone(),
                alloc::vec![(left, right)],
                Vec::new(),
                0,
            ));
            right += 1;
        }
        left += 1;
    }
}

fn exhaustive_cases() -> Vec<System> {
    let mut cases = Vec::new();
    let universes = [0u64, 1u64];
    let mut universe_at = 0usize;
    while universe_at < universes.len() {
        let universe = universes[universe_at];
        let firsts = node_choices(universe, 0);
        let mut first_at = 0usize;
        while first_at < firsts.len() {
            let first = firsts[first_at];
            push_equation_variants(&mut cases, universe, alloc::vec![first]);
            let seconds = node_choices(universe, 1);
            let mut second_at = 0usize;
            while second_at < seconds.len() {
                let second = seconds[second_at];
                push_equation_variants(&mut cases, universe, alloc::vec![first, second]);
                let thirds = node_choices(universe, 2);
                let mut third_at = 0usize;
                while third_at < thirds.len() {
                    push_equation_variants(
                        &mut cases,
                        universe,
                        alloc::vec![first, second, thirds[third_at]],
                    );
                    third_at += 1;
                }
                second_at += 1;
            }
            first_at += 1;
        }
        universe_at += 1;
    }
    cases
}

struct SuiteCounts {
    cases: usize,
    exhaustive: usize,
    agree_accept: usize,
    agree_refuse: usize,
    conservative: usize,
    wrong: usize,
    unsound: usize,
    mutants: usize,
    seeds: usize,
    low_work: usize,
    conservative_names: Vec<&'static str>,
}

fn count(counts: &mut SuiteCounts, class: Class) {
    match class {
        Class::AgreeAccept => counts.agree_accept += 1,
        Class::AgreeRefuse => counts.agree_refuse += 1,
        Class::ConservativeRefusal => counts.conservative += 1,
        Class::WrongAssignment => counts.wrong += 1,
        Class::UnsoundAccept => counts.unsound += 1,
        Class::ProductionUnsupported
        | Class::ProductionExhausted
        | Class::ProductionInternal
        | Class::OracleUnavailable
        | Class::OracleExhausted
        | Class::BuildFailed => {}
    }
}

fn run_suite() -> SuiteCounts {
    let mut counts = SuiteCounts {
        cases: 0,
        exhaustive: 0,
        agree_accept: 0,
        agree_refuse: 0,
        conservative: 0,
        wrong: 0,
        unsound: 0,
        mutants: 0,
        seeds: 0,
        low_work: 0,
        conservative_names: Vec::new(),
    };
    let cases = corpus();
    let mut at = 0usize;
    while at < cases.len() {
        let (name, system, expected) = &cases[at];
        let analysis = analyze(system);
        assert_eq!(
            analysis.class, *expected,
            "corpus case {name} produced {:?} with reference {:?}",
            analysis.class, analysis.reference
        );
        counts.cases += 1;
        count(&mut counts, analysis.class);
        if analysis.class == Class::ConservativeRefusal {
            counts.conservative_names.push(name);
        }
        at += 1;
    }
    let exhaustive = exhaustive_cases();
    let mut case = 0usize;
    while case < exhaustive.len() {
        let analysis = analyze(&exhaustive[case]);
        assert!(
            matches!(analysis.class, Class::AgreeAccept | Class::AgreeRefuse),
            "exhaustive case {case} produced {:?} with reference {:?}",
            analysis.class,
            analysis.reference
        );
        counts.cases += 1;
        counts.exhaustive += 1;
        count(&mut counts, analysis.class);
        case += 1;
    }
    let low_work_system = system(
        1,
        alloc::vec![RefNode::Hole(0), RefNode::Constant(1)],
        alloc::vec![(0, 1)],
        Vec::new(),
        0,
    );
    let low_limits = crate::Limits {
        work: 1,
        ..crate::Limits::default()
    };
    let low = analyze_with_limits(&low_work_system, low_limits);
    assert_eq!(
        low.class,
        Class::ProductionExhausted,
        "a one-work run must be exhaustion, not a semantic verdict"
    );
    counts.low_work += 1;
    let mutants = alloc::vec![
        (
            Mutant::ConstantNotExact,
            system(
                1,
                alloc::vec![RefNode::Constant(1), RefNode::Constant(0)],
                alloc::vec![(0, 1)],
                Vec::new(),
                0
            ),
            Class::UnsoundAccept
        ),
        (
            Mutant::UnionChildNotNarrowed,
            system(
                3,
                alloc::vec![
                    RefNode::Hole(0),
                    RefNode::Constant(1),
                    RefNode::Constant(2),
                    RefNode::Union(0, 1)
                ],
                alloc::vec![(3, 1)],
                Vec::new(),
                0
            ),
            Class::WrongAssignment
        ),
        (
            Mutant::EquationLeftOnly,
            system(
                3,
                alloc::vec![RefNode::Constant(1), RefNode::Hole(0)],
                alloc::vec![(0, 1)],
                Vec::new(),
                0
            ),
            Class::WrongAssignment
        ),
        (
            Mutant::RigidAsHole,
            system(
                1,
                alloc::vec![RefNode::Rigid(0), RefNode::Rigid(1)],
                Vec::new(),
                alloc::vec![(0, 1)],
                2
            ),
            Class::UnsoundAccept
        ),
        (
            Mutant::SinglePass,
            system(
                1,
                alloc::vec![RefNode::Hole(0), RefNode::Constant(0), RefNode::Union(0, 1)],
                alloc::vec![(2, 1)],
                Vec::new(),
                0
            ),
            Class::WrongAssignment
        ),
        (
            Mutant::HoleZero,
            system(1, alloc::vec![RefNode::Hole(0)], Vec::new(), Vec::new(), 0),
            Class::WrongAssignment
        ),
    ];
    let mut at = 0usize;
    while at < mutants.len() {
        let (kind, case, expected) = &mutants[at];
        let analysis = mutant_analyze(case, *kind);
        assert_eq!(
            analysis.class, *expected,
            "mutant {kind:?} was not caught with the expected class"
        );
        counts.mutants += 1;
        at += 1;
    }
    let seeds = [
        1u64,
        0x9e37_79b9_7f4a_7c15,
        0xdead_beef_cafe_babe,
        0x1234_5678_9abc_def0,
    ];
    let mut seed_at = 0usize;
    while seed_at < seeds.len() {
        let mut draw = 0u64;
        while draw < 3 {
            let seed = seeds[seed_at] ^ draw.wrapping_mul(0x9e37_79b9_7f4a_7c15);
            let generated = seeded_system(seed);
            assert_eq!(generated, seeded_system(seed), "seed replay diverged");
            let analysis = analyze(&generated);
            assert!(
                !matches!(
                    analysis.class,
                    Class::ProductionInternal | Class::BuildFailed
                ),
                "seeded case {seed} produced {:?}",
                analysis.class
            );
            counts.seeds += 1;
            draw += 1;
        }
        seed_at += 1;
    }
    std::println!(
        "NOBLE_EFFECT_ORACLE_RECEIPT {{\"oracle\":\"finite-effects/v1\",\"cases\":{},\"exhaustive_cases\":{},\"agree_accept\":{},\"agree_refuse\":{},\"conservative_refusal\":{},\"wrong_assignment\":{},\"unsound_accept\":{},\"mutants_caught\":{},\"seeds\":{},\"low_work_exhausted\":{},\"universe_bits_max\":3,\"holes_max\":3,\"nodes_max\":12,\"conservative\":{:?}}}",
        counts.cases,
        counts.exhaustive,
        counts.agree_accept,
        counts.agree_refuse,
        counts.conservative,
        counts.wrong,
        counts.unsound,
        counts.mutants,
        counts.seeds,
        counts.low_work,
        counts.conservative_names
    );
    counts
}

#[test]
fn deterministic_corpus_matches_reference() {
    let counts = run_suite();
    assert!(counts.cases >= 14);
    assert!(counts.exhaustive >= 100);
    assert_eq!(counts.wrong, 0, "unexpected wrong assignments");
    assert_eq!(counts.unsound, 0, "unexpected unsound accepts");
    assert_eq!(counts.conservative, 1);
    assert_eq!(counts.mutants, 6);
    assert_eq!(counts.low_work, 1);
}

fn accepted_assignment(system: &System) -> Vec<u64> {
    let analysis = analyze(system);
    assert_eq!(analysis.class, Class::AgreeAccept);
    match analysis.reference {
        Reference::Concrete(ConcreteReference::Satisfiable { assignment, .. }) => assignment,
        _ => panic!("expected a concrete satisfying reference"),
    }
}

fn compared_assignment(system: &System) -> Vec<u64> {
    match reference(system) {
        Reference::Concrete(ConcreteReference::Satisfiable { assignment, .. }) => assignment,
        other => panic!("expected a concrete satisfying reference, got {other:?}"),
    }
}

fn swap_unions(system: &System) -> System {
    let mut changed = system.clone();
    let mut at = 0usize;
    while at < changed.nodes.len() {
        if let RefNode::Union(left, right) = changed.nodes[at] {
            changed.nodes[at] = RefNode::Union(right, left);
        }
        at += 1;
    }
    changed
}

fn reverse_equations(system: &System) -> System {
    let mut changed = system.clone();
    changed.equations.reverse();
    changed
}

fn duplicate_equation(system: &System) -> System {
    let mut changed = system.clone();
    if let Some(first) = changed.equations.first().copied() {
        changed.equations.push(first);
    }
    changed
}

#[expect(
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-maintainers; the test-only renaming helper grows fixed vectors to system.nodes.len() and never appends in an unbounded loop"
)]
fn swap_leaves(system: &System) -> (System, Vec<usize>) {
    let mut changed = system.clone();
    let mut perm: Vec<usize> = (0..system.nodes.len()).collect();
    let mut holes = Vec::new();
    let mut constants = Vec::new();
    let mut at = 0usize;
    while at < system.nodes.len() {
        match system.nodes[at] {
            RefNode::Hole(_) => holes.push(at),
            RefNode::Constant(_) => constants.push(at),
            _ => {}
        }
        at += 1;
    }
    if holes.len() >= 2 {
        let left = holes[0];
        let right = holes[1];
        changed.nodes.swap(left, right);
        perm.swap(left, right);
    }
    if constants.len() >= 2 {
        let left = constants[0];
        let right = constants[1];
        changed.nodes.swap(left, right);
        perm.swap(left, right);
    }
    (changed, perm)
}

#[test]
fn metamorphic_checks_preserve_acceptance_and_assignment() {
    let nested = system(
        1,
        alloc::vec![
            RefNode::Hole(0),
            RefNode::Constant(0),
            RefNode::Union(0, 1),
            RefNode::Union(2, 0)
        ],
        Vec::new(),
        Vec::new(),
        0,
    );
    let neutral = system(
        1,
        alloc::vec![RefNode::Hole(0), RefNode::Constant(0), RefNode::Union(0, 1)],
        alloc::vec![(0, 2)],
        Vec::new(),
        0,
    );
    let two_holes = system(
        2,
        alloc::vec![
            RefNode::Hole(0),
            RefNode::Hole(1),
            RefNode::Union(0, 1),
            RefNode::Union(1, 0)
        ],
        alloc::vec![(2, 3)],
        Vec::new(),
        0,
    );
    let multi = system(
        3,
        alloc::vec![
            RefNode::Hole(0),
            RefNode::Hole(1),
            RefNode::Constant(1),
            RefNode::Constant(2)
        ],
        alloc::vec![(0, 2), (1, 3)],
        Vec::new(),
        0,
    );
    assert_eq!(
        accepted_assignment(&nested),
        compared_assignment(&swap_unions(&nested))
    );
    assert_eq!(
        accepted_assignment(&neutral),
        compared_assignment(&swap_unions(&neutral))
    );
    assert_eq!(
        accepted_assignment(&two_holes),
        compared_assignment(&reverse_equations(&two_holes))
    );
    assert_eq!(
        accepted_assignment(&two_holes),
        compared_assignment(&duplicate_equation(&two_holes))
    );
    assert_eq!(
        accepted_assignment(&multi),
        compared_assignment(&reverse_equations(&multi))
    );
    assert_eq!(
        accepted_assignment(&multi),
        compared_assignment(&duplicate_equation(&multi))
    );
    let (renamed, perm) = swap_leaves(&two_holes);
    let base = accepted_assignment(&two_holes);
    let changed = compared_assignment(&renamed);
    let mut at = 0usize;
    while at < base.len() {
        assert_eq!(base[at], changed[perm[at]]);
        at += 1;
    }
    assert_eq!(analyze(&renamed).class, Class::AgreeAccept);
    let (renamed_multi, perm_multi) = swap_leaves(&multi);
    let base_multi = accepted_assignment(&multi);
    let changed_multi = compared_assignment(&renamed_multi);
    let mut at = 0usize;
    while at < base_multi.len() {
        assert_eq!(base_multi[at], changed_multi[perm_multi[at]]);
        at += 1;
    }
}

#[test]
fn shrinking_preserves_the_mutant_outcome_class() {
    let case = system(
        3,
        alloc::vec![
            RefNode::Hole(0),
            RefNode::Constant(1),
            RefNode::Constant(2),
            RefNode::Union(0, 1)
        ],
        alloc::vec![(3, 1)],
        Vec::new(),
        0,
    );
    let class = mutant_analyze(&case, Mutant::UnionChildNotNarrowed).class;
    assert_eq!(class, Class::WrongAssignment);
    let minimized = shrink(&case, Some(Mutant::UnionChildNotNarrowed), class);
    assert_eq!(
        mutant_analyze(&minimized, Mutant::UnionChildNotNarrowed).class,
        class
    );
    assert!(minimized.nodes.len() < case.nodes.len());
}

#[test]
fn program_effect_value_matches_the_reference_assignment() {
    let case = system(
        1,
        alloc::vec![RefNode::Hole(0), RefNode::Constant(1), RefNode::Union(0, 1)],
        Vec::new(),
        Vec::new(),
        0,
    );
    let Reference::Concrete(ConcreteReference::Satisfiable { assignment, .. }) = reference(&case)
    else {
        panic!("expected a concrete satisfying reference");
    };
    let mut built = build(&case).expect("valid arena");
    let mut meter = crate::Meter::new(crate::Limits::default());
    let empty = built
        .arena
        .add(Term::Empty, SPAN, &mut meter)
        .expect("empty stack term");
    let _program = built
        .arena
        .program(
            Program {
                input: empty,
                output: empty,
                effect: built.ids[2],
            },
            SPAN,
            &mut meter,
        )
        .expect("program term");
    let mut solve_meter = crate::Meter::new(crate::Limits::default());
    built
        .arena
        .solve_effects(SPAN, &mut solve_meter)
        .expect("accepted system");
    let effect = built
        .arena
        .effect_value(built.ids[2], SPAN)
        .expect("effect value");
    let mut bits = 0u64;
    let mut at = 0usize;
    while at < effect.as_slice().len() {
        bits |= 1u64 << effect.as_slice()[at].0;
        at += 1;
    }
    assert_eq!(bits, assignment[2]);
}

#[test]
fn malformed_arenas_return_a_defined_outcome() {
    let out_of_range = system(
        1,
        alloc::vec![RefNode::Hole(0)],
        alloc::vec![(0, 99)],
        Vec::new(),
        0,
    );
    let cyclic = system(
        1,
        alloc::vec![RefNode::Union(0, 0)],
        Vec::new(),
        Vec::new(),
        0,
    );
    let outside = system(
        1,
        alloc::vec![RefNode::Constant(2)],
        Vec::new(),
        Vec::new(),
        0,
    );
    for case in [&out_of_range, &cyclic, &outside] {
        let analysis = analyze(case);
        assert!(matches!(
            analysis.class,
            Class::BuildFailed | Class::OracleUnavailable
        ));
    }
}

#[test]
fn rigid_inconsistency_takes_precedence_over_a_counterexample() {
    let case = system(
        1,
        alloc::vec![
            RefNode::Rigid(0),
            RefNode::Rigid(1),
            RefNode::Constant(1),
            RefNode::Constant(0)
        ],
        alloc::vec![(1, 2)],
        alloc::vec![(0, 3)],
        2,
    );
    assert!(matches!(
        reference(&case),
        Reference::Rigid(RigidReference::Inconsistent)
    ));
    assert_eq!(analyze(&case).class, Class::AgreeRefuse);
}
