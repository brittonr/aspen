#![expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-maintainers; the module path names the domain and the item name keeps its canonical operation spelling"
)]

//! Independent validation of a declared packed layout against its graph.
//!
//! The scheduler stays untrusted: this module never regenerates the schedule and
//! never reads an instruction body. It checks the declared compiler revision and build key
//! against the selected compiler and the checker's own key for the graph, the declared register
//! map, the constant and slot tables and the register count against the graph's own ops, and
//! the layer ranges and failure-suffix minima against the checker's layer
//! assignment, so a wrong declaration is refused rather than compared with another
//! copy of itself. Each table entry must account for a distinct node, and the layer
//! ranges must tile the instruction stream with each layer's exact count. It is the
//! Rust side of the correspondence the Lean model in
//! `proofs/foundation/NobleFoundation/PackedSchedule.lean` states abstractly;
//! **no correspondence theorem binds these bytes to that model**.

/// Layout declaration revision bound into every accepted layout.
pub const LAYOUT_REVISION: &str = "ngraph-layout/0";

/// Layout options the compiler builds every packed layout under, bound into its build key.
const LAYOUT_OPTIONS: &str = "order=(layer,canonical);regs=dense;one-reg=last";

/// `PackedLayoutKey = H(noble.ngraph.packed-layout.v0 ‖ GraphLogicalId ‖ compiler revision ‖ layout
/// options)`: the build key every layout of the graph `graph` is declared under. It binds a layout
/// to its graph and its compiler; it is a build key, not semantic identity.
#[must_use]
pub fn packed_layout_key(
    ids: &dyn crate::identity::IdentityScheme,
    graph: &crate::ir::Digest,
) -> crate::ir::Digest {
    let mut canon = crate::ir::Canon::new(ids, "noble.ngraph.packed-layout.v0");
    canon.bytes(graph);
    canon.str(crate::compiled::COMPILER_REVISION);
    canon.str(LAYOUT_OPTIONS);
    canon.finish()
}

/// The declared packed layout of one compiled graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackedLayout {
    /// Layout declaration revision.
    pub revision: alloc::string::String,
    /// Scheduler/compiler revision the layout was built by.
    pub compiler_revision: alloc::string::String,
    /// Declared register count, including the constant-one register.
    pub regs: u32,
    /// Declared register of each node, in node order (`u32::MAX` for guards).
    pub reg: alloc::vec::Vec<u32>,
    /// Declared constant registers and their exact bits.
    pub consts: alloc::vec::Vec<(u32, u64)>,
    /// Declared interface-slot registers and their slot ordinals.
    pub slot_regs: alloc::vec::Vec<(u32, u32)>,
    /// Declared instruction range `(start, end)` of each layer.
    pub layers: alloc::vec::Vec<(u32, u32)>,
    /// Declared failure-suffix minimum of each layer, plus one trailing entry.
    pub fail_suffix_min: alloc::vec::Vec<u32>,
    /// Build key the layout was produced under.
    pub layout_key: crate::ir::Digest,
}

/// Why a layout was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The layout declaration revision is not the current one.
    StaleRevision,
    /// The declared compiler revision is not the selected compiler's.
    StaleCompilerRevision,
    /// The declared build key is not the checked graph's packed-layout key under the selected
    /// compiler revision and layout options.
    LayoutKey,
    /// The register map does not have one entry per node.
    RegisterMapShape,
    /// A declared register lies outside the declared register count.
    RegisterOutOfRange,
    /// Two distinct non-guard nodes share a register.
    AliasedRegisters,
    /// The declared register count is not one register per non-guard node plus the constant-one
    /// register: a count without the constant-one register, or with headroom beyond it.
    RegisterCount,
    /// A node holds the constant-one register, the last declared register, which
    /// `AffineOperand::One` reads as exact `1.0` (issue #104).
    OneRegisterHeld,
    /// A constant register does not name exactly one constant node's bits.
    ConstantTable,
    /// A slot register does not name exactly one interface input node.
    SlotTable,
    /// A guard node was declared a register.
    GuardRegister,
    /// The declared layer table does not have one range per assigned layer.
    LayerShape,
    /// A layer range is not ordered inside itself or against its predecessor.
    LayerOrder,
    /// The declared failure-suffix minima are not the graph's own prefix minima.
    FailSuffixMismatch,
    /// The declared layer ranges do not cover exactly the graph's instruction nodes.
    LayerCoverage,
}

