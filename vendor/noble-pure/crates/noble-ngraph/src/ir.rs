//! `ngraph-ir/0`: the graph IR of design D2 for both sub-profiles.
//!
//! A [`Graph`] is producer data. Nothing in it is trusted until
//! [`crate::check::check`] re-derives types, topology, payload rules, fact
//! provenance and budgets (design D7 step 5, D13: producer metadata is never
//! independent validation).
//!
//! API stability: after the `IR_READY` milestone this module only changes
//! additively (new variants/fields are added with defaults; nothing is renamed),
//! except where a recorded boundary fix needs otherwise: `numeric-graph-v1`
//! D4 made [`ShapeTree`] opaque so that its depth is bounded at construction.

use crate::identity::{IdentityHasher, IdentityScheme};
/// IR version tag (D2).
pub const IR_VERSION: &str = "ngraph-ir/0";
/// Semantic revision of the D3 binary32 contract and the K-NUM-01 `I64` reading.
pub const SEMANTIC_REVISION: &str = "ngraph-sem/0";
/// Revision of the D9 fact-transfer rules implemented by the checker.
pub const FACT_RULE_REVISION: &str = "ngraph-facts/0";

/// The two disjoint sub-profiles of `NumericGraph-Design` (D1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SubProfile {
    /// `ngraph-i64/0`: exact, extraction-only, no derivatives.
    I64,
    /// `ngraph-f32/0`: experimental differentiable slice.
    F32,
}

impl SubProfile {
    /// Stable profile name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::I64 => "ngraph-i64/0",
            Self::F32 => "ngraph-f32/0",
        }
    }

    /// Parses a profile name; unknown names are `None` (caller reports `E-PROFILE`).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "ngraph-i64/0" => Some(Self::I64),
            "ngraph-f32/0" => Some(Self::F32),
            _ => None,
        }
    }

    /// Whether `ty` is a value type of this sub-profile.
    #[must_use]
    pub const fn admits(self, ty: ScalarType) -> bool {
        matches!(
            (self, ty),
            (Self::I64, ScalarType::I64 | ScalarType::Bool) | (Self::F32, ScalarType::F32)
        )
    }
}

/// Scalar value types. `F32` is profile-local (D3); there is no conversion edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScalarType {
    /// K-NUM-01 signed wrapping 64-bit integer.
    I64,
    /// Noble `Bool` (`i64.eq` output only; no `Bool` operations in `/0`).
    Bool,
    /// IEEE 754 binary32, roundTiesToEven.
    F32,
}

impl ScalarType {
    /// Stable type name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::I64 => "i64",
            Self::Bool => "bool",
            Self::F32 => "f32",
        }
    }

    /// ABI width in bytes (D3: 8, 1, 4).
    #[must_use]
    pub const fn abi_bytes(self) -> usize {
        match self {
            Self::I64 => 8,
            Self::Bool => 1,
            Self::F32 => 4,
        }
    }
}

/// Node identity: the index of the node in [`Graph::nodes`]. Each node defines
/// at most one value, so a `NodeId` is also the value identity (D7 `Scalar(id, T)`);
/// `dup` and captures share the same id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Index into [`Graph::nodes`].
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Interface slot kind (D2). `Param` exists only in `ngraph-f32/0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotKind {
    /// Caller-supplied value.
    Input,
    /// Trainable parameter, supplied only through a snapshot (D11).
    Param,
}

/// One ordered interface slot.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Slot {
    /// Input or parameter.
    pub kind: SlotKind,
    /// Scalar type of the slot.
    pub ty: ScalarType,
}

/// A literal payload. `F32` literals are carried as exact binary32 bits; the
/// text form rejects inexact decimals (`E-LITERAL-INEXACT`) and the checker
/// rejects non-finite bits (`E-LITERAL-NONFINITE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Literal {
    /// `I64` literal.
    I64(i64),
    /// `Bool` literal.
    Bool(bool),
    /// `F32` literal as binary32 bits.
    F32(u32),
}

impl Literal {
    /// Type of the literal.
    #[must_use]
    pub const fn ty(self) -> ScalarType {
        match self {
            Self::I64(_) => ScalarType::I64,
            Self::Bool(_) => ScalarType::Bool,
            Self::F32(_) => ScalarType::F32,
        }
    }

