#![expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-maintainers; the module path names the domain and the item name keeps its canonical operation spelling"
)]

//! D8 forward observations, evaluation preflight, scratch accounting (D12)
//! and the normative CPU reference interpreter (leg L3).

/// `Obs(G, inputs, snapshot)` (D8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Obs {
    /// Fresh immutable outputs, compared by `≡obs`.
    Normal(alloc::vec::Vec<crate::sem::Value>),
    /// Refused before evaluation (admission, snapshot, input validation, budget).
    Refused(crate::diag::Diagnostic),
    /// `G-FACT-FALSE(node, fact)`: a guard (or a refusing index read) failed; no outputs.
    GuardFailed {
        /// Failing node.
        node: crate::ir::NodeId,
        /// Failing fact.
        fact: crate::ir::Fact,
    },
    /// Runtime exhaustion of a host budget.
    Exhausted(crate::diag::BudgetKind),
    /// `C-CANCELLED` at a layer boundary.
    Cancelled,
}

impl Obs {
    /// `≡obs` on observations: outputs bitwise except one NaN class; other
    /// classes compare by code/node/fact (refusal detail text is ignored).
    #[must_use]
    pub fn obs_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Normal(a), Self::Normal(b)) => crate::sem::obs_eq_all(a, b),
            (Self::Refused(a), Self::Refused(b)) => a.code == b.code && a.site == b.site,
            (
                Self::GuardFailed { node: n1, fact: f1 },
                Self::GuardFailed { node: n2, fact: f2 },
            ) => n1 == n2 && f1 == f2,
            (Self::Exhausted(a), Self::Exhausted(b)) => a == b,
            (Self::Cancelled, Self::Cancelled) => true,
            _ => false,
        }
    }

    /// Outputs of a normal observation.
    #[must_use]
    pub fn outputs(&self) -> Option<&[crate::sem::Value]> {
        match self {
            Self::Normal(v) => Some(v),
            _ => None,
        }
    }
}

/// Instrumented scratch counters (B6), owned by the caller and lent to one
/// evaluation or training call at a time ([`Control::scratch`],
/// [`crate::train::TrainControl::scratch`]). Every evaluation acquires its
/// scratch through a [`ScratchLease`] whose drop releases it exactly once.
/// Plain fields under an exclusive borrow: the library holds no interior
/// mutability, atomics or shared mutable globals (design D2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScratchCounters {
    /// Leases acquired.
    pub acquired: u64,
    /// Leases released.
    pub released: u64,
    /// Bytes currently leased.
    pub live_bytes: u64,
    /// Peak leased bytes.
    pub peak_bytes: u64,
}

impl ScratchCounters {
    /// `(acquired, released, live_bytes)`.
    #[must_use]
    pub const fn snapshot(&self) -> (u64, u64, u64) {
        (self.acquired, self.released, self.live_bytes)
    }
}

/// A scratch lease; its drop is the single release. It holds the counters'
/// exclusive borrow for its lifetime.
#[derive(Debug)]
pub struct ScratchLease<'a> {
    counters: Option<&'a mut ScratchCounters>,
    bytes: u64,
}

impl<'a> ScratchLease<'a> {
    /// Acquires `bytes` of scratch.
    #[must_use]
    pub fn acquire(mut counters: Option<&'a mut ScratchCounters>, bytes: u64) -> Self {
        if let Some(c) = counters.as_deref_mut() {
            c.acquired += 1;
            c.live_bytes += bytes;
            c.peak_bytes = c.peak_bytes.max(c.live_bytes);
        }
        Self { counters, bytes }
    }
}

impl Drop for ScratchLease<'_> {
    fn drop(&mut self) {
        if let Some(c) = self.counters.as_deref_mut() {
            c.released += 1;
            c.live_bytes -= self.bytes;
        }
    }
}