fn guard(op: &crate::ir::Op) -> bool {
    matches!(op, crate::ir::Op::Guard { .. })
}

/// The exact bits a constant node occupies in its register, for every literal
/// kind the compiler packs: `I64` as wrapping unsigned bits, `Bool` as 0 or 1 and
/// `F32` as its binary32 bits.
fn constant_bits(lit: crate::ir::Literal) -> u64 {
    match lit {
        crate::ir::Literal::I64(value) => value.cast_unsigned(),
        crate::ir::Literal::Bool(value) => u64::from(value),
        crate::ir::Literal::F32(bits) => u64::from(bits),
    }
}

fn fails_suffix(node: &crate::ir::Node) -> bool {
    match &node.op {
        crate::ir::Op::Guard { .. } => true,
        crate::ir::Op::F32Index { below, above, .. } => {
            *below == crate::ir::OutOfRange::Refuse || *above == crate::ir::OutOfRange::Refuse
        }
        _ => false,
    }
}

/// The first refusal of the register map, in node order: a guard given a register, a register
/// outside the declared count, or a register an earlier non-guard node already holds.
///
/// The aliasing scan sorts `(register, node)` pairs once instead of searching every earlier
/// register for each node, so it costs `O(n log n)` rather than `O(n^2)`, while still reporting the
/// refusal at the earliest node exactly as a node-by-node walk would. Returns the highest declared
/// register when the map is accepted.
fn register_map(graph: &crate::ir::Graph, layout: &PackedLayout) -> Result<Option<u32>, Refusal> {
    let mut first: Option<(usize, Refusal)> = None;
    let mut earliest = |index: usize, refusal: Refusal| {
        if first.is_none_or(|(at, _)| index < at) {
            first = Some((index, refusal));
        }
    };
    let mut held = alloc::vec::Vec::with_capacity(graph.nodes.len());
    let mut highest: Option<u32> = None;
    for (index, node) in graph.nodes.iter().enumerate() {
        let declared = layout
            .reg
            .get(index)
            .copied()
            .ok_or(Refusal::RegisterMapShape)?;
        if guard(&node.op) {
            if declared != u32::MAX {
                earliest(index, Refusal::GuardRegister);
            }
            continue;
        }
        if declared >= layout.regs {
            earliest(index, Refusal::RegisterOutOfRange);
            continue;
        }
        held.push((declared, index));
        highest = Some(highest.map_or(declared, |current| current.max(declared)));
    }
    // Sorted by register, then node: within one register's run every node after the first holds
    // an alias, and the earliest of those is where a walk in node order would have stopped.
    held.sort_unstable();
    for pair in held.windows(2) {
        if pair[0].0 == pair[1].0 {
            earliest(pair[1].1, Refusal::AliasedRegisters);
        }
    }
    match first {
        Some((_, refusal)) => Err(refusal),
        None => Ok(highest),
    }
}

/// How many nodes carry each `(register, key)` pair, for the nodes `key` selects. One pass over
/// the graph replaces a full rescan per declared table entry.
fn table_counts<K: Ord + Copy>(
    graph: &crate::ir::Graph,
    layout: &PackedLayout,
    key: impl Fn(&crate::ir::Op) -> Option<K>,
) -> alloc::collections::BTreeMap<(u32, K), usize> {
    let mut counts = alloc::collections::BTreeMap::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        if let (Some(key), Some(&register)) = (key(&node.op), layout.reg.get(index)) {
            *counts.entry((register, key)).or_insert(0) += 1;
        }
    }
    counts
}

