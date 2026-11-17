//! Opt-in bounded size-change closure evidence for signature-checked recursive definitions
//! (issue #101).
//!
//! The specification and its proof live in `proofs/m3/NobleM2/SizeChange*.lean`. This module is the
//! front end that builds the model's call graphs from a checked definition: it classifies what each
//! self-call site passes for each declared measure, builds a size-change
//! matrix, closes the bounded call graph under composition, and checks the idempotent strict-edge
//! criterion. Refusals identify an idempotent closure graph with no strict self-edge.
//!
//! ## What is analyzed
//!
//! A definition whose declared signature is one polymorphic stack prefix followed by its measures:
//! `S I64 -- S I64`, `S (List I64) -- S I64`, `S I64 (List I64) -- S I64`, and so on. Each concrete
//! position is a measure of one of two kinds:
//!
//! - `Counter`: a wrapping `I64`. A successor is not a decrease, and `n - 1` is a decrease only
//!   under an active `0 < n` guard at the site, because wrapping two's-complement subtraction can
//!   raise `i64::MIN - 1`.
//! - `Structural`: a `List` value. The cons branch of `list.case` gives its tail, which is strictly
//!   shorter, so a tail argument is a decrease with no guard.
//!
//! The walk is a symbolic stack machine over the checked body. Values are tracked only as far as
//! the classifier needs — the measure itself, its predecessor, a list tail, a `0 <` comparison —
//! and everything else is opaque. Guards are the `0 <` comparisons in force along the branch that
//! reaches the call site. Prefix rows of a stack prefix are never inspected: a word that would
//! consume or rearrange the prefix is refused, not modelled.
//!
//! ## Soundness of the verdict
//!
//! The Lean certificate checks the finite closure and every idempotent strict-edge witness against
//! the same matrix algebra. The bounded classifier derives edges only from the existing guarded
//! counter and structural-list facts. Its correspondence to source execution remains open. A
//! refusal means **not shown terminating**, never "diverges".
//!
//! ## What is not done here
//!
//! Mutual recursion and general source-to-model correspondence are not claimed by this one-
//! definition front end. The correspondence between `List` structure and its length, and between a
//! guarded wrapped `I64` counter and a natural number, remains an explicit open boundary documented
//! in the lane (`verification/size-change`).

/// Maximum number of concrete measures in one analyzed declaration.
const MAX_MEASURES: usize = 8;
/// The exclusive upper bound for a fixed-width measure index.
const MAX_MEASURE_INDEX: u8 = MAX_MEASURES as u8;
/// The most self-call sites one definition's report carries.
const MAX_SITES: usize = 256;
/// The most distinct matrices retained by one positive closure.
const MAX_CLOSURE: usize = 256;
/// The most stack cells the symbolic walk tracks; a checked body needs far fewer.
const MAX_FRAME: usize = 64;
/// The most word steps one definition's walk spends before refusing.
const MAX_STEPS: u32 = 65_536;
/// The most nested quotation runs one walk descends through.
const MAX_DEPTH: u32 = 32;

/// One declared measure's shape, from the definition's signature.
#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Measure {
    /// A wrapping `I64` counter: a predecessor decreases it only under `0 <` at the site.
    Counter,
    /// A `List` value: its cons tail is structurally shorter.
    Structural,
}

impl Measure {
    /// The spelling the report uses.
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Counter => "counter",
            Self::Structural => "structural",
        }
    }
}

/// What one self-call site passes for one measure.
#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Argument {
    /// The measure itself, unchanged.
    Same,
    /// The measure's predecessor, under the site's positivity guard.
    Predecessor,
    /// The structural tail of a list measure.
    Tail,
    /// Anything the bounded classifier cannot classify.
    Opaque,
}

/// A relation between one old parameter and one new parameter at a call site.
#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relation {
    /// The new value is strictly smaller than the old value.
    Strict,
    /// The new value is no larger than the old value.
    NonIncrease,
    /// The new value is equal to the old value.
    Same,
    /// No relation is established by the bounded classifier.
    Unknown,
}

impl Relation {
    /// The spelling used in reports and Lean certificates.
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::NonIncrease => "non-increase",
            Self::Same => "same",
            Self::Unknown => "unknown",
        }
    }

    /// The `NobleM2.SizeChange.Rel4` constructor this relation certifies.
    pub const fn constructor(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::NonIncrease => "nonIncrease",
            Self::Same => "same",
            Self::Unknown => "unknown",
        }
    }
}

/// One fixed-width parameter index in a size-change graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeasureIndex(u8);

impl MeasureIndex {
    /// Construct an index inside the graph's fixed eight-measure domain.
    pub const fn new(index: u8) -> Option<Self> {
        if index < MAX_MEASURE_INDEX {
            Some(Self(index))
        } else {
            None
        }
    }

    fn from_usize(index: usize) -> Option<Self> {
        Self::new(u8::try_from(index).ok()?)
    }

    fn as_usize(self) -> usize {
        usize::from(self.0)
    }
}

/// A named pair of graph axes, preventing source and target indices from being swapped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphEdge {
    /// The old measure's row.
    pub source: MeasureIndex,
    /// The new measure's column.
    pub target: MeasureIndex,
}

/// One size-change matrix with a fixed eight-by-eight representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Graph {
    /// Relation from each old measure to each new measure.
    pub relations: [[Relation; MAX_MEASURES]; MAX_MEASURES],
}

impl Graph {
    /// An all-unknown matrix.
    pub const fn unknown() -> Self {
        Self {
            relations: [[Relation::Unknown; MAX_MEASURES]; MAX_MEASURES],
        }
    }

    /// Read one relation using a validated pair of fixed-width graph axes.
    pub fn get(&self, edge: GraphEdge) -> Relation {
        self.relations[edge.source.as_usize()][edge.target.as_usize()]
    }

    /// Return a graph with one relation replaced.
    pub fn with_relation(self, edge: GraphEdge, relation: Relation) -> Self {
        let mut graph = self;
        graph.relations[edge.source.as_usize()][edge.target.as_usize()] = relation;
        graph
    }
}

/// One distinct closure matrix and the shortest known sequence of call sites producing it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosureEntry {
    /// The composed graph.
    pub graph: Graph,
    /// Indices into the definition's source-ordered `Evidence::sites`.
    pub path: alloc::vec::Vec<u32>,
}

/// A per-idempotent strict diagonal selected by the accepted certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrictWitness {
    /// Index into the accepted closure list.
    pub closure_entry: u32,
    /// Measure whose diagonal relation is strict.
    pub measure: u32,
}

/// Which declared bound made an otherwise supported analysis incomplete.
#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bound {
    /// More concrete parameters than the classifier supports.
    Parameters,
    /// More call sites than the classifier supports.
    CallSites,
    /// More distinct closure graphs than the certificate cap allows.
    ClosureSize,
}

impl Bound {
    /// Stable report spelling.
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Parameters => "parameters",
            Self::CallSites => "call-sites",
            Self::ClosureSize => "closure-size",
        }
    }
}

impl Argument {
    /// The spelling the report uses.
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Same => "same",
            Self::Predecessor => "predecessor",
            Self::Tail => "tail",
            Self::Opaque => "opaque",
        }
    }

    /// The `NobleM2.SizeChange.Arg` constructor this class certifies.
    pub const fn constructor(self) -> &'static str {
        match self {
            Self::Same => "same",
            Self::Predecessor | Self::Tail => "predOfSame",
            Self::Opaque => "opaque",
        }
    }
}

/// One self-call site the checker classified.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Site {
    /// The site's source span.
    pub span: crate::Span,
    /// What the site passes for each measure, in declaration order.
    pub arguments: alloc::vec::Vec<Argument>,
    /// Whether a `0 <` guard on each measure is in force at the site.
    pub guarded: alloc::vec::Vec<bool>,
    /// Full old-parameter to new-parameter graph; unobserved pairs are `Unknown`.
    pub graph: Graph,
}