    /// `F32` literal from an `f32` value.
    #[must_use]
    pub const fn f32(value: f32) -> Self {
        Self::F32(value.to_bits())
    }
}

/// An `affine` term operand: a graph value or the constant-one operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AffineOperand {
    /// The exact constant `1` (no node).
    One,
    /// A graph value.
    Node(NodeId),
}

/// One `affine` term `(c, x)`; the coefficient is exact binary32 bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AffineTerm {
    /// Coefficient bits (must be finite).
    pub coefficient: u32,
    /// Operand.
    pub operand: AffineOperand,
}

/// Out-of-range rule of `f32.index` (D3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OutOfRange {
    /// Default: guard failure (`G-FACT-FALSE`), no outputs.
    Refuse,
    /// Nearest end of the table.
    Clamp,
    /// `+0`.
    Zero,
}

impl OutOfRange {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Refuse => "refuse",
            Self::Clamp => "clamp",
            Self::Zero => "zero",
        }
    }
}

/// D9 fact kinds. Ranges are numeric (`-0` and `+0` compare equal) and closed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Fact {
    /// `lo ≤ v ≤ hi` (literals of the node's type; not NaN).
    Range {
        /// Lower bound.
        lo: Literal,
        /// Upper bound.
        hi: Literal,
    },
    /// `v` is a finite integer multiple of `2^log2`.
    Grid {
        /// Exponent `k` of the grid step `2^k`.
        log2: i32,
    },
    /// `v` is an exact bit: binary32 `+0` or `1` (`-0` is not a bit), or `I64` 0/1.
    Bit,
    /// Every member of `group` is a bit and at most one (exactly one unless
    /// `allow_zero`) is `1`. Attached to the first member.
    OneHot {
        /// Group members in order.
        group: alloc::vec::Vec<NodeId>,
        /// Whether the all-zero assignment is allowed.
        allow_zero: bool,
    },
    /// `v` is finite (not NaN, not ±∞).
    Finite,
    /// The `index` table operand list has exactly `n` entries (structural).
    Shape(u32),
    /// `v` is a finite integer.
    Integral,
}

impl Fact {
    /// Stable kind name.
    #[must_use]
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::Range { .. } => "range",
            Self::Grid { .. } => "grid",
            Self::Bit => "bit",
            Self::OneHot { .. } => "one-hot",
            Self::Finite => "finite",
            Self::Shape(_) => "shape",
            Self::Integral => "integral",
        }
    }
}

/// The single provenance of a fact record (D9).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Provenance {
    /// Claimed derivable by a transfer rule; the checker re-derives it.
    Derived(alloc::string::String),
    /// Checked on the actual input/param cell before evaluation (`E-FACT-INPUT`).
    InputValidated,
    /// Evidence-backed (#8 witness). Excluded from Phase B (approval P5):
    /// always `E-FACT-UNCHECKED`.
    Evidence(alloc::vec::Vec<u8>),
    /// Established by the named earlier `guard` node.
    Guard(NodeId),
    /// A caller assertion with no allowed provenance: always `E-FACT-UNCHECKED`.
    Asserted,
}

/// A fact record `(node, kind+payload, provenance)`. Graph identity, profile
/// revision and fact-rule revision are bound by the enclosing checked graph.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FactRecord {
    /// Subject value.
    pub node: NodeId,
    /// Fact.
    pub fact: Fact,
    /// Provenance.
    pub provenance: Provenance,
}

