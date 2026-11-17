//! Guard-scoped in-place `I64` rewrites: certificate format `ngraph-cert-scoped/0`
//! (issue #108).
//!
//! This is a **separately versioned** extension of [`crate::cert`], not a new
//! `ngraph-cert/0` rule: `ngraph-cert/0` keeps exactly its three rules, so the Lean
//! models and the retained receipts of that revision keep their meaning. A
//! `ngraph-cert-scoped/0` derivation rewrites one binary `I64` node **in place** into
//! the constant the node denotes, and accepts that rewrite only when every operand's
//! single value is justified by a scope the checker re-derives from the graph:
//!
//! - a graph constant (the preflight case),
//! - an interface input whose checked, input-validated fact is a single value (the
//!   preflight case; the input domain is the admitted one, unchanged by the rewrite),
//! - a `guard` node **below** the rewritten node that pins the operand to a single
//!   value (the guarded case).
//!
//! The witness is explicit and checked, every field of it is re-derived and compared,
//! and the two endpoints must differ in nothing else: interface slots, fact records,
//! shapes, outputs, subject and every guard stay exactly as they were.
//!
//! # Why a guard below the node is sound
//!
//! Both evaluators report the first failing guard in node order
//! ([`crate::eval::reference`], [`crate::compiled`]). A guard that precedes the
//! rewritten node is therefore *observed* whenever it fails, in both graphs, at the
//! same node with the same fact; for the inputs where it holds, the operand has the
//! pinned value and the fold is value-preserving. That is the failure-preserving
//! observation relation [`Relation::Equality`] claims: equal observations for every
//! input, not equal values on the inputs that satisfy the scope. The model in
//! `proofs/foundation/NobleFoundation/GuardedRewrite.lean` states and proves that
//! relation; **no correspondence theorem binds these bytes to that model yet**.
//!
//! Fuel, cancellation, allocation, `F32`, sharing relabelling and dependent
//! multi-node substitution are outside this format.

use crate::identity::IdentityScheme;

/// Certificate format revision bound into every accepted derivation.
pub const SCOPED_REVISION: &str = "ngraph-cert-scoped/0";

/// Declared bound on derivation steps: a certificate larger than this is exhausted
/// before any node is inspected.
pub const MAX_STEPS: usize = 64;

/// Declared bound on the nodes one witness scan may traverse. The graph checker
/// admits far more live nodes; a graph beyond this bound is outside the certified
/// relation rather than silently half-scanned.
pub const MAX_WITNESS_NODES: usize = 4096;

/// The relation a derivation claims between its two endpoints.
///
/// Only [`Relation::Equality`] is certified by this revision. The other two are named
/// because the format's vocabulary has to distinguish them, and refusing them is what
/// keeps a weaker claim from standing in for the observation equality the format
/// promises: a scoped claim is silent exactly on the inputs where a failed guard is
/// the observation, and a directed refinement is one direction only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relation {
    /// Equal observations for every input, including which guard fails first.
    Equality,
    /// Equal observations only on the inputs inside the recorded scope.
    ScopedEquality,
    /// One observation refines the other in one direction.
    DirectedRefinement,
}

impl Relation {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Equality => "equality",
            Self::ScopedEquality => "scoped-equality",
            Self::DirectedRefinement => "directed-refinement",
        }
    }
}

/// One accepted rule: the premise class that must justify the fold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// At least one operand is pinned by a guard below the rewritten node.
    Scoped,
    /// Every operand is a graph constant or a single-value input-validated fact.
    Preflight,
}

impl Rule {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Scoped => "fold-scoped",
            Self::Preflight => "fold-preflight",
        }
    }
}

/// How one operand's single value is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// The operand node is the graph constant holding the recorded bits.
    Constant,
    /// The operand is an interface input whose checked fact is one exact value.
    InputFact,
    /// The named guard, below the rewritten node, pins the operand exactly.
    Guard(crate::ir::NodeId),
}

impl Scope {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Constant => "constant",
            Self::InputFact => "input-fact",
            Self::Guard(_) => "guard",
        }
    }
}

