//! Independent validation of the **actual packed instruction stream** against its checked graph
//! (issue #43, the graph-to-packed-schedule edge).
//!
//! [`crate::layout`] validates a *declaration*: register map, constant and slot tables, register
//! count, layer ranges and failure-suffix minima. Instruction bodies stay the scheduler's own
//! business there, which leaves the real gap this module closes: a stream may name the right
//! registers and still compute the wrong operation, the wrong operand order or the wrong
//! destination.
//!
//! The relation checked here is the Rust counterpart of the Lean model's `bodiesMatch` and
//! `operandsDefined` over the total I64/Bool fragment
//! (`proofs/foundation/NobleFoundation/PackedSchedule.lean`):
//!
//! - **Declarations first.** `layout::check` runs here, so a caller cannot validate bodies against
//!   a layout that was never admitted, and its refusal keeps its own class.
//! - **Ordered bodies.** The stream is in layer order and each layer's instruction-emitting nodes
//!   are in canonical order, so position `i` must be *exactly* the body the graph node at position
//!   `i` supplies under the declared register map: operation identity, ordered source registers and
//!   destination register. Operand order is part of the body, so a swapped-operand stream is
//!   refused instead of accepted because both registers happen to be reachable.
//! - **Shape.** The stream length is exactly the number of instruction-emitting nodes. That each
//!   declared layer range holds exactly that layer's count is a declaration obligation, refused
//!   by `layout::check` as `Declarations(LayerCoverage)`.
//! - **A caller's cap.** [`check_within`] refuses inputs whose [`declared_work`] exceeds a cap as
//!   `Exhausted`, before reading anything but their lengths; within the cap it is [`check`].
//!
//! Definition-before-use, single-writer registers and ordered output retention are *implied* by
//! those obligations for this slice rather than checked a second time: the graph is acyclic, the
//! declared register map is injective over the scheduled nodes, and a matching body names its own
//! node's register as destination. A slice with reordering freedom inside a layer, or with guards
//! that write, would have to check them explicitly; this is stated so a future slice cannot
//! inherit the implication by accident.
//!
//! Guard ordering and `fail_suffix_min` behavior have a separate Lean model and a checked real
//! artifact, but there is no general Lean-to-Rust correspondence theorem. Not covered: F32 (it
//! needs #38's expectations and its own semantics), byte-level decoding (there is no decoder here),
//! and anything about the Rust compiler, the host or the engine.

/// One instruction of the packed I64/Bool stream. Computation instructions carry no node identity:
/// the stream is in layer order and each layer's instruction-emitting nodes are in canonical order,
/// so the graph supplies the identity the validator compares against. A guard carries its node
/// identity because it has no destination register to identify that node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum I64Instruction {
    /// Wrapping `a + b` into `d`.
    Add(u32, u32, u32),
    /// Wrapping `a - b` into `d`.
    Sub(u32, u32, u32),
    /// Wrapping `a * b` into `d`.
    Mul(u32, u32, u32),
    /// `a = b` into `d`.
    Eq(u32, u32, u32),
    /// A guard; it writes no register.
    Guard {
        /// Canonical index of the guarded node.
        node: u32,
    },
}

/// A refused packed stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamRefusal {
    /// The graph is not the total I64/Bool fragment this validator covers.
    UnsupportedProfile,
    /// The declared layout was refused; the class is the layout's own.
    Declarations(crate::layout::Refusal),
    /// The stream length is not the number of instruction-emitting nodes.
    StreamShape,
    /// An instruction body is not the body its node supplies, or its destination is not the node's
    /// declared register.
    BodyMismatch,
    /// The inputs declare more work than the caller's cap ([`check_within`]). Nothing after the
    /// profile check was read, so this is not a verdict on the stream.
    Exhausted {
        /// The work the inputs declare ([`declared_work`]).
        work: u64,
        /// The caller's cap.
        cap: u64,
    },
}

/// The nodes that emit an instruction, in layer order then canonical order. Computed from the
/// graph's layer assignment, never from the stream.
///
/// One counting pass and one placing pass bucket the nodes by layer, so the cost is linear in the
/// node and layer counts; rescanning every node once per layer was quadratic for deep graphs.
fn instruction_nodes(
    checked: &crate::check::CheckedGraph,
    layers: &[(u32, u32)],
) -> alloc::vec::Vec<crate::ir::NodeId> {
    let graph = checked.graph();
    let emitting = |index: usize| -> Option<usize> {
        let layer = checked.layer(crate::ir::NodeId(index as u32)) as usize;
        let is_table_entry = matches!(
            graph.nodes[index].op,
            crate::ir::Op::Input(_) | crate::ir::Op::Param(_) | crate::ir::Op::Const(_)
        );
        (layer < layers.len() && !is_table_entry).then_some(layer)
    };
    let mut counts = alloc::vec![0usize; layers.len()];
    for index in 0..graph.nodes.len() {
        if let Some(layer) = emitting(index) {
            counts[layer] += 1;
        }
    }
    let mut next = alloc::vec::Vec::with_capacity(layers.len());
    let mut total = 0usize;
    for count in &counts {
        next.push(total);
        total += count;
    }
    let mut nodes = alloc::vec![crate::ir::NodeId(0); total];
    for index in 0..graph.nodes.len() {
        if let Some(layer) = emitting(index) {
            nodes[next[layer]] = crate::ir::NodeId(index as u32);
            next[layer] += 1;
        }
    }
    nodes
}

