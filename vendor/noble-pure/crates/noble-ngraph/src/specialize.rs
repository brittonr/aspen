//! Bounded static-input specialization for checked `ngraph-i64/0` graphs
//! (issue #32; design `partial-evaluation-v1`, D1–D11).
//!
//! The transformation follows the foundation model
//! `proofs/foundation/NobleFoundation/Specialize.lean`:
//!
//! 1. a **static map** gives one optional `I64` per ordered input leaf; a
//!    static leaf becomes a constant and the remaining leaves are renumbered
//!    by [`rank`] — the model's `rank`/`merge` representation (D2);
//! 2. all-static nodes are folded for wrapping `I64` arithmetic and equality;
//!    total I64 algebraic identities (zero multiplication, addition of zero,
//!    subtraction of zero, multiplication by one) are simplified by forwarding
//!    operands before dead-node pruning. No operations are reordered (D3,
//!    issue #34's separately tested dispatch elimination);
//! 3. nodes that reach no output are **pruned** and the residual is
//!    renumbered densely, then independently re-checked by [`crate::check`]
//!    (D4);
//! 4. the residual is a **new** subject and graph identity with an explicit
//!    derivation record (D5, D11);
//! 5. residual creation is bounded by the `ngraph-pe/0` caps
//!    [`crate::check::Budgets::residual_nodes`] and
//!    [`crate::check::Budgets::variants`] (D7).
//!
//! [`emit_source`] renders a residual as `let`-based Noble source and
//! [`readmit`] independently re-accepts that source through the ordinary
//! Declared-Text-v1 frontend, the kernel and the extractor, so a residual is a
//! Program on the ordinary path before anything is published (D9).
//!
//! Claims and non-claims: `specialize_sound` is a model theorem about graph
//! values. This module is an implementation with finite differential tests; it
//! does **not** claim Rust/model or backend/Program correspondence, or an
//! equal budget, fuel or work relation between an original and its residual
//! (H-LIMIT-03). The extended residual model covers exact `I64Eq` folding
//! under its valid-survivor-map premise; it is not a Rust correspondence proof.

use crate::identity::IdentityScheme;

/// Session-local exact-key reuse of successful residual Programs.
pub mod cache;
/// Opt-in specialization of an exact, checked source Program.
pub mod program;

/// Accounting revision of every cap this module charges and every count it
/// reports (`ngraph-pe/0`, D7).
pub const ACCOUNTING_REVISION: &str = "ngraph-pe/0";

/// One specialization request.
#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    /// One optional `I64` per ordered input leaf (input slots in slot order).
    /// A length other than the input-leaf count is `E-INTERFACE`.
    pub static_inputs: &'a [Option<i64>],
    /// The `/0` budgets: the two `ngraph-pe/0` caps (a request may only lower
    /// them) and the budgets the residual re-check charges.
    pub budgets: &'a crate::check::Budgets,
}

/// What one specialization did. Counts are charges under
/// [`ACCOUNTING_REVISION`]; they are not a claim that the two sides consume
/// equal work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// [`ACCOUNTING_REVISION`].
    pub accounting: &'static str,
    /// `GraphLogicalId` of the checked original.
    pub original_graph: crate::ir::Digest,
    /// `GraphLogicalId` of the checked residual.
    pub residual_graph: crate::ir::Digest,
    /// The accepted static map, indexed by input leaf.
    pub static_inputs: alloc::vec::Vec<Option<i64>>,
    /// Residual interface input types, in slot order.
    pub dynamic_inputs: alloc::vec::Vec<crate::ir::ScalarType>,
    /// Nodes of the original graph.
    pub original_nodes: u64,
    /// Nodes whose operation became a constant, including every substituted
    /// static input leaf.
    pub folded_nodes: u64,
    /// Nodes removed because they reach no output.
    pub pruned_nodes: u64,
    /// Live nodes of the residual.
    pub residual_nodes: u64,
    /// Residual variants produced by this request (always 1 in v1).
    pub variants: u64,
}

impl Report {
    /// Input leaves that were supplied statically.
    #[must_use]
    pub fn static_leaves(&self) -> u64 {
        self.static_inputs
            .iter()
            .filter(|entry| entry.is_some())
            .count() as u64
    }

    /// Input leaves that remain dynamic.
    #[must_use]
    pub fn dynamic_leaves(&self) -> u64 {
        self.dynamic_inputs.len() as u64
    }
}

/// A checked residual graph and its derivation record.
#[derive(Clone, Debug)]
pub struct Residual {
    /// The independently re-checked residual.
    pub checked: crate::check::CheckedGraph,
    /// Derivation record.
    pub report: Report,
}

impl Residual {
    /// The residual graph.
    #[must_use]
    pub fn graph(&self) -> &crate::ir::Graph {
        self.checked.graph()
    }

    /// Input values of the original that this residual's `dynamic` values
    /// stand for, in original input-leaf order (the model's `merge`).
    /// Dynamic values must match the residual interface types.
    #[must_use]
    pub fn merge(
        &self,
        dynamic: &[crate::sem::Value],
    ) -> Option<alloc::vec::Vec<crate::sem::Value>> {
        if dynamic.len() != self.report.dynamic_inputs.len()
            || dynamic
                .iter()
                .zip(&self.report.dynamic_inputs)
                .any(|(value, ty)| value.ty() != *ty)
        {
            return None;
        }
        merge_with(&self.report.static_inputs, dynamic.len(), |index| {
            dynamic.get(index).copied()
        })
    }

    /// Reconstructs the original input vector for an all-I64 residual without
    /// first materializing an intermediate `Value` vector.
    #[must_use]
    pub fn merge_i64(&self, dynamic: &[i64]) -> Option<alloc::vec::Vec<crate::sem::Value>> {
        if dynamic.len() != self.report.dynamic_inputs.len()
            || self
                .report
                .dynamic_inputs
                .iter()
                .any(|ty| *ty != crate::ir::ScalarType::I64)
        {
            return None;
        }
        merge_with(&self.report.static_inputs, dynamic.len(), |index| {
            dynamic.get(index).copied().map(crate::sem::Value::I64)
        })
    }
}

/// The model's `merge`: reconstruct the original input values from the static
/// map and dynamic values, in input-leaf order. The values are copied without
/// type conversion; callers with a checked residual should use
/// [`Residual::merge`] to validate its dynamic interface.
///
/// `None` when `dynamic` does not have exactly one value per `None` entry.
#[must_use]
pub fn merge(
    static_inputs: &[Option<i64>],
    dynamic: &[crate::sem::Value],
) -> Option<alloc::vec::Vec<crate::sem::Value>> {
    merge_with(static_inputs, dynamic.len(), |index| {
        dynamic.get(index).copied()
    })
}

fn merge_with(
    static_inputs: &[Option<i64>],
    dynamic_len: usize,
    mut dynamic_value: impl FnMut(usize) -> Option<crate::sem::Value>,
) -> Option<alloc::vec::Vec<crate::sem::Value>> {
    let expected = static_inputs.iter().filter(|entry| entry.is_none()).count();
    if dynamic_len != expected {
        return None;
    }
    let mut next = 0usize;
    let mut values = alloc::vec::Vec::with_capacity(static_inputs.len());
    for entry in static_inputs {
        let value = match entry {
            Some(value) => crate::sem::Value::I64(*value),
            None => {
                let value = dynamic_value(next)?;
                next += 1;
                value
            }
        };
        values.push(value);
    }
    Some(values)
}

/// The number of dynamic input leaves strictly below `slot` (the model's
/// `rank`), given the static map indexed by input-leaf position.
#[must_use]
pub fn rank(static_inputs: &[Option<i64>], slot: usize) -> Option<u32> {
    let mut rank = 0u32;
    for entry in static_inputs.get(..slot)? {
        if entry.is_none() {
            rank = rank.checked_add(1)?;
        }
    }
    Some(rank)
}

fn refuse(
    code: crate::diag::Code,
    detail: impl Into<alloc::string::String>,
) -> crate::diag::Diagnostic {
    crate::diag::Diagnostic::new(code, crate::diag::Site::Graph, detail)
}