/// One derivation step: the whole premise and the whole scope witness.
///
/// Every field is re-derived from the graph the step is checked against, and a step
/// that disagrees with the graph is refused: `operands` are the node's own operands in
/// destination order, `scopes` are the justifications the checker can re-derive,
/// `operand_values` are the single values those justifications pin, and `claimed` is
/// the wrapping result of the node's operation on those two values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// Rule identity.
    pub rule: Rule,
    /// The rewritten node.
    pub node: crate::ir::NodeId,
    /// Ordered operand identities, destination order preserved.
    pub operands: [crate::ir::NodeId; 2],
    /// The scope witness, one checked justification per operand.
    pub scopes: [Scope; 2],
    /// The single value each operand is pinned to.
    pub operand_values: [u64; 2],
    /// The claimed constant: the node's wrapping operation on the two values.
    pub claimed: u64,
}

/// A replayable derivation over exact endpoints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derivation {
    /// Format revision.
    pub revision: alloc::string::String,
    /// The observation relation claimed between the endpoints.
    pub relation: Relation,
    /// Ordered steps; node identities strictly increase.
    pub steps: alloc::vec::Vec<Step>,
    /// Digest of the original graph.
    pub left: crate::ir::Digest,
    /// Digest of the resulting graph.
    pub right: crate::ir::Digest,
}

/// Why a certificate was refused: the claim is not established for these bytes.
///
/// A refusal never asserts that the rewrite is wrong for every input; it asserts that
/// this certificate does not establish the claimed relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The format revision is not the current one.
    StaleRevision,
    /// The steps are not in strictly increasing node order.
    Order,
    /// A step's node, operands or claim does not describe the graph it names.
    Malformed,
    /// The witness names a justification the graph does not contain: no such constant,
    /// no single-valued checked input fact, or no guard below the node naming this
    /// operand with a single-valued fact.
    ScopeUnavailable,
    /// The recorded operand value is not the value the named justification pins.
    ScopeValueMismatch,
    /// Two justifications available for one operand disagree, so no single value is
    /// established for it.
    ScopeConflict,
    /// The rule's premise class does not match the scopes the step records.
    ScopeKindMismatch,
    /// The claimed constant is not the node's wrapping operation on the two values.
    ClaimMismatch,
    /// The endpoints differ in something other than a rewritten node: slots, fact
    /// records, shapes, outputs or subject. This is the changed-input-domain control.
    InterfaceChanged,
    /// A guard of the original graph is not the guard of the resulting graph. A
    /// rewrite may not remove, relocate or retune an observation.
    GuardChanged,
    /// The certificate does not name the endpoints it was checked against.
    EndpointMismatch,
    /// The resulting graph is not the original with the steps replayed in place.
    ResultMismatch,
}

impl Refusal {
    /// Stable label, used by the #64 cost lane.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::StaleRevision => "stale-revision",
            Self::Order => "order",
            Self::Malformed => "malformed",
            Self::ScopeUnavailable => "scope-unavailable",
            Self::ScopeValueMismatch => "scope-value-mismatch",
            Self::ScopeConflict => "scope-conflict",
            Self::ScopeKindMismatch => "scope-kind-mismatch",
            Self::ClaimMismatch => "claim-mismatch",
            Self::InterfaceChanged => "interface-changed",
            Self::GuardChanged => "guard-changed",
            Self::EndpointMismatch => "endpoint-mismatch",
            Self::ResultMismatch => "result-mismatch",
        }
    }
}

/// Work this format does not do, named rather than folded into a refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// The claimed relation kind is outside this revision.
    Relation(Relation),
    /// The step's node holds an operation no rule of this revision rewrites.
    Operation(&'static str),
}

impl Unsupported {
    /// Stable label, used by the #64 cost lane.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Relation(relation) => relation.name(),
            Self::Operation(_) => "operation",
        }
    }
}

/// Proof work this revision bounds and cannot finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exhausted {
    /// The derivation exceeds [`MAX_STEPS`].
    Steps,
    /// An endpoint exceeds [`MAX_WITNESS_NODES`].
    WitnessNodes,
}

impl Exhausted {
    /// Stable label, used by the #64 cost lane.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Steps => "exhausted-steps",
            Self::WitnessNodes => "exhausted-witness-nodes",
        }
    }
}

