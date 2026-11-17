//! D10 derivative policy `ngraph-deriv-real/0`: the derivative of the
//! intended real-valued arithmetic, ignoring binary32 rounding. Not the
//! derivative of the floating-point function.
//!
//! * Static training admission: `E-DERIV-I64`, `E-PARAM-UNDECLARED`,
//!   `E-DERIV-BOOLEAN` (a param reaches the loss through `step` or a `select`
//!   gate under the default policy, or a boolean opt-in names no `step` or
//!   `select` it applies to), `E-DERIV-INDEX` (a param reaches an index
//!   operand under the default policy, or an index opt-in names no `index`
//!   node).
//! * Reverse mode visits nodes in reverse canonical order and accumulates each
//!   node's contributions in stored operand/term order, so FP32 gradients are
//!   bitwise reproducible. [`Real`] instantiates it in binary32 (the reference
//!   backward, G2) and binary64 (G1).
//! * Surrogates (`straight-through`, `clamped-linear`, index
//!   `backward-difference`, `select` gate straight-through) are opt-ins and are
//!   labelled as surrogates; they are never described as exact derivatives.
//!   A `clamped-linear` width must be a finite, strictly positive, normal
//!   binary32 value: the surrogate's window and slope are defined only there,
//!   and a subnormal width overflows the `g/w` division.

/// Derivative policy revision.
pub const DERIV_POLICY: &str = "ngraph-deriv-real/0";

/// Policy for hard-boolean paths (`step`, `select` gate).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BooleanPolicy {
    /// Default: static refusal `E-DERIV-BOOLEAN`.
    Refuse,
    /// Surrogate: identity through `step`; `(on − off)` through a `select` gate.
    StraightThrough,
    /// Surrogate: slope `1/w` on `|s − 0.5| < w/2`, otherwise 0 (`step` only).
    ClampedLinear(f32),
}

/// Policy for the index operand of `index` (candidate gradients are always
/// the exact derivative for a fixed index and are allowed by default).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexPolicy {
    /// Default: static refusal `E-DERIV-INDEX`.
    Refuse,
    /// Explicitly no index gradient.
    Zero,
    /// Surrogate: `read(i) − read(i−1)`.
    BackwardDifference,
}

/// Per-node derivative options (defaults: refuse).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DerivOptions {
    /// Per-node boolean policy overrides.
    pub boolean: alloc::collections::BTreeMap<crate::ir::NodeId, BooleanPolicy>,
    /// Per-node index policy overrides.
    pub index: alloc::collections::BTreeMap<crate::ir::NodeId, IndexPolicy>,
}

impl DerivOptions {
    fn boolean(&self, n: crate::ir::NodeId) -> BooleanPolicy {
        self.boolean
            .get(&n)
            .copied()
            .unwrap_or(BooleanPolicy::Refuse)
    }
    fn index(&self, n: crate::ir::NodeId) -> IndexPolicy {
        self.index.get(&n).copied().unwrap_or(IndexPolicy::Refuse)
    }

    /// Whether two option sets are the same at the level the recorded policy
    /// uses: the same nodes with the same policies, and, for
    /// [`BooleanPolicy::ClampedLinear`], the same width bits.
    ///
    /// `PartialEq` compares the width as an `f32`, so `NaN` never equals
    /// itself and two distinct `NaN` payloads never differ. Provenance replay
    /// must compare the recorded bits instead, exactly as
    /// [`crate::train::Adam::same_bits`] does for the optimizer settings.
    #[must_use]
    pub fn same_bits(&self, other: &Self) -> bool {
        let width = |p: &BooleanPolicy| match p {
            BooleanPolicy::ClampedLinear(w) => Some(w.to_bits()),
            BooleanPolicy::Refuse | BooleanPolicy::StraightThrough => None,
        };
        self.boolean.len() == other.boolean.len()
            && self.boolean.iter().all(|(n, p)| {
                other.boolean.get(n).is_some_and(|q| {
                    core::mem::discriminant(p) == core::mem::discriminant(q) && width(p) == width(q)
                })
            })
            && self.index == other.index
    }