/// The reason a definition is outside the bounded classifier.
#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// The declared signature is not a stack prefix followed by `I64`/`List` measures.
    SignatureOutsideProfile,
    /// The walk would consume or rearrange the declared stack prefix.
    PrefixTouched,
    /// A word form the symbolic walk does not model.
    UnhandledWord,
    /// A named helper call, quotation or call whose behavior is not modelled.
    UnmodelledCall,
    /// A self call whose current frame is not the declared argument list.
    SelfArityMismatch,
    /// The symbolic walk exhausted its step, depth, or frame budget.
    BudgetExceeded,
    /// The number of recursive call sites exceeded its declared bound.
    CallSiteLimit,
    /// The body's final frame does not match the declared result.
    FinalFrameMismatch,
    /// The definition declares no self call, so there is nothing to classify.
    NoSelfCall,
    /// A closure invariant expected by the bounded algebra did not hold.
    ClosureInvariant,
}

impl Unsupported {
    /// The spelling the report uses.
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::SignatureOutsideProfile => "signature-outside-profile",
            Self::PrefixTouched => "prefix-touched",
            Self::UnhandledWord => "unhandled-word",
            Self::UnmodelledCall => "unmodelled-call",
            Self::SelfArityMismatch => "self-arity-mismatch",
            Self::BudgetExceeded => "budget-exceeded",
            Self::CallSiteLimit => "call-site-limit",
            Self::FinalFrameMismatch => "final-frame-mismatch",
            Self::NoSelfCall => "no-self-call",
            Self::ClosureInvariant => "closure-invariant",
        }
    }
}

/// A measure a refusal reports, with the first site that does not decrease it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Witness {
    /// The measure index that fails.
    pub measure: u32,
    /// The site index (into [`Evidence::sites`]) that fails it.
    pub site: u32,
    /// What that site passes for the measure.
    pub argument: Argument,
}

/// The checker's verdict.
#[octet::sealed_enum]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Every idempotent matrix in the exact bounded closure has a strict diagonal.
    Accepted {
        /// A shared measure, when the legacy stronger criterion also applies.
        shared_measure: Option<u32>,
        /// The complete, deduplicated positive closure with one call-site path per matrix.
        closure: alloc::vec::Vec<ClosureEntry>,
        /// One strict diagonal witness for each idempotent matrix.
        strict_witnesses: alloc::vec::Vec<StrictWitness>,
    },
    /// An idempotent graph in the closure has no strict self-edge: termination is not shown.
    Refused {
        /// Legacy shared-measure failure details, in declaration order.
        witnesses: alloc::vec::Vec<Witness>,
        /// The complete, deduplicated positive closure.
        closure: alloc::vec::Vec<ClosureEntry>,
        /// The idempotent graph and call-site cycle that witness refusal.
        cycle: ClosureEntry,
    },
    /// A declared analysis bound was exceeded; no termination verdict is inferred.
    BoundExceeded {
        /// Which bound was reached.
        bound: Bound,
        /// Where the bound was observed.
        span: crate::Span,
    },
    /// The definition is outside the bounded classifier; no claim is made either way.
    Unsupported {
        /// Why the walk stopped.
        reason: Unsupported,
        /// Where it stopped.
        span: crate::Span,
    },
}

/// The evidence one recursive definition carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    /// The definition's name.
    pub definition: alloc::string::String,
    /// The declared measures, in signature order.
    pub measures: alloc::vec::Vec<Measure>,
    /// The self-call sites, in body order.
    pub sites: alloc::vec::Vec<Site>,
    /// The verdict.
    pub verdict: Verdict,
}

/// One symbolic stack value: only what the classifier needs is tracked.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    /// The declared stack prefix, as one opaque cell below the concrete frame.
    Prefix,
    /// The entry value of measure `j`, unchanged.
    Formal(u32),
    /// The predecessor of measure `j`.
    Predecessor(u32),
    /// The structural tail of list measure `j`.
    Tail(u32),
    /// The literal `0`.
    Zero,
    /// The literal `1`.
    One,
    /// A boolean that is the comparison `0 < formal (j)`.
    Positive(u32),
    /// A boolean of any other provenance.
    OtherBoolean,
    /// A quotation the walk may run, by node identity.
    Program(u32),
    /// Anything else.
    Other,
}

/// One word step's outcome: the walk either continues or records its halt.
type Step = Result<(), ()>;

#[derive(Clone, Copy)]
struct ProgramRun {
    node: u32,
    depth: u32,
}

#[derive(Clone, Copy)]
struct BranchRun {
    node: u32,
    guard: u32,
    depth: u32,
}

#[derive(Clone, Copy)]
struct BuiltinRun {
    identity: u32,
    depth: u32,
}

#[derive(Clone, Copy)]
struct StackEffect {
    inputs: usize,
    outputs: usize,
}

/// The bounded symbolic walk over one checked body.
struct Walker<'a> {
    tree: &'a crate::source::Tree,
    environment: &'a noble_kernel::contracts::Env,
    measures: &'a [Measure],
    /// Concrete cells the declared result frame carries.
    result_frame: usize,
    stack: alloc::vec::Vec<Value>,
    guard: u32,
    sites: alloc::vec::Vec<Site>,
    halt: Option<(Unsupported, crate::Span)>,
    steps: u32,
}

#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; Walker is created per definition and never escapes evidence(). Its mutations are confined to its owned symbolic stack, guard, and site buffers; no external state or caller-owned program is changed, and every budget/profile refusal is returned as Unsupported."
)]
impl<'a> Walker<'a> {
    /// Record the first halt and stop the walk.
    const fn fail<T>(&mut self, reason: Unsupported, span: crate::Span) -> Result<T, ()> {
        if self.halt.is_none() {
            self.halt = Some((reason, span));
        }
        Err(())
    }