/// The outcome of checking one derivation.
///
/// `Unsupported` and `Exhausted` are kept apart from `Refused`: neither says the claim
/// is false, and neither may be reported as acceptance. Cancellation has no outcome
/// here unless a caller supplies a cancellation probe to [`check_with_cancel`]; the
/// certified observation relation assumes sufficient budgets and no cancellation, and
/// `Obs::Cancelled` is outside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The steps are accepted: the relation holds for the named endpoints.
    Accepted,
    /// The bytes do not establish the claim.
    Refused(Refusal),
    /// This revision does not cover the claim.
    Unsupported(Unsupported),
    /// The declared work bound stopped the check.
    Exhausted(Exhausted),
    /// The caller's cancellation probe stopped proof work before acceptance.
    Cancelled,
}

impl Outcome {
    /// Whether the derivation was accepted.
    #[must_use]
    pub const fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }

    /// Stable label, used by the #64 cost lane.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Refused(refusal) => refusal.label(),
            Self::Unsupported(unsupported) => unsupported.label(),
            Self::Exhausted(exhausted) => exhausted.label(),
            Self::Cancelled => "cancelled",
        }
    }
}

fn literal_bits(lit: crate::ir::Literal) -> Option<u64> {
    match lit {
        crate::ir::Literal::I64(value) => Some(value.cast_unsigned()),
        crate::ir::Literal::Bool(_) | crate::ir::Literal::F32(_) => None,
    }
}

fn constant_bits(graph: &crate::ir::Graph, node: crate::ir::NodeId) -> Option<u64> {
    let op = &graph.nodes.get(node.index())?.op;
    match op {
        crate::ir::Op::Const(lit) => literal_bits(*lit),
        _ => None,
    }
}

/// The one value a fact pins, if it pins exactly one.
fn single_value(fact: &crate::ir::Fact) -> Option<u64> {
    match fact {
        crate::ir::Fact::Range {
            lo: crate::ir::Literal::I64(lo),
            hi: crate::ir::Literal::I64(hi),
        } if lo == hi => Some(lo.cast_unsigned()),
        _ => None,
    }
}

/// The single value the checked input facts give `node`, when they agree.
///
/// `Err(())` means two single-valued input-validated facts on the same node disagree,
/// so the graph establishes no single value for it.
fn input_fact_value(graph: &crate::ir::Graph, node: crate::ir::NodeId) -> Result<Option<u64>, ()> {
    if !matches!(
        graph.nodes.get(node.index()).map(|n| &n.op),
        Some(crate::ir::Op::Input(_))
    ) {
        return Ok(None);
    }
    let mut agreed: Option<u64> = None;
    for record in &graph.facts {
        if record.node != node || record.provenance != crate::ir::Provenance::InputValidated {
            continue;
        }
        let Some(bits) = single_value(&record.fact) else {
            continue;
        };
        if !merge(&mut agreed, bits) {
            return Err(());
        }
    }
    Ok(agreed)
}

/// Folds one more candidate value into an agreed value; `false` on disagreement.
fn merge(agreed: &mut Option<u64>, value: u64) -> bool {
    match *agreed {
        Some(previous) if previous != value => false,
        Some(_) => true,
        None => {
            *agreed = Some(value);
            true
        }
    }
}

/// The value `guard` pins `operand` to, when `guard` precedes `rewritten`, names
/// `operand` as its value and holds a single-valued fact.
fn guard_pins(
    graph: &crate::ir::Graph,
    rewritten: crate::ir::NodeId,
    guard: crate::ir::NodeId,
    operand: crate::ir::NodeId,
) -> Option<u64> {
    if guard.0 >= rewritten.0 {
        return None;
    }
    match &graph.nodes.get(guard.index())?.op {
        crate::ir::Op::Guard { fact, value } if *value == operand => single_value(fact),
        _ => None,
    }
}

