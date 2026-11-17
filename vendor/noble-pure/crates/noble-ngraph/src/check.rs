#![expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-maintainers; the module path names the domain and the item name keeps its canonical operation spelling"
)]

//! The independent graph checker (D7 step 5).
//!
//! It re-derives node types from the D2 whitelist signatures and checks
//! topology (unknown nodes, forward references, cycles, dangling nodes),
//! interface slots, D3 payload rules (finite exact constants and
//! coefficients), D9 fact provenance (re-deriving `derived` claims), the facts
//! required by `step`/`select`/`index`, and the D5 post-normalization budgets.
//! It trusts nothing the producer recorded (node origins, counts, fact rules).
//!
//! The checker is ordinary Rust; its soundness is the open obligation NG-PO-B.

use crate::identity::IdentityScheme;
/// D5 budgets with the `/0` policy defaults, plus the two bounded
/// partial-evaluation caps of `ngraph-pe/0` (`residual_nodes`, `variants`). A
/// policy may lower a budget; raising one requires a new profile policy
/// revision (refused, `E-PROFILE`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Budgets {
    /// Extraction work units (symbolic steps, including repeated expansions).
    pub work_units: u64,
    /// Expanded nodes, before pruning, CSE or fusion.
    pub expanded_nodes: u64,
    /// Inline depth (named call, `run`, `dip` nesting).
    pub inline_depth: u64,
    /// Live nodes after normalization.
    pub live_nodes: u64,
    /// Layers (critical path).
    pub layers: u64,
    /// Inputs and params.
    pub inputs: u64,
    /// Outputs.
    pub outputs: u64,
    /// Terms per `affine`.
    pub affine_terms: u64,
    /// Index table length.
    #[expect(
        tigerstyle::numeric_units,
        reason = "Owner: noble-ngraph-maintainers; this is an abstract graph-budget row count in the profile's canonical entry units, not a time or byte quantity."
    )]
    pub table_length_entries: u64,
    /// Total index table entries.
    pub table_entries: u64,
    /// Facts.
    pub facts: u64,
    /// Retained graph bytes.
    pub graph_bytes: u64,
    /// Snapshot bytes.
    pub snapshot_bytes: u64,
    /// Forward scratch per evaluation (bytes).
    pub forward_scratch: u64,
    /// Backward scratch per evaluation (bytes).
    pub backward_scratch: u64,
    /// Optimizer steps per host call.
    pub optimizer_steps: u64,
    /// Residual nodes one static-input specialization may create, before
    /// pruning (`ngraph-pe/0`; issue #32). Charged by
    /// [`crate::specialize`], not by the checker.
    pub residual_nodes: u64,
    /// Residual variants one specialization request may produce. The v1
    /// entry point supports exactly one, so the `/0` default is 1.
    pub variants: u64,
}

impl Default for Budgets {
    fn default() -> Self {
        Self {
            work_units: 4_194_304,
            expanded_nodes: 262_144,
            inline_depth: 64,
            live_nodes: 262_144,
            layers: 16_384,
            inputs: 65_536,
            outputs: 65_536,
            affine_terms: 4_096,
            table_length_entries: 65_536,
            table_entries: 1_048_576,
            facts: 262_144,
            graph_bytes: 32 * 1024 * 1024,
            snapshot_bytes: 4 * 1024 * 1024,
            forward_scratch: 64 * 1024 * 1024,
            backward_scratch: 128 * 1024 * 1024,
            optimizer_steps: 1_000_000,
            residual_nodes: 262_144,
            variants: 1,
        }
    }
}

impl Budgets {
    fn pairs(&self) -> [u64; 18] {
        [
            self.work_units,
            self.expanded_nodes,
            self.inline_depth,
            self.live_nodes,
            self.layers,
            self.inputs,
            self.outputs,
            self.affine_terms,
            self.table_length_entries,
            self.table_entries,
            self.facts,
            self.graph_bytes,
            self.snapshot_bytes,
            self.forward_scratch,
            self.backward_scratch,
            self.optimizer_steps,
            self.residual_nodes,
            self.variants,
        ]
    }

    /// Refuses a policy that raises any budget above the `/0` default.
    ///
    /// # Errors
    /// `E-PROFILE` if any budget exceeds its default.
    pub fn validate_policy(&self) -> Result<(), crate::diag::Diagnostic> {
        let d = Self::default();
        if self.pairs().iter().zip(d.pairs()).all(|(a, b)| *a <= b) {
            Ok(())
        } else {
            Err(crate::diag::Diagnostic::new(
                crate::diag::Code::Profile,
                crate::diag::Site::Graph,
                "raising a /0 budget requires a new profile policy revision",
            ))
        }
    }
}

/// Charges `observed` against `limit`.
///
/// # Errors
/// `E-BUDGET-<kind>` with the limit and observed count.
#[expect(
    tigerstyle::numeric_units,
    reason = "Owner: noble-ngraph-maintainers; the limit is a count in the selected budget kind's abstract units, named by the diagnostic rather than by a time or byte suffix."
)]
pub const fn charge(
    kind: crate::diag::BudgetKind,
    limit_units: u64,
    observed: u64,
) -> Result<(), crate::diag::BudgetKind> {
    if observed > limit_units {
        Err(kind)
    } else {
        Ok(())
    }
}

#[expect(
    tigerstyle::numeric_units,
    reason = "Owner: noble-ngraph-maintainers; the limit is a count in the selected budget kind's abstract units, named by the diagnostic rather than by a time or byte suffix."
)]
fn budget(
    kind: crate::diag::BudgetKind,
    limit_units: u64,
    observed: u64,
) -> Result<(), crate::diag::Diagnostic> {
    charge(kind, limit_units, observed)
        .map_err(|k| crate::diag::Diagnostic::budget(k, limit_units, observed))
}