fn at(
    code: crate::diag::Code,
    site: crate::diag::Site,
    detail: impl Into<alloc::string::String>,
) -> crate::diag::Diagnostic {
    crate::diag::Diagnostic::new(code, site, detail)
}

/// A statically known value of one node (D3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Known {
    I64(i64),
    Bool(bool),
}

impl Known {
    const fn i64(self) -> Option<i64> {
        match self {
            Self::I64(value) => Some(value),
            Self::Bool(_) => None,
        }
    }

    const fn as_literal(self) -> crate::ir::Literal {
        match self {
            Self::I64(value) => crate::ir::Literal::I64(value),
            Self::Bool(value) => crate::ir::Literal::Bool(value),
        }
    }
}

/// The static value of a literal, if the literal is statically known.
const fn known_literal(literal: crate::ir::Literal) -> Option<Known> {
    match literal {
        crate::ir::Literal::I64(value) => Some(Known::I64(value)),
        crate::ir::Literal::Bool(value) => Some(Known::Bool(value)),
        crate::ir::Literal::F32(_) => None,
    }
}

/// The statically known result of `op` when its operands are `known`,
/// following D3's whitelist exactly.
fn static_result(op: &crate::ir::Op, known: &[Option<Known>]) -> Option<Known> {
    match op {
        crate::ir::Op::I64Add(first, second)
        | crate::ir::Op::I64Sub(first, second)
        | crate::ir::Op::I64Mul(first, second) => {
            let x = known.get(first.index())?.as_ref()?.i64()?;
            let y = known.get(second.index())?.as_ref()?.i64()?;
            Some(Known::I64(match op {
                crate::ir::Op::I64Add(..) => x.wrapping_add(y),
                crate::ir::Op::I64Sub(..) => x.wrapping_sub(y),
                _ => x.wrapping_mul(y),
            }))
        }
        crate::ir::Op::I64Eq(first, second) => {
            let x = known.get(first.index())?.as_ref()?.i64()?;
            let y = known.get(second.index())?.as_ref()?.i64()?;
            Some(Known::Bool(x == y))
        }
        _ => None,
    }
}

/// Total wrapping-I64 identities only. The admitted slice has no guards,
/// traps, effects, or lazily evaluated operands; these rules are not valid for
/// an arbitrary graph profile with observable failures in an erased operand.
enum Algebraic {
    Zero,
    Forward(crate::ir::NodeId),
}

fn algebraic_result(op: &crate::ir::Op, known: &[Option<Known>]) -> Option<Algebraic> {
    let value = |id: crate::ir::NodeId| {
        known
            .get(id.index())
            .copied()
            .flatten()
            .and_then(Known::i64)
    };
    match op {
        crate::ir::Op::I64Add(a, b) if value(*a) == Some(0) => Some(Algebraic::Forward(*b)),
        crate::ir::Op::I64Add(a, b) if value(*b) == Some(0) => Some(Algebraic::Forward(*a)),
        crate::ir::Op::I64Sub(a, b) if value(*b) == Some(0) => Some(Algebraic::Forward(*a)),
        crate::ir::Op::I64Mul(a, b) if value(*a) == Some(0) || value(*b) == Some(0) => {
            Some(Algebraic::Zero)
        }
        crate::ir::Op::I64Mul(a, b) if value(*a) == Some(1) => Some(Algebraic::Forward(*b)),
        crate::ir::Op::I64Mul(a, b) if value(*b) == Some(1) => Some(Algebraic::Forward(*a)),
        _ => None,
    }
}

/// Refuses a graph outside the `ngraph-pe/0` subject slice (D1).
fn admit_slice(checked: &crate::check::CheckedGraph) -> Result<(), crate::diag::Diagnostic> {
    let graph = checked.graph();
    if graph.profile != crate::ir::SubProfile::I64 {
        return Err(refuse(crate::diag::Code::Profile, {
            let mut detail = alloc::string::String::new();
            crate::format::write_text!(
                detail,
                "static-input specialization admits ngraph-i64/0 graphs; this graph is {}",
                graph.profile.name()
            );
            detail
        }));
    }
    let mut detail = alloc::string::String::new();
    for (k, slot) in graph.slots.iter().enumerate() {
        if slot.kind == crate::ir::SlotKind::Param {
            detail.clear();
            crate::format::write_text!(detail, "param slot {k} is outside the ngraph-pe/0 slice");
            return Err(refuse(
                crate::diag::Code::Profile,
                core::mem::take(&mut detail),
            ));
        }
    }
    if graph.input_shape.is_some() || graph.output_shape.is_some() {
        return Err(refuse(
            crate::diag::Code::Aggregate,
            "an interface shape tree is outside the ngraph-pe/0 slice; v1 admits flat scalar interfaces only",
        ));
    }
    if !graph.facts.is_empty() {
        return Err(refuse(crate::diag::Code::Profile, {
            let mut detail = alloc::string::String::new();
            crate::format::write_text!(
                detail,
                "{} fact record(s) are outside the ngraph-pe/0 slice",
                graph.facts.len()
            );
            detail
        }));
    }
    for (i, node) in graph.nodes.iter().enumerate() {
        match node.op {
            crate::ir::Op::Input(_)
            | crate::ir::Op::Const(_)
            | crate::ir::Op::I64Add(..)
            | crate::ir::Op::I64Sub(..)
            | crate::ir::Op::I64Mul(..)
            | crate::ir::Op::I64Eq(..) => {}
            _ => {
                detail.clear();
                crate::format::write_text!(
                    detail,
                    "{} is outside the ngraph-pe/0 slice: a guard or failing node is an observation the residual cannot preserve",
                    node.op.name()
                );
                return Err(at(
                    crate::diag::Code::Profile,
                    crate::diag::Site::Node(crate::ir::NodeId(i as u32)),
                    core::mem::take(&mut detail),
                ));
            }
        }
    }
    Ok(())
}

/// Maps a slot index to its position among the input leaves.
fn leaf_positions(graph: &crate::ir::Graph) -> alloc::vec::Vec<Option<usize>> {
    let mut positions = alloc::vec::Vec::with_capacity(graph.slots.len());
    let mut next = 0usize;
    for slot in &graph.slots {
        if slot.kind == crate::ir::SlotKind::Input {
            positions.push(Some(next));
            next += 1;
        } else {
            positions.push(None);
        }
    }
    positions
}

/// Rewrites `op`'s operands through `map`.
fn renumber(op: &crate::ir::Op, map: &[u32]) -> crate::ir::Op {
    let mapped = |id: crate::ir::NodeId| crate::ir::NodeId(map[id.index()]);
    match op {
        crate::ir::Op::Input(k) => crate::ir::Op::Input(*k),
        crate::ir::Op::Const(literal) => crate::ir::Op::Const(*literal),
        crate::ir::Op::I64Add(first, second) => {
            crate::ir::Op::I64Add(mapped(*first), mapped(*second))
        }
        crate::ir::Op::I64Sub(first, second) => {
            crate::ir::Op::I64Sub(mapped(*first), mapped(*second))
        }
        crate::ir::Op::I64Mul(first, second) => {
            crate::ir::Op::I64Mul(mapped(*first), mapped(*second))
        }
        crate::ir::Op::I64Eq(first, second) => {
            crate::ir::Op::I64Eq(mapped(*first), mapped(*second))
        }
        // `admit_slice` refused every other operation before this pass.
        other => other.clone(),
    }
}

/// Derives the residual subject identity from the same fields used by the
/// specializer. The cache rechecks this relation before it accepts a retained
/// residual; it is not a proof of specialization correctness.
pub(super) fn derive_residual_subject(
    ids: &dyn IdentityScheme,
    original_graph: crate::ir::Digest,
    closure: alloc::vec::Vec<u64>,
    static_inputs: &[Option<i64>],
) -> crate::ir::Subject {
    let mut encoded: alloc::vec::Vec<u8> = alloc::vec::Vec::with_capacity(
        1usize.saturating_add(9usize.saturating_mul(static_inputs.len())),
    );
    encoded.push(0x01);
    for entry in static_inputs {
        match entry {
            Some(value) => {
                encoded.push(0x01);
                encoded.extend_from_slice(&value.to_le_bytes());
            }
            None => encoded.push(0x00),
        }
    }
    crate::ir::Subject::derive(
        ids,
        crate::ir::SubjectKind::Extracted,
        &[
            b"partial-evaluation",
            ACCOUNTING_REVISION.as_bytes(),
            &original_graph,
            &encoded,
        ],
        closure,
    )
}