/// Every value the graph pins `operand` to, folded into one agreed value.
///
/// `Err(())` is [`Refusal::ScopeConflict`]: two available justifications disagree, so
/// the graph establishes no single value and this format refuses rather than choosing.
fn agreed_value(
    graph: &crate::ir::Graph,
    rewritten: crate::ir::NodeId,
    operand: crate::ir::NodeId,
) -> Result<Option<u64>, ()> {
    let mut agreed: Option<u64> = None;
    if let Some(bits) = constant_bits(graph, operand) {
        if !merge(&mut agreed, bits) {
            return Err(());
        }
    }
    match input_fact_value(graph, operand) {
        Ok(Some(bits)) => {
            if !merge(&mut agreed, bits) {
                return Err(());
            }
        }
        Ok(None) => {}
        Err(()) => return Err(()),
    }
    let bound = usize::min(rewritten.index(), graph.nodes.len());
    for (index, node) in graph.nodes.iter().enumerate().take(bound) {
        let crate::ir::Op::Guard { fact, value } = &node.op else {
            continue;
        };
        if *value != operand {
            continue;
        }
        let Some(bits) = single_value(fact) else {
            continue;
        };
        let named = crate::ir::NodeId(u32::try_from(index).map_err(|_| ())?);
        if guard_pins(graph, rewritten, named, operand).is_none() || !merge(&mut agreed, bits) {
            return Err(());
        }
    }
    Ok(agreed)
}

/// The claimed value of one rewrite: the node's own wrapping operation.
fn fold_value(op: &crate::ir::Op, first: u64, second: u64) -> Option<u64> {
    let wrap = |value: i64| value.cast_unsigned();
    match op {
        crate::ir::Op::I64Add(..) => Some(wrap(crate::sem::i64_add(
            first.cast_signed(),
            second.cast_signed(),
        ))),
        crate::ir::Op::I64Sub(..) => Some(wrap(crate::sem::i64_sub(
            first.cast_signed(),
            second.cast_signed(),
        ))),
        crate::ir::Op::I64Mul(..) => Some(wrap(crate::sem::i64_mul(
            first.cast_signed(),
            second.cast_signed(),
        ))),
        _ => None,
    }
}

/// Whether two endpoints describe the same interface: profile, slots, outputs, fact
/// records, subject and shapes. A rewrite may change node operations only.
fn same_interface(graph: &crate::ir::Graph, result: &crate::ir::Graph) -> bool {
    graph.profile == result.profile
        && graph.slots == result.slots
        && graph.outputs == result.outputs
        && graph.facts == result.facts
        && graph.subject == result.subject
        && graph.input_shape == result.input_shape
        && graph.output_shape == result.output_shape
}

/// The operand pair of an operation this revision rewrites.
fn arithmetic_operands(op: &crate::ir::Op) -> Option<[crate::ir::NodeId; 2]> {
    match op {
        crate::ir::Op::I64Add(first, second)
        | crate::ir::Op::I64Sub(first, second)
        | crate::ir::Op::I64Mul(first, second) => Some([*first, *second]),
        _ => None,
    }
}

/// The preferred scope for `operand`, when the graph pins it to a single value.
///
/// Preference is fixed so a proposer is deterministic: a constant, then a checked
/// input fact, then the lowest-index guard below the node. The checker does not use
/// the preference; it re-derives every justification and requires agreement.
fn preferred_scope(
    graph: &crate::ir::Graph,
    rewritten: crate::ir::NodeId,
    operand: crate::ir::NodeId,
) -> Option<(Scope, u64)> {
    if let Some(bits) = constant_bits(graph, operand) {
        return Some((Scope::Constant, bits));
    }
    if let Ok(Some(bits)) = input_fact_value(graph, operand) {
        return Some((Scope::InputFact, bits));
    }
    let bound = usize::min(rewritten.index(), graph.nodes.len());
    for (index, node) in graph.nodes.iter().enumerate().take(bound) {
        let crate::ir::Op::Guard { .. } = &node.op else {
            continue;
        };
        let named = crate::ir::NodeId(u32::try_from(index).ok()?);
        if let Some(bits) = guard_pins(graph, rewritten, named, operand) {
            return Some((Scope::Guard(named), bits));
        }
    }
    None
}