/// An input/param fact validated before evaluation (`E-FACT-INPUT`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputFact {
    /// crate::ir::Fact record index in the graph.
    pub record: usize,
    /// crate::ir::Slot node.
    pub node: crate::ir::NodeId,
    /// Interface slot.
    pub slot: u32,
    /// crate::ir::Slot kind.
    pub kind: crate::ir::SlotKind,
    /// crate::ir::Fact.
    pub fact: crate::ir::Fact,
}

/// A graph accepted by the checker, with re-derived types, layers and facts.
#[derive(Clone, Debug)]
pub struct CheckedGraph {
    graph: alloc::rc::Rc<crate::ir::Graph>,
    id: crate::ir::Digest,
    types: alloc::vec::Vec<Option<crate::ir::ScalarType>>,
    layer: alloc::vec::Vec<u32>,
    layers: u32,
    input_facts: alloc::vec::Vec<InputFact>,
    abs: alloc::vec::Vec<crate::facts::Abs>,
    slot_nodes: alloc::vec::Vec<crate::ir::NodeId>,
    param_slots: alloc::vec::Vec<u32>,
    param_layout: crate::ir::Digest,
    packed_layout: crate::ir::Digest,
}

impl CheckedGraph {
    /// The checked graph.
    #[must_use]
    pub fn graph(&self) -> &crate::ir::Graph {
        &self.graph
    }

    /// Shared owner of the checked graph (retained, immutable; D12).
    #[must_use]
    pub fn graph_rc(&self) -> alloc::rc::Rc<crate::ir::Graph> {
        alloc::rc::Rc::clone(&self.graph)
    }

    /// `GraphLogicalId`.
    #[must_use]
    pub const fn id(&self) -> crate::ir::Digest {
        self.id
    }

    /// Re-derived type of node `n` (`None` for guard roots).
    #[must_use]
    pub fn ty(&self, n: crate::ir::NodeId) -> Option<crate::ir::ScalarType> {
        self.types[n.index()]
    }

    /// Dependency layer of node `n` (handwired depth rule: sources at 0).
    #[must_use]
    pub fn layer(&self, n: crate::ir::NodeId) -> u32 {
        self.layer[n.index()]
    }

    /// Number of layers (critical path length + 1).
    #[must_use]
    pub const fn layer_count(&self) -> u32 {
        self.layers
    }

    /// Input/param facts to validate before evaluation.
    #[must_use]
    pub fn input_facts(&self) -> &[InputFact] {
        &self.input_facts
    }

    /// The checker's abstract summary of node `n` at its definition.
    #[must_use]
    pub fn summary(&self, n: crate::ir::NodeId) -> crate::facts::Abs {
        self.abs[n.index()]
    }

    /// Node of interface slot `k`.
    #[must_use]
    pub fn slot_node(&self, k: u32) -> crate::ir::NodeId {
        self.slot_nodes[k as usize]
    }

    /// Ordered param slots (computed once at check).
    #[must_use]
    pub fn param_slots(&self) -> &[u32] {
        &self.param_slots
    }

    /// Param layout key (computed once at check).
    #[must_use]
    pub const fn param_layout(&self) -> crate::ir::Digest {
        self.param_layout
    }

    /// The build key every packed layout of this graph must declare
    /// (`crate::layout::packed_layout_key`, computed once at check).
    #[must_use]
    pub const fn packed_layout_key(&self) -> crate::ir::Digest {
        self.packed_layout
    }

    /// crate::ir::Fact-binding digest: `H(noble.ngraph.fact-bindings.v0 ‖ GraphLogicalId ‖
    /// fact-rule revision ‖ fact records)`. crate::ir::Fact records are not part of
    /// `GraphLogicalId`, so snapshot binding also compares this (design D6).
    #[must_use]
    pub fn fact_bindings(&self, ids: &dyn IdentityScheme) -> crate::ir::Digest {
        let mut c = crate::ir::Canon::new(ids, "noble.ngraph.fact-bindings.v0");
        c.bytes(&self.id);
        c.str(crate::ir::FACT_RULE_REVISION);
        c.u64(self.graph.facts.len() as u64);
        for f in &self.graph.facts {
            c.u32(f.node.0);
            c.fact(&f.fact);
            c.str(match &f.provenance {
                crate::ir::Provenance::Derived(_) => "derived",
                crate::ir::Provenance::InputValidated => "input-validated",
                crate::ir::Provenance::Evidence(_) => "evidence",
                crate::ir::Provenance::Guard(_) => "guard",
                crate::ir::Provenance::Asserted => "asserted",
            });
        }
        c.finish()
    }

    /// Live node count.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.graph.nodes.len()
    }
}