/// Specializes `checked` under `request.static_inputs` (D1–D7, D11).
///
/// # Errors
/// `E-PROFILE` for a raised cap or a subject outside the slice, `E-INTERFACE`
/// for a static map of the wrong arity or a static `Bool` leaf,
/// `E-BUDGET-RESIDUAL-NODES` / `E-BUDGET-VARIANTS` for the caps, and any
/// refusal of the independent residual re-check.
#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-ngraph; this public operation intentionally shares the specialization module's domain name, matching the documented library API"
)]
pub fn specialize(
    ids: &dyn IdentityScheme,
    checked: &crate::check::CheckedGraph,
    request: &Request<'_>,
) -> Result<Residual, crate::diag::Diagnostic> {
    request.budgets.validate_policy()?;
    admit_slice(checked)?;
    let graph = checked.graph();
    let positions = leaf_positions(graph);
    let leaves = positions.iter().filter(|p| p.is_some()).count();
    if request.static_inputs.len() != leaves {
        return Err(at(
            crate::diag::Code::Interface,
            crate::diag::Site::Graph,
            {
                let mut detail = alloc::string::String::new();
                crate::format::write_text!(
                    detail,
                    "the static map has {} entr(y/ies) for {leaves} input leaf/leaves",
                    request.static_inputs.len()
                );
                detail
            },
        ));
    }
    if request.budgets.variants < 1 {
        return Err(crate::diag::Diagnostic::budget(
            crate::diag::BudgetKind::Variants,
            request.budgets.variants,
            1,
        ));
    }
    for (slot, position) in positions.iter().enumerate() {
        let Some(position) = position else { continue };
        if request.static_inputs[*position].is_some()
            && graph.slots[slot].ty != crate::ir::ScalarType::I64
        {
            return Err(at(
                crate::diag::Code::Interface,
                crate::diag::Site::Slot(slot as u32),
                "the static map is I64-valued; a Bool input leaf cannot be static in ngraph-pe/0",
            ));
        }
    }
    // The model's `rank` for every slot, and the residual's slots.
    let mut rank_of: alloc::vec::Vec<Option<u32>> =
        alloc::vec::Vec::with_capacity(graph.slots.len());
    let mut slots: alloc::vec::Vec<crate::ir::Slot> =
        alloc::vec::Vec::with_capacity(graph.slots.len());
    let mut next_rank = 0u32;
    for (slot, position) in positions.iter().enumerate() {
        match position {
            None => rank_of.push(None),
            Some(position) => {
                if request.static_inputs[*position].is_some() {
                    rank_of.push(None);
                } else {
                    rank_of.push(Some(next_rank));
                    slots.push(crate::ir::Slot {
                        kind: crate::ir::SlotKind::Input,
                        ty: graph.slots[slot].ty,
                    });
                    next_rank = next_rank.saturating_add(1);
                }
            }
        }
    }
    let original_nodes = graph.nodes.len();
    if original_nodes as u64 > request.budgets.residual_nodes {
        return Err(crate::diag::Diagnostic::budget(
            crate::diag::BudgetKind::ResidualNodes,
            request.budgets.residual_nodes,
            original_nodes as u64,
        ));
    }
    // Substitution, folding and total identities in dependency order. Forward
    // references always target an earlier node; aliases never enter the live
    // graph, and consumers see their canonical operand in the same pass.
    let mut known: alloc::vec::Vec<Option<Known>> = alloc::vec![None; original_nodes];
    let mut forward: alloc::vec::Vec<u32> = alloc::vec::Vec::with_capacity(original_nodes);
    let mut built: alloc::vec::Vec<crate::ir::Op> = alloc::vec::Vec::with_capacity(original_nodes);
    let mut detail = alloc::string::String::new();
    let mut folded = 0u64;
    for (index, node) in graph.nodes.iter().enumerate() {
        let mut value = None;
        let mut alias = None;
        let op = match &node.op {
            crate::ir::Op::Input(k) => {
                let position = positions
                    .get(*k as usize)
                    .copied()
                    .flatten()
                    .ok_or_else(|| {
                        detail.clear();
                        crate::format::write_text!(detail, "input node names undeclared slot {k}");
                        at(
                            crate::diag::Code::Interface,
                            crate::diag::Site::Node(crate::ir::NodeId(index as u32)),
                            core::mem::take(&mut detail),
                        )
                    })?;
                match request.static_inputs[position] {
                    Some(v) => {
                        value = Some(Known::I64(v));
                        folded = folded.saturating_add(1);
                        crate::ir::Op::Const(crate::ir::Literal::I64(v))
                    }
                    None => crate::ir::Op::Input(rank_of[*k as usize].ok_or_else(|| {
                        detail.clear();
                        crate::format::write_text!(detail, "slot {k} has no dynamic rank");
                        at(
                            crate::diag::Code::Interface,
                            crate::diag::Site::Node(crate::ir::NodeId(index as u32)),
                            core::mem::take(&mut detail),
                        )
                    })?),
                }
            }
            crate::ir::Op::Const(literal) => {
                value = known_literal(*literal);
                crate::ir::Op::Const(*literal)
            }
            other => {
                let canonical = renumber(other, &forward);
                match static_result(&canonical, &known) {
                    Some(folded_value) => {
                        value = Some(folded_value);
                        folded = folded.saturating_add(1);
                        crate::ir::Op::Const(folded_value.as_literal())
                    }
                    None => match algebraic_result(&canonical, &known) {
                        Some(Algebraic::Zero) => {
                            value = Some(Known::I64(0));
                            folded = folded.saturating_add(1);
                            crate::ir::Op::Const(crate::ir::Literal::I64(0))
                        }
                        Some(Algebraic::Forward(target)) => {
                            alias = Some(target.0);
                            canonical
                        }
                        None => canonical,
                    },
                }
            }
        };
        known[index] = value;
        forward.push(alias.unwrap_or(index as u32));
        built.push(op);
    }
    // Pruning: keep what the outputs reach, plus every dynamic input node so
    // each retained interface slot remains defined (D4).
    let mut keep = alloc::vec![false; original_nodes];
    let mut pending: alloc::vec::Vec<u32> =
        graph.outputs.iter().map(|o| forward[o.index()]).collect();
    while let Some(index) = pending.pop() {
        let slot = match keep.get_mut(index as usize) {
            Some(slot) => slot,
            None => {
                return Err(at(
                    crate::diag::Code::Interface,
                    crate::diag::Site::Graph,
                    "an output names a node that does not exist",
                ))
            }
        };
        if *slot {
            continue;
        }
        *slot = true;
        pending.extend(built[index as usize].operands().iter().map(|id| id.0));
    }
    for (index, op) in built.iter().enumerate() {
        if matches!(op, crate::ir::Op::Input(_)) {
            keep[index] = true;
        }
    }
    // Renumber densely in canonical order. Operand ids of a kept node are
    // always kept (they are what made it reachable), so the map is complete
    // before any node is rewritten.
    let mut map = alloc::vec![u32::MAX; original_nodes];
    let mut next = 0u32;
    for (index, kept) in keep.iter().enumerate() {
        if *kept {
            map[index] = next;
            next = next.saturating_add(1);
        }
    }
    let mut nodes = alloc::vec::Vec::with_capacity(next as usize);
    for (index, op) in built.iter().enumerate() {
        if keep[index] {
            nodes.push(crate::ir::Node {
                op: renumber(op, &map),
                // A residual node's provenance is the derivation record: the
                // source site it came from describes a Program the residual no
                // longer is (D5, D9).
                origin: None,
            });
        }
    }
    let mut outputs = alloc::vec::Vec::with_capacity(graph.outputs.len());
    let mut detail = alloc::string::String::new();
    for output in &graph.outputs {
        let id = map[forward[output.index()] as usize];
        if id == u32::MAX {
            detail.clear();
            crate::format::write_text!(detail, "%{} is an output but was pruned", output.0);
            return Err(at(
                crate::diag::Code::Interface,
                crate::diag::Site::Graph,
                core::mem::take(&mut detail),
            ));
        }
        outputs.push(crate::ir::NodeId(id));
    }
    let original_graph = checked.id();
    let subject = derive_residual_subject(
        ids,
        original_graph,
        graph.subject.closure.clone(),
        request.static_inputs,
    );
    let residual = crate::ir::Graph {
        profile: crate::ir::SubProfile::I64,
        slots: slots.clone(),
        nodes,
        outputs,
        facts: alloc::vec::Vec::new(),
        subject,
        input_shape: None,
        output_shape: None,
    };
    let kept = residual.nodes.len() as u64;
    let rechecked = crate::check::check_with(ids, residual, request.budgets)?;
    let report = Report {
        accounting: ACCOUNTING_REVISION,
        original_graph,
        residual_graph: rechecked.id(),
        static_inputs: request.static_inputs.to_vec(),
        dynamic_inputs: slots.iter().map(|slot| slot.ty).collect(),
        original_nodes: original_nodes as u64,
        folded_nodes: folded,
        pruned_nodes: (original_nodes as u64).saturating_sub(kept),
        residual_nodes: kept,
        variants: 1,
    };
    Ok(Residual {
        checked: rechecked,
        report,
    })
}