/// Host evaluation controls: fuel (node evaluations), cancellation probe
/// (called at each layer boundary with the layer about to start) and scratch
/// counters.
#[derive(Default)]
pub struct Control<'a> {
    /// Maximum node evaluations; exceeding it gives `Exhausted(Fuel)`.
    pub fuel: Option<u64>,
    /// Returns `true` to cancel before starting the given layer.
    pub cancel: Option<&'a dyn Fn(u32) -> bool>,
    /// Scratch instrumentation, lent for the evaluation.
    pub scratch: Option<&'a mut ScratchCounters>,
    /// Forward scratch budget in bytes (defaults to the `/0` budget).
    pub forward_scratch: Option<u64>,
    /// Caller-owned count of actual node evaluations. Early refusals leave
    /// it unchanged; successive evaluations accumulate into the same count.
    pub evaluated_nodes: Option<&'a mut u64>,
}

/// An evaluation result with the number of graph nodes evaluated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Observation.
    pub obs: Obs,
    /// Graph nodes evaluated (0 for every refusal before evaluation).
    pub evaluated_nodes: u64,
}

impl Report {
    const fn refused(d: crate::diag::Diagnostic) -> Self {
        Self {
            obs: Obs::Refused(d),
            evaluated_nodes: 0,
        }
    }
}

/// Slot values after preflight, indexed by slot.
pub(crate) type SlotValues = alloc::vec::Vec<crate::sem::Value>;

/// Evaluation preflight shared by all evaluators: snapshot identity and
/// layout (D11), input arity/types (`E-INTERFACE`), input-validated facts
/// (`E-FACT-INPUT`, including param facts on the snapshot values).
///
/// # Errors
/// The refusal diagnostic.
pub fn preflight(
    checked: &crate::check::CheckedGraph,
    inputs: &[crate::sem::Value],
    snapshot: Option<&crate::snapshot::ParamSnapshot>,
) -> Result<SlotValues, crate::diag::Diagnostic> {
    let g = checked.graph();
    let has_params = g.slots.iter().any(|s| s.kind == crate::ir::SlotKind::Param);
    match (snapshot, has_params) {
        (Some(s), _) => s.check_use(checked)?,
        (None, true) => {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::SnapshotLayout,
                crate::diag::Site::Graph,
                "graph has param slots but no snapshot was supplied",
            ));
        }
        (None, false) => {}
    }
    let mut values = alloc::vec::Vec::with_capacity(g.slots.len());
    let mut it = inputs.iter();
    let mut param_pos = 0usize;
    for (k, slot) in g.slots.iter().enumerate() {
        let k32 = k as u32;
        let v = match slot.kind {
            crate::ir::SlotKind::Input => match it.next() {
                Some(v) if v.ty() == slot.ty => *v,
                Some(v) => {
                    return Err(crate::diag::Diagnostic::new(
                        crate::diag::Code::Interface,
                        crate::diag::Site::Slot(k32),
                        alloc::format!("{} input for {} slot", v.ty().name(), slot.ty.name()),
                    ));
                }
                None => {
                    return Err(crate::diag::Diagnostic::new(
                        crate::diag::Code::Interface,
                        crate::diag::Site::Slot(k32),
                        "missing input",
                    ));
                }
            },
            crate::ir::SlotKind::Param => {
                // `check_use` established `snapshot.slots() == param_slots()`, so
                // the j-th param slot is snapshot position j.
                let pos = param_pos;
                param_pos += 1;
                crate::sem::Value::F32(snapshot.and_then(|s| s.bits_at(pos)).ok_or_else(|| {
                    crate::diag::Diagnostic::new(
                        crate::diag::Code::SnapshotLayout,
                        crate::diag::Site::Slot(k32),
                        "param missing",
                    )
                })?)
            }
        };
        values.push(v);
    }
    if it.next().is_some() {
        return Err(crate::diag::Diagnostic::new(
            crate::diag::Code::Interface,
            crate::diag::Site::Graph,
            "too many inputs",
        ));
    }
    for inf in checked.input_facts() {
        let v = values[inf.slot as usize];
        let group = crate::facts::group_values(&inf.fact, |m| match g.nodes[m.index()].op {
            crate::ir::Op::Input(s) | crate::ir::Op::Param(s) => values[s as usize],
            _ => v,
        });
        if !crate::facts::holds(&inf.fact, v, &group, None) {
            return Err(crate::diag::Diagnostic::new(
                crate::diag::Code::FactInput,
                crate::diag::Site::Slot(inf.slot),
                alloc::format!("input fails its {} fact", inf.fact.kind_name()),
            )
            .with_fact(inf.fact.clone()));
        }
    }
    Ok(values)
}