    fn push(&mut self, value: Value, span: crate::Span) -> Step {
        if self.stack.len() >= MAX_FRAME {
            return self.fail(Unsupported::BudgetExceeded, span);
        }
        self.stack.push(value);
        Ok(())
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; pop removes one runtime Vec stack cell, and the pinned Rust 1.96 toolchain does not provide const Vec::pop. Replacing the checked operation with manual indexing would weaken the stack boundary."
    )]
    fn pop(&mut self, span: crate::Span) -> Result<Value, ()> {
        if self.stack.len() <= 1 {
            return self.fail(Unsupported::PrefixTouched, span);
        }
        if matches!(self.stack.as_slice().last(), Some(Value::Prefix)) {
            return self.fail(Unsupported::PrefixTouched, span);
        }
        match self.stack.pop() {
            Some(value) => Ok(value),
            None => self.fail(Unsupported::PrefixTouched, span),
        }
    }

    /// Pop a program value the walk can run, or refuse.
    fn pop_program(&mut self, span: crate::Span) -> Result<u32, ()> {
        match self.pop(span)? {
            Value::Program(node) => Ok(node),
            _ => self.fail(Unsupported::UnmodelledCall, span),
        }
    }

    /// Run one program value on the current stack.
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; program follows a dynamically parsed source tree through Tree::node and recursively executes its checked quotation nodes; that runtime lookup is not const-evaluable."
    )]
    fn program(&mut self, invocation: ProgramRun) -> Step {
        if invocation.depth >= MAX_DEPTH {
            return self.fail(Unsupported::BudgetExceeded, self.tree.span);
        }
        let quotation = match self.tree.node(invocation.node) {
            Ok(quotation) => quotation,
            Err(_) => return self.fail(Unsupported::UnhandledWord, self.tree.span),
        };
        match &quotation.kind {
            crate::source::Kind::Quotation(items) => {
                self.sequence(items, invocation.depth.saturating_add(1))
            }
            _ => self.fail(Unsupported::UnhandledWord, quotation.span),
        }
    }

    /// Run a word sequence on the current stack.
    fn sequence(&mut self, items: &'a [u32], depth: u32) -> Step {
        for node in items {
            self.word(ProgramRun { node: *node, depth })?;
        }
        Ok(())
    }

    /// Run one branch from `base` under an explicit guard, restoring the caller's stack and guard.
    fn branch(
        &mut self,
        base: &[Value],
        invocation: BranchRun,
    ) -> Result<alloc::vec::Vec<Value>, ()> {
        let saved_guard = self.guard;
        self.stack = base.to_vec();
        self.guard = invocation.guard;
        let outcome = self.program(ProgramRun {
            node: invocation.node,
            depth: invocation.depth.saturating_add(1),
        });
        let produced = core::mem::take(&mut self.stack);
        self.guard = saved_guard;
        outcome?;
        Ok(produced)
    }

    /// Record one self-call site and replace the frame with the declared result.
    fn self_call(&mut self, span: crate::Span) -> Step {
        let measures = self.measures;
        if !matches!(self.stack.first(), Some(Value::Prefix)) {
            return self.fail(Unsupported::PrefixTouched, span);
        }
        if self.stack.len() != measures.len().saturating_add(1) {
            return self.fail(Unsupported::SelfArityMismatch, span);
        }
        if self.sites.len() >= MAX_SITES {
            return self.fail(Unsupported::CallSiteLimit, span);
        }
        let mut arguments = alloc::vec::Vec::with_capacity(measures.len());
        let mut guarded = alloc::vec::Vec::with_capacity(measures.len());
        let mut graph = Graph::unknown();
        for (index, measure) in measures.iter().enumerate() {
            let Some(target) = MeasureIndex::from_usize(index) else {
                return self.fail(Unsupported::SelfArityMismatch, span);
            };
            let is_guarded = self.guard & Self::measure_bit(index) != 0;
            let value = match self.stack.get(index.saturating_add(1)) {
                Some(value) => value,
                None => return self.fail(Unsupported::SelfArityMismatch, span),
            };
            let argument = classify(*measure, index, value, is_guarded);
            arguments.push(argument);
            guarded.push(is_guarded);
            match value {
                Value::Formal(source) => {
                    let Some(source_index) = Self::measure_index(*source, measures.len()) else {
                        continue;
                    };
                    if measures[source_index] == *measure {
                        let Some(source) = MeasureIndex::from_usize(source_index) else {
                            continue;
                        };
                        graph = graph.with_relation(GraphEdge { source, target }, Relation::Same);
                    }
                }
                Value::Predecessor(source) => {
                    let Some(source_index) = Self::measure_index(*source, measures.len()) else {
                        continue;
                    };
                    let is_source_guarded = self.guard & Self::measure_bit(source_index) != 0;
                    if is_source_guarded
                        && measures[source_index] == Measure::Counter
                        && *measure == Measure::Counter
                    {
                        let Some(source) = MeasureIndex::from_usize(source_index) else {
                            continue;
                        };
                        graph = graph.with_relation(GraphEdge { source, target }, Relation::Strict);
                    }
                }
                Value::Tail(source) => {
                    let Some(source_index) = Self::measure_index(*source, measures.len()) else {
                        continue;
                    };
                    if measures[source_index] == Measure::Structural
                        && *measure == Measure::Structural
                    {
                        let Some(source) = MeasureIndex::from_usize(source_index) else {
                            continue;
                        };
                        graph = graph.with_relation(GraphEdge { source, target }, Relation::Strict);
                    }
                }
                _ => {}
            }
        }
        self.sites.push(Site {
            span,
            arguments,
            guarded,
            graph,
        });
        self.stack.truncate(1);
        for _ in 0..self.result_frame {
            self.push(Value::Other, span)?;
        }
        Ok(())
    }

    /// The guard bit of one measure index.
    const fn measure_bit(index: usize) -> u32 {
        1_u32 << (index & 31)
    }

    /// The value of one measure index as a `u32` index, if it is in range.
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; this boundary converts a source u32 index to target-width usize with TryFrom and must remain checked on 16/32/64-bit targets rather than using a truncating cast."
    )]
    fn measure_index(candidate: u32, bound: usize) -> Option<usize> {
        usize::try_from(candidate)
            .ok()
            .filter(|value| *value < bound)
    }

    /// One word.
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; word resolves a runtime source node from the checked tree and updates the bounded, allocation-backed symbolic stack; const evaluation is not a valid execution mode."
    )]
    fn word(&mut self, invocation: ProgramRun) -> Step {
        if self.steps >= MAX_STEPS {
            return self.fail(Unsupported::BudgetExceeded, self.tree.span);
        }
        self.steps = self.steps.saturating_add(1);
        let node = match self.tree.node(invocation.node) {
            Ok(node) => node,
            Err(_) => return self.fail(Unsupported::UnhandledWord, self.tree.span),
        };
        let span = node.span;
        match &node.kind {
            crate::source::Kind::Literal(literal) => self.literal(*literal, span),
            crate::source::Kind::Text(_) => self.push(Value::Other, span),
            crate::source::Kind::Quotation(_) => self.push(Value::Program(invocation.node), span),
            crate::source::Kind::Word(_) | crate::source::Kind::EditorHole => {
                self.fail(Unsupported::UnhandledWord, span)
            }
            crate::source::Kind::Call(crate::source::Target::SelfOwner) => self.self_call(span),
            crate::source::Kind::Call(crate::source::Target::Builtin(identity)) => self.builtin(
                BuiltinRun {
                    identity: *identity,
                    depth: invocation.depth,
                },
                span,
            ),
            crate::source::Kind::Call(crate::source::Target::Named(_)) => {
                self.fail(Unsupported::UnmodelledCall, span)
            }
            crate::source::Kind::Call(crate::source::Target::SlotInvoke(_)) => {
                self.fail(Unsupported::UnmodelledCall, span)
            }
        }
    }

    /// One literal value.
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; literal updates the runtime Vec-backed symbolic stack; its bounded allocation state is part of the per-definition interpreter, not a const computation."
    )]
    fn literal(&mut self, literal: noble_kernel::untrusted::Lit, span: crate::Span) -> Step {
        match literal {
            noble_kernel::untrusted::Lit::I64(0) => self.push(Value::Zero, span),
            noble_kernel::untrusted::Lit::I64(1) => self.push(Value::One, span),
            noble_kernel::untrusted::Lit::I64(_)
            | noble_kernel::untrusted::Lit::Bool(_)
            | noble_kernel::untrusted::Lit::Text
            | noble_kernel::untrusted::Lit::Unit => self.push(Value::Other, span),
        }
    }

    /// One bootstrap or declared-text word.
    fn builtin(&mut self, invocation: BuiltinRun, span: crate::Span) -> Step {
        match invocation.identity {
            0 => self.duplicate(span),
            1 => self.pop(span).map(|_| ()),
            2 => self.swap(span),
            3 => self.dip(span, invocation.depth),
            4..=6 => self.arithmetic(invocation.identity, span),
            7 => self.compare_equals(span),
            9 => self.fail(Unsupported::UnmodelledCall, span),
            10 => {
                let program = self.pop_program(span)?;
                self.program(ProgramRun {
                    node: program,
                    depth: invocation.depth.saturating_add(1),
                })
            }
            17 => self.branch_both(span, invocation.depth),
            18 => self.conditional(span, invocation.depth),
            19 => self.push(Value::Other, span),
            20 => self.cons(span),
            21 => self.list_case(span, invocation.depth),
            27 => self.compare_less(span),
            _ => self.generic(
                self.environment
                    .scheme(noble_kernel::contracts::Definition(invocation.identity)),
                span,
            ),
        }
    }

    /// A word whose stack effect is read from its scheme: consume the declared frame, produce
    /// opaque results.
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; generic derives a stack effect from a runtime scheme and mutates the Vec-backed symbolic stack, so it is an interpreter step rather than a const helper."
    )]
    fn generic(&mut self, scheme: Option<&noble_kernel::words::Scheme>, span: crate::Span) -> Step {
        let (inputs, outputs) = match scheme.and_then(split_frame) {
            Some(frame) => frame,
            None => return self.fail(Unsupported::UnmodelledCall, span),
        };
        self.consume(StackEffect { inputs, outputs }, span)
    }

    /// Consume `inputs` concrete cells and produce `outputs` opaque ones.
    fn consume(&mut self, effect: StackEffect, span: crate::Span) -> Step {
        for _ in 0..effect.inputs {
            self.pop(span)?;
        }
        for _ in 0..effect.outputs {
            self.push(Value::Other, span)?;
        }
        Ok(())
    }

    /// `dup`.
    fn duplicate(&mut self, span: crate::Span) -> Step {
        let value = self.pop(span)?;
        self.push(value.clone(), span)?;
        self.push(value, span)
    }

    /// `swap`.
    fn swap(&mut self, span: crate::Span) -> Step {
        let second = self.pop(span)?;
        let first = self.pop(span)?;
        self.push(second, span)?;
        self.push(first, span)
    }

    /// `dip`: run one program under the top value, then restore it.
    fn dip(&mut self, span: crate::Span, depth: u32) -> Step {
        let program = self.pop_program(span)?;
        let value = self.pop(span)?;
        self.program(ProgramRun {
            node: program,
            depth: depth.saturating_add(1),
        })?;
        self.push(value, span)
    }

    /// `+`, `-` and `*`.
    fn arithmetic(&mut self, identity: u32, span: crate::Span) -> Step {
        let right = self.pop(span)?;
        let left = self.pop(span)?;
        let value = if identity == 5 {
            predecessor(self.measures, left, right)
        } else {
            Value::Other
        };
        self.push(value, span)
    }

    /// `=`.
    fn compare_equals(&mut self, span: crate::Span) -> Step {
        self.pop(span)?;
        self.pop(span)?;
        self.push(Value::OtherBoolean, span)
    }

    /// `<`: the signed comparison the declared-text profile adds. `0 < n` is the positivity guard.
    fn compare_less(&mut self, span: crate::Span) -> Step {
        let upper = self.pop(span)?;
        let lower = self.pop(span)?;
        let value = match (&lower, &upper) {
            (Value::Zero, Value::Formal(index))
                if Self::measure_index(*index, self.measures.len()).is_some() =>
            {
                Value::Positive(*index)
            }
            _ => Value::OtherBoolean,
        };
        self.push(value, span)
    }

    /// `cons`.
    fn cons(&mut self, span: crate::Span) -> Step {
        self.pop(span)?;
        self.pop(span)?;
        self.push(Value::Other, span)
    }

    /// `if`: the positive guard holds only in the taken branch.
    fn conditional(&mut self, span: crate::Span, depth: u32) -> Step {
        let alternative = self.pop_program(span)?;
        let consequent = self.pop_program(span)?;
        let condition = self.pop(span)?;
        let outer_guard = self.guard;
        let guarded = match condition {
            Value::Positive(index) => match Self::measure_index(index, self.measures.len()) {
                Some(index) => outer_guard | Self::measure_bit(index),
                None => outer_guard,
            },
            _ => outer_guard,
        };
        let base = self.stack.clone();
        let taken = self.branch(
            &base,
            BranchRun {
                node: consequent,
                guard: guarded,
                depth,
            },
        )?;
        let other = self.branch(
            &base,
            BranchRun {
                node: alternative,
                guard: outer_guard,
                depth,
            },
        )?;
        self.join_branches(taken, other, span)
    }

    /// `case`: either constructor is possible, so both payloads are opaque and both branches are
    /// joined without a positivity guard.
    fn branch_both(&mut self, span: crate::Span, depth: u32) -> Step {
        let right = self.pop_program(span)?;
        let left = self.pop_program(span)?;
        self.pop(span)?;
        let guard = self.guard;
        let mut payload = self.stack.clone();
        payload.push(Value::Other);
        let first = self.branch(
            &payload,
            BranchRun {
                node: left,
                guard,
                depth,
            },
        )?;
        let second = self.branch(
            &payload,
            BranchRun {
                node: right,
                guard,
                depth,
            },
        )?;
        self.join_branches(first, second, span)
    }

    /// `list.case`: the cons branch receives the structural tail of a list measure.
    fn list_case(&mut self, span: crate::Span, depth: u32) -> Step {
        let cons = self.pop_program(span)?;
        let nil = self.pop_program(span)?;
        let list = self.pop(span)?;
        let tail = match &list {
            Value::Formal(index)
                if Self::measure_index(*index, self.measures.len())
                    .is_some_and(|index| self.measures[index] == Measure::Structural) =>
            {
                Value::Tail(*index)
            }
            _ => Value::Other,
        };
        let base = self.stack.clone();
        let guard = self.guard;
        let mut extended = base.clone();
        extended.push(Value::Other);
        extended.push(tail);
        let first = self.branch(
            &base,
            BranchRun {
                node: nil,
                guard,
                depth,
            },
        )?;
        let second = self.branch(
            &extended,
            BranchRun {
                node: cons,
                guard,
                depth,
            },
        )?;
        self.join_branches(first, second, span)
    }

    /// Join two branch results; a cell that differs becomes opaque.
    fn join_branches(
        &mut self,
        first: alloc::vec::Vec<Value>,
        second: alloc::vec::Vec<Value>,
        span: crate::Span,
    ) -> Step {
        if first.len() != second.len() {
            return self.fail(Unsupported::FinalFrameMismatch, span);
        }
        let mut joined = alloc::vec::Vec::with_capacity(first.len());
        for (left, right) in first.into_iter().zip(second) {
            joined.push(if left == right { left } else { Value::Other });
        }
        self.stack = joined;
        Ok(())
    }
}