/// Proposes in-place folds at up to [`MAX_STEPS`] nodes this revision can justify, in
/// node order, and returns the derivation with the graph it produces.
///
/// Proposal is not acceptance: each step is justified against the graph as already
/// rewritten, and [`check`] is what establishes the relation. A proposal makes no
/// admission claim either — the produced graph must still pass [`crate::check::check`],
/// and a fold can leave a constant operand that only this node used dangling.
#[must_use]
pub fn propose(
    ids: &dyn IdentityScheme,
    graph: &crate::ir::Graph,
) -> (Derivation, crate::ir::Graph) {
    let mut current = graph.clone();
    let mut steps: alloc::vec::Vec<Step> = alloc::vec::Vec::new();
    let mut index = 0;
    while index < current.nodes.len() {
        let node = crate::ir::NodeId(u32::try_from(index).unwrap_or(u32::MAX));
        let op = current.nodes[index].op.clone();
        index += 1;
        let Some(operands) = arithmetic_operands(&op) else {
            continue;
        };
        let Some((first_scope, first_value)) = preferred_scope(&current, node, operands[0]) else {
            continue;
        };
        let Some((second_scope, second_value)) = preferred_scope(&current, node, operands[1])
        else {
            continue;
        };
        if agreed_value(&current, node, operands[0]) != Ok(Some(first_value))
            || agreed_value(&current, node, operands[1]) != Ok(Some(second_value))
        {
            continue;
        }
        let Some(claimed) = fold_value(&op, first_value, second_value) else {
            continue;
        };
        if steps.len() >= MAX_STEPS {
            break;
        }
        let has_guard_scope =
            matches!(first_scope, Scope::Guard(_)) || matches!(second_scope, Scope::Guard(_));
        steps.push(Step {
            rule: if has_guard_scope {
                Rule::Scoped
            } else {
                Rule::Preflight
            },
            node,
            operands,
            scopes: [first_scope, second_scope],
            operand_values: [first_value, second_value],
            claimed,
        });
        current.nodes[node.index()].op =
            crate::ir::Op::Const(crate::ir::Literal::I64(claimed.cast_signed()));
    }
    (
        Derivation {
            revision: alloc::string::String::from(SCOPED_REVISION),
            relation: Relation::Equality,
            steps,
            left: graph.logical_id(ids),
            right: current.logical_id(ids),
        },
        current,
    )
}

/// Validates a derivation against the original and the resulting graph.
///
/// The steps are replayed in place against the graph as already rewritten, so each
/// step's scope witness is re-derived from the very graph it rewrites. Every recorded
/// field is compared with the re-derived one, and the resulting graph must equal the
/// replay with its interface untouched.
#[must_use]
pub fn check(
    ids: &dyn IdentityScheme,
    graph: &crate::ir::Graph,
    result: &crate::ir::Graph,
    derivation: &Derivation,
) -> Outcome {
    check_with_cancel(ids, graph, result, derivation, || false)
}