    /// Canonical text of every opt-in (recorded in provenance).
    #[must_use]
    pub fn describe(&self) -> alloc::string::String {
        let mut s = alloc::string::String::from(DERIV_POLICY);
        for (n, p) in &self.boolean {
            s.push_str(&match p {
                BooleanPolicy::Refuse => alloc::format!(";%{}:boolean=refuse", n.0),
                BooleanPolicy::StraightThrough => {
                    alloc::format!(";%{}:boolean=straight-through(surrogate)", n.0)
                }
                BooleanPolicy::ClampedLinear(w) => {
                    alloc::format!(
                        ";%{}:boolean=clamped-linear(w=0x{:08x},surrogate)",
                        n.0,
                        w.to_bits()
                    )
                }
            });
        }
        for (n, p) in &self.index {
            s.push_str(&match p {
                IndexPolicy::Refuse => alloc::format!(";%{}:index=refuse", n.0),
                IndexPolicy::Zero => alloc::format!(";%{}:index=zero", n.0),
                IndexPolicy::BackwardDifference => {
                    alloc::format!(";%{}:index=backward-difference(surrogate)", n.0)
                }
            });
        }
        s
    }

    /// Whether any surrogate is in use.
    #[must_use]
    pub fn uses_surrogate(&self) -> bool {
        self.boolean.values().any(|p| *p != BooleanPolicy::Refuse)
            || self
                .index
                .values()
                .any(|p| *p == IndexPolicy::BackwardDifference)
    }
}

/// Static training admission (D10/D11) for `model` with trainable `params`
/// (slot indices). Every opt-in must name a node of `model` that its policy
/// applies to: `straight-through` and `refuse` a `step` or `select`,
/// `clamped-linear` a `step`, and every index policy an `index` (design D6
/// and D10 of `numeric-graph-v1`; replay re-admits recorded opt-ins here).
///
/// # Errors
/// `E-DERIV-I64`, `E-PARAM-UNDECLARED`, `E-DERIV-BOOLEAN` or `E-DERIV-INDEX`.
pub fn admit(
    model: &crate::check::CheckedGraph,
    params: &[u32],
    opts: &DerivOptions,
) -> Result<(), crate::diag::Diagnostic> {
    let g = model.graph();
    if g.profile == crate::ir::SubProfile::I64 {
        return Err(crate::diag::Diagnostic::new(
            crate::diag::Code::DerivI64,
            crate::diag::Site::Graph,
            "training contracts are refused on ngraph-i64/0",
        ));
    }
    for &k in params {
        if g.slots
            .get(k as usize)
            .is_none_or(|s| s.kind != crate::ir::SlotKind::Param)
        {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::ParamUndeclared,
                crate::diag::Site::Slot(k),
                "gradient requested for a non-param slot",
            ));
        }
    }
    admit_options(g, opts)?;
    let n = g.nodes.len();
    let mut dep = alloc::vec![false; n];
    for (i, node) in g.nodes.iter().enumerate() {
        let mut has_dependency = false;
        if let crate::ir::Op::Param(k) = node.op {
            has_dependency = params.contains(&k);
        } else {
            node.op.visit_operands(|o| has_dependency |= dep[o.index()]);
        }
        dep[i] = has_dependency;
    }
    admit_paths(g, &dep, opts)
}