/// What one self-call site passes for one measure.
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; classify safely converts a platform-width measure index into the model's u32 index; the checked TryFrom boundary must remain portable rather than becoming a cast."
)]
fn classify(measure: Measure, index: usize, value: &Value, guarded: bool) -> Argument {
    let candidate = match u32::try_from(index) {
        Ok(candidate) => candidate,
        Err(_) => return Argument::Opaque,
    };
    match (measure, value) {
        (_, Value::Formal(formal)) if *formal == candidate => Argument::Same,
        (Measure::Counter, Value::Predecessor(predecessor)) if *predecessor == candidate => {
            if guarded {
                Argument::Predecessor
            } else {
                Argument::Opaque
            }
        }
        (Measure::Structural, Value::Tail(tail)) if *tail == candidate => Argument::Tail,
        _ => Argument::Opaque,
    }
}

/// `n - 1` for a counter measure, tracked so a guarded site can classify it.
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; predecessor resolves a runtime u32 measure index into a target-width declaration slice using checked conversion and lookup."
)]
fn predecessor(measures: &[Measure], left: Value, right: Value) -> Value {
    match (&left, &right) {
        (Value::Formal(index), Value::One)
            if usize::try_from(*index)
                .ok()
                .and_then(|index| measures.get(index))
                .is_some_and(|measure| *measure == Measure::Counter) =>
        {
            Value::Predecessor(*index)
        }
        _ => Value::Other,
    }
}

/// The concrete frame one scheme exposes: a single stack prefix followed by fixed positions.
fn split_frame(scheme: &noble_kernel::words::Scheme) -> Option<(usize, usize)> {
    if scheme.stack_in.first()? != scheme.stack_out.first()? {
        return None;
    }
    Some((
        concrete_positions(&scheme.stack_in)?,
        concrete_positions(&scheme.stack_out)?,
    ))
}

/// The number of fixed positions after a scheme stack's single prefix variable.
fn concrete_positions(stack: &[noble_kernel::shapes::Pattern]) -> Option<usize> {
    let (prefix, fixed) = stack.split_first()?;
    if !matches!(prefix, noble_kernel::shapes::Pattern::StackVar(_)) {
        return None;
    }
    if fixed
        .iter()
        .any(|pattern| matches!(pattern, noble_kernel::shapes::Pattern::StackVar(_)))
    {
        return None;
    }
    Some(fixed.len())
}

#[octet::sealed_enum]
#[derive(Clone, Copy)]
enum ProfileError {
    Outside,
    BoundExceeded,
}

/// The measures and output frame one signature exposes, if it is in the bounded profile.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; measures_of is a fail-closed signature classifier: every unsupported prefix, stack shape, or measure returns Outside, while an excessive measure count is a distinct bound result; assertion panics would turn valid out-of-profile programs into crashes."
)]
fn measures_of(
    scheme: &noble_kernel::words::Scheme,
) -> Result<(alloc::vec::Vec<Measure>, usize), ProfileError> {
    let (prefix, fixed) = scheme.stack_in.split_first().ok_or(ProfileError::Outside)?;
    if !matches!(prefix, noble_kernel::shapes::Pattern::StackVar(_)) {
        return Err(ProfileError::Outside);
    }
    let result_frame = concrete_positions(&scheme.stack_out).ok_or(ProfileError::Outside)?;
    if scheme.stack_in.first() != scheme.stack_out.first() {
        return Err(ProfileError::Outside);
    }
    if fixed.len() > MAX_MEASURES {
        return Err(ProfileError::BoundExceeded);
    }
    if fixed.is_empty() {
        return Err(ProfileError::Outside);
    }
    let mut measures = alloc::vec::Vec::with_capacity(fixed.len());
    for pattern in fixed {
        measures.push(match pattern {
            noble_kernel::shapes::Pattern::I64 => Measure::Counter,
            noble_kernel::shapes::Pattern::List(_) => Measure::Structural,
            _ => return Err(ProfileError::Outside),
        });
    }
    Ok((measures, result_frame))
}