/// Validates a derivation with a caller-owned cancellation probe.
///
/// The probe is polled after bounded endpoint preflight and between steps. A true result
/// is [`Outcome::Cancelled`], never a refusal or an acceptance. The graph and result are
/// not changed, and the checker owns no cancellation state.
#[must_use]
pub fn check_with_cancel(
    ids: &dyn IdentityScheme,
    graph: &crate::ir::Graph,
    result: &crate::ir::Graph,
    derivation: &Derivation,
    mut is_cancelled: impl FnMut() -> bool,
) -> Outcome {
    if derivation.revision != SCOPED_REVISION {
        return Outcome::Refused(Refusal::StaleRevision);
    }
    if derivation.relation != Relation::Equality {
        return Outcome::Unsupported(Unsupported::Relation(derivation.relation));
    }
    if derivation.steps.len() > MAX_STEPS {
        return Outcome::Exhausted(Exhausted::Steps);
    }
    if graph.nodes.len() > MAX_WITNESS_NODES || result.nodes.len() > MAX_WITNESS_NODES {
        return Outcome::Exhausted(Exhausted::WitnessNodes);
    }
    if graph.logical_id(ids) != derivation.left || result.logical_id(ids) != derivation.right {
        return Outcome::Refused(Refusal::EndpointMismatch);
    }
    if !same_interface(graph, result) {
        return Outcome::Refused(Refusal::InterfaceChanged);
    }
    if is_cancelled() {
        return Outcome::Cancelled;
    }
    let mut current = graph.clone();
    let mut previous: Option<u32> = None;
    for step in &derivation.steps {
        if is_cancelled() {
            return Outcome::Cancelled;
        }
        if previous.is_some_and(|earlier| step.node.0 <= earlier) {
            return Outcome::Refused(Refusal::Order);
        }
        previous = Some(step.node.0);
        let Some(node) = current.nodes.get(step.node.index()) else {
            return Outcome::Refused(Refusal::Malformed);
        };
        let op = node.op.clone();
        let Some(operands) = arithmetic_operands(&op) else {
            return Outcome::Unsupported(Unsupported::Operation(op.name()));
        };
        if operands != step.operands {
            return Outcome::Refused(Refusal::Malformed);
        }
        for position in 0..2 {
            let operand = step.operands[position];
            let scope = step.scopes[position];
            let declared = step.operand_values[position];
            let available = match scope {
                Scope::Constant => constant_bits(&current, operand),
                Scope::InputFact => input_fact_value(&current, operand).ok().flatten(),
                Scope::Guard(named) => guard_pins(&current, step.node, named, operand),
            };
            let Some(available) = available else {
                return Outcome::Refused(Refusal::ScopeUnavailable);
            };
            if available != declared {
                return Outcome::Refused(Refusal::ScopeValueMismatch);
            }
            match agreed_value(&current, step.node, operand) {
                Ok(Some(agreed)) if agreed == declared => {}
                Ok(Some(_)) => return Outcome::Refused(Refusal::ScopeValueMismatch),
                Ok(None) => return Outcome::Refused(Refusal::ScopeUnavailable),
                Err(()) => return Outcome::Refused(Refusal::ScopeConflict),
            }
        }
        let has_guard_scope = step
            .scopes
            .iter()
            .any(|scope| matches!(scope, Scope::Guard(_)));
        let is_class_matching = matches!(
            (step.rule, has_guard_scope),
            (Rule::Scoped, true) | (Rule::Preflight, false)
        );
        if !is_class_matching {
            return Outcome::Refused(Refusal::ScopeKindMismatch);
        }
        let Some(claimed) = fold_value(&op, step.operand_values[0], step.operand_values[1]) else {
            return Outcome::Refused(Refusal::Malformed);
        };
        if claimed != step.claimed {
            return Outcome::Refused(Refusal::ClaimMismatch);
        }
        current.nodes[step.node.index()].op =
            crate::ir::Op::Const(crate::ir::Literal::I64(step.claimed.cast_signed()));
    }
    if current.nodes.len() != result.nodes.len() {
        return Outcome::Refused(Refusal::ResultMismatch);
    }
    for (index, node) in graph.nodes.iter().enumerate() {
        if node.op.is_guard() && result.nodes[index].op != node.op {
            return Outcome::Refused(Refusal::GuardChanged);
        }
    }
    if current != *result {
        return Outcome::Refused(Refusal::ResultMismatch);
    }
    Outcome::Accepted
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_range_pins_a_value_only_when_it_is_exact() {
        let exact = crate::ir::Fact::Range {
            lo: crate::ir::Literal::I64(-3),
            hi: crate::ir::Literal::I64(-3),
        };
        assert_eq!(super::single_value(&exact), Some((-3i64).cast_unsigned()));
        let wide = crate::ir::Fact::Range {
            lo: crate::ir::Literal::I64(0),
            hi: crate::ir::Literal::I64(1),
        };
        assert_eq!(super::single_value(&wide), None);
        assert_eq!(super::single_value(&crate::ir::Fact::Integral), None);
    }

    #[test]
    fn outcomes_report_stable_labels() {
        assert_eq!(super::Outcome::Accepted.label(), "accepted");
        assert_eq!(
            super::Outcome::Refused(super::Refusal::ScopeUnavailable).label(),
            "scope-unavailable"
        );
        assert_eq!(
            super::Outcome::Unsupported(super::Unsupported::Operation("i64.eq")).label(),
            "operation"
        );
        assert!(!super::Outcome::Refused(super::Refusal::Order).is_accepted());
    }
}