/// `ngraph-ir/0` operations (the complete D2 whitelist).
///
/// Binary `I64` operations take `(a, b)` where `a` is the **deeper** stack
/// operand (K-NUM-01): `i64.sub(a, b) = a - b`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    /// Interface slot `k` of kind `Input`.
    Input(u32),
    /// Interface slot `k` of kind `Param` (`f32` only).
    Param(u32),
    /// Literal.
    Const(Literal),
    /// Wrapping `a + b`.
    I64Add(NodeId, NodeId),
    /// Wrapping `a - b`.
    I64Sub(NodeId, NodeId),
    /// Wrapping `a * b`.
    I64Mul(NodeId, NodeId),
    /// `a = b` → `Bool`.
    I64Eq(NodeId, NodeId),
    /// `rn(a + b)`.
    F32Add(NodeId, NodeId),
    /// `rn(a - b)`.
    F32Sub(NodeId, NodeId),
    /// `rn(a × b)`.
    F32Mul(NodeId, NodeId),
    /// Ordered fold `v₀ = bias; vᵢ = rn(vᵢ₋₁ + rn(cᵢ × xᵢ))`.
    F32Affine {
        /// Bias bits (`-0` for a bias-free sum).
        bias: u32,
        /// Terms in stored order.
        terms: alloc::vec::Vec<AffineTerm>,
    },
    /// `relu(x)`: NaN↦NaN, `x > 0 ? x : +0`.
    F32Relu(NodeId),
    /// `rn(left × relu(gate))`.
    F32Reglu {
        /// Left operand.
        left: NodeId,
        /// Gate operand.
        gate: NodeId,
    },
    /// `s ≥ 0.5 ? 1 : +0`; requires the fact `s ∈ [0,1]`.
    F32Step(NodeId),
    /// Bitwise `bit = 1 ? on : off`; requires `bit` to be an exact bit.
    F32Select {
        /// Gate (exact bit).
        bit: NodeId,
        /// Value when the gate is `+0`.
        off: NodeId,
        /// Value when the gate is `1`.
        on: NodeId,
    },
    /// Bitwise `table[index]` or the out-of-range rule; requires `integral`
    /// and a range with `|i| < 2^24`.
    F32Index {
        /// Index value.
        index: NodeId,
        /// Candidates (repeats allowed), `1..T_max` entries.
        table: alloc::vec::Vec<NodeId>,
        /// Rule for `i < 0`.
        below: OutOfRange,
        /// Rule for `i ≥ n`.
        above: OutOfRange,
    },
    /// Root that fails closed (`G-FACT-FALSE`) if `fact` is false of `value`.
    Guard {
        /// Checked fact.
        fact: Fact,
        /// Subject value.
        value: NodeId,
    },
}

impl Op {
    /// Stable operation name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Input(_) => "input",
            Self::Param(_) => "param",
            Self::Const(_) => "const",
            Self::I64Add(..) => "i64.add",
            Self::I64Sub(..) => "i64.sub",
            Self::I64Mul(..) => "i64.mul",
            Self::I64Eq(..) => "i64.eq",
            Self::F32Add(..) => "f32.add",
            Self::F32Sub(..) => "f32.sub",
            Self::F32Mul(..) => "f32.mul",
            Self::F32Affine { .. } => "f32.affine",
            Self::F32Relu(_) => "f32.relu",
            Self::F32Reglu { .. } => "f32.reglu",
            Self::F32Step(_) => "f32.step",
            Self::F32Select { .. } => "f32.select",
            Self::F32Index { .. } => "f32.index",
            Self::Guard { .. } => "guard",
        }
    }

    /// Value operands in canonical order (affine `One` operands omitted).
    #[must_use]
    #[expect(
        tigerstyle::numeric_units,
        reason = "Owner: noble-ngraph-maintainers; the capacity is an abstract operand-count bound in the canonical node grammar, not a time or byte quantity."
    )]
    pub fn operands(&self) -> alloc::vec::Vec<NodeId> {
        let capacity_operands = match self {
            Self::Input(_) | Self::Param(_) | Self::Const(_) => 0,
            Self::F32Affine { terms, .. } => terms.len(),
            Self::F32Index { table, .. } => table.len() + 1,
            Self::Guard {
                fact: Fact::OneHot { group, .. },
                ..
            } => group.len() + 1,
            _ => 3,
        };
        let mut operands = alloc::vec::Vec::with_capacity(capacity_operands);
        self.visit_operands(|n| operands.push(n));
        operands
    }

    /// Visits value operands in canonical order without allocating.
    pub(crate) fn visit_operands(&self, mut visit: impl FnMut(NodeId)) {
        match self {
            Self::Input(_) | Self::Param(_) | Self::Const(_) => {}
            Self::I64Add(a, b)
            | Self::I64Sub(a, b)
            | Self::I64Mul(a, b)
            | Self::I64Eq(a, b)
            | Self::F32Add(a, b)
            | Self::F32Sub(a, b)
            | Self::F32Mul(a, b) => {
                visit(*a);
                visit(*b);
            }
            Self::F32Affine { terms, .. } => {
                for term in terms {
                    if let AffineOperand::Node(n) = term.operand {
                        visit(n);
                    }
                }
            }
            Self::F32Relu(x) | Self::F32Step(x) => visit(*x),
            Self::F32Reglu { left, gate } => {
                visit(*left);
                visit(*gate);
            }
            Self::F32Select { bit, off, on } => {
                visit(*bit);
                visit(*off);
                visit(*on);
            }
            Self::F32Index { index, table, .. } => {
                visit(*index);
                for &n in table {
                    visit(n);
                }
            }
            Self::Guard { value, fact } => {
                visit(*value);
                if let Fact::OneHot { group, .. } = fact {
                    for &n in group {
                        if n != *value {
                            visit(n);
                        }
                    }
                }
            }
        }
    }

    /// Whether this node is a guard root (defines no value).
    #[must_use]
    pub const fn is_guard(&self) -> bool {
        matches!(self, Self::Guard { .. })
    }
}