/// Whether a definition's tree contains a lexical self call.
pub fn calls_self(definition: &crate::source::Named) -> bool {
    definition.tree.nodes.iter().any(|node| {
        matches!(
            node.kind,
            crate::source::Kind::Call(crate::source::Target::SelfOwner)
        )
    })
}

/// Build the size-change evidence for one checked definition.
pub fn evidence(
    definition: &crate::source::Named,
    environment: &noble_kernel::contracts::Env,
) -> Evidence {
    let profile = definition
        .signature
        .as_ref()
        .map(measures_of)
        .unwrap_or(Err(ProfileError::Outside));
    let (measures, result_frame) = match profile {
        Ok(profile) => profile,
        Err(error) => return profile_refusal(definition, error),
    };
    let (halt, sites) = walk_definition(definition, environment, &measures, result_frame);
    finish_evidence(definition, measures, sites, halt)
}

fn profile_refusal(definition: &crate::source::Named, error: ProfileError) -> Evidence {
    let verdict = match error {
        ProfileError::BoundExceeded => Verdict::BoundExceeded {
            bound: Bound::Parameters,
            span: definition.tree.span,
        },
        ProfileError::Outside => Verdict::Unsupported {
            reason: Unsupported::SignatureOutsideProfile,
            span: definition.tree.span,
        },
    };
    Evidence {
        definition: definition.name.clone(),
        measures: alloc::vec::Vec::new(),
        sites: alloc::vec::Vec::new(),
        verdict,
    }
}

fn walk_definition(
    definition: &crate::source::Named,
    environment: &noble_kernel::contracts::Env,
    measures: &[Measure],
    result_frame: usize,
) -> (Option<(Unsupported, crate::Span)>, alloc::vec::Vec<Site>) {
    assert!(!measures.is_empty());
    assert!(measures.len() <= MAX_MEASURES);
    let mut walker = Walker {
        tree: &definition.tree,
        environment,
        measures,
        result_frame,
        stack: alloc::vec::Vec::with_capacity(MAX_FRAME),
        guard: 0,
        sites: alloc::vec::Vec::new(),
        halt: None,
        steps: 0,
    };
    walker.stack.push(Value::Prefix);
    for index in 0..MAX_MEASURE_INDEX {
        if usize::from(index) >= measures.len() {
            break;
        }
        walker.stack.push(Value::Formal(u32::from(index)));
    }
    let walked = walker.sequence(&definition.tree.body, 0);
    let halt = match walker.halt {
        Some(halt) => Some(halt),
        None => match walked {
            Err(()) => Some((Unsupported::UnhandledWord, definition.tree.span)),
            Ok(()) => final_shape(&walker),
        },
    };
    (halt, walker.sites)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; finishing evidence clones an owned definition name and transfers runtime Vec allocations into the public report."
)]
fn finish_evidence(
    definition: &crate::source::Named,
    measures: alloc::vec::Vec<Measure>,
    sites: alloc::vec::Vec<Site>,
    halt: Option<(Unsupported, crate::Span)>,
) -> Evidence {
    match halt {
        Some((Unsupported::CallSiteLimit, span)) => Evidence {
            definition: definition.name.clone(),
            measures,
            sites,
            verdict: Verdict::BoundExceeded {
                bound: Bound::CallSites,
                span,
            },
        },
        Some((reason, span)) => Evidence {
            definition: definition.name.clone(),
            measures,
            sites,
            verdict: Verdict::Unsupported { reason, span },
        },
        None => verdict(definition, measures, sites),
    }
}

/// Check the walked final frame against the declared result.
const fn final_shape(walker: &Walker<'_>) -> Option<(Unsupported, crate::Span)> {
    if walker.sites.is_empty() {
        return Some((Unsupported::NoSelfCall, walker.tree.span));
    }
    if walker.stack.len() != walker.result_frame.saturating_add(1) {
        return Some((Unsupported::FinalFrameMismatch, walker.tree.span));
    }
    if !matches!(walker.stack.as_slice().first(), Some(Value::Prefix)) {
        return Some((Unsupported::PrefixTouched, walker.tree.span));
    }
    debug_assert!(!walker.measures.is_empty());
    None
}

/// Sequential composition matches the four-relation model's absorbing unknown.
const fn compose_relation(left: Relation, right: Relation) -> Relation {
    match (left, right) {
        (Relation::Unknown, _) | (_, Relation::Unknown) => Relation::Unknown,
        (Relation::Strict, _) | (_, Relation::Strict) => Relation::Strict,
        (Relation::NonIncrease, _) | (_, Relation::NonIncrease) => Relation::NonIncrease,
        (Relation::Same, Relation::Same) => Relation::Same,
    }
}

/// Parallel paths retain the least informative common relation from the model's join table.
const fn join_relation(left: Relation, right: Relation) -> Relation {
    match (left, right) {
        (Relation::Strict, _) | (_, Relation::Strict) => Relation::Strict,
        (Relation::NonIncrease, _) | (_, Relation::NonIncrease) => Relation::NonIncrease,
        (Relation::Same, _) | (_, Relation::Same) => Relation::Same,
        (Relation::Unknown, Relation::Unknown) => Relation::Unknown,
    }
}

/// Compose two bounded matrices in the same row/column orientation as `MGraph`.
fn compose_graph(left: &Graph, right: &Graph, measure_count: usize) -> Graph {
    assert!(measure_count > 0);
    assert!(measure_count <= MAX_MEASURES);
    let mut product = Graph::unknown();
    for source in 0..MAX_MEASURE_INDEX {
        if usize::from(source) >= measure_count {
            break;
        }
        for target in 0..MAX_MEASURE_INDEX {
            if usize::from(target) >= measure_count {
                break;
            }
            let mut joined = Relation::Unknown;
            for middle in 0..MAX_MEASURE_INDEX {
                if usize::from(middle) >= measure_count {
                    break;
                }
                joined = join_relation(
                    compose_relation(
                        left.get(GraphEdge {
                            source: MeasureIndex(source),
                            target: MeasureIndex(middle),
                        }),
                        right.get(GraphEdge {
                            source: MeasureIndex(middle),
                            target: MeasureIndex(target),
                        }),
                    ),
                    joined,
                );
            }
            product = product.with_relation(
                GraphEdge {
                    source: MeasureIndex(source),
                    target: MeasureIndex(target),
                },
                joined,
            );
        }
    }
    product
}

/// Build the positive closure by breadth-first right-composition with each site matrix.
fn close_graphs(sites: &[Site], measure_count: usize) -> Result<alloc::vec::Vec<ClosureEntry>, ()> {
    assert!(!sites.is_empty());
    assert!(sites.len() <= MAX_SITES);
    assert!(measure_count > 0);
    assert!(measure_count <= MAX_MEASURES);
    let mut closure = alloc::vec::Vec::with_capacity(sites.len().min(MAX_CLOSURE));
    for (site_index, site) in sites.iter().enumerate() {
        if closure
            .iter()
            .any(|entry: &ClosureEntry| entry.graph == site.graph)
        {
            continue;
        }
        if closure.len() >= MAX_CLOSURE {
            return Err(());
        }
        let site_index = u32::try_from(site_index).map_err(|_| ())?;
        closure.push(ClosureEntry {
            graph: site.graph,
            path: alloc::vec![site_index],
        });
    }
    assert!(!closure.is_empty());
    assert!(closure.len() <= MAX_CLOSURE);
    let mut cursor = 0;
    while cursor < closure.len() {
        let graph = closure[cursor].graph;
        let path = closure[cursor].path.clone();
        for (site_index, site) in sites.iter().enumerate() {
            let product = compose_graph(&graph, &site.graph, measure_count);
            if closure.iter().any(|entry| entry.graph == product) {
                continue;
            }
            if closure.len() >= MAX_CLOSURE || path.len() >= MAX_CLOSURE {
                return Err(());
            }
            let site_index = u32::try_from(site_index).map_err(|_| ())?;
            let mut product_path = path.clone();
            product_path.push(site_index);
            assert!(product_path.len() <= MAX_CLOSURE);
            closure.push(ClosureEntry {
                graph: product,
                path: product_path,
            });
        }
        cursor += 1;
    }
    Ok(closure)
}