/// Estimated retained bytes of a graph (charged against `graph_bytes`).
///
/// The estimate covers the `crate::ir::Graph` header plus every graph-owned heap
/// allocation: the interface slot and output vectors, the node and fact vectors
/// with their operation payloads and `origin.expansion` vectors, `one-hot`
/// groups and `derived`/`evidence` provenance payloads, the subject
/// closure, and the input/output shape vectors together with the boxed pair
/// children of every shape tree.
///
/// Element storage is charged at `alloc::vec::Vec::capacity`, so spare capacity retained
/// by a producer counts against the budget. The estimator is conservative and
/// never wraps: every sum and product saturates, so an unrepresentable total
/// is charged as `u64::MAX` and refused rather than admitted. Allocator
/// bookkeeping and the checker's own derived caches (`CheckedGraph` maps and
/// layers) are outside this contract.
#[must_use]
pub fn retained_bytes(g: &crate::ir::Graph) -> u64 {
    /// Element storage retained by one vector, at its current capacity.
    #[expect(
        tigerstyle::borrowed_argument_types,
        reason = "Owner: noble-maintainers; capacity is the measured property, so a slice cannot replace the owned vector"
    )]
    fn vec_bytes<T>(v: &alloc::vec::Vec<T>) -> u64 {
        (v.capacity() as u64).saturating_mul(core::mem::size_of::<T>() as u64)
    }
    /// Bytes retained by one shape tree: one allocation of two `crate::ir::ShapeTree`
    /// per pair node, counted recursively.
    #[expect(
        tigerstyle::no_recursion,
        reason = "Owner: noble-maintainers; the walk is bounded by the declared shape nesting limit"
    )]
    fn shape_pairs(t: &crate::ir::ShapeTree) -> u64 {
        match t.node() {
            crate::ir::ShapeNode::Pair(a, b) => 2u64
                .saturating_mul(core::mem::size_of::<crate::ir::ShapeTree>() as u64)
                .saturating_add(shape_pairs(a))
                .saturating_add(shape_pairs(b)),
            crate::ir::ShapeNode::Scalar | crate::ir::ShapeNode::Unit => 0,
        }
    }
    let mut b = core::mem::size_of::<crate::ir::Graph>() as u64;
    b = b.saturating_add(vec_bytes(&g.slots));
    b = b.saturating_add(vec_bytes(&g.outputs));
    b = b.saturating_add(vec_bytes(&g.nodes));
    b = b.saturating_add(vec_bytes(&g.facts));
    for n in &g.nodes {
        b = b.saturating_add(match &n.op {
            crate::ir::Op::F32Affine { terms, .. } => vec_bytes(terms),
            crate::ir::Op::F32Index { table, .. } => vec_bytes(table),
            crate::ir::Op::Guard {
                fact: crate::ir::Fact::OneHot { group, .. },
                ..
            } => vec_bytes(group),
            _ => 0,
        });
        if let Some(origin) = &n.origin {
            b = b.saturating_add(vec_bytes(&origin.expansion));
        }
    }
    for f in &g.facts {
        b = b.saturating_add(match &f.fact {
            crate::ir::Fact::OneHot { group, .. } => vec_bytes(group),
            _ => 0,
        });
        b = b.saturating_add(match &f.provenance {
            crate::ir::Provenance::Derived(rule) => rule.capacity() as u64,
            crate::ir::Provenance::Evidence(bytes) => vec_bytes(bytes),
            _ => 0,
        });
    }
    b = b.saturating_add(vec_bytes(&g.subject.closure));
    for shapes in [&g.input_shape, &g.output_shape].into_iter().flatten() {
        b = b.saturating_add(vec_bytes(shapes));
        for t in shapes {
            b = b.saturating_add(shape_pairs(t));
        }
    }
    b
}

fn op_profile(op: &crate::ir::Op) -> Option<crate::ir::SubProfile> {
    match op {
        crate::ir::Op::Input(_)
        | crate::ir::Op::Param(_)
        | crate::ir::Op::Const(_)
        | crate::ir::Op::Guard { .. } => None,
        crate::ir::Op::I64Add(..)
        | crate::ir::Op::I64Sub(..)
        | crate::ir::Op::I64Mul(..)
        | crate::ir::Op::I64Eq(..) => Some(crate::ir::SubProfile::I64),
        _ => Some(crate::ir::SubProfile::F32),
    }
}

fn check_authored_nickel_policy(g: &crate::ir::Graph) -> Result<(), crate::diag::Diagnostic> {
    if g.subject.kind != crate::ir::SubjectKind::AuthoredNickel {
        return Ok(());
    }
    if g.profile != crate::ir::SubProfile::I64 {
        return Err(crate::diag::Diagnostic::new(
            crate::diag::Code::Profile,
            crate::diag::Site::Graph,
            "authored-nickel admits only ngraph-i64/0",
        ));
    }
    if !g.subject.closure.is_empty() {
        return Err(crate::diag::Diagnostic::new(
            crate::diag::Code::Subject,
            crate::diag::Site::Graph,
            "authored-nickel subjects have no source closure",
        ));
    }
    if !g.facts.is_empty()
        || g.slots.iter().any(|slot| {
            slot.kind != crate::ir::SlotKind::Input || slot.ty != crate::ir::ScalarType::I64
        })
    {
        return Err(crate::diag::Diagnostic::new(
            crate::diag::Code::Profile,
            crate::diag::Site::Graph,
            "authored-nickel requires I64 inputs and has no params or facts",
        ));
    }
    for (index, node) in g.nodes.iter().enumerate() {
        if !matches!(
            node.op,
            crate::ir::Op::Input(_)
                | crate::ir::Op::Const(crate::ir::Literal::I64(_))
                | crate::ir::Op::I64Add(..)
                | crate::ir::Op::I64Sub(..)
                | crate::ir::Op::I64Mul(..)
        ) {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::Profile,
                crate::diag::Site::Node(crate::ir::NodeId(index as u32)),
                alloc::format!(
                    "{} is outside the authored-nickel I64 arithmetic profile",
                    node.op.name()
                ),
            ));
        }
    }
    Ok(())
}

/// Checks `graph` under the `/0` default budgets.
///
/// # Errors
/// The first D6 refusal in stage order.
pub fn check(
    ids: &dyn IdentityScheme,
    graph: crate::ir::Graph,
) -> Result<CheckedGraph, crate::diag::Diagnostic> {
    check_with(ids, graph, &Budgets::default())
}