/// A source location inside the subject: definition identity (`None` = the
/// root body or a closed root Program) and body node index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceRef {
    /// Exact definition identity (`named_call` resolved identity), if any.
    pub definition: Option<u64>,
    /// Node index within that body (source order).
    pub node: u32,
}

/// Extraction provenance of a node (filled by the `I64` extractor; `None`
/// for directly authored graphs). Never part of `GraphLogicalId` and never
/// trusted by the checker.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Origin {
    /// Source node that created this graph node.
    pub site: SourceRef,
    /// Expansion stack (outermost first): named calls, `run`, `dip` sites.
    pub expansion: alloc::vec::Vec<SourceRef>,
}

/// One graph node.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Node {
    /// Operation.
    pub op: Op,
    /// Producer provenance (untrusted metadata).
    pub origin: Option<Origin>,
}

/// How the subject was obtained.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SubjectKind {
    /// Authored directly in `ngraph-ir/0` (approval P2, option B0(b)).
    Authored,
    /// Authored by the bounded, staged Nickel constructor route.
    AuthoredNickel,
    /// Extracted from a kernel-accepted definition or closed Program (`I64`).
    Extracted,
    /// A new subject created by capturing snapshot values (D11 "Capture").
    Captured,
}

impl SubjectKind {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Authored => "authored",
            Self::AuthoredNickel => "authored-nickel",
            Self::Extracted => "extracted",
            Self::Captured => "captured",
        }
    }
}

/// The root subject and its exact closure (bound into `GraphLogicalId`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Subject {
    /// Kind.
    pub kind: SubjectKind,
    /// 32-byte subject identity digest.
    pub identity: [u8; 32],
    /// Exact closure definition identities, in the producer's canonical order.
    pub closure: alloc::vec::Vec<u64>,
}

impl Subject {
    /// A subject identity derived from domain-separated parts under `ids`.
    #[must_use]
    pub fn derive(
        ids: &dyn IdentityScheme,
        kind: SubjectKind,
        parts: &[&[u8]],
        closure: alloc::vec::Vec<u64>,
    ) -> Self {
        let mut h = ids.start();
        h.update(b"noble.ngraph.subject.v0");
        h.update(kind.name().as_bytes());
        for part in parts {
            h.update(&(part.len() as u64).to_le_bytes());
            h.update(part);
        }
        Self {
            kind,
            identity: h.finish(),
            closure,
        }
    }

    /// Subject of a directly authored graph identified by its author name/text.
    #[must_use]
    pub fn authored(ids: &dyn IdentityScheme, name: &str) -> Self {
        Self::derive(
            ids,
            SubjectKind::Authored,
            &[name.as_bytes()],
            alloc::vec::Vec::new(),
        )
    }