/// The residual interface as kernel stack types, in slot order.
///
/// # Errors
/// `E-INTERFACE` if the graph still has a param slot (outside the slice).
pub fn interface_types(
    checked: &crate::check::CheckedGraph,
) -> Result<alloc::vec::Vec<noble_kernel::types::Ty>, crate::diag::Diagnostic> {
    let mut types = alloc::vec::Vec::with_capacity(checked.graph().slots.len());
    for (k, slot) in checked.graph().slots.iter().enumerate() {
        match (slot.kind, slot.ty) {
            (crate::ir::SlotKind::Input, crate::ir::ScalarType::I64) => {
                types.push(noble_kernel::types::Ty::I64)
            }
            (crate::ir::SlotKind::Input, crate::ir::ScalarType::Bool) => {
                types.push(noble_kernel::types::Ty::Bool)
            }
            (crate::ir::SlotKind::Input, crate::ir::ScalarType::F32) => {
                return Err(at(
                    crate::diag::Code::Profile,
                    crate::diag::Site::Slot(k as u32),
                    "an F32 input slot is outside the ngraph-pe/0 slice",
                ))
            }
            (crate::ir::SlotKind::Param, _) => {
                return Err(at(
                    crate::diag::Code::Profile,
                    crate::diag::Site::Slot(k as u32),
                    "a param slot is outside the ngraph-pe/0 slice",
                ))
            }
        }
    }
    Ok(types)
}

/// Renders `checked` as `let`-based Noble source (D9).
///
/// The program's stack effect is the graph's interface: dynamic inputs enter
/// bottom-first, every node is bound by `let v<node>`, and the outputs are read
/// in output order. A zero-interface, zero-node graph uses `0 drop` as an
/// effect-free source body because the ordinary compiler refuses an empty
/// source module; that body re-extracts to the empty graph. The result is not
/// accepted here; [`readmit`] is the independent acceptance step.
///
/// # Errors
/// `E-PROFILE` for a graph outside the slice (a guard, a fact record or an
/// operation the emitter cannot spell) and `E-AGGREGATE` for an interface
/// shape tree.
pub fn emit_source(
    checked: &crate::check::CheckedGraph,
) -> Result<alloc::string::String, crate::diag::Diagnostic> {
    admit_slice(checked)?;
    let graph = checked.graph();
    let capacity_bytes = graph
        .nodes
        .len()
        .saturating_mul(44)
        .saturating_add(graph.outputs.len().saturating_mul(12));
    let mut text = alloc::string::String::with_capacity(capacity_bytes);
    let mut bound = alloc::vec![false; graph.nodes.len()];
    // Inputs are bound deepest last: the residual's stack enters bottom-first.
    for (k, slot) in graph.slots.iter().enumerate().rev() {
        if slot.kind != crate::ir::SlotKind::Input {
            continue;
        }
        let node = checked.slot_node(k as u32);
        match graph.nodes[node.index()].op {
            crate::ir::Op::Input(slot_index) if slot_index == k as u32 => {
                bound[node.index()] = true;
                crate::format::write_text!(text, "let v{}\n", node.0);
            }
            _ => {
                return Err(at(
                    crate::diag::Code::Interface,
                    crate::diag::Site::Slot(k as u32),
                    "an input slot has no input node",
                ))
            }
        }
    }
    let mut detail = alloc::string::String::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        if bound[index] {
            continue;
        }
        let id = crate::ir::NodeId(index as u32);
        match &node.op {
            crate::ir::Op::Input(_) => {
                return Err(at(
                    crate::diag::Code::Interface,
                    crate::diag::Site::Node(id),
                    "a static input leaf is a constant, not an input node",
                ))
            }
            crate::ir::Op::Const(crate::ir::Literal::I64(value)) => {
                crate::format::write_text!(text, "{value}");
            }
            crate::ir::Op::Const(crate::ir::Literal::Bool(value)) => {
                if *value {
                    text.push_str("true");
                } else {
                    text.push_str("false");
                }
            }
            crate::ir::Op::Const(crate::ir::Literal::F32(_)) => {
                return Err(at(
                    crate::diag::Code::Profile,
                    crate::diag::Site::Node(id),
                    "an F32 constant is outside the ngraph-pe/0 slice",
                ))
            }
            crate::ir::Op::I64Add(first, second)
            | crate::ir::Op::I64Sub(first, second)
            | crate::ir::Op::I64Mul(first, second) => {
                crate::format::write_text!(
                    text,
                    "v{} v{} {}",
                    first.0,
                    second.0,
                    match node.op {
                        crate::ir::Op::I64Add(..) => "+",
                        crate::ir::Op::I64Sub(..) => "-",
                        _ => "*",
                    }
                );
            }
            crate::ir::Op::I64Eq(first, second) => {
                crate::format::write_text!(text, "v{} v{} =", first.0, second.0);
            }
            other => {
                detail.clear();
                crate::format::write_text!(
                    detail,
                    "{} cannot be emitted as Noble source",
                    other.name()
                );
                return Err(at(
                    crate::diag::Code::Profile,
                    crate::diag::Site::Node(id),
                    core::mem::take(&mut detail),
                ));
            }
        }
        bound[index] = true;
        crate::format::write_text!(text, " let v{}\n", id.0);
    }
    for output in &graph.outputs {
        crate::format::write_text!(text, "v{}\n", output.0);
    }
    if graph.nodes.is_empty() && graph.slots.is_empty() && graph.outputs.is_empty() {
        text.push_str("0 drop\n");
    }
    Ok(text)
}

fn source_refusal(error: noble_contracts::source::Error) -> crate::diag::Diagnostic {
    refuse(crate::diag::Code::Subject, {
        let mut detail = alloc::string::String::new();
        crate::format::write_text!(detail, "the residual source was refused: {error:?}");
        detail
    })
}