/// Checks `graph` under `budgets`. Stage order: policy, interface/size
/// budgets, references and topology, slots, types and payloads, dangling
/// nodes, facts, layers, retained bytes.
///
/// # Errors
/// The first D6 refusal in stage order.
pub fn check_with(
    ids: &dyn IdentityScheme,
    graph: crate::ir::Graph,
    budgets: &Budgets,
) -> Result<CheckedGraph, crate::diag::Diagnostic> {
    budgets.validate_policy()?;
    check_authored_nickel_policy(&graph)?;
    let n = graph.nodes.len();
    budget(
        crate::diag::BudgetKind::LiveNodes,
        budgets.live_nodes,
        n as u64,
    )?;
    budget(
        crate::diag::BudgetKind::Inputs,
        budgets.inputs,
        graph.slots.len() as u64,
    )?;
    budget(
        crate::diag::BudgetKind::Outputs,
        budgets.outputs,
        graph.outputs.len() as u64,
    )?;
    budget(
        crate::diag::BudgetKind::Facts,
        budgets.facts,
        graph.facts.len() as u64,
    )?;
    let mut table_entries = 0u64;
    for node in &graph.nodes {
        match &node.op {
            crate::ir::Op::F32Affine { terms, .. } => {
                budget(
                    crate::diag::BudgetKind::AffineTerms,
                    budgets.affine_terms,
                    terms.len() as u64,
                )?;
            }
            crate::ir::Op::F32Index { table, .. } => {
                budget(
                    crate::diag::BudgetKind::TableLength,
                    budgets.table_length_entries,
                    table.len() as u64,
                )?;
                table_entries += table.len() as u64;
                budget(
                    crate::diag::BudgetKind::TableEntries,
                    budgets.table_entries,
                    table_entries,
                )?;
            }
            _ => {}
        }
    }
    check_topology(&graph)?;
    let slot_nodes = check_slots(&graph)?;
    let types = check_types(&graph)?;
    check_dangling(&graph)?;
    let (abs, input_facts) = check_facts(&graph, &types)?;
    let (layer, layers) = layering(&graph);
    budget(
        crate::diag::BudgetKind::Layers,
        budgets.layers,
        u64::from(layers),
    )?;
    budget(
        crate::diag::BudgetKind::GraphBytes,
        budgets.graph_bytes,
        retained_bytes(&graph),
    )?;
    let id = graph.logical_id(ids);
    let param_slots = graph.param_slots();
    let param_layout = crate::snapshot::layout_key(ids, &graph);
    let packed_layout = crate::layout::packed_layout_key(ids, &id);
    Ok(CheckedGraph {
        graph: alloc::rc::Rc::new(graph),
        id,
        types,
        layer,
        layers,
        input_facts,
        abs,
        slot_nodes,
        param_slots,
        param_layout,
        packed_layout,
    })
}

fn check_topology(g: &crate::ir::Graph) -> Result<(), crate::diag::Diagnostic> {
    let n = g.nodes.len();
    let mut forward = None;
    for (i, node) in g.nodes.iter().enumerate() {
        let site = crate::diag::Site::Node(crate::ir::NodeId(i as u32));
        for o in node.op.operands() {
            if o.index() >= n {
                return Err(crate::diag::Diagnostic::malformed(
                    crate::diag::Malformed::UnknownNode,
                    site,
                    alloc::format!("operand %{} does not exist", o.0),
                ));
            }
            if o.index() >= i && forward.is_none() {
                forward = Some((i, o));
            }
            if g.nodes[o.index()].op.is_guard() {
                return Err(crate::diag::Diagnostic::malformed(
                    crate::diag::Malformed::GuardAsValue,
                    site,
                    "a guard root is not a value",
                ));
            }
        }
    }
    for (k, o) in g.outputs.iter().enumerate() {
        if o.index() >= n {
            return Err(crate::diag::Diagnostic::malformed(
                crate::diag::Malformed::UnknownNode,
                crate::diag::Site::Graph,
                alloc::format!("output {k} names missing node %{}", o.0),
            ));
        }
        if g.nodes[o.index()].op.is_guard() {
            return Err(crate::diag::Diagnostic::malformed(
                crate::diag::Malformed::GuardAsValue,
                crate::diag::Site::Node(*o),
                "a guard root is not an output",
            ));
        }
    }
    for (k, f) in g.facts.iter().enumerate() {
        let mut refs = alloc::vec![f.node];
        if let crate::ir::Fact::OneHot { group, .. } = &f.fact {
            refs.extend_from_slice(group);
        }
        if let crate::ir::Provenance::Guard(gd) = &f.provenance {
            refs.push(*gd);
        }
        if refs.iter().any(|r| r.index() >= n) {
            return Err(crate::diag::Diagnostic::malformed(
                crate::diag::Malformed::UnknownNode,
                crate::diag::Site::Fact(k),
                "fact names a missing node",
            ));
        }
    }
    if let Some((at, operand)) = forward {
        // Distinguish a genuine cycle from an acyclic forward reference.
        if let Some(c) = find_cycle(g) {
            return Err(crate::diag::Diagnostic::malformed(
                crate::diag::Malformed::Cycle,
                crate::diag::Site::Node(c),
                "dependency cycle",
            ));
        }
        return Err(crate::diag::Diagnostic::malformed(
            crate::diag::Malformed::ForwardReference,
            crate::diag::Site::Node(crate::ir::NodeId(at as u32)),
            alloc::format!("operand %{} is defined later", operand.0),
        ));
    }
    Ok(())
}

fn find_cycle(g: &crate::ir::Graph) -> Option<crate::ir::NodeId> {
    // Iterative three-color DFS.
    let n = g.nodes.len();
    let mut color = alloc::vec![0u8; n];
    let ops: alloc::vec::Vec<alloc::vec::Vec<crate::ir::NodeId>> =
        g.nodes.iter().map(|x| x.op.operands()).collect();
    for start in 0..n {
        if color[start] != 0 {
            continue;
        }
        let mut stack = alloc::vec![(start, 0usize)];
        color[start] = 1;
        while let Some((v, next)) = stack.last_mut() {
            let v = *v;
            if let Some(&w) = ops[v].get(*next) {
                *next += 1;
                match color[w.index()] {
                    0 => {
                        color[w.index()] = 1;
                        stack.push((w.index(), 0));
                    }
                    1 => return Some(w),
                    _ => {}
                }
            } else {
                color[v] = 2;
                stack.pop();
            }
        }
    }
    None
}