    /// A Nickel-authored subject bound to the exact construction-bundle digest.
    #[must_use]
    pub const fn authored_nickel(bundle_digest: [u8; 32]) -> Self {
        Self {
            kind: SubjectKind::AuthoredNickel,
            identity: bundle_digest,
            closure: alloc::vec::Vec::new(),
        }
    }
}

/// The most pairs one [`ShapeTree`] may nest (design D4 of
/// `numeric-graph-v1`).
///
/// Extraction is the only producer of non-flat shapes from Core source, and
/// the Core frontend bounds every value type to 256 structural type nodes
/// (`noble-contracts` `TYPE_CAP`, sent to the kernel as the request's
/// `type_size`; the kernel's `Ty::size` counts one node per constructor). A
/// path through `d` nested pairs needs at least `2d + 1` nodes, because every
/// pair on it has a sibling subtree and the path ends in a leaf, so an
/// extracted interface item nests at most 127 pairs. Authored files and the
/// builder get the same bound: a deeper tree describes no extractable
/// interface and would only exhaust the recursive walks over shapes
/// (identity, printing, comparison, drop).
pub const MAX_SHAPE_DEPTH: u32 = 127;

/// The refusal of a shape tree deeper than [`MAX_SHAPE_DEPTH`] at `site`.
#[must_use]
pub fn shape_depth_refusal(site: crate::diag::Site) -> crate::diag::Diagnostic {
    crate::diag::Diagnostic::malformed(
        crate::diag::Malformed::Payload,
        site,
        alloc::format!("a shape tree nests at most {MAX_SHAPE_DEPTH} pairs"),
    )
}

/// Shape of one interface stack item; pairs flatten to ordered scalars (D7).
///
/// Pairs are built only by [`ShapeTree::pair`], which refuses a tree that
/// would nest more than [`MAX_SHAPE_DEPTH`] pairs, so every tree that exists
/// is shallow enough for recursive walks.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShapeTree(Shape);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Shape {
    Scalar,
    Unit,
    /// Nested-pair depth (one more than the deeper part's) and the parts.
    Pair(u32, alloc::boxed::Box<(ShapeTree, ShapeTree)>),
}

/// One node of a [`ShapeTree`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeNode<'a> {
    /// One scalar slot/output.
    Scalar,
    /// `Unit`: no slot/output.
    Unit,
    /// `Pair(a, b)`: `a`'s scalars then `b`'s.
    Pair(&'a ShapeTree, &'a ShapeTree),
}

impl ShapeTree {
    /// One scalar slot/output.
    pub const SCALAR: Self = Self(Shape::Scalar);
    /// `Unit`: no slot/output.
    pub const UNIT: Self = Self(Shape::Unit);

    /// `Pair(a, b)`: `a`'s scalars then `b`'s.
    ///
    /// # Errors
    /// `E-GRAPH-MALFORMED` (payload) when the pair would nest more than
    /// [`MAX_SHAPE_DEPTH`] pairs.
    pub fn pair(a: Self, b: Self) -> Result<Self, crate::diag::Diagnostic> {
        let depth = a.depth().max(b.depth()).saturating_add(1);
        if depth > MAX_SHAPE_DEPTH {
            return Err(shape_depth_refusal(crate::diag::Site::Graph));
        }
        Ok(Self(Shape::Pair(depth, alloc::boxed::Box::new((a, b)))))
    }

    /// Nested pairs on the deepest path (0 for a leaf).
    #[must_use]
    pub const fn depth(&self) -> u32 {
        match &self.0 {
            Shape::Pair(depth, _) => *depth,
            Shape::Scalar | Shape::Unit => 0,
        }
    }

    /// The tree's root node.
    #[must_use]
    pub fn node(&self) -> ShapeNode<'_> {
        match &self.0 {
            Shape::Scalar => ShapeNode::Scalar,
            Shape::Unit => ShapeNode::Unit,
            Shape::Pair(_, parts) => ShapeNode::Pair(&parts.0, &parts.1),
        }
    }

    /// Whether the tree is one scalar.
    #[must_use]
    pub const fn is_scalar(&self) -> bool {
        matches!(self.0, Shape::Scalar)
    }
}