/// Bytes of reference-interpreter scratch for `checked`.
#[must_use]
pub fn reference_scratch_bytes(checked: &crate::check::CheckedGraph) -> u64 {
    (checked.node_count() * core::mem::size_of::<crate::sem::Value>()) as u64
}

pub(crate) fn scratch_preflight(
    bytes: u64,
    control: &Control<'_>,
) -> Result<(), crate::diag::Diagnostic> {
    let limit_bytes = control
        .forward_scratch
        .unwrap_or_else(|| crate::check::Budgets::default().forward_scratch);
    if bytes > limit_bytes {
        Err(crate::diag::Diagnostic::budget(
            crate::diag::BudgetKind::ForwardScratch,
            limit_bytes,
            bytes,
        ))
    } else {
        Ok(())
    }
}

/// The fact an `index` read fails under `refuse`, for `G-FACT-FALSE`.
#[must_use]
pub fn index_refusal_fact(len: usize) -> crate::ir::Fact {
    let hi = (len.saturating_sub(1)) as f32;
    crate::ir::Fact::Range {
        lo: crate::ir::Literal::F32(0),
        hi: crate::ir::Literal::f32(hi),
    }
}

/// Evaluates one value node given operand access. Returns `Err(fact)` for a
/// refusing index read. Guards are handled by the caller.
#[expect(
    tigerstyle::catch_all_on_enum,
    reason = "Owner: noble-maintainers; non-F32 values are the closed NaN observation and non-I64 values the closed zero observation"
)]
pub(crate) fn eval_node(
    op: &crate::ir::Op,
    slot: &[crate::sem::Value],
    get: impl Fn(crate::ir::NodeId) -> crate::sem::Value,
) -> Result<crate::sem::Value, crate::ir::Fact> {
    let f = |x: crate::ir::NodeId| match get(x) {
        crate::sem::Value::F32(b) => f32::from_bits(b),
        _ => f32::NAN,
    };
    let i = |x: crate::ir::NodeId| match get(x) {
        crate::sem::Value::I64(v) => v,
        _ => 0,
    };
    Ok(match op {
        crate::ir::Op::Input(k) | crate::ir::Op::Param(k) => slot[*k as usize],
        crate::ir::Op::Const(l) => crate::sem::Value::of_literal(*l),
        crate::ir::Op::I64Add(a, b) => crate::sem::Value::I64(crate::sem::i64_add(i(*a), i(*b))),
        crate::ir::Op::I64Sub(a, b) => crate::sem::Value::I64(crate::sem::i64_sub(i(*a), i(*b))),
        crate::ir::Op::I64Mul(a, b) => crate::sem::Value::I64(crate::sem::i64_mul(i(*a), i(*b))),
        crate::ir::Op::I64Eq(a, b) => crate::sem::Value::Bool(i(*a) == i(*b)),
        crate::ir::Op::F32Add(a, b) => crate::sem::Value::f32(crate::sem::add(f(*a), f(*b))),
        crate::ir::Op::F32Sub(a, b) => crate::sem::Value::f32(crate::sem::sub(f(*a), f(*b))),
        crate::ir::Op::F32Mul(a, b) => crate::sem::Value::f32(crate::sem::mul(f(*a), f(*b))),
        crate::ir::Op::F32Affine { bias, terms } => crate::sem::Value::f32(crate::sem::affine(
            f32::from_bits(*bias),
            terms.iter().map(|t| {
                (
                    f32::from_bits(t.coefficient),
                    match t.operand {
                        crate::ir::AffineOperand::One => 1.0,
                        crate::ir::AffineOperand::Node(x) => f(x),
                    },
                )
            }),
        )),
        crate::ir::Op::F32Relu(x) => crate::sem::Value::f32(crate::sem::relu(f(*x))),
        crate::ir::Op::F32Reglu { left, gate } => {
            crate::sem::Value::f32(crate::sem::reglu(f(*left), f(*gate)))
        }
        crate::ir::Op::F32Step(s) => crate::sem::Value::f32(crate::sem::step(f(*s))),
        crate::ir::Op::F32Select { bit, off, on } => {
            let (crate::sem::Value::F32(b), crate::sem::Value::F32(o0), crate::sem::Value::F32(o1)) =
                (get(*bit), get(*off), get(*on))
            else {
                return Ok(crate::sem::Value::F32(crate::sem::POS_ZERO));
            };
            crate::sem::Value::F32(crate::sem::select(b, o0, o1))
        }
        crate::ir::Op::F32Index {
            index,
            table,
            below,
            above,
        } => match crate::sem::index_position(f(*index), table.len(), *below, *above) {
            Ok(Some(k)) => get(table[k]),
            Ok(None) => crate::sem::Value::F32(crate::sem::POS_ZERO),
            Err(crate::sem::IndexRefused) => return Err(index_refusal_fact(table.len())),
        },
        crate::ir::Op::Guard { .. } => crate::sem::Value::Bool(true),
    })
}