fn check_slots(
    g: &crate::ir::Graph,
) -> Result<alloc::vec::Vec<crate::ir::NodeId>, crate::diag::Diagnostic> {
    let mut seen: alloc::vec::Vec<Option<crate::ir::NodeId>> = alloc::vec![None; g.slots.len()];
    for (i, node) in g.nodes.iter().enumerate() {
        let id = crate::ir::NodeId(i as u32);
        let (k, kind) = match node.op {
            crate::ir::Op::Input(k) => (k, crate::ir::SlotKind::Input),
            crate::ir::Op::Param(k) => (k, crate::ir::SlotKind::Param),
            _ => continue,
        };
        let Some(slot) = g.slots.get(k as usize) else {
            return Err(crate::diag::Diagnostic::malformed(
                crate::diag::Malformed::SlotMismatch,
                crate::diag::Site::Node(id),
                alloc::format!("slot {k} is not declared"),
            ));
        };
        if slot.kind != kind {
            return Err(crate::diag::Diagnostic::malformed(
                crate::diag::Malformed::SlotMismatch,
                crate::diag::Site::Node(id),
                alloc::format!("slot {k} kind differs from its node"),
            ));
        }
        if let Some(prev) = seen[k as usize] {
            return Err(crate::diag::Diagnostic::malformed(
                crate::diag::Malformed::DuplicateSlot,
                crate::diag::Site::Node(id),
                alloc::format!("slot {k} already defined by %{}", prev.0),
            ));
        }
        seen[k as usize] = Some(id);
    }
    let mut out = alloc::vec::Vec::with_capacity(seen.len());
    for (k, (s, slot)) in seen.iter().zip(&g.slots).enumerate() {
        let Some(id) = s else {
            return Err(crate::diag::Diagnostic::malformed(
                crate::diag::Malformed::SlotMismatch,
                crate::diag::Site::Slot(k as u32),
                "slot has no node",
            ));
        };
        if !g.profile.admits(slot.ty) {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::TypeMix,
                crate::diag::Site::Slot(k as u32),
                alloc::format!("{} slot in {}", slot.ty.name(), g.profile.name()),
            ));
        }
        if slot.kind == crate::ir::SlotKind::Param && g.profile != crate::ir::SubProfile::F32 {
            return Err(crate::diag::Diagnostic::malformed(
                crate::diag::Malformed::SlotMismatch,
                crate::diag::Site::Slot(k as u32),
                "param slots exist only in ngraph-f32/0",
            ));
        }
        out.push(*id);
    }
    Ok(out)
}

fn finite_bits(bits: u32) -> bool {
    f32::from_bits(bits).is_finite()
}

fn check_types(
    g: &crate::ir::Graph,
) -> Result<alloc::vec::Vec<Option<crate::ir::ScalarType>>, crate::diag::Diagnostic> {
    let mut types: alloc::vec::Vec<Option<crate::ir::ScalarType>> =
        alloc::vec::Vec::with_capacity(g.nodes.len());
    for (i, node) in g.nodes.iter().enumerate() {
        let site = crate::diag::Site::Node(crate::ir::NodeId(i as u32));
        if let Some(p) = op_profile(&node.op) {
            if p != g.profile {
                return Err(crate::diag::Diagnostic::new(
                    crate::diag::Code::TypeMix,
                    site,
                    alloc::format!("{} in {}", node.op.name(), g.profile.name()),
                ));
            }
        }
        let want = |o: crate::ir::NodeId,
                    ty: crate::ir::ScalarType|
         -> Result<(), crate::diag::Diagnostic> {
            match types[o.index()] {
                Some(t) if t == ty => Ok(()),
                Some(t) if !g.profile.admits(t) || !g.profile.admits(ty) => {
                    Err(crate::diag::Diagnostic::new(
                        crate::diag::Code::TypeMix,
                        site.clone(),
                        alloc::format!("operand %{} is {}", o.0, t.name()),
                    ))
                }
                Some(_) | None => Err(crate::diag::Diagnostic::malformed(
                    crate::diag::Malformed::TypeError,
                    site.clone(),
                    alloc::format!("operand %{} is not {}", o.0, ty.name()),
                )),
            }
        };
        let ty = match &node.op {
            crate::ir::Op::Input(k) | crate::ir::Op::Param(k) => Some(g.slots[*k as usize].ty),
            crate::ir::Op::Const(l) => {
                if !g.profile.admits(l.ty()) {
                    return Err(crate::diag::Diagnostic::new(
                        crate::diag::Code::TypeMix,
                        site,
                        alloc::format!("{} literal in {}", l.ty().name(), g.profile.name()),
                    ));
                }
                if let crate::ir::Literal::F32(b) = l {
                    if !finite_bits(*b) {
                        return Err(crate::diag::Diagnostic::new(
                            crate::diag::Code::LiteralNonfinite,
                            site,
                            "non-finite f32 constant",
                        ));
                    }
                }
                Some(l.ty())
            }
            crate::ir::Op::I64Add(a, b)
            | crate::ir::Op::I64Sub(a, b)
            | crate::ir::Op::I64Mul(a, b) => {
                want(*a, crate::ir::ScalarType::I64)?;
                want(*b, crate::ir::ScalarType::I64)?;
                Some(crate::ir::ScalarType::I64)
            }
            crate::ir::Op::I64Eq(a, b) => {
                want(*a, crate::ir::ScalarType::I64)?;
                want(*b, crate::ir::ScalarType::I64)?;
                Some(crate::ir::ScalarType::Bool)
            }
            crate::ir::Op::F32Add(a, b)
            | crate::ir::Op::F32Sub(a, b)
            | crate::ir::Op::F32Mul(a, b) => {
                want(*a, crate::ir::ScalarType::F32)?;
                want(*b, crate::ir::ScalarType::F32)?;
                Some(crate::ir::ScalarType::F32)
            }
            crate::ir::Op::F32Affine { bias, terms } => {
                if !finite_bits(*bias) || terms.iter().any(|t| !finite_bits(t.coefficient)) {
                    return Err(crate::diag::Diagnostic::new(
                        crate::diag::Code::LiteralNonfinite,
                        site,
                        "non-finite affine bias or coefficient",
                    ));
                }
                for t in terms {
                    if let crate::ir::AffineOperand::Node(x) = t.operand {
                        want(x, crate::ir::ScalarType::F32)?;
                    }
                }
                Some(crate::ir::ScalarType::F32)
            }
            crate::ir::Op::F32Relu(x) | crate::ir::Op::F32Step(x) => {
                want(*x, crate::ir::ScalarType::F32)?;
                Some(crate::ir::ScalarType::F32)
            }
            crate::ir::Op::F32Reglu { left, gate } => {
                want(*left, crate::ir::ScalarType::F32)?;
                want(*gate, crate::ir::ScalarType::F32)?;
                Some(crate::ir::ScalarType::F32)
            }
            crate::ir::Op::F32Select { bit, off, on } => {
                for x in [bit, off, on] {
                    want(*x, crate::ir::ScalarType::F32)?;
                }
                Some(crate::ir::ScalarType::F32)
            }
            crate::ir::Op::F32Index { index, table, .. } => {
                if table.is_empty() {
                    return Err(crate::diag::Diagnostic::malformed(
                        crate::diag::Malformed::Payload,
                        site,
                        "index table is empty",
                    ));
                }
                want(*index, crate::ir::ScalarType::F32)?;
                for x in table {
                    want(*x, crate::ir::ScalarType::F32)?;
                }
                Some(crate::ir::ScalarType::F32)
            }
            crate::ir::Op::Guard { fact, value } => {
                let t = types[value.index()].unwrap_or(crate::ir::ScalarType::Bool);
                if !crate::facts::payload_ok(fact, t) {
                    return Err(crate::diag::Diagnostic::malformed(
                        crate::diag::Malformed::Payload,
                        site,
                        alloc::format!("{} fact payload for {}", fact.kind_name(), t.name()),
                    ));
                }
                if let crate::ir::Fact::OneHot { group, .. } = fact {
                    if group.first() != Some(value) {
                        return Err(crate::diag::Diagnostic::malformed(
                            crate::diag::Malformed::Payload,
                            site,
                            "one-hot fact must be attached to its first member",
                        ));
                    }
                    for m in group {
                        want(*m, t)?;
                    }
                }
                None
            }
        };
        types.push(ty);
    }
    Ok(types)
}