#[octet::sealed_enum]
enum ClosureCriterion {
    Accepted(alloc::vec::Vec<StrictWitness>),
    Refused(ClosureEntry),
    MissingIdempotent,
}

/// Evaluate the idempotent strict-diagonal criterion over a completed finite closure.
fn check_closure(closure: &[ClosureEntry], measure_count: usize) -> ClosureCriterion {
    assert!(closure.len() <= MAX_CLOSURE);
    assert!(measure_count > 0);
    assert!(measure_count <= MAX_MEASURES);
    let mut has_idempotent = false;
    let mut strict_witnesses = alloc::vec::Vec::with_capacity(closure.len().min(MAX_CLOSURE));
    for (closure_index, entry) in closure.iter().enumerate() {
        let square = compose_graph(&entry.graph, &entry.graph, measure_count);
        if square != entry.graph {
            continue;
        }
        has_idempotent = true;
        let measure = (0..measure_count).find(|measure| {
            let Some(index) = MeasureIndex::from_usize(*measure) else {
                return false;
            };
            entry.graph.get(GraphEdge {
                source: index,
                target: index,
            }) == Relation::Strict
        });
        let Some(measure) = measure else {
            return ClosureCriterion::Refused(entry.clone());
        };
        let (Ok(closure_entry), Ok(measure)) =
            (u32::try_from(closure_index), u32::try_from(measure))
        else {
            return ClosureCriterion::MissingIdempotent;
        };
        if strict_witnesses.len() >= MAX_CLOSURE {
            return ClosureCriterion::MissingIdempotent;
        }
        strict_witnesses.push(StrictWitness {
            closure_entry,
            measure,
        });
    }
    if has_idempotent {
        ClosureCriterion::Accepted(strict_witnesses)
    } else {
        ClosureCriterion::MissingIdempotent
    }
}

/// Decide the bounded closure criterion and build the evidence.
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; verdict allocates the refusal-witness vector and clones the definition name into the owned report, which cannot be a const operation."
)]
fn verdict(
    definition: &crate::source::Named,
    measures: alloc::vec::Vec<Measure>,
    sites: alloc::vec::Vec<Site>,
) -> Evidence {
    assert!(!measures.is_empty());
    assert!(measures.len() <= MAX_MEASURES);
    assert!(!sites.is_empty());
    assert!(sites.len() <= MAX_SITES);
    let outcome = match close_graphs(&sites, measures.len()) {
        Err(()) => Verdict::BoundExceeded {
            bound: Bound::ClosureSize,
            span: definition.tree.span,
        },
        Ok(closure) => match check_closure(&closure, measures.len()) {
            ClosureCriterion::Accepted(strict_witnesses) => Verdict::Accepted {
                shared_measure: shared_measure(&measures, &sites),
                closure,
                strict_witnesses,
            },
            ClosureCriterion::Refused(cycle) => Verdict::Refused {
                witnesses: refusal_witnesses(&measures, &sites),
                closure,
                cycle,
            },
            ClosureCriterion::MissingIdempotent => Verdict::Unsupported {
                reason: Unsupported::ClosureInvariant,
                span: definition.tree.span,
            },
        },
    };
    Evidence {
        definition: definition.name.clone(),
        measures,
        sites,
        verdict: outcome,
    }
}

/// The first measure that decreases at every site, mirroring `certifiedAll`.
fn shared_measure(measures: &[Measure], sites: &[Site]) -> Option<u32> {
    for index in 0..measures.len() {
        let is_everywhere = sites.iter().all(|site| {
            matches!(
                site.arguments.get(index),
                Some(Argument::Predecessor | Argument::Tail)
            )
        });
        if is_everywhere {
            return u32::try_from(index).ok();
        }
    }
    None
}

/// One witness per measure: the first site that does not decrease it.
fn refusal_witnesses(measures: &[Measure], sites: &[Site]) -> alloc::vec::Vec<Witness> {
    debug_assert!(!measures.is_empty());
    debug_assert!(!sites.is_empty());
    let mut witnesses = alloc::vec::Vec::with_capacity(measures.len());
    for index in 0..measures.len() {
        let failing = sites.iter().enumerate().find(|(_, site)| {
            !matches!(
                site.arguments.get(index),
                Some(Argument::Predecessor | Argument::Tail)
            )
        });
        if let Some((site, entry)) = failing {
            witnesses.push(Witness {
                measure: u32::try_from(index).unwrap_or(0),
                site: u32::try_from(site).unwrap_or(0),
                argument: entry
                    .arguments
                    .get(index)
                    .copied()
                    .unwrap_or(Argument::Opaque),
            });
        }
    }
    witnesses
}

fn append_named_index(
    mut output: alloc::string::String,
    name: &str,
    index: usize,
) -> alloc::string::String {
    output.push_str(name);
    append_decimal(output, index)
}

fn append_named_u32(
    mut output: alloc::string::String,
    name: &str,
    index: u32,
) -> alloc::string::String {
    output.push_str(name);
    append_u32(output, index)
}

#[derive(Clone, Copy)]
struct CertificateShape {
    site_count: usize,
    measure_count: usize,
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; this renderer grows an owned String through runtime push_str calls to emit a concrete Lean definition."
)]
fn append_graph_header(
    mut output: alloc::string::String,
    name: &str,
    index: usize,
    shape: CertificateShape,
) -> alloc::string::String {
    output.push_str("def ");
    output = append_named_index(output, name, index);
    output.push_str(" : MGraph ");
    output = append_decimal(output, shape.measure_count);
    output.push_str(" := fun oldIndex newIndex =>\n  match oldIndex.val, newIndex.val with\n");
    output
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; this renderer appends dynamic matrix indices and relation names to an owned runtime String."
)]
fn append_graph_edge(
    mut output: alloc::string::String,
    graph: &Graph,
    edge: GraphEdge,
) -> alloc::string::String {
    let relation = graph.get(edge);
    if relation != Relation::Unknown {
        output.push_str("  | ");
        output = append_decimal(output, edge.source.as_usize());
        output.push_str(", ");
        output = append_decimal(output, edge.target.as_usize());
        output.push_str(" => .");
        output.push_str(relation.constructor());
        output.push('\n');
    }
    output
}

fn append_graph_edges(
    mut output: alloc::string::String,
    graph: &Graph,
    measure_count: usize,
) -> alloc::string::String {
    assert!(measure_count > 0);
    assert!(measure_count <= MAX_MEASURES);
    for source in 0..MAX_MEASURE_INDEX {
        if usize::from(source) >= measure_count {
            break;
        }
        for target in 0..MAX_MEASURE_INDEX {
            if usize::from(target) >= measure_count {
                break;
            }
            output = append_graph_edge(
                output,
                graph,
                GraphEdge {
                    source: MeasureIndex(source),
                    target: MeasureIndex(target),
                },
            );
        }
    }
    output
}

fn append_graph_definition(
    output: alloc::string::String,
    name: &str,
    index: usize,
    graph: &Graph,
    shape: CertificateShape,
) -> alloc::string::String {
    let output = append_graph_header(output, name, index, shape);
    let mut output = append_graph_edges(output, graph, shape.measure_count);
    output.push_str("  | _, _ => .unknown\n\n");
    output
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; path validation traverses a runtime slice iterator with all, which is not const on the pinned compiler."
)]
fn is_valid_site_path(path: &[u32], site_count: u32) -> bool {
    !path.is_empty() && path.len() <= MAX_CLOSURE && path.iter().all(|site| *site < site_count)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; closure validation performs checked TryFrom and runtime slice iteration with derived Graph equality."
)]
fn is_valid_closure_model(sites: &[Site], measure_count: usize, closure: &[ClosureEntry]) -> bool {
    if !closure_dimensions_valid(sites, measure_count, closure) {
        return false;
    }
    let Some(site_count) = u32::try_from(sites.len()).ok() else {
        return false;
    };
    sites
        .iter()
        .all(|site| closure.iter().any(|entry| entry.graph == site.graph))
        && closure
            .iter()
            .all(|entry| is_valid_site_path(&entry.path, site_count))
}