/// A `ngraph-ir/0` graph (producer data until checked).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Graph {
    /// Sub-profile.
    pub profile: SubProfile,
    /// Ordered interface slots.
    pub slots: alloc::vec::Vec<Slot>,
    /// Nodes in canonical (creation) order.
    pub nodes: alloc::vec::Vec<Node>,
    /// Ordered outputs.
    pub outputs: alloc::vec::Vec<NodeId>,
    /// Fact records.
    pub facts: alloc::vec::Vec<FactRecord>,
    /// Root subject.
    pub subject: Subject,
    /// Input stack shape (`None`: every slot is one flat stack item).
    pub input_shape: Option<alloc::vec::Vec<ShapeTree>>,
    /// Output stack shape (`None`: every output is one flat stack item).
    pub output_shape: Option<alloc::vec::Vec<ShapeTree>>,
}

/// Incremental graph builder (no validation; the checker validates).
#[derive(Clone, Debug)]
pub struct GraphBuilder {
    graph: Graph,
}

impl GraphBuilder {
    /// A new empty graph for `profile` and `subject`.
    #[must_use]
    pub const fn new(profile: SubProfile, subject: Subject) -> Self {
        Self {
            graph: Graph {
                profile,
                slots: alloc::vec::Vec::new(),
                nodes: alloc::vec::Vec::new(),
                outputs: alloc::vec::Vec::new(),
                facts: alloc::vec::Vec::new(),
                subject,
                input_shape: None,
                output_shape: None,
            },
        }
    }

    /// Appends a node with optional provenance; returns its id.
    pub fn push(&mut self, op: Op, origin: Option<Origin>) -> NodeId {
        let id = NodeId(u32::try_from(self.graph.nodes.len()).unwrap_or(u32::MAX));
        self.graph.nodes.push(Node { op, origin });
        id
    }

    /// Appends a node without provenance.
    pub fn op(&mut self, op: Op) -> NodeId {
        self.push(op, None)
    }

    fn slot(&mut self, kind: SlotKind, ty: ScalarType) -> NodeId {
        let k = u32::try_from(self.graph.slots.len()).unwrap_or(u32::MAX);
        self.graph.slots.push(Slot { kind, ty });
        self.op(match kind {
            SlotKind::Input => Op::Input(k),
            SlotKind::Param => Op::Param(k),
        })
    }

    /// Declares the next input slot and its node.
    pub fn input(&mut self, ty: ScalarType) -> NodeId {
        self.slot(SlotKind::Input, ty)
    }

    /// Declares the next param slot (`f32`) and its node.
    pub fn param(&mut self) -> NodeId {
        self.slot(SlotKind::Param, ScalarType::F32)
    }

    /// Declares an interface slot without creating its node.
    pub fn declare_slot(&mut self, kind: SlotKind, ty: ScalarType) -> u32 {
        let k = u32::try_from(self.graph.slots.len()).unwrap_or(u32::MAX);
        self.graph.slots.push(Slot { kind, ty });
        k
    }

    /// `const`.
    pub fn constant(&mut self, lit: Literal) -> NodeId {
        self.op(Op::Const(lit))
    }

    /// `f32` constant from a value.
    pub fn f32_const(&mut self, value: f32) -> NodeId {
        self.constant(Literal::f32(value))
    }

    /// `i64` constant.
    pub fn i64_const(&mut self, value: i64) -> NodeId {
        self.constant(Literal::I64(value))
    }

    /// `f32.affine` with bias value and `(coefficient, operand)` terms.
    pub fn affine(&mut self, bias: f32, terms: &[(f32, AffineOperand)]) -> NodeId {
        self.op(Op::F32Affine {
            bias: bias.to_bits(),
            terms: terms
                .iter()
                .map(|&(c, operand)| AffineTerm {
                    coefficient: c.to_bits(),
                    operand,
                })
                .collect(),
        })
    }

    /// `guard(fact, value)`.
    pub fn guard(&mut self, fact: Fact, value: NodeId) -> NodeId {
        self.op(Op::Guard { fact, value })
    }

    /// Appends an output.
    pub fn output(&mut self, id: NodeId) {
        self.graph.outputs.push(id);
    }