/// Tests a guard root.
pub(crate) fn guard_holds(
    checked: &crate::check::CheckedGraph,
    fact: &crate::ir::Fact,
    value: crate::ir::NodeId,
    get: impl Fn(crate::ir::NodeId) -> crate::sem::Value,
) -> bool {
    let group = crate::facts::group_values(fact, &get);
    let shape = match &checked.graph().nodes[value.index()].op {
        crate::ir::Op::F32Index { table, .. } => Some(table.len() as u32),
        _ => None,
    };
    crate::facts::holds(fact, get(value), &group, shape)
}

/// Graph outputs as fresh values.
pub(crate) fn collect_outputs(
    checked: &crate::check::CheckedGraph,
    get: impl Fn(crate::ir::NodeId) -> crate::sem::Value,
) -> alloc::vec::Vec<crate::sem::Value> {
    checked.graph().outputs.iter().map(|o| get(*o)).collect()
}

/// L3: the normative CPU reference interpreter. Visits nodes in canonical
/// order; the first failing guard (or refusing index read) in canonical
/// order is the observation. Cancellation is probed when evaluation first
/// enters a new layer; fuel is charged per node.
#[must_use]
pub fn reference(
    checked: &crate::check::CheckedGraph,
    inputs: &[crate::sem::Value],
    snapshot: Option<&crate::snapshot::ParamSnapshot>,
    control: &mut Control<'_>,
) -> Report {
    let slots = match preflight(checked, inputs, snapshot) {
        Ok(v) => v,
        Err(d) => return Report::refused(d),
    };
    let bytes = reference_scratch_bytes(checked);
    if let Err(d) = scratch_preflight(bytes, control) {
        return Report::refused(d);
    }
    let _lease = ScratchLease::acquire(control.scratch.as_deref_mut(), bytes);
    let g = checked.graph();
    let mut vals: alloc::vec::Vec<crate::sem::Value> =
        alloc::vec::Vec::with_capacity(g.nodes.len());
    let mut evaluated = 0u64;
    let mut entered: Option<u32> = None;
    for (i, node) in g.nodes.iter().enumerate() {
        let id = crate::ir::NodeId(i as u32);
        let layer = checked.layer(id);
        if entered.is_none_or(|e| layer > e) {
            entered = Some(layer);
            if control.cancel.is_some_and(|c| c(layer)) {
                return Report {
                    obs: Obs::Cancelled,
                    evaluated_nodes: evaluated,
                };
            }
        }
        if control.fuel.is_some_and(|f| evaluated >= f) {
            return Report {
                obs: Obs::Exhausted(crate::diag::BudgetKind::Fuel),
                evaluated_nodes: evaluated,
            };
        }
        evaluated += 1;
        if let Some(count) = control.evaluated_nodes.as_deref_mut() {
            *count += 1;
        }
        let get = |x: crate::ir::NodeId| vals[x.index()];
        if let crate::ir::Op::Guard { fact, value } = &node.op {
            if !guard_holds(checked, fact, *value, get) {
                return Report {
                    obs: Obs::GuardFailed {
                        node: id,
                        fact: fact.clone(),
                    },
                    evaluated_nodes: evaluated,
                };
            }
            vals.push(crate::sem::Value::Bool(true));
            continue;
        }
        match eval_node(&node.op, &slots, get) {
            Ok(v) => vals.push(v),
            Err(fact) => {
                return Report {
                    obs: Obs::GuardFailed { node: id, fact },
                    evaluated_nodes: evaluated,
                };
            }
        }
    }
    Report {
        obs: Obs::Normal(collect_outputs(checked, |x| vals[x.index()])),
        evaluated_nodes: evaluated,
    }
}