/// Independently re-accepts emitted residual source on the ordinary path (D9).
///
/// The source is prepared by the ordinary Declared-Text-v1 frontend under the
/// caller-provided source limits, then its kernel-checked submission is re-admitted
/// and re-extracted, so the returned graph is a checked `ngraph-i64/0` graph
/// reached through the same frontend, kernel and checker as any other subject.
///
/// # Errors
/// `E-SUBJECT` for a frontend or kernel refusal, the admission and extraction
/// codes, and whatever the residual re-check reports.
pub fn readmit(
    ids: &dyn IdentityScheme,
    source: &str,
    inputs: &[noble_kernel::types::Ty],
    source_limits: noble_contracts::Limits,
    budgets: &crate::check::Budgets,
) -> Result<crate::check::CheckedGraph, crate::diag::Diagnostic> {
    let retained = noble_contracts::source::RetainedSourceLimit::new(source_limits.bytes)
        .ok_or_else(|| {
            refuse(
                crate::diag::Code::Profile,
                "the source byte limit exceeds the retained-source cap",
            )
        })?;
    let session = noble_contracts::source::ModuleSession::new_declared_text(&[], false, retained)
        .map_err(source_refusal)?;
    let prepared = session
        .prepare(source.as_bytes(), inputs, source_limits)
        .map_err(source_refusal)?;
    let submission = prepared.submission().ok_or_else(|| {
        refuse(
            crate::diag::Code::Subject,
            "declared-text preparation produced no submission",
        )
    })?;
    let mut claimed: alloc::vec::Vec<u64> = submission
        .definitions
        .iter()
        .map(|definition| definition.identity)
        .collect();
    claimed.sort_unstable();
    claimed.dedup();
    let admitted = crate::extract::admit(
        ids,
        submission,
        &crate::extract::Request {
            profile: "ngraph-i64/0",
            claimed_closure: &claimed,
            budget: crate::extract::Budget {
                work_units: budgets.work_units,
                expanded_nodes: budgets.expanded_nodes,
                inline_depth: budgets.inline_depth,
            },
        },
    )?;
    Ok(crate::extract::extract(ids, &admitted, budgets)?.checked)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHasher(u64);

    impl crate::identity::IdentityHasher for TestHasher {
        fn update(&mut self, bytes: &[u8]) {
            for &byte in bytes {
                self.0 = self.0.wrapping_mul(31).wrapping_add(u64::from(byte));
            }
        }

        fn finish(self: alloc::boxed::Box<Self>) -> crate::ir::Digest {
            let mut digest = [0u8; 32];
            digest[..8].copy_from_slice(&self.0.to_le_bytes());
            digest
        }
    }

    struct TestIds;

    impl IdentityScheme for TestIds {
        fn name(&self) -> &'static str {
            "test"
        }

        fn start(&self) -> alloc::boxed::Box<dyn crate::identity::IdentityHasher + '_> {
            alloc::boxed::Box::new(TestHasher(0))
        }
    }

    fn checked(graph: crate::ir::Graph) -> crate::check::CheckedGraph {
        crate::check::check(&TestIds, graph).expect("the fixture graph is checked")
    }

    /// The model's mixed fixture: `%0 = input 0; %1 = input 1;
    /// %2 = i64.mul %0 %1; %3 = i64.add %2 %0; out %3` (`y·x + x`).
    fn product_plus() -> crate::check::CheckedGraph {
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-product-plus"),
        );
        let x = builder.input(crate::ir::ScalarType::I64);
        let y = builder.input(crate::ir::ScalarType::I64);
        let product = builder.op(crate::ir::Op::I64Mul(x, y));
        let sum = builder.op(crate::ir::Op::I64Add(product, x));
        builder.output(sum);
        checked(builder.finish())
    }

    fn run(
        checked: &crate::check::CheckedGraph,
        inputs: &[i64],
    ) -> alloc::vec::Vec<crate::sem::Value> {
        match crate::eval::reference(
            checked,
            &crate::eval::i64_inputs(inputs),
            None,
            &mut crate::eval::Control::default(),
        )
        .obs
        {
            crate::eval::Obs::Normal(values) => values,
            other => panic!("unexpected observation {other:?}"),
        }
    }

    fn with_static_map(
        checked: &crate::check::CheckedGraph,
        static_inputs: &[Option<i64>],
    ) -> Result<Residual, crate::diag::Diagnostic> {
        let budgets = crate::check::Budgets::default();
        specialize(
            &TestIds,
            checked,
            &Request {
                static_inputs,
                budgets: &budgets,
            },
        )
    }

    #[test]
    fn no_static_inputs_keeps_the_graph_and_index_order() {
        let original = product_plus();
        let residual = with_static_map(&original, &[None, None]).expect("specialized");
        assert_eq!(residual.report.folded_nodes, 0);
        assert_eq!(residual.report.pruned_nodes, 0);
        assert_eq!(residual.report.residual_nodes, 4);
        assert_eq!(
            residual.report.dynamic_inputs,
            alloc::vec![crate::ir::ScalarType::I64; 2]
        );
        for (x, y) in [(5i64, 4i64), (-1, i64::MAX), (0, 0)] {
            assert_eq!(
                run(&residual.checked, &[x, y]),
                run(&original, &[x, y]),
                "no-static ({x}, {y})"
            );
        }
    }

    #[test]
    fn total_identities_forward_chains_and_zero_mul_but_not_zero_minus_x() {
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-forward-total-identities"),
        );
        let x = builder.input(crate::ir::ScalarType::I64);
        let y = builder.input(crate::ir::ScalarType::I64);
        let one = builder.i64_const(1);
        let zero = builder.i64_const(0);
        let first = builder.op(crate::ir::Op::I64Mul(one, x));
        let second = builder.op(crate::ir::Op::I64Add(first, zero));
        let annihilated = builder.op(crate::ir::Op::I64Mul(zero, y));
        let third = builder.op(crate::ir::Op::I64Add(second, annihilated));
        let fourth = builder.op(crate::ir::Op::I64Sub(third, zero));
        let negative = builder.op(crate::ir::Op::I64Sub(zero, x));
        builder.output(fourth);
        builder.output(negative);
        let original = checked(builder.finish());
        let residual = with_static_map(&original, &[None, None]).expect("total rewrite");
        assert_eq!(residual.checked.graph().nodes.len(), 4);
        assert_eq!(residual.checked.graph().outputs[0], crate::ir::NodeId(0));
        assert!(matches!(
            residual.checked.graph().nodes[3].op,
            crate::ir::Op::I64Sub(..)
        ));
        for (x, y) in [(7, 2), (i64::MIN, -1), (0, i64::MAX)] {
            assert_eq!(run(&residual.checked, &[x, y]), run(&original, &[x, y]));
        }
        let source = emit_source(&residual.checked).expect("emitted");
        let readmitted = readmit(
            &TestIds,
            &source,
            &interface_types(&residual.checked).expect("interface"),
            noble_contracts::Limits::default(),
            &crate::check::Budgets::default(),
        )
        .expect("ordinary admission");
        assert_eq!(run(&readmitted, &[7, 2]), run(&original, &[7, 2]));
    }

    #[test]
    fn an_unused_dynamic_input_keeps_its_residual_slot_defined() {
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-unused-input"),
        );
        let used = builder.input(crate::ir::ScalarType::I64);
        let _unused = builder.input(crate::ir::ScalarType::I64);
        builder.output(used);
        let original = checked(builder.finish());
        let residual = with_static_map(&original, &[None, None]).expect("specialized");
        assert_eq!(residual.report.pruned_nodes, 0);
        assert_eq!(
            residual.report.dynamic_inputs,
            alloc::vec![crate::ir::ScalarType::I64; 2]
        );
        assert_eq!(residual.checked.graph().slots.len(), 2);
        assert_eq!(run(&residual.checked, &[4, 9]), run(&original, &[4, 9]));

        let source = emit_source(&residual.checked).expect("emitted");
        let types = interface_types(&residual.checked).expect("types");
        let readmitted = readmit(
            &TestIds,
            &source,
            &types,
            noble_contracts::Limits::default(),
            &crate::check::Budgets::default(),
        )
        .expect("re-admitted");
        assert_eq!(readmitted.graph().slots.len(), 2);
        assert_eq!(run(&readmitted, &[4, 9]), run(&original, &[4, 9]));
    }

    #[test]
    fn static_prefix_leaf_becomes_a_constant_and_collapses() {
        let original = product_plus();
        // `x = 5` static: `%2 = 5·y` stays residual, `%3 = %2 + 5`.
        let residual = with_static_map(&original, &[Some(5), None]).expect("specialized");
        assert_eq!(residual.report.folded_nodes, 1);
        assert_eq!(
            residual.report.dynamic_inputs,
            alloc::vec![crate::ir::ScalarType::I64; 1]
        );
        for y in [0i64, 4, -3, i64::MIN] {
            assert_eq!(
                run(&residual.checked, &[y]),
                run(&original, &[5, y]),
                "static-prefix y = {y}"
            );
            assert_eq!(
                residual.merge_i64(&[y]).expect("merge"),
                crate::eval::i64_inputs(&[5, y])
            );
        }
    }

    #[test]
    fn non_prefix_static_slot_is_ranked_and_the_prefix_control_disagrees() {
        let original = product_plus();
        // `y = 7` static, `x` dynamic: the residual's only input is `x` (rank 0).
        let residual = with_static_map(&original, &[None, Some(7)]).expect("specialized");
        assert_eq!(
            residual.report.dynamic_inputs,
            alloc::vec![crate::ir::ScalarType::I64; 1]
        );
        for x in [0i64, 3, -9] {
            assert_eq!(run(&residual.checked, &[x]), run(&original, &[x, 7]));
        }
        // A prefix-based merge puts the known value in slot 0 instead of
        // slot 1. It therefore reconstructs [7, x], not [x, 7], and this
        // non-commutative fixture makes the resulting outputs disagree.
        let dynamic = [crate::sem::Value::I64(3)];
        let merged = merge(&[None, Some(7)], &dynamic).expect("ranked merge");
        let prefix_merged = merge(&[Some(7), None], &dynamic).expect("prefix merge control");
        assert_eq!(merged, crate::eval::i64_inputs(&[3, 7]));
        assert_eq!(prefix_merged, crate::eval::i64_inputs(&[7, 3]));
        assert_ne!(
            run(&original, &[3, 7]),
            run(&original, &[7, 3]),
            "the prefix merge must disagree on the non-prefix fixture"
        );
        assert_ne!(
            rank(&[None, Some(7)], 1),
            rank(&[Some(7), None], 1),
            "the prefix map ranks the dynamic slot differently"
        );
    }

    #[test]
    fn all_static_inputs_fold_the_whole_graph() {
        let original = product_plus();
        let residual = with_static_map(&original, &[Some(6), Some(7)]).expect("specialized");
        // Two static leaves and two folded arithmetic nodes.
        assert_eq!(residual.report.folded_nodes, 4);
        assert_eq!(residual.report.dynamic_inputs, alloc::vec::Vec::new());
        assert_eq!(residual.report.residual_nodes, 1);
        assert_eq!(residual.report.pruned_nodes, 3);
        assert_eq!(run(&residual.checked, &[]), run(&original, &[6, 7]));
        assert_eq!(
            run(&residual.checked, &[]),
            alloc::vec![crate::sem::Value::I64(48)]
        );
    }

    #[test]
    fn empty_output_program_is_emitted_and_re_admitted() {
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-empty-output"),
        );
        let _input = builder.input(crate::ir::ScalarType::I64);
        let original = checked(builder.finish());
        assert!(original.graph().outputs.is_empty());

        let residual = with_static_map(&original, &[None]).expect("specialized");
        let source = emit_source(&residual.checked).expect("residual source");
        assert_eq!(source, "let v0\n");
        let readmitted = readmit(
            &TestIds,
            &source,
            &[noble_kernel::types::Ty::I64],
            noble_contracts::Limits::default(),
            &crate::check::Budgets::default(),
        )
        .expect("ordinary Declared-Text-v1 re-admission");
        assert!(readmitted.graph().outputs.is_empty());
        assert_eq!(run(&readmitted, &[17]), alloc::vec::Vec::new());

        let closed = with_static_map(&original, &[Some(17)]).expect("all-static residual");
        let closed_source = emit_source(&closed.checked).expect("zero-effect residual source");
        assert_eq!(closed_source, "0 drop\n");
        let closed_readmitted = readmit(
            &TestIds,
            &closed_source,
            &[],
            noble_contracts::Limits::default(),
            &crate::check::Budgets::default(),
        )
        .expect("empty Program ordinary re-admission");
        assert!(closed_readmitted.graph().outputs.is_empty());
        assert_eq!(closed_readmitted.node_count(), 0);
        assert_eq!(run(&closed_readmitted, &[]), alloc::vec::Vec::new());
    }

    #[test]
    fn wraparound_folds_to_the_wrapping_value() {
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-wrap"),
        );
        let max = builder.i64_const(i64::MAX);
        let one = builder.i64_const(1);
        let sum = builder.op(crate::ir::Op::I64Add(max, one));
        builder.output(sum);
        let original = checked(builder.finish());
        let residual = with_static_map(&original, &[]).expect("specialized");
        assert_eq!(
            run(&residual.checked, &[]),
            alloc::vec![crate::sem::Value::I64(i64::MIN)]
        );
        assert_eq!(run(&residual.checked, &[]), run(&original, &[]));
    }

    #[test]
    fn minimum_times_negative_one_folds_with_wrapping_semantics() {
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-wrap-mul"),
        );
        let minimum = builder.i64_const(i64::MIN);
        let negative_one = builder.i64_const(-1);
        let product = builder.op(crate::ir::Op::I64Mul(minimum, negative_one));
        builder.output(product);
        let original = checked(builder.finish());
        let residual = with_static_map(&original, &[]).expect("specialized");
        assert_eq!(
            run(&residual.checked, &[]),
            alloc::vec![crate::sem::Value::I64(i64::MIN)]
        );
        assert_eq!(run(&residual.checked, &[]), run(&original, &[]));
    }

    #[test]
    fn equality_folds_to_a_bool_constant() {
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-eq"),
        );
        let x = builder.input(crate::ir::ScalarType::I64);
        let three = builder.i64_const(3);
        let same = builder.op(crate::ir::Op::I64Eq(x, three));
        builder.output(same);
        let original = checked(builder.finish());
        let residual = with_static_map(&original, &[Some(3)]).expect("specialized");
        assert_eq!(residual.report.folded_nodes, 2);
        assert_eq!(
            run(&residual.checked, &[]),
            alloc::vec![crate::sem::Value::Bool(true)]
        );
        assert_eq!(run(&residual.checked, &[]), run(&original, &[3]));
        let unequal = with_static_map(&original, &[Some(4)]).expect("specialized");
        assert_eq!(
            run(&unequal.checked, &[]),
            alloc::vec![crate::sem::Value::Bool(false)]
        );
    }

    #[test]
    fn rust_model_fixture_eq_prune_renumber() {
        // Shared with `SpecializeResidual.rust_fixture_{equal,unequal}_shape`:
        // %0=input 0; %1=input 1; %2=const 3; %3=eq %0 %2; %4=add %1 %1;
        // outputs=[%3,%4], static map=[3,None]. The residual keeps the dynamic
        // input and both outputs, drops %0/%2, folds %3, and renumbers %4's
        // operands from %1 to %0.
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-rust-model-eq-prune-v1"),
        );
        let static_input = builder.input(crate::ir::ScalarType::I64);
        let dynamic_input = builder.input(crate::ir::ScalarType::I64);
        let three = builder.i64_const(3);
        let equality = builder.op(crate::ir::Op::I64Eq(static_input, three));
        let doubled = builder.op(crate::ir::Op::I64Add(dynamic_input, dynamic_input));
        builder.output(equality);
        builder.output(doubled);
        let original = checked(builder.finish());

        for (static_value, expected_bool) in [(3, true), (4, false)] {
            let residual = with_static_map(&original, &[Some(static_value), None])
                .expect("the shared equality/pruning fixture specializes");
            assert_eq!(residual.report.folded_nodes, 2);
            assert_eq!(residual.report.pruned_nodes, 2);
            assert_eq!(residual.report.residual_nodes, 3);
            assert_eq!(residual.report.dynamic_inputs, [crate::ir::ScalarType::I64]);
            assert_eq!(
                residual.checked.graph().outputs,
                [crate::ir::NodeId(1), crate::ir::NodeId(2),]
            );
            assert_eq!(
                residual.checked.graph().nodes[0].op,
                crate::ir::Op::Input(0),
                "the unused dynamic input remains defined"
            );
            assert_eq!(
                residual.checked.graph().nodes[1].op,
                crate::ir::Op::Const(crate::ir::Literal::Bool(expected_bool))
            );
            assert_eq!(
                residual.checked.graph().nodes[2].op,
                crate::ir::Op::I64Add(crate::ir::NodeId(0), crate::ir::NodeId(0))
            );
            for dynamic in [0, 1, -9, i64::MAX, i64::MIN] {
                assert_eq!(
                    run(&residual.checked, &[dynamic]),
                    alloc::vec![
                        crate::sem::Value::Bool(expected_bool),
                        crate::sem::Value::I64(dynamic.wrapping_add(dynamic)),
                    ],
                    "static={static_value}, dynamic={dynamic}"
                );
                assert_eq!(
                    run(&residual.checked, &[dynamic]),
                    run(&original, &[static_value, dynamic]),
                    "static={static_value}, dynamic={dynamic}"
                );
            }
        }
    }

    #[test]
    fn operand_order_survives_subtraction() {
        // `%0 = input 0; %1 = input 1; %2 = i64.sub %0 %1` keeps deeper-first.
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-sub"),
        );
        let x = builder.input(crate::ir::ScalarType::I64);
        let y = builder.input(crate::ir::ScalarType::I64);
        let difference = builder.op(crate::ir::Op::I64Sub(x, y));
        builder.output(difference);
        let original = checked(builder.finish());
        let residual = with_static_map(&original, &[Some(10), None]).expect("specialized");
        assert_eq!(residual.report.folded_nodes, 1);
        assert_eq!(run(&residual.checked, &[4]), run(&original, &[10, 4]));
        assert_eq!(
            run(&residual.checked, &[4]),
            alloc::vec![crate::sem::Value::I64(6)]
        );
    }

    #[test]
    fn mixed_expression_keeps_its_shape_and_sharing() {
        // `(x + 2) + 3` with `x` dynamic stays two additions: no reassociation.
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-offset"),
        );
        let x = builder.input(crate::ir::ScalarType::I64);
        let two = builder.i64_const(2);
        let inner = builder.op(crate::ir::Op::I64Add(x, two));
        let three = builder.i64_const(3);
        let outer = builder.op(crate::ir::Op::I64Add(inner, three));
        builder.output(outer);
        let original = checked(builder.finish());
        let residual = with_static_map(&original, &[None]).expect("specialized");
        assert_eq!(residual.report.folded_nodes, 0);
        assert_eq!(residual.report.residual_nodes, 5);
        assert_eq!(residual.report.pruned_nodes, 0);
        assert_eq!(run(&residual.checked, &[4]), run(&original, &[4]));
    }

    #[test]
    fn pruning_renumbers_the_operands_of_kept_nodes() {
        // `%0 = input 0; %1 = input 1; %2 = const 3; %3 = add %0 %2;
        // %4 = add %1 %3`. With `x = 2` static, `%3` folds to a constant,
        // making its input and the literal node unreachable. The kept `%4`
        // must be renumbered onto `%1` and `%3` and its operands rewritten.
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-prune"),
        );
        let x = builder.input(crate::ir::ScalarType::I64);
        let y = builder.input(crate::ir::ScalarType::I64);
        let three = builder.i64_const(3);
        let static_sum = builder.op(crate::ir::Op::I64Add(x, three));
        let output = builder.op(crate::ir::Op::I64Add(y, static_sum));
        builder.output(output);
        let original = checked(builder.finish());
        assert_eq!(
            run(&original, &[4, 3]),
            alloc::vec![crate::sem::Value::I64(10)]
        );
        let residual = with_static_map(&original, &[Some(2), None]).expect("specialized");
        assert_eq!(residual.report.pruned_nodes, 2);
        assert_eq!(residual.report.residual_nodes, 3);
        assert_eq!(
            residual.checked.graph().nodes[2].op,
            crate::ir::Op::I64Add(crate::ir::NodeId(0), crate::ir::NodeId(1)),
            "the kept addition is renumbered onto its kept operands"
        );
        assert!(residual.checked.graph().nodes.len() < original.graph().nodes.len());
        for y in [0i64, 3, -2] {
            assert_eq!(
                run(&residual.checked, &[y]),
                run(&original, &[2, y]),
                "pruned y = {y}"
            );
        }
        // The emitted source only names the kept nodes, so it re-admits.
        let source = emit_source(&residual.checked).expect("emitted");
        assert!(!source.contains("v3"), "{source}");
        assert!(!source.contains("v4"), "{source}");
        let readmitted = readmit(
            &TestIds,
            &source,
            &interface_types(&residual.checked).expect("types"),
            noble_contracts::Limits::default(),
            &crate::check::Budgets::default(),
        )
        .expect("re-admitted");
        for y in [0i64, 3] {
            assert_eq!(run(&readmitted, &[y]), run(&residual.checked, &[y]));
        }
    }

    #[test]
    fn residual_identity_is_new_and_the_closure_is_kept() {
        let original = product_plus();
        let residual = with_static_map(&original, &[Some(5), None]).expect("specialized");
        assert_ne!(
            residual.report.residual_graph,
            residual.report.original_graph
        );
        assert_eq!(residual.report.original_graph, original.id());
        assert_eq!(residual.checked.id(), residual.report.residual_graph);
        assert_eq!(
            residual.checked.graph().subject.closure,
            original.graph().subject.closure
        );
        assert_ne!(
            residual.checked.graph().subject.identity,
            original.graph().subject.identity
        );
        assert_eq!(residual.report.accounting, ACCOUNTING_REVISION);
        assert_eq!(residual.report.variants, 1);
        // A different static map is a different residual identity (D11).
        let other = with_static_map(&original, &[Some(6), None]).expect("specialized");
        assert_ne!(other.report.residual_graph, residual.report.residual_graph);
    }

    #[test]
    fn pair_input_shape_is_refused_before_specialization() {
        let pair =
            crate::ir::ShapeTree::pair(crate::ir::ShapeTree::SCALAR, crate::ir::ShapeTree::SCALAR)
                .expect("bounded pair shape");
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-pair-shape"),
        );
        let first = builder.input(crate::ir::ScalarType::I64);
        let _second = builder.input(crate::ir::ScalarType::I64);
        builder.output(first);
        builder.shapes(Some(alloc::vec![pair]), None);
        let original = checked(builder.finish());
        let refusal = with_static_map(&original, &[None, None]).expect_err("pair is excluded");
        assert_eq!(refusal.code, crate::diag::Code::Aggregate, "{refusal}");
    }

    #[test]
    fn unit_output_shape_is_refused_before_specialization() {
        let unit_pair =
            crate::ir::ShapeTree::pair(crate::ir::ShapeTree::UNIT, crate::ir::ShapeTree::SCALAR)
                .expect("bounded Unit pair shape");
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-unit-shape"),
        );
        let input = builder.input(crate::ir::ScalarType::I64);
        builder.output(input);
        builder.shapes(None, Some(alloc::vec![unit_pair]));
        let original = checked(builder.finish());
        let refusal = with_static_map(&original, &[None]).expect_err("Unit is excluded");
        assert_eq!(refusal.code, crate::diag::Code::Aggregate, "{refusal}");
    }

    #[test]
    fn repeated_outputs_survive_residualization_and_readmission() {
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-repeated-output"),
        );
        let input = builder.input(crate::ir::ScalarType::I64);
        builder.output(input);
        builder.output(input);
        let original = checked(builder.finish());
        let residual = with_static_map(&original, &[None]).expect("specialized");
        assert_eq!(
            residual.checked.graph().outputs,
            alloc::vec![crate::ir::NodeId(0), crate::ir::NodeId(0)]
        );
        assert_eq!(
            run(&residual.checked, &[7]),
            alloc::vec![crate::sem::Value::I64(7), crate::sem::Value::I64(7)]
        );
        let source = emit_source(&residual.checked).expect("emitted");
        assert_eq!(source, "let v0\nv0\nv0\n");
        let readmitted = readmit(
            &TestIds,
            &source,
            &interface_types(&residual.checked).expect("interface types"),
            noble_contracts::Limits::default(),
            &crate::check::Budgets::default(),
        )
        .expect("re-admitted");
        assert_eq!(run(&readmitted, &[7]), run(&original, &[7]));
    }

    #[test]
    fn wrong_arity_is_refused_with_interface() {
        let original = product_plus();
        for map in [&[Some(1)][..], &[Some(1), None, None][..]] {
            let refusal = with_static_map(&original, map).expect_err("refused");
            assert_eq!(refusal.code, crate::diag::Code::Interface, "{refusal}");
            assert_eq!(refusal.site, crate::diag::Site::Graph);
        }
    }

    #[test]
    fn caps_are_charged_and_a_raised_cap_is_a_profile_refusal() {
        let original = product_plus();
        let default = crate::check::Budgets::default();
        let below = crate::check::Budgets {
            residual_nodes: 3,
            ..default.clone()
        };
        let refusal = specialize(
            &TestIds,
            &original,
            &Request {
                static_inputs: &[Some(5), None],
                budgets: &below,
            },
        )
        .expect_err("the fourth created node exceeds the cap");
        assert_eq!(
            refusal.code,
            crate::diag::Code::Budget(crate::diag::BudgetKind::ResidualNodes)
        );
        assert_eq!(refusal.budget, Some((3, 4)));
        let at = crate::check::Budgets {
            residual_nodes: 4,
            ..default.clone()
        };
        let exact = specialize(
            &TestIds,
            &original,
            &Request {
                static_inputs: &[Some(5), None],
                budgets: &at,
            },
        )
        .expect("the cap is inclusive");
        assert_eq!(exact.report.residual_nodes, 4);
        assert_eq!(exact.report.variants, 1);
        let above = crate::check::Budgets {
            residual_nodes: 5,
            ..default.clone()
        };
        let above_residual = specialize(
            &TestIds,
            &original,
            &Request {
                static_inputs: &[Some(5), None],
                budgets: &above,
            },
        )
        .expect("one node above the request cap is still within profile policy");
        assert_eq!(above_residual.report.residual_nodes, 4);
        let zero_variants = crate::check::Budgets {
            variants: 0,
            ..default.clone()
        };
        let refusal = specialize(
            &TestIds,
            &original,
            &Request {
                static_inputs: &[Some(5), None],
                budgets: &zero_variants,
            },
        )
        .expect_err("a zero variant cap refuses the one variant");
        assert_eq!(
            refusal.code,
            crate::diag::Code::Budget(crate::diag::BudgetKind::Variants)
        );
        assert_eq!(refusal.budget, Some((0, 1)));
        let raised_variants = crate::check::Budgets {
            variants: 2,
            ..default.clone()
        };
        let refusal = specialize(
            &TestIds,
            &original,
            &Request {
                static_inputs: &[Some(5), None],
                budgets: &raised_variants,
            },
        )
        .expect_err("raising the variant cap is a profile refusal");
        assert_eq!(refusal.code, crate::diag::Code::Profile);
        let raised = crate::check::Budgets {
            residual_nodes: default.residual_nodes + 1,
            ..default.clone()
        };
        let refusal = specialize(
            &TestIds,
            &original,
            &Request {
                static_inputs: &[Some(5), None],
                budgets: &raised,
            },
        )
        .expect_err("a raised cap is a profile refusal");
        assert_eq!(refusal.code, crate::diag::Code::Profile);
    }

    #[test]
    fn a_fact_or_guard_graph_is_refused_before_any_node() {
        // An `I64` input-validated range fact is admissible for the profile and
        // outside the `ngraph-pe/0` slice: specialization changes the interface
        // the fact speaks about. A guard needs a checked fact, so this control
        // covers both refusals' shared cause.
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-fact"),
        );
        let x = builder.input(crate::ir::ScalarType::I64);
        let one = builder.i64_const(1);
        let sum = builder.op(crate::ir::Op::I64Add(x, one));
        builder.fact(
            x,
            crate::ir::Fact::Range {
                lo: crate::ir::Literal::I64(0),
                hi: crate::ir::Literal::I64(10),
            },
            crate::ir::Provenance::InputValidated,
        );
        builder.output(sum);
        let graph = checked(builder.finish());
        let refusal = with_static_map(&graph, &[None]).expect_err("refused");
        assert_eq!(refusal.code, crate::diag::Code::Profile, "{refusal}");
        assert!(refusal.detail.contains("fact record"), "{refusal}");
    }

    #[test]
    fn emitting_and_readmitting_a_residual_agrees_with_the_residual() {
        let original = product_plus();
        let residual = with_static_map(&original, &[Some(5), None]).expect("specialized");
        let source = emit_source(&residual.checked).expect("emitted");
        assert!(source.contains("let v0"), "{source}");
        assert!(source.contains("5 let"), "{source}");
        let types = interface_types(&residual.checked).expect("types");
        assert_eq!(types.len(), 1);
        let budgets = crate::check::Budgets::default();
        let readmitted = readmit(
            &TestIds,
            &source,
            &types,
            noble_contracts::Limits::default(),
            &budgets,
        )
        .expect("re-admitted");
        for y in [0i64, 4, -3] {
            assert_eq!(
                run(&readmitted, &[y]),
                run(&residual.checked, &[y]),
                "re-admitted y = {y}"
            );
            assert_eq!(run(&readmitted, &[y]), run(&original, &[5, y]));
        }
    }

    #[test]
    fn readmit_uses_the_callers_frontend_limits() {
        let source = "7 let v0\nv0\n";
        let limits = noble_contracts::Limits {
            bytes: 1,
            ..noble_contracts::Limits::default()
        };
        let refusal = readmit(
            &TestIds,
            source,
            &[],
            limits,
            &crate::check::Budgets::default(),
        )
        .expect_err("the caller's one-byte source limit rejects the residual source");
        assert_eq!(refusal.code, crate::diag::Code::Subject, "{refusal}");
    }

    #[test]
    fn merge_reports_its_arity_and_rejects_a_short_vector() {
        assert_eq!(
            merge(
                &[Some(1), None, None],
                &[crate::sem::Value::I64(7), crate::sem::Value::I64(8)]
            ),
            Some(crate::eval::i64_inputs(&[1, 7, 8]))
        );
        assert_eq!(merge(&[Some(1), None], &[]), None);
        assert_eq!(
            merge(
                &[None],
                &[crate::sem::Value::I64(1), crate::sem::Value::I64(2)]
            ),
            None
        );
        assert_eq!(rank(&[None, Some(2)], 1), Some(1));
        assert_eq!(rank(&[Some(2), None], 1), Some(0));
        assert_eq!(rank(&[None, Some(2)], 2), Some(1));
    }

    #[test]
    fn bool_dynamic_input_is_preserved_and_type_checked() {
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&TestIds, "pe-bool-input"),
        );
        let bool_input = builder.input(crate::ir::ScalarType::Bool);
        let i64_input = builder.input(crate::ir::ScalarType::I64);
        builder.output(bool_input);
        builder.output(i64_input);
        let original = checked(builder.finish());
        let residual = with_static_map(&original, &[None, None]).expect("specialized");
        let dynamic = [crate::sem::Value::Bool(true), crate::sem::Value::I64(9)];
        assert_eq!(residual.merge(&dynamic), Some(dynamic.to_vec()));
        assert_eq!(residual.merge_i64(&[9]), None);
        assert_eq!(
            residual.merge(&[crate::sem::Value::I64(1), crate::sem::Value::I64(9)]),
            None,
            "a dynamic value with the wrong slot type is not merged"
        );
        let source = emit_source(&residual.checked).expect("emitted");
        assert_eq!(source, "let v1\nlet v0\nv0\nv1\n");
        let types = interface_types(&residual.checked).expect("interface types");
        assert_eq!(
            types,
            alloc::vec![noble_kernel::types::Ty::Bool, noble_kernel::types::Ty::I64]
        );
        let readmitted = readmit(
            &TestIds,
            &source,
            &types,
            noble_contracts::Limits::default(),
            &crate::check::Budgets::default(),
        )
        .expect("re-admitted");
        let observe = |checked: &crate::check::CheckedGraph| match crate::eval::reference(
            checked,
            &dynamic,
            None,
            &mut crate::eval::Control::default(),
        )
        .obs
        {
            crate::eval::Obs::Normal(values) => values,
            other => panic!("unexpected observation {other:?}"),
        };
        assert_eq!(observe(&residual.checked), dynamic.to_vec());
        assert_eq!(observe(&readmitted), dynamic.to_vec());
        let refusal = with_static_map(&original, &[Some(1), None]).expect_err("Bool is not static");
        assert_eq!(refusal.code, crate::diag::Code::Interface);
        assert_eq!(refusal.site, crate::diag::Site::Slot(0));
    }
}