fn register(layout: &crate::layout::PackedLayout, id: crate::ir::NodeId) -> Option<u32> {
    layout
        .reg
        .get(id.0 as usize)
        .copied()
        .filter(|slot| *slot != u32::MAX)
}

/// The body the graph supplies for one instruction-emitting node under the declared register map.
/// `None` means a register-writing body's operand or destination has no declared register, which is
/// already a declaration refusal; the caller reports `BodyMismatch` rather than inventing a register.
///
/// A guard writes no register by construction, so it must **not** be routed through the destination
/// lookup: `layout::check` requires its `u32::MAX` sentinel, and a demanded destination would refuse
/// every genuine guarded graph (issue #85).
fn expected(
    checked: &crate::check::CheckedGraph,
    layout: &crate::layout::PackedLayout,
    id: crate::ir::NodeId,
) -> Option<I64Instruction> {
    Some(match checked.graph().nodes[id.0 as usize].op {
        crate::ir::Op::Guard { .. } => I64Instruction::Guard { node: id.0 },
        crate::ir::Op::I64Add(a, b) => I64Instruction::Add(
            register(layout, id)?,
            register(layout, a)?,
            register(layout, b)?,
        ),
        crate::ir::Op::I64Sub(a, b) => I64Instruction::Sub(
            register(layout, id)?,
            register(layout, a)?,
            register(layout, b)?,
        ),
        crate::ir::Op::I64Mul(a, b) => I64Instruction::Mul(
            register(layout, id)?,
            register(layout, a)?,
            register(layout, b)?,
        ),
        crate::ir::Op::I64Eq(a, b) => I64Instruction::Eq(
            register(layout, id)?,
            register(layout, a)?,
            register(layout, b)?,
        ),
        _ => return None,
    })
}

/// Validate a packed I64/Bool stream against the graph it claims to implement.
///
/// The work is linear in the stream, node and layer counts after the declaration check, whose own
/// cost is `O((n + k) log n)` (`crate::layout::check`).
pub fn check(
    checked: &crate::check::CheckedGraph,
    layout: &crate::layout::PackedLayout,
    ins: &[I64Instruction],
) -> Result<(), StreamRefusal> {
    if checked.graph().profile != crate::ir::SubProfile::I64 {
        return Err(StreamRefusal::UnsupportedProfile);
    }
    crate::layout::check(checked, layout).map_err(StreamRefusal::Declarations)?;
    let nodes = instruction_nodes(checked, &layout.layers);
    if nodes.len() != ins.len() {
        return Err(StreamRefusal::StreamShape);
    }
    for (index, id) in nodes.iter().enumerate() {
        let want = expected(checked, layout, *id).ok_or(StreamRefusal::BodyMismatch)?;
        if ins[index] != want {
            return Err(StreamRefusal::BodyMismatch);
        }
    }
    Ok(())
}

/// The work [`check`] is bounded by: the graph's nodes, every declared table, the layer and
/// failure-suffix entries, and the stream's instructions. Only lengths are read, so a padded
/// declaration is counted before any of it is walked.
#[must_use]
pub fn declared_work(
    checked: &crate::check::CheckedGraph,
    layout: &crate::layout::PackedLayout,
    ins: &[I64Instruction],
) -> u64 {
    [
        checked.graph().nodes.len(),
        layout.reg.len(),
        layout.consts.len(),
        layout.slot_regs.len(),
        layout.layers.len(),
        layout.fail_suffix_min.len(),
        ins.len(),
    ]
    .into_iter()
    .fold(0u64, |total, count| total.saturating_add(count as u64))
}

/// [`check`] under a caller's cap on [`declared_work`]. Inputs that declare more work than `cap`
/// are refused as [`StreamRefusal::Exhausted`] before any declaration, table or body is read, so
/// exhaustion is never success and never a verdict on the stream. Only the constant-time profile
/// check comes first. Within the cap the result is exactly [`check`]'s.
pub fn check_within(
    checked: &crate::check::CheckedGraph,
    layout: &crate::layout::PackedLayout,
    ins: &[I64Instruction],
    cap: u64,
) -> Result<(), StreamRefusal> {
    if checked.graph().profile != crate::ir::SubProfile::I64 {
        return Err(StreamRefusal::UnsupportedProfile);
    }
    let work = declared_work(checked, layout, ins);
    if work > cap {
        return Err(StreamRefusal::Exhausted { work, cap });
    }
    check(checked, layout, ins)
}