fn check_dangling(g: &crate::ir::Graph) -> Result<(), crate::diag::Diagnostic> {
    let n = g.nodes.len();
    let mut live = alloc::vec![false; n];
    let mut work: alloc::vec::Vec<crate::ir::NodeId> = g.outputs.clone();
    for (i, node) in g.nodes.iter().enumerate() {
        if node.op.is_guard() {
            work.push(crate::ir::NodeId(i as u32));
        }
    }
    while let Some(v) = work.pop() {
        if core::mem::replace(&mut live[v.index()], true) {
            continue;
        }
        work.extend(g.nodes[v.index()].op.operands());
    }
    for (i, node) in g.nodes.iter().enumerate() {
        if !live[i] && !matches!(node.op, crate::ir::Op::Input(_) | crate::ir::Op::Param(_)) {
            return Err(crate::diag::Diagnostic::malformed(
                crate::diag::Malformed::DanglingNode,
                crate::diag::Site::Node(crate::ir::NodeId(i as u32)),
                "node reaches no output and no guard",
            ));
        }
    }
    Ok(())
}

fn unchecked(site: crate::diag::Site, fact: crate::ir::Fact, why: &str) -> crate::diag::Diagnostic {
    crate::diag::Diagnostic::new(crate::diag::Code::FactUnchecked, site, why).with_fact(fact)
}

fn check_facts(
    g: &crate::ir::Graph,
    types: &[Option<crate::ir::ScalarType>],
) -> Result<
    (
        alloc::vec::Vec<crate::facts::Abs>,
        alloc::vec::Vec<InputFact>,
    ),
    crate::diag::Diagnostic,