    /// Appends a fact record.
    pub fn fact(&mut self, node: NodeId, fact: Fact, provenance: Provenance) {
        self.graph.facts.push(FactRecord {
            node,
            fact,
            provenance,
        });
    }

    /// Records interface shapes.
    pub fn shapes(
        &mut self,
        input: Option<alloc::vec::Vec<ShapeTree>>,
        output: Option<alloc::vec::Vec<ShapeTree>>,
    ) {
        self.graph.input_shape = input;
        self.graph.output_shape = output;
    }

    /// Current node count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.graph.nodes.len()
    }

    /// Whether no node was created yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.graph.nodes.is_empty()
    }

    /// Read access to the graph under construction.
    #[must_use]
    pub const fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Finishes building.
    #[must_use]
    pub fn finish(self) -> Graph {
        self.graph
    }
}

/// 32-byte identity digest.
pub type Digest = [u8; 32];

/// Lowercase hex of a digest.
#[must_use]
pub fn hex(d: &[u8]) -> alloc::string::String {
    let mut s = alloc::string::String::with_capacity(d.len() * 2);
    for b in d {
        crate::format::write_text!(s, "{b:02x}");
    }
    s
}

/// Canonical byte encoder used for identities (length-prefixed, little-endian),
/// streamed into the injected [`IdentityScheme`].
pub(crate) struct Canon<'s> {
    h: alloc::boxed::Box<dyn IdentityHasher + 's>,
}

impl<'s> Canon<'s> {
    pub(crate) fn new(ids: &'s dyn IdentityScheme, domain: &str) -> Self {
        let mut h = ids.start();
        h.update(domain.as_bytes());
        h.update(&[0]);
        Self { h }
    }
    fn hasher(&mut self) -> &mut (dyn IdentityHasher + 's) {
        &mut *self.h
    }
    pub(crate) fn bytes(&mut self, b: &[u8]) {
        let len = (b.len() as u64).to_le_bytes();
        self.hasher().update(&len);
        self.hasher().update(b);
    }
    pub(crate) fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
    pub(crate) fn u8(&mut self, v: u8) {
        self.hasher().update(&[v]);
    }
    pub(crate) fn u32(&mut self, v: u32) {
        self.hasher().update(&v.to_le_bytes());
    }
    pub(crate) fn u64(&mut self, v: u64) {
        self.hasher().update(&v.to_le_bytes());
    }
    pub(crate) fn i64(&mut self, v: i64) {
        self.hasher().update(&v.to_le_bytes());
    }
    pub(crate) fn finish(self) -> Digest {
        self.h.finish()
    }

    fn literal(&mut self, l: Literal) {
        match l {
            Literal::I64(v) => {
                self.u8(0);
                self.i64(v);
            }
            Literal::Bool(v) => {
                self.u8(1);
                self.u8(u8::from(v));
            }
            Literal::F32(v) => {
                self.u8(2);
                self.u32(v);
            }
        }
    }

    fn node_id(&mut self, n: NodeId) {
        self.u32(n.0);
    }

    pub(crate) fn fact(&mut self, f: &Fact) {
        self.str(f.kind_name());
        match f {
            Fact::Range { lo, hi } => {
                self.literal(*lo);
                self.literal(*hi);
            }
            Fact::Grid { log2 } => self.i64(i64::from(*log2)),
            Fact::Bit | Fact::Finite | Fact::Integral => {}
            Fact::OneHot { group, allow_zero } => {
                self.u64(group.len() as u64);
                for g in group {
                    self.node_id(*g);
                }
                self.u8(u8::from(*allow_zero));
            }
            Fact::Shape(n) => self.u32(*n),
        }
    }