/// Each declared layer range starts where its predecessor ends and holds exactly the
/// instruction-emitting nodes the checker assigned to that layer, so the ranges tile the
/// instruction stream; slots and constants are table entries, not instructions. The counts come
/// from the checker's layer assignment, never from the declaration, and the work is linear in the
/// node and layer counts. A same-sized table whose last range ends at the right total but that
/// empties one layer, or moves an instruction into a neighbouring layer, is refused.
fn layer_coverage(
    checked: &crate::check::CheckedGraph,
    layout: &PackedLayout,
    layer_count: usize,
) -> Result<(), Refusal> {
    let mut per_layer = alloc::vec![0u32; layer_count];
    for (index, node) in checked.graph().nodes.iter().enumerate() {
        if matches!(
            node.op,
            crate::ir::Op::Input(_) | crate::ir::Op::Param(_) | crate::ir::Op::Const(_)
        ) {
            continue;
        }
        let index = u32::try_from(index).map_err(|_| Refusal::LayerCoverage)?;
        let layer = usize::try_from(checked.layer(crate::ir::NodeId(index)))
            .map_err(|_| Refusal::LayerShape)?;
        let count = per_layer.get_mut(layer).ok_or(Refusal::LayerShape)?;
        *count = count.checked_add(1).ok_or(Refusal::LayerCoverage)?;
    }
    let mut at = 0u32;
    for (&(start, end), &count) in layout.layers.iter().zip(&per_layer) {
        if start != at || end.checked_sub(start) != Some(count) {
            return Err(Refusal::LayerCoverage);
        }
        at = end;
    }
    Ok(())
}