/// The first failure of the reference semantics over complete `F32`
/// forward values (one per node, as [`crate::grad::forward`] computes them):
/// the first failing guard or refusing `index` read in canonical order, with
/// the failing fact. Values before that node are the reference
/// interpreter's, so this is exactly where [`reference`] would report
/// `G-FACT-FALSE` (design D10).
#[must_use]
pub fn first_failure(
    checked: &crate::check::CheckedGraph,
    vals: &[f32],
) -> Option<(crate::ir::NodeId, crate::ir::Fact)> {
    first_failure_observed(checked, vals, &mut None)
}

pub(crate) fn first_failure_observed(
    checked: &crate::check::CheckedGraph,
    vals: &[f32],
    evaluated_nodes: &mut Option<&mut u64>,
) -> Option<(crate::ir::NodeId, crate::ir::Fact)> {
    let get = |x: crate::ir::NodeId| crate::sem::Value::f32(vals[x.index()]);
    for (i, node) in checked.graph().nodes.iter().enumerate() {
        let id = crate::ir::NodeId(i as u32);
        if matches!(
            node.op,
            crate::ir::Op::Guard { .. } | crate::ir::Op::F32Index { .. }
        ) {
            if let Some(counter) = evaluated_nodes.as_deref_mut() {
                *counter += 1;
            }
        }
        match &node.op {
            crate::ir::Op::Guard { fact, value } if !guard_holds(checked, fact, *value, get) => {
                return Some((id, fact.clone()));
            }
            crate::ir::Op::F32Index {
                index,
                table,
                below,
                above,
            } if crate::sem::index_position(vals[index.index()], table.len(), *below, *above)
                .is_err() =>
            {
                return Some((id, index_refusal_fact(table.len())));
            }
            _ => {}
        }
    }
    None
}

/// Helper: `F32` inputs from `f32` values.
#[must_use]
pub fn f32_inputs(values: &[f32]) -> alloc::vec::Vec<crate::sem::Value> {
    values.iter().map(|v| crate::sem::Value::f32(*v)).collect()
}

/// Helper: `I64` inputs.
#[must_use]
pub fn i64_inputs(values: &[i64]) -> alloc::vec::Vec<crate::sem::Value> {
    values.iter().map(|v| crate::sem::Value::I64(*v)).collect()
}

/// The scalar type of a value node's output (for evaluators' buffers).
#[must_use]
pub fn value_type(
    checked: &crate::check::CheckedGraph,
    n: crate::ir::NodeId,
) -> Option<crate::ir::ScalarType> {
    checked.ty(n)
}