> {
    // crate::ir::Provenance of each record (before any use).
    let mut input_facts = alloc::vec::Vec::with_capacity(g.nodes.len());
    let mut derived_claims: alloc::collections::BTreeMap<
        usize,
        alloc::vec::Vec<(usize, crate::ir::Fact)>,
    > = alloc::collections::BTreeMap::new();
    for (k, rec) in g.facts.iter().enumerate() {
        let site = crate::diag::Site::Fact(k);
        let ty = types[rec.node.index()];
        let Some(ty) = ty else {
            return Err(unchecked(site, rec.fact.clone(), "fact on a guard root"));
        };
        if !crate::facts::payload_ok(&rec.fact, ty) {
            return Err(crate::diag::Diagnostic::malformed(
                crate::diag::Malformed::Payload,
                site,
                "fact payload does not fit the value type",
            ));
        }
        match &rec.provenance {
            crate::ir::Provenance::Asserted => {
                return Err(unchecked(
                    site,
                    rec.fact.clone(),
                    "caller assertion without derived, input-validated or guard provenance",
                ));
            }
            crate::ir::Provenance::Evidence(_) => {
                return Err(unchecked(
                    site,
                    rec.fact.clone(),
                    "evidence facts are excluded from Phase B (approval P5)",
                ));
            }
            crate::ir::Provenance::InputValidated => {
                let (slot, kind) = match g.nodes[rec.node.index()].op {
                    crate::ir::Op::Input(s) => (s, crate::ir::SlotKind::Input),
                    crate::ir::Op::Param(s) => (s, crate::ir::SlotKind::Param),
                    _ => {
                        return Err(unchecked(
                            site,
                            rec.fact.clone(),
                            "input-validated provenance on a non-interface node",
                        ));
                    }
                };
                if let crate::ir::Fact::OneHot { group, .. } = &rec.fact {
                    if group.first() != Some(&rec.node)
                        || group.iter().any(|m| {
                            !matches!(
                                g.nodes[m.index()].op,
                                crate::ir::Op::Input(_) | crate::ir::Op::Param(_)
                            )
                        })
                    {
                        return Err(unchecked(
                            site,
                            rec.fact.clone(),
                            "input-validated one-hot group must be interface slots led by the record node",
                        ));
                    }
                }
                input_facts.push(InputFact {
                    record: k,
                    node: rec.node,
                    slot,
                    kind,
                    fact: rec.fact.clone(),
                });
            }
            crate::ir::Provenance::Guard(gd) => match &g.nodes[gd.index()].op {
                crate::ir::Op::Guard { fact, value } if *value == rec.node && *fact == rec.fact => {
                }
                _ => {
                    return Err(unchecked(
                        site,
                        rec.fact.clone(),
                        "guard provenance does not name a matching guard",
                    ));
                }
            },
            crate::ir::Provenance::Derived(_) => {
                derived_claims
                    .entry(rec.node.index())
                    .or_default()
                    .push((k, rec.fact.clone()));
            }
        }
    }

    // Abstract interpretation in canonical order. `cur` holds the summary of
    // each value as refined by input validation and earlier guards.
    let n = g.nodes.len();
    let mut cur: alloc::vec::Vec<crate::facts::Abs> = alloc::vec::Vec::with_capacity(n);
    let mut at_def: alloc::vec::Vec<crate::facts::Abs> = alloc::vec::Vec::with_capacity(n);
    let f = |cur: &[crate::facts::Abs], x: crate::ir::NodeId| -> crate::facts::AbsF32 {
        match cur[x.index()] {
            crate::facts::Abs::F32(a) => a,
            _ => crate::facts::AbsF32::UNKNOWN,
        }
    };
    let iv = |cur: &[crate::facts::Abs], x: crate::ir::NodeId| -> crate::facts::AbsI64 {
        match cur[x.index()] {
            crate::facts::Abs::I64(a) => a,
            _ => crate::facts::AbsI64::UNKNOWN,
        }
    };
    for (i, node) in g.nodes.iter().enumerate() {
        let id = crate::ir::NodeId(i as u32);
        let need = |cur: &[crate::facts::Abs],
                    x: crate::ir::NodeId,
                    r: crate::facts::Requirement|
         -> Result<(), crate::diag::Diagnostic> {
            if r.satisfied_by(&f(cur, x)) {
                Ok(())
            } else {
                Err(unchecked(
                    crate::diag::Site::Node(id),
                    r.fact(),
                    &alloc::format!(
                        "{} operand %{} lacks a checked {} fact",
                        node.op.name(),
                        x.0,
                        r.fact().kind_name()
                    ),
                ))
            }
        };
        let a = match &node.op {
            crate::ir::Op::Input(_) | crate::ir::Op::Param(_) => {
                let mut a = match types[i] {
                    Some(crate::ir::ScalarType::F32) => {
                        crate::facts::Abs::F32(crate::facts::AbsF32::UNKNOWN)
                    }
                    Some(crate::ir::ScalarType::I64) => {
                        crate::facts::Abs::I64(crate::facts::AbsI64::UNKNOWN)
                    }
                    Some(crate::ir::ScalarType::Bool) | None => crate::facts::Abs::Bool,
                };
                for inf in &input_facts {
                    if inf.node == id {
                        a = a.refine(&inf.fact);
                    }
                    if let crate::ir::Fact::OneHot { group, .. } = &inf.fact {
                        if group.contains(&id) {
                            a = a.refine(&inf.fact);
                        }
                    }
                }
                a
            }
            crate::ir::Op::Const(l) => match l {
                crate::ir::Literal::F32(b) => {
                    crate::facts::Abs::F32(crate::facts::AbsF32::exact(f32::from_bits(*b)))
                }
                crate::ir::Literal::I64(v) => {
                    crate::facts::Abs::I64(crate::facts::AbsI64 { lo: *v, hi: *v })
                }
                crate::ir::Literal::Bool(_) => crate::facts::Abs::Bool,
            },
            crate::ir::Op::I64Add(a, b) => {
                crate::facts::Abs::I64(crate::facts::AbsI64::radd(iv(&cur, *a), iv(&cur, *b)))
            }
            crate::ir::Op::I64Sub(a, b) => {
                crate::facts::Abs::I64(crate::facts::AbsI64::rsub(iv(&cur, *a), iv(&cur, *b)))
            }
            crate::ir::Op::I64Mul(a, b) => {
                crate::facts::Abs::I64(crate::facts::AbsI64::rmul(iv(&cur, *a), iv(&cur, *b)))
            }
            crate::ir::Op::I64Eq(..) => crate::facts::Abs::Bool,
            crate::ir::Op::F32Add(a, b) => {
                crate::facts::Abs::F32(crate::facts::AbsF32::radd(f(&cur, *a), f(&cur, *b)))
            }
            crate::ir::Op::F32Sub(a, b) => {
                crate::facts::Abs::F32(crate::facts::AbsF32::rsub(f(&cur, *a), f(&cur, *b)))
            }
            crate::ir::Op::F32Mul(a, b) => {
                crate::facts::Abs::F32(crate::facts::AbsF32::rmul(f(&cur, *a), f(&cur, *b)))
            }
            crate::ir::Op::F32Affine { bias, terms } => {
                let mut v = crate::facts::AbsF32::exact(f32::from_bits(*bias));
                for t in terms {
                    let x = match t.operand {
                        crate::ir::AffineOperand::One => crate::facts::AbsF32::exact(1.0),
                        crate::ir::AffineOperand::Node(x) => f(&cur, x),
                    };
                    let p = crate::facts::AbsF32::rmul(
                        crate::facts::AbsF32::exact(f32::from_bits(t.coefficient)),
                        x,
                    );
                    v = crate::facts::AbsF32::radd(v, p);
                }
                crate::facts::Abs::F32(v)
            }
            crate::ir::Op::F32Relu(x) => {
                crate::facts::Abs::F32(crate::facts::AbsF32::relu(f(&cur, *x)))
            }
            crate::ir::Op::F32Reglu { left, gate } => {
                crate::facts::Abs::F32(crate::facts::AbsF32::rmul(
                    f(&cur, *left),
                    crate::facts::AbsF32::relu(f(&cur, *gate)),
                ))
            }
            crate::ir::Op::F32Step(s) => {
                need(&cur, *s, crate::facts::Requirement::UnitRange)?;
                crate::facts::Abs::F32(crate::facts::AbsF32::step())
            }
            crate::ir::Op::F32Select { bit, off, on } => {
                need(&cur, *bit, crate::facts::Requirement::Bit)?;
                crate::facts::Abs::F32(crate::facts::AbsF32::hull(f(&cur, *off), f(&cur, *on)))
            }
            crate::ir::Op::F32Index {
                index,
                table,
                below,
                above,
            } => {
                need(&cur, *index, crate::facts::Requirement::Index)?;
                let mut h = f(&cur, table[0]);
                for t in &table[1..] {
                    h = crate::facts::AbsF32::hull(h, f(&cur, *t));
                }
                if *below == crate::ir::OutOfRange::Zero || *above == crate::ir::OutOfRange::Zero {
                    h = crate::facts::AbsF32::hull(h, crate::facts::AbsF32::exact(0.0));
                }
                crate::facts::Abs::F32(h)
            }
            crate::ir::Op::Guard { fact, value } => {
                cur[value.index()] = cur[value.index()].refine(fact);
                if let crate::ir::Fact::OneHot { group, .. } = fact {
                    for m in group {
                        cur[m.index()] = cur[m.index()].refine(fact);
                    }
                }
                crate::facts::Abs::None
            }
        };
        cur.push(a);
        at_def.push(a);
        if let Some(claims) = derived_claims.get(&i) {
            for (k, fact) in claims {
                let is_structural = matches!((fact, &node.op), (crate::ir::Fact::Shape(s), crate::ir::Op::F32Index { table, .. }) if *s as usize == table.len());
                if !is_structural && !a.implies(fact) {
                    return Err(unchecked(
                        crate::diag::Site::Fact(*k),
                        fact.clone(),
                        "the checker's transfer rules do not derive this fact",
                    ));
                }
            }
        }
    }
    Ok((at_def, input_facts))
}