/// Validates a declared layout against the graph it claims to pack. The declared compiler
/// revision must be the selected compiler's, and the declared build key must be the key the
/// checker computed for the graph (`CheckedGraph::packed_layout_key`); a key the layout supplies
/// is never its own expected value. The layer
/// assignment comes from the checker, not from the scheduler: the declared layer
/// ranges and failure-suffix minima are compared against it, never regenerated.
///
/// The work is `O((n + k) log n)` for `n` graph nodes and `k` declared table entries, and every
/// allocation is sized by the graph, not by the declaration.
pub fn check(checked: &crate::check::CheckedGraph, layout: &PackedLayout) -> Result<(), Refusal> {
    let graph = checked.graph();
    if layout.revision != LAYOUT_REVISION {
        return Err(Refusal::StaleRevision);
    }
    if layout.compiler_revision != crate::compiled::COMPILER_REVISION {
        return Err(Refusal::StaleCompilerRevision);
    }
    if layout.layout_key != checked.packed_layout_key() {
        return Err(Refusal::LayoutKey);
    }
    if layout.reg.len() != graph.nodes.len() {
        return Err(Refusal::RegisterMapShape);
    }
    let highest = register_map(graph, layout)?;
    // The layout options bind `regs=dense;one-reg=last`: one register per non-guard node, then
    // the constant-one register that `AffineOperand::One` reads, which is the last and which no
    // node holds (issue #104). A count without it, or with headroom beyond it, is a wrong
    // declaration. With every node's register distinct and below the count, refusing a node at
    // the last register leaves the nodes exactly the registers below it.
    let held = graph.nodes.iter().filter(|node| !guard(&node.op)).count();
    let one = u32::try_from(held).map_err(|_| Refusal::RegisterCount)?;
    if one.checked_add(1) != Some(layout.regs) {
        return Err(Refusal::RegisterCount);
    }
    if highest.is_some_and(|register| register >= one) {
        return Err(Refusal::OneRegisterHeld);
    }
    let mut constants = table_counts(graph, layout, |op| match op {
        crate::ir::Op::Const(lit) => Some(constant_bits(*lit)),
        _ => None,
    });
    // Each declared row consumes the constant it names, so a row repeated in place of another
    // constant's row finds nothing the second time. With the count check below, the table is a
    // bijection with the graph's constants rather than a list of individually valid rows.
    for &(register, bits) in &layout.consts {
        if constants.remove(&(register, bits)) != Some(1) {
            return Err(Refusal::ConstantTable);
        }
    }
    // Both slot kinds share one table in the compiler, so an entry may describe an input node or a
    // parameter node. Each declared slot is still covered exactly once: a second entry for the same
    // slot, a swapped kind, or a register that no node of that slot carries is refused.
    let slots = table_counts(graph, layout, |op| match op {
        crate::ir::Op::Input(slot) | crate::ir::Op::Param(slot) => Some(*slot),
        _ => None,
    });
    let mut covered = alloc::vec![false; graph.slots.len()];
    for &(register, slot) in &layout.slot_regs {
        if slots.get(&(register, slot)).copied() != Some(1) {
            return Err(Refusal::SlotTable);
        }
        let Some(seen) = covered.get_mut(slot as usize) else {
            return Err(Refusal::SlotTable);
        };
        if *seen {
            return Err(Refusal::SlotTable);
        }
        *seen = true;
    }
    let declared_constants = layout.consts.len();
    let constant_nodes = graph
        .nodes
        .iter()
        .filter(|node| matches!(node.op, crate::ir::Op::Const(_)))
        .count();
    if declared_constants != constant_nodes {
        return Err(Refusal::ConstantTable);
    }
    let layer_count = usize::try_from(checked.layer_count()).unwrap_or(usize::MAX);
    if layout.layers.len() != layer_count || layout.fail_suffix_min.len() != layer_count + 1 {
        return Err(Refusal::LayerShape);
    }
    let mut previous_end = 0u32;
    for &(start, end) in &layout.layers {
        if start > end || start < previous_end {
            return Err(Refusal::LayerOrder);
        }
        previous_end = end;
    }
    // The graph's own prefix minima: the least canonical index of a node that
    // needs a failure suffix, over every layer at or after each layer.
    let mut minima = alloc::vec![u32::MAX; layer_count + 1];
    let mut at_layer = alloc::vec![u32::MAX; layer_count.max(1)];
    for (index, node) in graph.nodes.iter().enumerate() {
        if !fails_suffix(node) {
            continue;
        }
        let Ok(index) = u32::try_from(index) else {
            return Err(Refusal::FailSuffixMismatch);
        };
        let layer = usize::try_from(checked.layer(crate::ir::NodeId(index)))
            .map_err(|_| Refusal::FailSuffixMismatch)?;
        let slot = at_layer.get_mut(layer).ok_or(Refusal::LayerShape)?;
        *slot = (*slot).min(index);
    }
    for layer in (0..layer_count).rev() {
        minima[layer] = minima[layer + 1].min(at_layer[layer]);
    }
    if minima != layout.fail_suffix_min {
        return Err(Refusal::FailSuffixMismatch);
    }
    layer_coverage(checked, layout, layer_count)?;
    let declared_slots = layout.slot_regs.len();
    let slot_nodes = graph
        .nodes
        .iter()
        .filter(|node| matches!(node.op, crate::ir::Op::Input(_) | crate::ir::Op::Param(_)))
        .count();
    if declared_slots != slot_nodes {
        return Err(Refusal::SlotTable);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::identity::IdentityScheme;

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

    pub(super) fn sample() -> (
        crate::ir::Graph,
        crate::check::CheckedGraph,
        super::PackedLayout,
    ) {
        sample_named("layout-sample")
    }

    fn sample_named(
        name: &str,
    ) -> (
        crate::ir::Graph,
        crate::check::CheckedGraph,
        super::PackedLayout,
    ) {
        let scheme = TestIds;
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&scheme, name),
        );
        let input = builder.input(crate::ir::ScalarType::I64);
        let three = builder.i64_const(3);
        let sum = builder.op(crate::ir::Op::I64Add(input, three));
        builder.guard(crate::ir::Fact::Bit, sum);
        builder.output(sum);
        let graph = builder.finish();
        let checked = crate::check::check(&scheme, graph.clone()).expect("the sample checks");
        let layout = crate::compiled::Compiled::new(&scheme, &checked).packed_layout();
        (graph, checked, layout)
    }

    #[test]
    fn a_real_compiled_layout_is_accepted() {
        let (_graph, checked, layout) = sample();
        assert_eq!(layout.revision, super::LAYOUT_REVISION);
        assert!(super::check(&checked, &layout).is_ok());
    }

    /// #98: the declared compiler revision and build key are part of admission, and the expected
    /// key is the checker's, never the key the layout supplies.
    #[test]
    fn compiler_revision_and_build_key_are_part_of_layout_admission() {
        let (_graph, checked, layout) = sample();
        assert_eq!(super::check(&checked, &layout), Ok(()));

        let mut stale = layout.clone();
        stale.compiler_revision = alloc::string::String::from("ngraph-layered/stale");
        assert_eq!(
            super::check(&checked, &stale),
            Err(super::Refusal::StaleCompilerRevision)
        );

        let mut wrong_key = layout.clone();
        wrong_key.layout_key[0] ^= 1;
        assert_eq!(
            super::check(&checked, &wrong_key),
            Err(super::Refusal::LayoutKey)
        );

        let mut both = stale.clone();
        both.layout_key = wrong_key.layout_key;
        assert_eq!(
            super::check(&checked, &both),
            Err(super::Refusal::StaleCompilerRevision)
        );

        // A structurally identical layout compiled for another graph carries that graph's own,
        // internally consistent key, and is refused for this one.
        let (_other_graph, other_checked, other) = sample_named("layout-sample-other");
        assert_ne!(other_checked.id(), checked.id());
        assert_eq!(super::check(&other_checked, &other), Ok(()));
        assert_eq!(
            super::check(&checked, &other),
            Err(super::Refusal::LayoutKey)
        );
    }

    #[test]
    fn mutated_layouts_are_refused_for_their_own_premise() {
        let (graph, checked, layout) = sample();
        let check = |layout: &super::PackedLayout| super::check(&checked, layout);
        let mut aliased = layout.clone();
        aliased.reg = alloc::vec![0; layout.reg.len()];
        assert_eq!(check(&aliased), Err(super::Refusal::AliasedRegisters));

        let mut out_of_range = layout.clone();
        out_of_range.regs = 0;
        assert_eq!(
            check(&out_of_range),
            Err(super::Refusal::RegisterOutOfRange)
        );

        let mut wrong_bits = layout.clone();
        if let Some(entry) = wrong_bits.consts.first_mut() {
            entry.1 = entry.1.wrapping_add(1);
        }
        assert_eq!(check(&wrong_bits), Err(super::Refusal::ConstantTable));

        let mut wrong_slot = layout.clone();
        if let Some(entry) = wrong_slot.slot_regs.first_mut() {
            entry.1 = entry.1.saturating_add(9);
        }
        assert_eq!(check(&wrong_slot), Err(super::Refusal::SlotTable));

        let mut dropped = layout.clone();
        dropped.consts.clear();
        assert_eq!(check(&dropped), Err(super::Refusal::ConstantTable));

        let mut stale = layout.clone();
        stale.revision = alloc::string::String::from("ngraph-layout/1");
        assert_eq!(check(&stale), Err(super::Refusal::StaleRevision));

        let mut wrong_count = layout.clone();
        wrong_count.regs = layout.regs.saturating_add(2);
        assert_eq!(check(&wrong_count), Err(super::Refusal::RegisterCount));

        // A count one short keeps every node's register in range but drops the constant-one
        // register (issue #104).
        let mut without_one = layout.clone();
        without_one.regs = layout.regs.saturating_sub(1);
        assert_eq!(check(&without_one), Err(super::Refusal::RegisterCount));

        // Declaring the sample's real guard a register is refused by the guard rule.
        let mut guard_register = layout.clone();
        let guard_index = graph
            .nodes
            .iter()
            .position(|node| matches!(node.op, crate::ir::Op::Guard { .. }))
            .expect("the sample declares one guard");
        guard_register.reg[guard_index] = 0;
        assert_eq!(check(&guard_register), Err(super::Refusal::GuardRegister));

        let mut dropped_layer = layout.clone();
        dropped_layer.layers.pop();
        assert_eq!(check(&dropped_layer), Err(super::Refusal::LayerShape));

        let mut unordered_layer = layout.clone();
        unordered_layer.layers = alloc::vec![(5, 2); layout.layers.len()];
        assert_eq!(check(&unordered_layer), Err(super::Refusal::LayerOrder));

        let mut smoothed_suffix = layout.clone();
        smoothed_suffix.fail_suffix_min = alloc::vec![u32::MAX; layout.fail_suffix_min.len()];
        assert_eq!(
            check(&smoothed_suffix),
            Err(super::Refusal::FailSuffixMismatch)
        );

        let mut shifted_suffix = layout.clone();
        if let Some(entry) = shifted_suffix.fail_suffix_min.last_mut() {
            *entry = 0;
        }
        assert_eq!(
            check(&shifted_suffix),
            Err(super::Refusal::FailSuffixMismatch)
        );
    }

    /// Two register-map faults in one declaration: the refusal is the one at the earlier node,
    /// in either order, as a walk in node order reports it. The aliasing scan sorts registers, so
    /// it must not let a later alias outrank an earlier out-of-range register, or the reverse.
    #[test]
    fn the_earlier_register_fault_is_the_refusal() {
        let (graph, checked, layout) = sample();
        let check = |layout: &super::PackedLayout| super::check(&checked, layout);
        let held: alloc::vec::Vec<usize> = graph
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| !matches!(node.op, crate::ir::Op::Guard { .. }))
            .map(|(index, _)| index)
            .collect();
        let [first, second, third] = held[..] else {
            panic!("the sample holds three registers, found {}", held.len());
        };

        let mut alias_first = layout.clone();
        alias_first.reg[second] = layout.reg[first];
        alias_first.reg[third] = layout.regs;
        assert_eq!(check(&alias_first), Err(super::Refusal::AliasedRegisters));

        let mut range_first = layout.clone();
        range_first.reg[second] = layout.regs;
        range_first.reg[third] = layout.reg[first];
        assert_eq!(check(&range_first), Err(super::Refusal::RegisterOutOfRange));
    }

    /// A constant row repeated in place of another constant's row keeps the table's length and
    /// every row individually valid, but leaves the second constant undeclared (issue #94).
    #[test]
    fn a_duplicated_constant_row_cannot_hide_another_constant() {
        let scheme = TestIds;
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::I64,
            crate::ir::Subject::authored(&scheme, "duplicate-constant-row"),
        );
        let three = builder.i64_const(3);
        let four = builder.i64_const(4);
        let sum = builder.op(crate::ir::Op::I64Add(three, four));
        builder.output(sum);
        let checked =
            crate::check::check(&scheme, builder.finish()).expect("the two-constant graph checks");
        let layout = crate::compiled::Compiled::new(&scheme, &checked).packed_layout();
        assert_eq!(layout.consts.len(), 2);
        assert_ne!(layout.consts[0], layout.consts[1]);
        assert_eq!(super::check(&checked, &layout), Ok(()));

        let mut duplicated = layout.clone();
        duplicated.consts[1] = duplicated.consts[0];
        assert_eq!(
            super::check(&checked, &duplicated),
            Err(super::Refusal::ConstantTable)
        );
    }

    /// The layer ranges must hold each layer's own instructions, not merely end at the right
    /// total (issue #95): emptying a range, or shifting an instruction into a neighbouring layer
    /// while every range stays ordered and the last end stays put, is refused.
    #[test]
    fn layer_ranges_must_hold_each_layers_own_instructions() {
        let (_graph, checked, layout) = sample();
        let check = |layout: &super::PackedLayout| super::check(&checked, layout);
        assert_eq!(check(&layout), Ok(()));
        let index = layout
            .layers
            .iter()
            .position(|&(start, end)| start < end)
            .expect("the sample has an instruction-emitting layer");

        let mut emptied = layout.clone();
        emptied.layers[index].0 = emptied.layers[index].1;
        assert_eq!(check(&emptied), Err(super::Refusal::LayerCoverage));

        // Move the first instruction of that layer into the previous layer's range, keeping the
        // ranges contiguous and the total unchanged.
        let mut shifted = layout.clone();
        let previous = index
            .checked_sub(1)
            .expect("an instruction layer follows layer 0");
        shifted.layers[previous].1 = shifted.layers[previous].1.saturating_add(1);
        shifted.layers[index].0 = shifted.layers[index].0.saturating_add(1);
        assert_eq!(check(&shifted), Err(super::Refusal::LayerCoverage));
    }

    /// `AffineOperand::One` reads the last register as exact `1.0` (issue #104). The compiler
    /// reserves it after one register per node; a declaration without it, or one that gives it to
    /// a node, is refused, so an accepted layout always has the register the instruction reads.
    #[test]
    fn f32_affine_one_requires_a_distinct_reserved_register() {
        let scheme = TestIds;
        let mut builder = crate::ir::GraphBuilder::new(
            crate::ir::SubProfile::F32,
            crate::ir::Subject::authored(&scheme, "affine-one-register"),
        );
        let affine = builder.affine(0.0, &[(2.0, crate::ir::AffineOperand::One)]);
        builder.output(affine);
        let checked =
            crate::check::check(&scheme, builder.finish()).expect("the affine graph checks");
        let layout = crate::compiled::Compiled::new(&scheme, &checked).packed_layout();
        assert_eq!(layout.reg, alloc::vec![0]);
        assert_eq!(layout.regs, 2);
        assert_eq!(super::check(&checked, &layout), Ok(()));

        let mut without_one = layout.clone();
        without_one.regs = 1;
        assert_eq!(
            super::check(&checked, &without_one),
            Err(super::Refusal::RegisterCount)
        );

        let mut one_held = layout.clone();
        one_held.reg[0] = 1;
        assert_eq!(
            super::check(&checked, &one_held),
            Err(super::Refusal::OneRegisterHeld)
        );
    }
}