const fn closure_dimensions_valid(
    sites: &[Site],
    measure_count: usize,
    closure: &[ClosureEntry],
) -> bool {
    if sites.is_empty() || sites.len() > MAX_SITES {
        return false;
    }
    if measure_count == 0 || measure_count > MAX_MEASURES {
        return false;
    }
    !closure.is_empty() && closure.len() <= MAX_CLOSURE
}

fn append_closure_graphs(
    mut output: alloc::string::String,
    name: &str,
    shape: CertificateShape,
    closure: &[ClosureEntry],
) -> alloc::string::String {
    for (index, entry) in closure.iter().enumerate() {
        output = append_graph_definition(output, name, index, &entry.graph, shape);
    }
    output
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; Lean site-table emission grows an owned String through runtime push_str calls."
)]
fn append_site_table_header(
    mut output: alloc::string::String,
    name: &str,
    shape: CertificateShape,
) -> alloc::string::String {
    output.push_str("def ");
    output.push_str(name);
    output.push_str("Sites : Fin ");
    output = append_decimal(output, shape.site_count);
    output.push_str(" → MGraph ");
    output = append_decimal(output, shape.measure_count);
    output.push_str(" := fun site =>\n  match site.val with\n");
    output
}

fn append_site_table_rows(
    mut output: alloc::string::String,
    name: &str,
    sites: &[Site],
    closure: &[ClosureEntry],
) -> alloc::string::String {
    for (site_index, site) in sites.iter().enumerate() {
        let Some(graph_index) = closure.iter().position(|entry| entry.graph == site.graph) else {
            return output;
        };
        output.push_str("  | ");
        output = append_decimal(output, site_index);
        output.push_str(" => ");
        output = append_named_index(output, name, graph_index);
        output.push('\n');
    }
    output
}

fn append_closure_entry(
    mut output: alloc::string::String,
    name: &str,
    index: usize,
    entry: &ClosureEntry,
    has_next: bool,
) -> alloc::string::String {
    output.push_str("    ⟨");
    output = append_named_index(output, name, index);
    output.push_str(", [");
    output = append_site_path(output, &entry.path);
    output.push_str("]⟩");
    if has_next {
        output.push(',');
    }
    output.push('\n');
    output
}

fn append_closure_entries(
    output: alloc::string::String,
    name: &str,
    shape: CertificateShape,
    closure: &[ClosureEntry],
) -> alloc::string::String {
    let output = append_closure_entries_header(output, name, shape);
    let output = append_closure_entries_rows(output, name, closure);
    append_closure_entries_footer(output)
}

fn append_closure_entries_header(
    mut output: alloc::string::String,
    name: &str,
    shape: CertificateShape,
) -> alloc::string::String {
    output.push_str("def ");
    output.push_str(name);
    output.push_str("Entries : List (MatrixClosureEntry ");
    output = append_decimal(output, shape.site_count);
    output.push(' ');
    output = append_decimal(output, shape.measure_count);
    output.push_str(") :=\n  [\n");
    output
}

fn append_closure_entries_rows(
    mut output: alloc::string::String,
    name: &str,
    closure: &[ClosureEntry],
) -> alloc::string::String {
    for (index, entry) in closure.iter().enumerate() {
        output = append_closure_entry(
            output,
            name,
            index,
            entry,
            index < closure.len().saturating_sub(1),
        );
    }
    output
}

fn append_closure_entries_footer(mut output: alloc::string::String) -> alloc::string::String {
    output.push_str("  ]\n\n");
    output
}

/// Emit graph definitions, the site function, and path-bearing closure entries.
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; closure-model serialization performs runtime collection traversal and owned String growth."
)]
fn append_closure_model(
    mut output: alloc::string::String,
    name: &str,
    sites: &[Site],
    shape: CertificateShape,
    closure: &[ClosureEntry],
) -> (alloc::string::String, bool) {
    if shape.site_count != sites.len()
        || !is_valid_closure_model(sites, shape.measure_count, closure)
    {
        return (output, false);
    }
    output = append_closure_graphs(output, name, shape, closure);
    output = append_site_table_header(output, name, shape);
    output = append_site_table_rows(output, name, sites, closure);
    output.push_str("  | _ => (fun _ _ => .unknown)\n\n");
    output = append_closure_entries(output, name, shape, closure);
    (output, true)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; validation uses non-const TryFrom and runtime Vec indexing over the bounded certificate."
)]
fn valid_accepted_evidence(
    measures: &[Measure],
    sites: &[Site],
    closure: &[ClosureEntry],
    witnesses: &[StrictWitness],
) -> bool {
    if !is_valid_closure_model(sites, measures.len(), closure) || witnesses.len() > closure.len() {
        return false;
    }
    let Some(measure_count) = u32::try_from(measures.len()).ok() else {
        return false;
    };
    witnesses.iter().all(|witness| {
        let Some(closure_index) = usize::try_from(witness.closure_entry).ok() else {
            return false;
        };
        closure.get(closure_index).is_some() && witness.measure < measure_count
    })
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; validation traverses runtime closure entries and uses derived Graph equality and TryFrom."
)]
fn valid_refused_evidence(
    measures: &[Measure],
    sites: &[Site],
    closure: &[ClosureEntry],
    cycle: &ClosureEntry,
) -> bool {
    is_valid_closure_model(sites, measures.len(), closure)
        && closure.iter().any(|entry| entry.graph == cycle.graph)
        && u32::try_from(sites.len())
            .ok()
            .is_some_and(|site_count| is_valid_site_path(&cycle.path, site_count))
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; certificate validation delegates to runtime collection checks and derived enum/Graph equality."
)]
fn is_valid_certificate_entry(evidence: &Evidence) -> bool {
    match &evidence.verdict {
        Verdict::Accepted {
            closure,
            strict_witnesses,
            ..
        } => valid_accepted_evidence(
            &evidence.measures,
            &evidence.sites,
            closure,
            strict_witnesses,
        ),
        Verdict::Refused { closure, cycle, .. } => {
            valid_refused_evidence(&evidence.measures, &evidence.sites, closure, cycle)
        }
        Verdict::BoundExceeded { .. } | Verdict::Unsupported { .. } => false,
    }
}

fn append_certificate_comment(
    mut output: alloc::string::String,
    index: usize,
    shape: CertificateShape,
) -> alloc::string::String {
    output.push_str("\n/-- Bounded size-change evidence for definition ");
    output = append_decimal(output, index);
    output.push_str(": ");
    output = append_decimal(output, shape.site_count);
    output.push_str(" self-call site(s) over ");
    output = append_decimal(output, shape.measure_count);
    output.push_str(
        " declared measure(s). The matrices and paths below are checked by the model. -/\n",
    );
    output
}

fn append_one_certificate(
    output: alloc::string::String,
    entry: &Evidence,
    entry_index: usize,
) -> alloc::string::String {
    if !is_valid_certificate_entry(entry) {
        return output;
    }
    let name = append_decimal(alloc::string::String::from("definition_"), entry_index);
    let shape = CertificateShape {
        site_count: entry.sites.len(),
        measure_count: entry.measures.len(),
    };
    let output = append_certificate_comment(output, entry_index, shape);
    append_validated_certificate(output, entry, &name, shape)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; this renderer appends runtime-owned String output while validating dynamically sized report data."
)]
fn append_validated_certificate(
    output: alloc::string::String,
    entry: &Evidence,
    name: &str,
    shape: CertificateShape,
) -> alloc::string::String {
    let closure = match &entry.verdict {
        Verdict::Accepted { closure, .. } | Verdict::Refused { closure, .. } => closure.as_slice(),
        Verdict::BoundExceeded { .. } | Verdict::Unsupported { .. } => return output,
    };
    let (model, is_valid) = append_closure_model(output, name, &entry.sites, shape, closure);
    if !is_valid {
        return model;
    }
    append_certificate_proof(model, entry, name, shape)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; accepted/refused proof emission builds runtime String output from dynamically sized witnesses and paths."
)]
fn append_certificate_proof(
    output: alloc::string::String,
    entry: &Evidence,
    name: &str,
    shape: CertificateShape,
) -> alloc::string::String {
    match &entry.verdict {
        Verdict::Accepted {
            strict_witnesses, ..
        } => append_accepted_proof(output, name, shape, strict_witnesses),
        Verdict::Refused { closure, cycle, .. } => {
            append_refused_proof(output, name, shape, closure, cycle)
        }
        Verdict::BoundExceeded { .. } | Verdict::Unsupported { .. } => output,
    }
}