    fn op(&mut self, op: &Op) {
        self.str(op.name());
        match op {
            Op::Input(k) | Op::Param(k) => self.u32(*k),
            Op::Const(l) => self.literal(*l),
            Op::I64Add(a, b)
            | Op::I64Sub(a, b)
            | Op::I64Mul(a, b)
            | Op::I64Eq(a, b)
            | Op::F32Add(a, b)
            | Op::F32Sub(a, b)
            | Op::F32Mul(a, b) => {
                self.node_id(*a);
                self.node_id(*b);
            }
            Op::F32Affine { bias, terms } => {
                self.u32(*bias);
                self.u64(terms.len() as u64);
                for t in terms {
                    self.u32(t.coefficient);
                    match t.operand {
                        AffineOperand::One => self.u8(0),
                        AffineOperand::Node(n) => {
                            self.u8(1);
                            self.node_id(n);
                        }
                    }
                }
            }
            Op::F32Relu(x) | Op::F32Step(x) => self.node_id(*x),
            Op::F32Reglu { left, gate } => {
                self.node_id(*left);
                self.node_id(*gate);
            }
            Op::F32Select { bit, off, on } => {
                self.node_id(*bit);
                self.node_id(*off);
                self.node_id(*on);
            }
            Op::F32Index {
                index,
                table,
                below,
                above,
            } => {
                self.node_id(*index);
                self.u64(table.len() as u64);
                for t in table {
                    self.node_id(*t);
                }
                self.str(below.name());
                self.str(above.name());
            }
            Op::Guard { fact, value } => {
                self.fact(fact);
                self.node_id(*value);
            }
        }
    }

    fn shapes(&mut self, s: Option<&alloc::vec::Vec<ShapeTree>>) {
        // Recursion depth is bounded by `MAX_SHAPE_DEPTH`.
        #[expect(
            tigerstyle::no_recursion,
            reason = "Owner: noble-maintainers; canonical encoding is bounded by the declared shape nesting limit"
        )]
        fn tree(c: &mut Canon<'_>, t: &ShapeTree) {
            match t.node() {
                ShapeNode::Scalar => c.u8(0),
                ShapeNode::Unit => c.u8(1),
                ShapeNode::Pair(a, b) => {
                    c.u8(2);
                    tree(c, a);
                    tree(c, b);
                }
            }
        }
        match s {
            None => self.u8(0),
            Some(v) => {
                self.u8(1);
                self.u64(v.len() as u64);
                for t in v {
                    tree(self, t);
                }
            }
        }
    }
}

impl Graph {
    /// `GraphLogicalId` (D7 step 6): domain ‖ profile+semantic rev ‖ fact-rule
    /// rev ‖ canonical nodes in creation order ‖ interface ‖ outputs ‖ subject
    /// ‖ closure. Node origins and fact records are excluded. Meaningful only for
    /// a graph that passed the checker; the checker computes it.
    #[must_use]
    pub fn logical_id(&self, ids: &dyn IdentityScheme) -> Digest {
        let mut c = Canon::new(ids, "noble.ngraph.graph.v0");
        c.str(IR_VERSION);
        c.str(self.profile.name());
        c.str(SEMANTIC_REVISION);
        c.str(FACT_RULE_REVISION);
        c.u64(self.nodes.len() as u64);
        for n in &self.nodes {
            c.op(&n.op);
        }
        c.u64(self.slots.len() as u64);
        for s in &self.slots {
            c.u8(match s.kind {
                SlotKind::Input => 0,
                SlotKind::Param => 1,
            });
            c.str(s.ty.name());
        }
        c.shapes(self.input_shape.as_ref());
        c.u64(self.outputs.len() as u64);
        for o in &self.outputs {
            c.u32(o.0);
        }
        c.shapes(self.output_shape.as_ref());
        c.str(self.subject.kind.name());
        c.bytes(&self.subject.identity);
        c.u64(self.subject.closure.len() as u64);
        for d in &self.subject.closure {
            c.u64(*d);
        }
        c.finish()
    }

    /// Ordered param slot indices.
    #[must_use]
    pub fn param_slots(&self) -> alloc::vec::Vec<u32> {
        (0u32..)
            .zip(&self.slots)
            .filter(|(_, s)| s.kind == SlotKind::Param)
            .map(|(k, _)| k)
            .collect()
    }

    /// Ordered input slot indices.
    #[must_use]
    pub fn input_slots(&self) -> alloc::vec::Vec<u32> {
        (0u32..)
            .zip(&self.slots)
            .filter(|(_, s)| s.kind == SlotKind::Input)
            .map(|(k, _)| k)
            .collect()
    }
}