/// Layers by handwired's depth rule: sources (inputs, params, constants) at 0,
/// otherwise `1 + max(operand layers)`. Returns per-node layers and the count.
fn layering(g: &crate::ir::Graph) -> (alloc::vec::Vec<u32>, u32) {
    let mut layer = alloc::vec![0u32; g.nodes.len()];
    let mut max = 0u32;
    for (i, node) in g.nodes.iter().enumerate() {
        let l = node
            .op
            .operands()
            .iter()
            .map(|o| layer[o.index()] + 1)
            .max()
            .unwrap_or(0);
        layer[i] = l;
        max = max.max(l);
    }
    (layer, if g.nodes.is_empty() { 0 } else { max + 1 })
}

/// One closure definition for the named-call expansion bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallGraphEntry {
    /// Exact definition identity.
    pub identity: u64,
    /// Nodes created by the body excluding named calls.
    pub local_nodes: u64,
    /// Named callees, one entry per call site (repeats count).
    pub callees: alloc::vec::Vec<u64>,
}

/// Saturating upper bound on expanded nodes of `root` over the closure call
/// graph, memoized per definition (D7 step 5; does not trust the extractor's
/// count). `None` if the call graph is recursive or names a missing identity.
#[must_use]
#[expect(
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-maintainers; the DFS stack and on-stack map grow at most once per closure entry"
)]
pub fn expansion_bound(closure: &[CallGraphEntry], root: u64) -> Option<u64> {
    let by_id: alloc::collections::BTreeMap<u64, &CallGraphEntry> =
        closure.iter().map(|e| (e.identity, e)).collect();
    let mut memo: alloc::collections::BTreeMap<u64, u64> = alloc::collections::BTreeMap::new();
    let mut on_stack: alloc::collections::BTreeMap<u64, ()> = alloc::collections::BTreeMap::new();
    // Iterative post-order.
    let mut stack: alloc::vec::Vec<(u64, usize)> = alloc::vec::Vec::with_capacity(closure.len());
    stack.push((root, 0));
    on_stack.insert(root, ());
    while let Some((id, next)) = stack.last_mut() {
        let id = *id;
        let e = by_id.get(&id)?;
        if let Some(&c) = e.callees.get(*next) {
            *next += 1;
            if memo.contains_key(&c) {
                continue;
            }
            if on_stack.contains_key(&c) {
                return None;
            }
            on_stack.insert(c, ());
            stack.push((c, 0));
        } else {
            let mut total = e.local_nodes;
            for c in &e.callees {
                total = total.saturating_add(*memo.get(c)?);
            }
            memo.insert(id, total);
            on_stack.remove(&id);
            stack.pop();
        }
    }
    memo.get(&root).copied()
}

#[cfg(test)]
mod tests {
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

    impl crate::identity::IdentityScheme for TestIds {
        fn name(&self) -> &'static str {
            "test"
        }

        fn start(&self) -> alloc::boxed::Box<dyn crate::identity::IdentityHasher + '_> {
            alloc::boxed::Box::new(TestHasher(0))
        }
    }

    /// A graph whose only addition names a node that does not exist: a topology refusal
    /// that makes the graph malformed.
    fn unknown_operand() -> crate::ir::Graph {
        let scheme = TestIds;
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&scheme, "stage-order"),
        );
        let missing = builder.op(crate::ir::Op::I64Add(
            crate::ir::NodeId(9),
            crate::ir::NodeId(9),
        ));
        builder.output(missing);
        builder.finish()
    }

    #[test]
    fn a_malformed_graph_is_refused_before_the_retained_byte_budget() {
        let scheme = TestIds;
        let budgets = crate::check::Budgets {
            graph_bytes: 0,
            ..crate::check::Budgets::default()
        };
        let refusal = super::check_with(&scheme, unknown_operand(), &budgets)
            .expect_err("a graph naming a missing node is refused");
        assert_eq!(
            refusal.code,
            crate::diag::Code::GraphMalformed(crate::diag::Malformed::UnknownNode),
            "{refusal}"
        );
        assert!(
            refusal.budget.is_none(),
            "a topology refusal is not a budget refusal: {refusal}"
        );
    }
}