/// Render Lean-checkable closure certificates and idempotent refusal witnesses for one report.
pub fn certificate(evidence: &[Evidence]) -> alloc::string::String {
    let mut output = alloc::string::String::with_capacity(4096);
    output.push_str(
        "/-\n  Generated bounded size-change closure certificate (issue #101).\n\n  The finite matrix set and each call-site path are checked in the Lean model. Accepted\n  definitions carry one strict diagonal witness for every idempotent matrix; refusals name a\n  real idempotent closure entry without such an edge. Front-end/source correspondence is separate.\n-/\nimport NobleM2.SizeChangeSCT\n\nnamespace NobleSizeChangeCertificate\n\nopen NobleM2.SizeChange\n",
    );
    for (entry_index, entry) in evidence.iter().enumerate() {
        output = append_one_certificate(output, entry, entry_index);
    }
    output.push_str("\nend NobleSizeChangeCertificate");
    output
}

fn append_accepted_proof(
    output: alloc::string::String,
    name: &str,
    shape: CertificateShape,
    witnesses: &[StrictWitness],
) -> alloc::string::String {
    let output = append_accepted_definition(output, name, shape, witnesses);
    append_criterion_theorem(output, name, shape.measure_count)
}

fn append_accepted_definition(
    output: alloc::string::String,
    name: &str,
    shape: CertificateShape,
    witnesses: &[StrictWitness],
) -> alloc::string::String {
    let output = append_accepted_head(output, name, shape);
    let mut output = append_strict_witnesses(output, name, witnesses);
    output.push_str("], by decide, by decide⟩\n\n");
    output
}

fn append_accepted_head(
    mut output: alloc::string::String,
    name: &str,
    shape: CertificateShape,
) -> alloc::string::String {
    output.push_str("def ");
    output.push_str(name);
    output.push_str("Certificate : MatrixClosureCertificate ");
    output = append_decimal(output, shape.site_count);
    output.push(' ');
    output = append_decimal(output, shape.measure_count);
    output.push(' ');
    output.push_str(name);
    output.push_str("Sites :=\n  ⟨");
    output.push_str(name);
    output.push_str("Entries, [");
    output
}

fn append_strict_witnesses(
    mut output: alloc::string::String,
    name: &str,
    witnesses: &[StrictWitness],
) -> alloc::string::String {
    for (witness_index, witness) in witnesses.iter().enumerate() {
        if witness_index > 0 {
            output.push_str(", ");
        }
        output.push('⟨');
        output = append_named_u32(output, name, witness.closure_entry);
        output.push_str(", ");
        output = append_u32(output, witness.measure);
        output.push('⟩');
    }
    output
}

fn append_criterion_theorem(
    mut output: alloc::string::String,
    name: &str,
    measure_count: usize,
) -> alloc::string::String {
    output.push_str("theorem ");
    output.push_str(name);
    output.push_str("Criterion : ∀ {graph : MGraph ");
    output = append_decimal(output, measure_count);
    output.push_str("}, MatrixClosure ");
    output.push_str(name);
    output.push_str("Sites graph → composeMatrix graph graph = graph → HasStrictDiagonal graph := by\n  intro graph member idempotent\n  exact certificate_closure_criterion ");
    output.push_str(name);
    output.push_str("Certificate member idempotent\n");
    output
}

fn append_refused_proof(
    output: alloc::string::String,
    name: &str,
    shape: CertificateShape,
    closure: &[ClosureEntry],
    cycle: &ClosureEntry,
) -> alloc::string::String {
    let output = append_closure_set_certificate(output, name, shape);
    let output = append_refusal_definition(output, name, closure, cycle);
    append_refusal_theorems(output, name)
}

fn append_closure_set_certificate(
    mut output: alloc::string::String,
    name: &str,
    shape: CertificateShape,
) -> alloc::string::String {
    output.push_str("def ");
    output.push_str(name);
    output.push_str("ClosureSetCertificate : MatrixClosureSetCertificate ");
    output = append_decimal(output, shape.site_count);
    output.push(' ');
    output = append_decimal(output, shape.measure_count);
    output.push(' ');
    output.push_str(name);
    output.push_str("Sites := ⟨");
    output.push_str(name);
    output.push_str("Entries, by decide⟩\n\ndef ");
    output
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; refusal emission searches runtime closure entries and grows an owned String for the selected path."
)]
fn append_refusal_definition(
    mut output: alloc::string::String,
    name: &str,
    closure: &[ClosureEntry],
    cycle: &ClosureEntry,
) -> alloc::string::String {
    let Some(graph_index) = closure.iter().position(|entry| entry.graph == cycle.graph) else {
        return output;
    };
    output.push_str(name);
    output.push_str("Refusal : MatrixIdempotentRefusal ");
    output.push_str(name);
    output.push_str("ClosureSetCertificate :=\n  ⟨⟨");
    output = append_named_index(output, name, graph_index);
    output.push_str(", [");
    output = append_site_path(output, &cycle.path);
    output.push_str("]⟩, by decide, by decide, by decide⟩\n\n");
    output
}

fn append_site_path(mut output: alloc::string::String, path: &[u32]) -> alloc::string::String {
    for (path_index, site) in path.iter().enumerate() {
        if path_index > 0 {
            output.push_str(", ");
        }
        output = append_u32(output, *site);
    }
    output
}

fn append_refusal_theorems(output: alloc::string::String, name: &str) -> alloc::string::String {
    let output = append_refusal_closure_theorem(output, name);
    append_refusal_diagonal_theorem(output, name)
}

fn append_refusal_closure_theorem(
    mut output: alloc::string::String,
    name: &str,
) -> alloc::string::String {
    output.push_str("theorem ");
    output.push_str(name);
    output.push_str("RefusalGraphIsClosure : MatrixClosure ");
    output.push_str(name);
    output.push_str("Sites ");
    output.push_str(name);
    output.push_str("Refusal.entry.graph := refusal_graph_in_closure ");
    output.push_str(name);
    output.push_str("Refusal\n\n");
    output
}

fn append_refusal_diagonal_theorem(
    mut output: alloc::string::String,
    name: &str,
) -> alloc::string::String {
    output.push_str("theorem ");
    output.push_str(name);
    output.push_str("RefusalHasNoStrictDiagonal : ¬ HasStrictDiagonal ");
    output.push_str(name);
    output.push_str("Refusal.entry.graph := refusal_no_strict_diagonal ");
    output.push_str(name);
    output.push_str("Refusal\n");
    output
}

/// Append one decimal number without pulling in a formatting layer.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; append_decimal is a total serializer for usize values and returns before emitting a partial digit sequence if checked digit conversion or buffer bounds fail; serialization must not assert on its numeric input."
)]
fn append_decimal(mut output: alloc::string::String, mut value: usize) -> alloc::string::String {
    let mut digits = [0_u8; core::mem::size_of::<usize>().saturating_mul(3)];
    let mut at = digits.len();
    loop {
        at = match at.checked_sub(1) {
            Some(at) => at,
            None => return output,
        };
        let digit = match u8::try_from(value % 10) {
            Ok(digit) => digit,
            Err(_) => return output,
        };
        digits[at] = match b'0'.checked_add(digit) {
            Some(ascii) => ascii,
            None => return output,
        };
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for digit in &digits[at..] {
        output.push(char::from(*digit));
    }
    output
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; append_u32 is a total serializer for a fixed-width report index and returns without asserting if its digit buffer or conversion invariant is broken."
)]
fn append_u32(mut output: alloc::string::String, mut value: u32) -> alloc::string::String {
    let mut digits = [0_u8; 10];
    let mut at = digits.len();
    loop {
        let Some(previous) = at.checked_sub(1) else {
            return output;
        };
        at = previous;
        let Some(digit) = u8::try_from(value % 10).ok() else {
            return output;
        };
        let Some(ascii) = b'0'.checked_add(digit) else {
            return output;
        };
        digits[at] = ascii;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for digit in &digits[at..] {
        output.push(char::from(*digit));
    }
    output
}