/// Refuses a declared-parameter path to an output through a hard boolean node
/// or the index operand of an index read. `dep[i]` says whether node `i`
/// carries a declared parameter (the model's params, or a model output feeding
/// the loss graph); `opts` names nodes of the model graph and is the default
/// for a loss graph, whose nodes no opt-in can name.
///
/// # Errors
/// `E-DERIV-BOOLEAN` or `E-DERIV-INDEX`.
pub(crate) fn admit_paths(
    g: &crate::ir::Graph,
    dep: &[bool],
    opts: &DerivOptions,
) -> Result<(), crate::diag::Diagnostic> {
    debug_assert_eq!(dep.len(), g.nodes.len());
    let n = g.nodes.len();
    let mut reaches = alloc::vec![false; n];
    for o in &g.outputs {
        reaches[o.index()] = true;
    }
    for i in (0..n).rev() {
        if reaches[i] && !g.nodes[i].op.is_guard() {
            g.nodes[i].op.visit_operands(|o| reaches[o.index()] = true);
        }
    }
    for (i, node) in g.nodes.iter().enumerate() {
        if !reaches[i] {
            continue;
        }
        let id = crate::ir::NodeId(i as u32);
        match &node.op {
            crate::ir::Op::F32Step(s)
                if dep[s.index()] && opts.boolean(id) == BooleanPolicy::Refuse =>
            {
                return Err(crate::diag::Diagnostic::new(
                    crate::diag::Code::DerivBoolean,
                    crate::diag::Site::Node(id),
                    "a param reaches the loss through step",
                ));
            }
            crate::ir::Op::F32Select { bit, .. }
                if dep[bit.index()]
                    && !matches!(opts.boolean(id), BooleanPolicy::StraightThrough) =>
            {
                return Err(crate::diag::Diagnostic::new(
                    crate::diag::Code::DerivBoolean,
                    crate::diag::Site::Node(id),
                    "a param reaches the loss through a select gate",
                ));
            }
            crate::ir::Op::F32Index { index, .. }
                if dep[index.index()] && opts.index(id) == IndexPolicy::Refuse =>
            {
                return Err(crate::diag::Diagnostic::new(
                    crate::diag::Code::DerivIndex,
                    crate::diag::Site::Node(id),
                    "a param reaches an index operand",
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

/// Every opt-in names a node of `g` that its policy applies to.
fn admit_options(g: &crate::ir::Graph, opts: &DerivOptions) -> Result<(), crate::diag::Diagnostic> {
    let op = |n: crate::ir::NodeId| g.nodes.get(n.index()).map(|node| &node.op);
    let refuse = |code: crate::diag::Code, n: crate::ir::NodeId, what: &str| {
        let why = match op(n) {
            Some(o) => alloc::format!("the {what} opt-in does not apply to {}", o.name()),
            None => alloc::format!("the {what} opt-in names no node of the model"),
        };
        Err(crate::diag::Diagnostic::new(
            code,
            crate::diag::Site::Node(n),
            why,
        ))
    };
    for (&n, policy) in &opts.boolean {
        if let BooleanPolicy::ClampedLinear(w) = policy {
            // The surrogate divides by the width and defines its window from
            // it: only a finite, strictly positive, normal binary32 value is
            // a width the policy defines (a subnormal width overflows `g/w`).
            if !(w.is_normal() && *w > 0.0) {
                return Err(crate::diag::Diagnostic::new(
                    crate::diag::Code::DerivBoolean,
                    crate::diag::Site::Node(n),
                    alloc::format!(
                        "clamped-linear width 0x{:08x} is not a finite, strictly positive, normal value",
                        w.to_bits()
                    ),
                ));
            }
        }
        let is_applicable = matches!(
            (policy, op(n)),
            (
                BooleanPolicy::ClampedLinear(_),
                Some(crate::ir::Op::F32Step(_))
            ) | (
                BooleanPolicy::Refuse | BooleanPolicy::StraightThrough,
                Some(crate::ir::Op::F32Step(_) | crate::ir::Op::F32Select { .. }),
            )
        );
        if !is_applicable {
            return refuse(crate::diag::Code::DerivBoolean, n, "boolean");
        }
    }
    for &n in opts.index.keys() {
        if !matches!(op(n), Some(crate::ir::Op::F32Index { .. })) {
            return refuse(crate::diag::Code::DerivIndex, n, "index");
        }
    }
    Ok(())
}

/// Scalar arithmetic used by reverse mode (binary32 or binary64, RNE).
pub trait Real: Copy + PartialOrd + core::fmt::Debug {
    /// `+0`.
    const ZERO: Self;
    /// From binary32 (exact).
    fn of_f32(x: f32) -> Self;
    /// To binary64 (exact).
    fn to_f64(self) -> f64;
    /// `rn(a+b)`.
    fn add(self, o: Self) -> Self;
    /// `rn(a−b)`.
    fn sub(self, o: Self) -> Self;
    /// `rn(a×b)`.
    fn mul(self, o: Self) -> Self;
    /// `rn(a÷b)`.
    fn div(self, o: Self) -> Self;
    /// Whether the value is exactly `1`.
    fn is_one(self) -> bool;
}

impl Real for f32 {
    const ZERO: Self = 0.0;
    fn of_f32(x: f32) -> Self {
        x
    }
    fn to_f64(self) -> f64 {
        f64::from(self)
    }
    fn add(self, o: Self) -> Self {
        self + o
    }
    fn sub(self, o: Self) -> Self {
        self - o
    }
    fn mul(self, o: Self) -> Self {
        self * o
    }
    fn div(self, o: Self) -> Self {
        self / o
    }
    fn is_one(self) -> bool {
        self.to_bits() == crate::sem::ONE
    }
}

impl Real for f64 {
    const ZERO: Self = 0.0;
    fn of_f32(x: f32) -> Self {
        Self::from(x)
    }
    fn to_f64(self) -> f64 {
        self
    }
    fn add(self, o: Self) -> Self {
        self + o
    }
    fn sub(self, o: Self) -> Self {
        self - o
    }
    fn mul(self, o: Self) -> Self {
        self * o
    }
    fn div(self, o: Self) -> Self {
        self / o
    }
    fn is_one(self) -> bool {
        self == 1.0
    }
}

fn relu<R: Real>(x: R) -> R {
    if x > R::ZERO || x.to_f64().is_nan() {
        x
    } else {
        R::ZERO
    }
}

fn position<R: Real>(
    i: R,
    len: usize,
    below: crate::ir::OutOfRange,
    above: crate::ir::OutOfRange,
) -> Option<Option<usize>> {
    let f = i.to_f64() as f32;
    crate::sem::index_position(f, len, below, above).ok()
}

/// `R`-valued forward pass (guards are not evaluated; callers sample points
/// where guards and index reads succeed). In binary32 this equals the L3
/// reference bitwise on such points.
#[must_use]
pub fn forward<R: Real>(g: &crate::check::CheckedGraph, slots: &[R]) -> alloc::vec::Vec<R> {
    forward_observed(g, slots, &mut None)
}

fn forward_observed<R: Real>(
    g: &crate::check::CheckedGraph,
    slots: &[R],
    evaluated_nodes: &mut Option<&mut u64>,
) -> alloc::vec::Vec<R> {
    let graph = g.graph();
    let mut v: alloc::vec::Vec<R> = alloc::vec::Vec::with_capacity(graph.nodes.len());
    for node in &graph.nodes {
        // Guards are actually tested by first_failure_observed, not by this
        // forward pass's zero placeholder. Count only real value operations.
        if !node.op.is_guard() {
            if let Some(counter) = evaluated_nodes.as_deref_mut() {
                *counter += 1;
            }
        }
        let x = |n: &crate::ir::NodeId| v[n.index()];
        let r = match &node.op {
            crate::ir::Op::Input(k) | crate::ir::Op::Param(k) => slots[*k as usize],
            crate::ir::Op::Const(crate::ir::Literal::F32(b)) => R::of_f32(f32::from_bits(*b)),
            crate::ir::Op::F32Add(a, b) => x(a).add(x(b)),
            crate::ir::Op::F32Sub(a, b) => x(a).sub(x(b)),
            crate::ir::Op::F32Mul(a, b) => x(a).mul(x(b)),
            crate::ir::Op::F32Affine { bias, terms } => {
                let mut acc = R::of_f32(f32::from_bits(*bias));
                for t in terms {
                    let o = match t.operand {
                        crate::ir::AffineOperand::One => R::of_f32(1.0),
                        crate::ir::AffineOperand::Node(n) => x(&n),
                    };
                    acc = acc.add(R::of_f32(f32::from_bits(t.coefficient)).mul(o));
                }
                acc
            }
            crate::ir::Op::F32Relu(a) => relu(x(a)),
            crate::ir::Op::F32Reglu { left, gate } => x(left).mul(relu(x(gate))),
            crate::ir::Op::F32Step(s) => {
                if x(s).to_f64() >= 0.5 {
                    R::of_f32(1.0)
                } else {
                    R::ZERO
                }
            }
            crate::ir::Op::F32Select { bit, off, on } => {
                if x(bit).is_one() {
                    x(on)
                } else {
                    x(off)
                }
            }
            crate::ir::Op::F32Index {
                index,
                table,
                below,
                above,
            } => match position(x(index), table.len(), *below, *above) {
                Some(Some(k)) => x(&table[k]),
                Some(None) | None => R::ZERO,
            },
            _ => R::ZERO,
        };
        v.push(r);
    }
    v
}

/// Reverse mode: given forward values and output seeds `(output position,
/// gradient)`, returns the gradient of every node.
#[must_use]
pub fn backward<R: Real>(
    g: &crate::check::CheckedGraph,
    vals: &[R],
    seeds: &[R],
    opts: &DerivOptions,
) -> alloc::vec::Vec<R> {
    let graph = g.graph();
    let mut gr = alloc::vec![R::ZERO; graph.nodes.len()];
    for (o, s) in graph.outputs.iter().zip(seeds) {
        gr[o.index()] = gr[o.index()].add(*s);
    }
    for (i, node) in graph.nodes.iter().enumerate().rev() {
        let id = crate::ir::NodeId(i as u32);
        let gn = gr[i];
        let acc = |n: &crate::ir::NodeId, d: R, gr: &mut alloc::vec::Vec<R>| {
            gr[n.index()] = gr[n.index()].add(d);
        };
        match &node.op {
            crate::ir::Op::F32Add(a, b) => {
                acc(a, gn, &mut gr);
                acc(b, gn, &mut gr);
            }
            crate::ir::Op::F32Sub(a, b) => {
                acc(a, gn, &mut gr);
                acc(b, R::ZERO.sub(gn), &mut gr);
            }
            crate::ir::Op::F32Mul(a, b) => {
                let (va, vb) = (vals[a.index()], vals[b.index()]);
                acc(a, gn.mul(vb), &mut gr);
                acc(b, gn.mul(va), &mut gr);
            }
            crate::ir::Op::F32Affine { terms, .. } => {
                for t in terms {
                    if let crate::ir::AffineOperand::Node(n) = t.operand {
                        acc(
                            &n,
                            gn.mul(R::of_f32(f32::from_bits(t.coefficient))),
                            &mut gr,
                        );
                    }
                }
            }
            crate::ir::Op::F32Relu(a) => {
                let d = if vals[a.index()] > R::ZERO {
                    gn
                } else {
                    R::ZERO
                };
                acc(a, d, &mut gr);
            }
            crate::ir::Op::F32Reglu { left, gate } => {
                let (vl, vg) = (vals[left.index()], vals[gate.index()]);
                acc(left, gn.mul(relu(vg)), &mut gr);
                let d = if vg > R::ZERO { gn.mul(vl) } else { R::ZERO };
                acc(gate, d, &mut gr);
            }
            crate::ir::Op::F32Step(s) => {
                let d = match opts.boolean(id) {
                    BooleanPolicy::Refuse => R::ZERO,
                    BooleanPolicy::StraightThrough => gn,
                    BooleanPolicy::ClampedLinear(w) => {
                        let dist = (vals[s.index()].to_f64() - 0.5).abs();
                        if dist < f64::from(w) / 2.0 {
                            gn.div(R::of_f32(w))
                        } else {
                            R::ZERO
                        }
                    }
                };
                acc(s, d, &mut gr);
            }
            crate::ir::Op::F32Select { bit, off, on } => {
                let is_one = vals[bit.index()].is_one();
                let gate = match opts.boolean(id) {
                    BooleanPolicy::StraightThrough => {
                        gn.mul(vals[on.index()].sub(vals[off.index()]))
                    }
                    _ => R::ZERO,
                };
                acc(bit, gate, &mut gr);
                if is_one {
                    acc(on, gn, &mut gr);
                } else {
                    acc(off, gn, &mut gr);
                }
            }
            crate::ir::Op::F32Index {
                index,
                table,
                below,
                above,
            } => {
                let i = vals[index.index()];
                let d = match opts.index(id) {
                    IndexPolicy::BackwardDifference => {
                        let read = |p: Option<Option<usize>>| match p {
                            Some(Some(k)) => vals[table[k].index()],
                            Some(None) | None => R::ZERO,
                        };
                        let here = read(position(i, table.len(), *below, *above));
                        let prev =
                            read(position(i.sub(R::of_f32(1.0)), table.len(), *below, *above));
                        gn.mul(here.sub(prev))
                    }
                    _ => R::ZERO,
                };
                acc(index, d, &mut gr);
                if let Some(Some(k)) = position(i, table.len(), *below, *above) {
                    acc(&table[k], gn, &mut gr);
                }
            }
            _ => {}
        }
    }
    gr
}

/// Gradient of every param slot (in slot order of `params`) from node gradients.
#[must_use]
pub fn param_grads<R: Real>(
    g: &crate::check::CheckedGraph,
    node_grads: &[R],
    params: &[u32],
) -> alloc::vec::Vec<R> {
    params
        .iter()
        .map(|k| node_grads[g.slot_node(*k).index()])
        .collect()
}

/// Gradient of every slot (inputs and params) in slot order.
#[must_use]
pub fn slot_grads<R: Real>(g: &crate::check::CheckedGraph, node_grads: &[R]) -> alloc::vec::Vec<R> {
    (0..g.graph().slots.len() as u32)
        .map(|k| node_grads[g.slot_node(k).index()])
        .collect()
}

/// Composed scalar objective: `loss(model(inputs, params), targets)`, with
/// the model output vector feeding the loss graph's first inputs and
/// `targets` its remaining inputs. Returns `(loss, d loss / d slot of model)`.
#[must_use]
pub fn objective<R: Real>(
    model: &crate::check::CheckedGraph,
    loss: &crate::check::CheckedGraph,
    model_slots: &[R],
    targets: &[R],
    opts: &DerivOptions,
) -> (R, alloc::vec::Vec<R>) {
    let mv = forward(model, model_slots);
    let ys: alloc::vec::Vec<R> = model
        .graph()
        .outputs
        .iter()
        .map(|o| mv[o.index()])
        .collect();
    let mut ls = ys.clone();
    ls.extend_from_slice(targets);
    let lv = forward(loss, &ls);
    let l = lv[loss.graph().outputs[0].index()];
    let lg = backward(loss, &lv, &[R::of_f32(1.0)], &DerivOptions::default());
    let dy: alloc::vec::Vec<R> = slot_grads(loss, &lg)[..ys.len()].to_vec();
    let mg = backward(model, &mv, &dy, opts);
    (l, slot_grads(model, &mg))
}

/// [`objective`] in binary32 with the reference failure semantics (design
/// D10): the model's first failing guard or refusing index read in canonical
/// order, then the loss graph's input facts and its first such failure, stop
/// the point. On success the result equals `objective::<f32>` bitwise.
///
/// # Errors
/// `G-FACT-FALSE` (model or loss graph) or `E-FACT-INPUT` (loss graph).
pub fn objective_checked(
    model: &crate::check::CheckedGraph,
    loss: &crate::check::CheckedGraph,
    model_slots: &[f32],
    targets: &[f32],
    opts: &DerivOptions,
) -> Result<(f32, alloc::vec::Vec<f32>), crate::diag::Diagnostic> {
    objective_observed(model, loss, model_slots, targets, opts, &mut None)
}

#[expect(
    tigerstyle::too_many_parameters,
    reason = "Owner: noble-maintainers; the signature mirrors the model/loss, slot, option and observation inputs of the checked objective"
)]
pub(crate) fn objective_observed(
    model: &crate::check::CheckedGraph,
    loss: &crate::check::CheckedGraph,
    model_slots: &[f32],
    targets: &[f32],
    opts: &DerivOptions,
    evaluated_nodes: &mut Option<&mut u64>,
) -> Result<(f32, alloc::vec::Vec<f32>), crate::diag::Diagnostic> {
    let mv = forward_observed(model, model_slots, evaluated_nodes);
    if let Some((node, fact)) = crate::eval::first_failure_observed(model, &mv, evaluated_nodes) {
        return Err(crate::diag::Diagnostic::new(
            crate::diag::Code::FactFalse,
            crate::diag::Site::Node(node),
            "model guard failed on a training point",
        )
        .with_fact(fact));
    }
    let ys: alloc::vec::Vec<f32> = model
        .graph()
        .outputs
        .iter()
        .map(|o| mv[o.index()])
        .collect();
    let mut ls = ys.clone();
    ls.extend_from_slice(targets);
    crate::eval::preflight(loss, &crate::eval::f32_inputs(&ls), None)?;
    let lv = forward_observed(loss, &ls, evaluated_nodes);
    if let Some((node, fact)) = crate::eval::first_failure_observed(loss, &lv, evaluated_nodes) {
        return Err(crate::diag::Diagnostic::new(
            crate::diag::Code::FactFalse,
            crate::diag::Site::Node(node),
            "loss guard failed on a training point",
        )
        .with_fact(fact));
    }
    let l = lv[loss.graph().outputs[0].index()];
    let lg = backward(loss, &lv, &[1.0], &DerivOptions::default());
    let dy: alloc::vec::Vec<f32> = slot_grads(loss, &lg)[..ys.len()].to_vec();
    let mg = backward(model, &mv, &dy, opts);
    Ok((l, slot_grads(model, &mg)))
}

/// Scale for the G2 rounding bound: the same reverse pass over absolute
/// values of every local derivative and forward value (a running-error
/// magnitude, ≥ |gradient|).
#[must_use]
pub fn magnitude(
    model: &crate::check::CheckedGraph,
    loss: &crate::check::CheckedGraph,
    model_slots: &[f64],
    targets: &[f64],
    opts: &DerivOptions,
) -> alloc::vec::Vec<f64> {
    let mv: alloc::vec::Vec<f64> = forward(model, model_slots);
    let sa: alloc::vec::Vec<f64> = model_slots.iter().map(|v| v.abs()).collect();
    let ma = forward_abs(model, &mv, &sa);
    let outs = &model.graph().outputs;
    let mut ls: alloc::vec::Vec<f64> = outs.iter().map(|o| mv[o.index()]).collect();
    let mut la: alloc::vec::Vec<f64> = outs.iter().map(|o| ma[o.index()]).collect();
    ls.extend_from_slice(targets);
    la.extend(targets.iter().map(|t| t.abs()));
    let lv: alloc::vec::Vec<f64> = forward(loss, &ls);
    let lva = forward_abs(loss, &lv, &la);
    let lg = backward_abs_with_signs(loss, &lv, &lva, &[1.0], &DerivOptions::default());
    let dy: alloc::vec::Vec<f64> = slot_grads(loss, &lg)[..outs.len()].to_vec();
    let mg = backward_abs_with_signs(model, &mv, &ma, &dy, opts);
    slot_grads(model, &mg)
}

/// Forward magnitudes: the computation over absolute values (`|a|+|b|` for
/// `±`, `|a|·|b|` for `×`), with branches taken from the signed forward values.
fn forward_abs(
    g: &crate::check::CheckedGraph,
    signed: &[f64],
    slot_abs: &[f64],
) -> alloc::vec::Vec<f64> {
    let graph = g.graph();
    let mut v: alloc::vec::Vec<f64> = alloc::vec::Vec::with_capacity(graph.nodes.len());
    for node in &graph.nodes {
        let r = match &node.op {
            crate::ir::Op::Input(k) | crate::ir::Op::Param(k) => slot_abs[*k as usize],
            crate::ir::Op::Const(crate::ir::Literal::F32(b)) => f64::from(f32::from_bits(*b)).abs(),
            crate::ir::Op::F32Add(a, b) | crate::ir::Op::F32Sub(a, b) => {
                v[a.index()] + v[b.index()]
            }
            crate::ir::Op::F32Mul(a, b) => v[a.index()] * v[b.index()],
            crate::ir::Op::F32Affine { bias, terms } => {
                let mut acc = f64::from(f32::from_bits(*bias)).abs();
                for t in terms {
                    let x = match t.operand {
                        crate::ir::AffineOperand::One => 1.0,
                        crate::ir::AffineOperand::Node(n) => v[n.index()],
                    };
                    acc += f64::from(f32::from_bits(t.coefficient)).abs() * x;
                }
                acc
            }
            crate::ir::Op::F32Relu(x) if signed[x.index()] > 0.0 => v[x.index()],
            crate::ir::Op::F32Reglu { left, gate } if signed[gate.index()] > 0.0 => {
                v[left.index()] * v[gate.index()]
            }
            crate::ir::Op::F32Step(_)
            | crate::ir::Op::F32Select { .. }
            | crate::ir::Op::F32Index { .. } => signed[v.len()].abs(),
            _ => 0.0,
        };
        v.push(r);
    }
    v
}

fn backward_abs_with_signs(
    g: &crate::check::CheckedGraph,
    vals: &[f64],
    absvals: &[f64],
    seeds: &[f64],
    opts: &DerivOptions,
) -> alloc::vec::Vec<f64> {
    // Same structure as `backward`, with |local derivative| and |value|;
    // branch decisions use the signed forward values.
    let graph = g.graph();
    let mut gr = alloc::vec![0.0f64; graph.nodes.len()];
    for (o, s) in graph.outputs.iter().zip(seeds) {
        gr[o.index()] += s.abs();
    }
    for (i, node) in graph.nodes.iter().enumerate().rev() {
        let id = crate::ir::NodeId(i as u32);
        let gn = gr[i];
        match &node.op {
            crate::ir::Op::F32Add(a, b) | crate::ir::Op::F32Sub(a, b) => {
                gr[a.index()] += gn;
                gr[b.index()] += gn;
            }
            crate::ir::Op::F32Mul(a, b) => {
                gr[a.index()] += gn * absvals[b.index()];
                gr[b.index()] += gn * absvals[a.index()];
            }
            crate::ir::Op::F32Affine { terms, .. } => {
                for t in terms {
                    if let crate::ir::AffineOperand::Node(n) = t.operand {
                        gr[n.index()] += gn * f64::from(f32::from_bits(t.coefficient)).abs();
                    }
                }
            }
            crate::ir::Op::F32Relu(a) if vals[a.index()] > 0.0 => gr[a.index()] += gn,
            crate::ir::Op::F32Reglu { left, gate } => {
                let vg = vals[gate.index()];
                gr[left.index()] += gn * if vg > 0.0 { absvals[gate.index()] } else { 0.0 };
                if vg > 0.0 {
                    gr[gate.index()] += gn * absvals[left.index()];
                }
            }
            crate::ir::Op::F32Step(s) => {
                if let BooleanPolicy::StraightThrough = opts.boolean(id) {
                    gr[s.index()] += gn;
                }
            }
            crate::ir::Op::F32Select { bit, off, on } => {
                if vals[bit.index()] == 1.0 {
                    gr[on.index()] += gn;
                } else {
                    gr[off.index()] += gn;
                }
            }
            crate::ir::Op::F32Index {
                index,
                table,
                below,
                above,
            } => {
                if let Some(Some(k)) = position(vals[index.index()], table.len(), *below, *above) {
                    gr[table[k].index()] += gn;
                }
            }
            _ => {}
        }
    }
    gr
}

/// Binary64 central difference of the composed objective in slot `k`.
#[must_use]
#[expect(
    tigerstyle::too_many_parameters,
    reason = "Owner: noble-maintainers; the signature mirrors the model/loss, slot, option and step inputs of the difference"
)]
pub fn central_difference(
    model: &crate::check::CheckedGraph,
    loss: &crate::check::CheckedGraph,
    model_slots: &[f64],
    targets: &[f64],
    k: usize,
    h: f64,
) -> f64 {
    let mut plus = model_slots.to_vec();
    let mut minus = model_slots.to_vec();
    plus[k] += h;
    minus[k] -= h;
    let opts = DerivOptions::default();
    let (lp, _) = objective(model, loss, &plus, targets, &opts);
    let (lm, _) = objective(model, loss, &minus, targets, &opts);
    (lp - lm) / (2.0 * h)
}

/// Critical-path depth of the composed objective (model layers + loss layers).
#[must_use]
pub fn depth(model: &crate::check::CheckedGraph, loss: &crate::check::CheckedGraph) -> u32 {
    model.layer_count() + loss.layer_count()
}
