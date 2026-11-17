//! Bounded, non-executable source proof terms. No Lean source is read from a module.
#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
)]
use crate::{Diagnostic, DiagnosticKind, Limits, Span};
#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
)]
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};

mod named_v2;
pub use named_v2::prepare_named_contract;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Form {
    pub kind: FormKind,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormKind {
    Atom(String),
    List(Vec<Form>),
}
#[derive(Clone, Debug)]
pub struct ResolvedSourceModule {
    pub module: String,
    pub version: u32,
    pub full_source: Vec<u8>,
    /// The retained namespace owner; never a portable module identity.
    pub owner: u64,
}
#[derive(Clone, Debug)]
pub struct NamedImportBinding {
    /// The import alias as resolved when the original caller was installed.
    pub alias: String,
    pub owner: u64,
    pub module_index: usize,
}
#[derive(Clone, Debug)]
pub struct NamedUseOrigin {
    pub definition: noble_kernel::contracts::Definition,
    pub definition_identity: u64,
    pub definition_owner: u64,
    pub definition_name: String,
    /// Lexical definition order in the original owner module, not a kernel slot.
    pub definition_ordinal: u32,
    pub definition_span: Span,
    pub body_span: Span,
    pub module_index: usize,
    /// None only for the enclosing, single-invocation subject root.
    pub caller_definition: Option<noble_kernel::contracts::Definition>,
    pub source_node: Option<u32>,
    pub source_span: Option<Span>,
    pub candidate_node: noble_kernel::untrusted::NodeId,
}
#[derive(Clone, Debug)]
pub struct SubjectBinding {
    pub module: String,
    pub version: u32,
    pub module_source: Vec<u8>,
    pub definition: String,
    pub definition_source: Vec<u8>,
    pub source_dependencies: Vec<ResolvedSourceModule>,
    /// Alias snapshot at source installation, not the current mutable alias map.
    pub named_imports: Vec<NamedImportBinding>,
    /// Source-resolved, per-use provenance; empty for historical contract 1.
    pub named_uses: Vec<NamedUseOrigin>,
    /// Session-local only; this number is never used as a portable artifact hash.
    pub definition_identity: u64,
    /// Declaration order within the original module (not a specialization slot).
    pub definition_ordinal: u32,
    /// Session-local owner recorded on the resolved definition.
    pub definition_owner: u64,
    pub accepted_submission: noble_kernel::execution::Submission,
    pub input_types: Vec<noble_kernel::types::Ty>,
    pub output_types: Vec<noble_kernel::types::Ty>,
    pub effects: noble_kernel::types::EffSet,
    pub source_span: Span,
}
#[derive(Clone, Debug)]
pub struct ContractGoal {
    pub contract_name: String,
    pub revision: u32,
    pub contract_span: Span,
    pub subject: SubjectBinding,
    pub inputs: Vec<(String, String)>,
    pub outputs: Vec<(String, String)>,
    pub requires: Form,
    pub ensures: Form,
}
#[derive(Clone, Debug)]
pub enum PendingGoal {
    Pure { proposition: Form },
    Contract { contract: Box<ContractGoal> },
}
#[derive(Clone, Debug)]
pub struct ProofObligation {
    pub name: String,
    pub revision: u32,
    pub goal: PendingGoal,
    pub term: Form,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct ProofDependency {
    pub name: String,
    pub module: String,
    pub version: u32,
    pub source: Vec<u8>,
    pub declaration: ProofObligation,
    pub exported: bool,
    pub dependencies: Vec<ProofDependency>,
    /// Exact host-reviewed model library identity at original acceptance.
    pub model_revision: String,
    /// Exact checker/lowering/consumer identity at original acceptance.
    pub checker_revision: String,
}
#[derive(Clone, Debug)]
pub struct ProofBatch {
    pub module: String,
    pub version: u32,
    pub source: Vec<u8>,
    pub generation: u64,
    pub dependencies: Vec<ProofDependency>,
    pub obligations: Vec<ProofObligation>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofKind {
    Pure,
    Contract,
    NamedContract,
}
#[derive(Clone, Debug)]
pub struct CheckedProof {
    pub name: String,
    pub kind: ProofKind,
    pub lean_term: String,
    pub claim: String,
    pub model_revision: String,
    pub checker_revision: String,
}

#[derive(Clone, Copy, Debug)]
pub struct ProofBudgets {
    pub normalization_work: u32,
    pub substitution_work: u32,
}
impl ProofBudgets {
    pub const fn from_limits(limits: Limits) -> Self {
        Self {
            normalization_work: limits.work,
            substitution_work: limits.work,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ProofWork {
    pub normalization: u32,
    pub substitution: u32,
    pub total: u32,
}
fn reject(span: Span, reason: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticKind::Invalid, span, reason)
}
fn exhausted(span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticKind::Exhausted,
        span,
        "proof size, depth or work limit exhausted",
    )
}
fn deeper(depth: u32, span: Span) -> Result<u32, Diagnostic> {
    depth.checked_add(1).ok_or_else(|| exhausted(span))
}
#[derive(Clone, Copy)]
struct TreeBounds {
    frames: usize,
    bytes: usize,
}
impl TreeBounds {
    fn new(limits: Limits) -> Option<Self> {
        // Source Code nodes are visited; Eq contributes two unvisited index units,
        // and a Pi/Codes sort contributes one. Four units per visited node plus
        // four covers every freshly parsed proposition. Before a replacement,
        // Meter::substitute charges its entire body (at most limits.work); the
        // only expanding argument is a freshly parsed Code (at most limits.nodes).
        // Thus even repeated replacements cannot grow a surviving tree beyond
        // this bound: each replacement charges its input anew.
        let source_nodes = usize::try_from(limits.nodes).ok()?;
        let work = usize::try_from(limits.work).ok()?;
        let source = source_nodes.saturating_mul(4).saturating_add(4);
        let substituted = work.saturating_mul(source_nodes.max(1));
        let nodes = source.max(substituted);
        // Three pending frames per structural unit (plus the initial root)
        // cover deferred children and Lean punctuation, even for nested Pi.
        // Use saturating caps so 32-bit targets cannot reject a realizable tree
        // merely because user-supplied limits exceed addressable memory.
        // The longest leaf is a decimal usize (20 bytes on 64-bit); all
        // constructor punctuation is shorter than 128 bytes per size unit.
        Some(Self {
            frames: nodes.saturating_mul(3).saturating_add(2),
            bytes: nodes.saturating_mul(128),
        })
    }
}
#[derive(Clone, Copy)]
struct Meter<'a> {
    nodes: u32,
    work: u32,
    normalization: u32,
    substitution: u32,
    config: &'a MeterConfig,
}
struct MeterConfig {
    budgets: ProofBudgets,
    limits: Limits,
    tree_bounds: TreeBounds,
}
impl core::ops::Deref for Meter<'_> {
    type Target = MeterConfig;
    fn deref(&self) -> &MeterConfig {
        self.config
    }
}
// A failed visit may already have incremented work. Intro continuations still
// charge their wrappers after that failure and can override its diagnostic, so
// every transition returns its owned meter on both paths.
type Transition<'a, T> = (Meter<'a>, Result<T, Diagnostic>);
macro_rules! advance {
    ($meter:ident,$transition:expr) => {{
        let (next, result) = $transition;
        $meter = next;
        result?
    }};
}
impl<'a> Meter<'a> {
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
    )]
    fn visit(mut self, depth: u32, span: Span) -> Transition<'a, ()> {
        let Some(nodes) = self.nodes.checked_add(1) else {
            return (self, Err(exhausted(span)));
        };
        self.nodes = nodes;
        let Some(work) = self.work.checked_add(1) else {
            return (self, Err(exhausted(span)));
        };
        self.work = work;
        if self.nodes > self.limits.nodes
            || self.work > self.limits.work
            || depth > self.limits.depth
        {
            return (self, Err(exhausted(span)));
        }
        (self, Ok(()))
    }
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
    )]
    fn charge(mut self, amount: usize, span: Span) -> Transition<'a, ()> {
        let Ok(units) = u32::try_from(amount) else {
            return (self, Err(exhausted(span)));
        };
        let Some(work) = self.work.checked_add(units) else {
            return (self, Err(exhausted(span)));
        };
        self.work = work;
        if self.work > self.limits.work {
            return (self, Err(exhausted(span)));
        }
        (self, Ok(()))
    }
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
    )]
    fn normalize(mut self, amount: usize, span: Span) -> Transition<'a, ()> {
        let Ok(units) = u32::try_from(amount) else {
            return (self, Err(exhausted(span)));
        };
        let Some(normalization) = self.normalization.checked_add(units) else {
            return (self, Err(exhausted(span)));
        };
        self.normalization = normalization;
        if self.normalization > self.budgets.normalization_work {
            return (
                self,
                Err(Diagnostic::new(
                    DiagnosticKind::Exhausted,
                    span,
                    "normalization work exhausted",
                )),
            );
        }
        self.charge(amount, span)
    }
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
    )]
    fn substitute(mut self, amount: usize, span: Span) -> Transition<'a, ()> {
        let Ok(units) = u32::try_from(amount) else {
            return (self, Err(exhausted(span)));
        };
        let Some(substitution) = self.substitution.checked_add(units) else {
            return (self, Err(exhausted(span)));
        };
        self.substitution = substitution;
        if self.substitution > self.budgets.substitution_work {
            return (
                self,
                Err(Diagnostic::new(
                    DiagnosticKind::Exhausted,
                    span,
                    "substitution work exhausted",
                )),
            );
        }
        self.charge(amount, span)
    }
}
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn atom(form: &Form) -> Result<&str, Diagnostic> {
    match &form.kind {
        FormKind::Atom(value) => Ok(value),
        _ => Err(reject(form.span, "expected logical atom")),
    }
}
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn list(form: &Form) -> Result<&[Form], Diagnostic> {
    match &form.kind {
        FormKind::List(items) => Ok(items),
        _ => Err(reject(form.span, "expected logical form")),
    }
}
fn operation<'a>(form: &'a Form, head: &str, count: usize) -> Result<&'a [Form], Diagnostic> {
    let items = list(form)?;
    if items.len().checked_sub(1) != Some(count)
        || items.first().and_then(|a| atom(a).ok()) != Some(head)
    {
        return Err(reject(form.span, "unknown or ill-formed proof constructor"));
    }
    Ok(&items[1..])
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Code {
    Unit,
    Bool,
    I64,
    Text,
    Variable(usize),
    Pair(Box<Code>, Box<Code>),
    Sum(Box<Code>, Box<Code>),
    List(Box<Code>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Sort {
    Codes,
    Value(Code),
    Premise(Box<Proposition>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Proposition {
    Pi(usize, Sort, Box<Proposition>),
    Eq(Code, usize, usize),
}
const INLINE_TREE_FRAMES: usize = 32;
struct WorkStack<T> {
    inline: [Option<T>; INLINE_TREE_FRAMES],
    inline_len: usize,
    spill: Vec<T>,
    total: usize,
}
impl<T> WorkStack<T> {
    fn new() -> Self {
        Self {
            inline: core::array::from_fn(|_| None),
            inline_len: 0,
            spill: Vec::new(),
            total: 0,
        }
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-contracts; the traversal stack is locally owned state and its mutation is the operation"
    )]
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the operation pushes into owned storage, which is not const"
    )]
    #[expect(
        tigerstyle::numeric_units,
        reason = "Owner: noble-contracts; the bound is the frame count supplied by the tree budget"
    )]
    /// Push one frame, spilling once the inline slots are exhausted.
    fn push_bounded(&mut self, value: T, limit: usize) -> Result<(), ()> {
        let Some(total) = self.total.checked_add(1).filter(|next| *next <= limit) else {
            return Err(());
        };
        if self.inline_len < INLINE_TREE_FRAMES && self.spill.is_empty() {
            self.inline[self.inline_len] = Some(value);
            self.inline_len = self.inline_len.saturating_add(1);
            self.total = total;
            return Ok(());
        }
        if self.spill.len() >= limit {
            return Err(());
        }
        // Spill only when a real walk exhausts its inline slots.
        self.spill.push(value);
        self.total = total;
        Ok(())
    }

    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-contracts; the traversal stack is locally owned state and its mutation is the operation"
    )]
    /// Pop one frame from the spill or the inline slots.
    fn pop_bounded(&mut self) -> Option<T> {
        if let Some(value) = self.spill.pop() {
            self.total = self.total.saturating_sub(1);
            return Some(value);
        }
        let next = self.inline_len.checked_sub(1)?;
        self.inline_len = next;
        let value = self.inline[next].take();
        self.total = self.total.saturating_sub(usize::from(value.is_some()));
        value
    }
}
// These operations mutate only a locally owned traversal stack. Keeping the
// mechanics at the callsite avoids passing a mutable proof-checking capability.
macro_rules! work_push {
    ($stack:ident,$value:expr,$bound:expr) => {
        $stack.push_bounded($value, $bound)
    };
}
macro_rules! work_pop {
    ($stack:ident) => {
        $stack.pop_bounded()
    };
}
enum ValidationFrame<'a> {
    Enter(&'a Form, u32),
    Children(&'a [Form], usize, u32),
}
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-contracts; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn validated<'m>(form: &Form, mut meter: Meter<'m>, depth: u32) -> Transition<'m, ()> {
    let result = (|| -> Result<(), Diagnostic> {
        let mut pending = WorkStack::new();
        let bound = meter.tree_bounds.frames;
        work_push!(pending, ValidationFrame::Enter(form, depth), bound)
            .map_err(|()| exhausted(form.span))?;
        while let Some(frame) = work_pop!(pending) {
            match frame {
                ValidationFrame::Enter(node, depth) => {
                    advance!(meter, meter.visit(depth, node.span));
                    match &node.kind {
                        FormKind::Atom(text) => {
                            advance!(meter, meter.charge(text.len(), node.span));
                            if text.is_empty()
                                || !text.bytes().all(|b| {
                                    b.is_ascii_alphanumeric()
                                        || b == b'-'
                                        || b == b'_'
                                        || b == b'.'
                                        || b == b'@'
                                })
                            {
                                return Err(reject(node.span, "invalid logical identifier"));
                            }
                        }
                        FormKind::List(items) if !items.is_empty() => {
                            work_push!(pending, ValidationFrame::Children(items, 0, depth), bound)
                                .map_err(|()| exhausted(node.span))?
                        }
                        FormKind::List(_) => {}
                    }
                }
                ValidationFrame::Children(items, index, depth) => {
                    let Some(item) = items.get(index) else {
                        continue;
                    };
                    let child_depth = deeper(depth, item.span)?;
                    let next = index.checked_add(1).ok_or_else(|| exhausted(item.span))?;
                    if next < items.len() {
                        work_push!(
                            pending,
                            ValidationFrame::Children(items, next, depth),
                            bound
                        )
                        .map_err(|()| exhausted(item.span))?;
                    }
                    work_push!(pending, ValidationFrame::Enter(item, child_depth), bound)
                        .map_err(|()| exhausted(item.span))?;
                }
            }
        }
        Ok(())
    })();
    (meter, result)
}
enum SizeNode<'a> {
    Code(&'a Code),
    Sort(&'a Sort),
    Proposition(&'a Proposition),
}
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-contracts; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn tree_size(root: SizeNode<'_>, bound: usize) -> Option<usize> {
    let mut pending = WorkStack::new();
    work_push!(pending, root, bound).ok()?;
    #[expect(
        tigerstyle::numeric_units,
        reason = "Owner: noble-contracts; the constant is a count of source elements, and the name states its unit at every use site through the surrounding type and call"
    )]
    let mut size = 0usize;
    while let Some(node) = work_pop!(pending) {
        match node {
            SizeNode::Code(Code::Pair(left, right) | Code::Sum(left, right)) => {
                size = size.checked_add(1)?;
                work_push!(pending, SizeNode::Code(right), bound).ok()?;
                work_push!(pending, SizeNode::Code(left), bound).ok()?;
            }
            SizeNode::Code(Code::List(item)) => {
                size = size.checked_add(1)?;
                work_push!(pending, SizeNode::Code(item), bound).ok()?;
            }
            SizeNode::Code(_) | SizeNode::Sort(Sort::Codes) => size = size.checked_add(1)?,
            SizeNode::Sort(Sort::Value(code)) => {
                work_push!(pending, SizeNode::Code(code), bound).ok()?;
            }
            SizeNode::Sort(Sort::Premise(prop)) => {
                work_push!(pending, SizeNode::Proposition(prop), bound).ok()?;
            }
            SizeNode::Proposition(Proposition::Pi(_, sort, body)) => {
                size = size.checked_add(1)?;
                work_push!(pending, SizeNode::Proposition(body), bound).ok()?;
                work_push!(pending, SizeNode::Sort(sort), bound).ok()?;
            }
            SizeNode::Proposition(Proposition::Eq(code, _, _)) => {
                size = size.checked_add(3)?;
                work_push!(pending, SizeNode::Code(code), bound).ok()?;
            }
        }
    }
    Some(size)
}
fn code_size(code: &Code, bound: usize) -> Option<usize> {
    if !matches!(code, Code::Pair(..) | Code::Sum(..) | Code::List(..)) {
        return Some(1);
    }
    tree_size(SizeNode::Code(code), bound)
}
fn proposition_size(prop: &Proposition, bound: usize) -> Option<usize> {
    tree_size(SizeNode::Proposition(prop), bound)
}
#[derive(Clone)]
struct Binder {
    name: String,
    sort: Sort,
}
#[derive(Clone)]
struct Available {
    name: String,
    goal: Proposition,
    lean_term: String,
}
#[derive(Clone, Copy)]
enum Rewrite<'a> {
    Shift(usize),
    Replace(usize, &'a Argument),
}
enum RewriteFrame<'a> {
    Code(&'a Code, bool),
    Sort(&'a Sort),
    Proposition(&'a Proposition),
    Pair,
    Sum,
    List,
    Value,
    Premise,
    Pi(usize),
    Eq(usize, usize),
}
enum Rewritten {
    Code(Code),
    Sort(Sort),
    Proposition(Proposition),
}
macro_rules! push_rewrite_frame {
    ($stack:ident,$value:expr,$bound:expr,$span:expr) => {
        work_push!($stack, $value, $bound).map_err(|()| exhausted($span))
    };
}
macro_rules! push_rewritten {
    ($stack:ident,$value:expr,$bound:expr,$span:expr) => {
        work_push!($stack, $value, $bound).map_err(|()| exhausted($span))
    };
}
macro_rules! take_code {
    ($stack:ident,$span:expr) => {
        match work_pop!($stack) {
            Some(Rewritten::Code(code)) => Ok(code),
            Some(Rewritten::Sort(_) | Rewritten::Proposition(_)) | None => Err(exhausted($span)),
        }
    };
}
macro_rules! take_sort {
    ($stack:ident,$span:expr) => {
        match work_pop!($stack) {
            Some(Rewritten::Sort(sort)) => Ok(sort),
            Some(Rewritten::Code(_) | Rewritten::Proposition(_)) | None => Err(exhausted($span)),
        }
    };
}
macro_rules! take_prop {
    ($stack:ident,$span:expr) => {
        match work_pop!($stack) {
            Some(Rewritten::Proposition(prop)) => Ok(prop),
            Some(Rewritten::Code(_) | Rewritten::Sort(_)) | None => Err(exhausted($span)),
        }
    };
}
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-contracts; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn rewrite_prop(
    prop: &Proposition,
    mode: Rewrite<'_>,
    span: Span,
    bound: usize,
) -> Result<Proposition, Diagnostic> {
    let mut frames = WorkStack::new();
    let mut values = WorkStack::new();
    push_rewrite_frame!(frames, RewriteFrame::Proposition(prop), bound, span)?;
    while let Some(frame) = work_pop!(frames) {
        match frame {
            RewriteFrame::Code(code, copy) => match code {
                Code::Pair(left, right) | Code::Sum(left, right) => {
                    push_rewrite_frame!(
                        frames,
                        if matches!(code, Code::Pair(..)) {
                            RewriteFrame::Pair
                        } else {
                            RewriteFrame::Sum
                        },
                        bound,
                        span
                    )?;
                    push_rewrite_frame!(frames, RewriteFrame::Code(right, copy), bound, span)?;
                    push_rewrite_frame!(frames, RewriteFrame::Code(left, copy), bound, span)?;
                }
                Code::List(item) => {
                    push_rewrite_frame!(frames, RewriteFrame::List, bound, span)?;
                    push_rewrite_frame!(frames, RewriteFrame::Code(item, copy), bound, span)?;
                }
                Code::Variable(index) => {
                    let rewritten = if copy {
                        *index
                    } else {
                        match mode {
                            Rewrite::Shift(base) => {
                                index.checked_add(base).ok_or_else(|| exhausted(span))?
                            }
                            Rewrite::Replace(binder, Argument::Code(argument))
                                if *index == binder =>
                            {
                                // Copy the argument without applying the same substitution again.
                                push_rewrite_frame!(
                                    frames,
                                    RewriteFrame::Code(argument, true),
                                    bound,
                                    span
                                )?;
                                continue;
                            }
                            // Every eliminated binder shifts later Type0 positions, not only Code binders.
                            Rewrite::Replace(binder, _) if *index > binder => {
                                index.checked_sub(1).ok_or_else(|| exhausted(span))?
                            }
                            Rewrite::Replace(..) => *index,
                        }
                    };
                    push_rewritten!(
                        values,
                        Rewritten::Code(Code::Variable(rewritten)),
                        bound,
                        span
                    )?;
                }
                Code::Unit => push_rewritten!(values, Rewritten::Code(Code::Unit), bound, span)?,
                Code::Bool => push_rewritten!(values, Rewritten::Code(Code::Bool), bound, span)?,
                Code::I64 => push_rewritten!(values, Rewritten::Code(Code::I64), bound, span)?,
                Code::Text => push_rewritten!(values, Rewritten::Code(Code::Text), bound, span)?,
            },
            RewriteFrame::Sort(sort) => match sort {
                Sort::Codes => push_rewritten!(values, Rewritten::Sort(Sort::Codes), bound, span)?,
                Sort::Value(code) => {
                    push_rewrite_frame!(frames, RewriteFrame::Value, bound, span)?;
                    push_rewrite_frame!(frames, RewriteFrame::Code(code, false), bound, span)?;
                }
                Sort::Premise(prop) => {
                    push_rewrite_frame!(frames, RewriteFrame::Premise, bound, span)?;
                    push_rewrite_frame!(frames, RewriteFrame::Proposition(prop), bound, span)?;
                }
            },
            RewriteFrame::Proposition(prop) => match prop {
                Proposition::Pi(index, sort, body) => {
                    let rewritten = match mode {
                        Rewrite::Shift(base) => {
                            index.checked_add(base).ok_or_else(|| exhausted(span))?
                        }
                        Rewrite::Replace(..) => {
                            index.checked_sub(1).ok_or_else(|| exhausted(span))?
                        }
                    };
                    push_rewrite_frame!(frames, RewriteFrame::Pi(rewritten), bound, span)?;
                    push_rewrite_frame!(frames, RewriteFrame::Proposition(body), bound, span)?;
                    push_rewrite_frame!(frames, RewriteFrame::Sort(sort), bound, span)?;
                }
                Proposition::Eq(code, left, right) => {
                    push_rewrite_frame!(frames, RewriteFrame::Eq(*left, *right), bound, span)?;
                    push_rewrite_frame!(frames, RewriteFrame::Code(code, false), bound, span)?;
                }
            },
            RewriteFrame::Pair | RewriteFrame::Sum => {
                let right = take_code!(values, span)?;
                let left = take_code!(values, span)?;
                let code = if matches!(frame, RewriteFrame::Pair) {
                    Code::Pair(Box::new(left), Box::new(right))
                } else {
                    Code::Sum(Box::new(left), Box::new(right))
                };
                push_rewritten!(values, Rewritten::Code(code), bound, span)?;
            }
            RewriteFrame::List => {
                let item = take_code!(values, span)?;
                push_rewritten!(
                    values,
                    Rewritten::Code(Code::List(Box::new(item))),
                    bound,
                    span
                )?;
            }
            RewriteFrame::Value => {
                let code = take_code!(values, span)?;
                push_rewritten!(values, Rewritten::Sort(Sort::Value(code)), bound, span)?;
            }
            RewriteFrame::Premise => {
                let prop = take_prop!(values, span)?;
                push_rewritten!(
                    values,
                    Rewritten::Sort(Sort::Premise(Box::new(prop))),
                    bound,
                    span
                )?;
            }
            RewriteFrame::Pi(index) => {
                let body = take_prop!(values, span)?;
                let sort = take_sort!(values, span)?;
                push_rewritten!(
                    values,
                    Rewritten::Proposition(Proposition::Pi(index, sort, Box::new(body))),
                    bound,
                    span
                )?;
            }
            RewriteFrame::Eq(left, right) => {
                let code = take_code!(values, span)?;
                let (left, right) = match mode {
                    Rewrite::Shift(base) => (
                        left.checked_add(base).ok_or_else(|| exhausted(span))?,
                        right.checked_add(base).ok_or_else(|| exhausted(span))?,
                    ),
                    Rewrite::Replace(binder, argument) => (
                        replace_index(left, binder, argument, span)?,
                        replace_index(right, binder, argument, span)?,
                    ),
                };
                push_rewritten!(
                    values,
                    Rewritten::Proposition(Proposition::Eq(code, left, right)),
                    bound,
                    span
                )?;
            }
        }
    }
    take_prop!(values, span)
}
#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-contracts; the same-typed parameters are positional by contract and named at every call site"
)]
fn shifted_prop(
    prop: &Proposition,
    base: usize,
    span: Span,
    bound: usize,
) -> Result<Proposition, Diagnostic> {
    rewrite_prop(prop, Rewrite::Shift(base), span, bound)
}
#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-contracts; the same-typed parameters are positional by contract and named at every call site"
)]
fn replace_prop(
    prop: &Proposition,
    binder: usize,
    argument: &Argument,
    span: Span,
    bound: usize,
) -> Result<Proposition, Diagnostic> {
    rewrite_prop(prop, Rewrite::Replace(binder, argument), span, bound)
}
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-contracts; the same-typed parameters are positional by contract and named at every call site"
)]
fn replace_index(
    index: usize,
    binder: usize,
    argument: &Argument,
    span: Span,
) -> Result<usize, Diagnostic> {
    if index == binder {
        Ok(if let Argument::Value(i) = argument {
            *i
        } else {
            index
        })
    } else if index > binder {
        index.checked_sub(1).ok_or_else(|| exhausted(span))
    } else {
        Ok(index)
    }
}
enum Argument {
    Code(Code),
    Value(usize),
    Premise,
}
enum ParseFrame<'a> {
    Code(&'a Form, u32),
    CodeSecond(&'a Form, u32, bool),
    CodePair(bool, Span),
    CodeList(Span),
    Sort(&'a Form, u32),
    SortValue(Span),
    SortPremise(Span),
    Proposition(&'a Form, u32),
    PiAfterSort {
        name: String,
        body: &'a Form,
        depth: u32,
    },
    PiFinish(usize, Span),
    EqAfterCode(&'a Form, &'a Form),
}
fn stack_binder<'a>(stack: &'a WorkStack<Binder>, name: &str) -> Option<(usize, &'a Sort)> {
    if let Some(index) = stack.spill.iter().rposition(|item| item.name == name) {
        let position = stack.inline_len.checked_add(index)?;
        return Some((position, &stack.spill[index].sort));
    }
    if let Some(index) = stack.inline[..stack.inline_len]
        .iter()
        .rposition(|item| item.as_ref().is_some_and(|binder| binder.name == name))
    {
        return stack.inline[index]
            .as_ref()
            .map(|binder| (index, &binder.sort));
    }
    None
}
fn binder<'a>(
    context: &'a WorkStack<Binder>,
    local: &'a WorkStack<Binder>,
    name: &str,
) -> Option<(usize, &'a Sort)> {
    if let Some((position, sort)) = stack_binder(local, name) {
        return Some((context.total.checked_add(position)?, sort));
    }
    stack_binder(context, name)
}
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-contracts; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
)]
#[expect(
    tigerstyle::nested_conditionals,
    reason = "Owner: noble-contracts; the guard chain states the admission rule verbatim; extracting it would obscure the predicate it states"
)]
#[expect(
    tigerstyle::foreign_enum_catch_all,
    reason = "Owner: noble-contracts; the foreign enum is matched only to translate its known variants, and a new variant is a deliberate fail-closed default"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
fn parsed<'m>(
    root: ParseFrame<'_>,
    context: &WorkStack<Binder>,
    mut meter: Meter<'m>,
    span: Span,
) -> Transition<'m, Rewritten> {
    let result = (|| -> Result<Rewritten, Diagnostic> {
        let bound = meter.tree_bounds.frames;
        let mut frames = WorkStack::new();
        let mut values = WorkStack::new();
        // Only Pi binders enter this suffix. An existing caller context is borrowed,
        // never copied at each nested Pi.
        let mut local = WorkStack::new();
        macro_rules! push_frame {
            ($frame:expr,$at:expr) => {
                work_push!(frames, $frame, bound).map_err(|()| exhausted($at))?
            };
        }
        macro_rules! push_value {
            ($value:expr,$at:expr) => {
                push_rewritten!(values, $value, bound, $at)?
            };
        }
        push_frame!(root, span);
        while let Some(frame) = work_pop!(frames) {
            match frame {
                ParseFrame::Code(form, depth) => {
                    advance!(meter, meter.visit(depth, form.span));
                    if let Ok(value) = atom(form) {
                        let ty = match value {
                            "Unit" => Code::Unit,
                            "Bool" => Code::Bool,
                            "I64" => Code::I64,
                            "Text" => Code::Text,
                            _ => {
                                let (position, sort) = binder(context, &local, value)
                                    .ok_or_else(|| reject(form.span, "not a bound Type0 code"))?;
                                if *sort != Sort::Codes {
                                    return Err(reject(form.span, "not a bound Type0 code"));
                                }
                                Code::Variable(position)
                            }
                        };
                        push_value!(Rewritten::Code(ty), form.span);
                        continue;
                    }
                    let items = list(form)?;
                    match items.first().and_then(|item| atom(item).ok()) {
                        Some("Pair") | Some("Sum") if items.len() == 3 => {
                            let first_depth = deeper(depth, items[1].span)?;
                            let is_pair = atom(&items[0])? == "Pair";
                            push_frame!(
                                ParseFrame::CodeSecond(&items[2], depth, is_pair),
                                form.span
                            );
                            push_frame!(ParseFrame::Code(&items[1], first_depth), items[1].span);
                        }
                        Some("List") if items.len() == 2 => {
                            let child_depth = deeper(depth, items[1].span)?;
                            push_frame!(ParseFrame::CodeList(form.span), form.span);
                            push_frame!(ParseFrame::Code(&items[1], child_depth), items[1].span);
                        }
                        _ => return Err(reject(form.span, "not a predicative Type0 code")),
                    }
                }
                ParseFrame::CodeSecond(second, depth, is_pair) => {
                    let next_depth = deeper(depth, second.span)?;
                    push_frame!(ParseFrame::CodePair(is_pair, second.span), second.span);
                    push_frame!(ParseFrame::Code(second, next_depth), second.span);
                }
                ParseFrame::CodePair(is_pair, at) => {
                    let right = take_code!(values, at)?;
                    let left = take_code!(values, at)?;
                    let ty = if is_pair {
                        Code::Pair(Box::new(left), Box::new(right))
                    } else {
                        Code::Sum(Box::new(left), Box::new(right))
                    };
                    push_value!(Rewritten::Code(ty), at);
                }
                ParseFrame::CodeList(at) => {
                    let item = take_code!(values, at)?;
                    push_value!(Rewritten::Code(Code::List(Box::new(item))), at);
                }
                ParseFrame::Sort(form, depth) => {
                    if atom(form).ok() == Some("Type0") {
                        push_value!(Rewritten::Sort(Sort::Codes), form.span);
                    } else if list(form)
                        .ok()
                        .and_then(|items| items.first())
                        .and_then(|item| atom(item).ok())
                        == Some("Eq")
                    {
                        let next_depth = deeper(depth, form.span)?;
                        push_frame!(ParseFrame::SortPremise(form.span), form.span);
                        push_frame!(ParseFrame::Proposition(form, next_depth), form.span);
                    } else {
                        let next_depth = deeper(depth, form.span)?;
                        push_frame!(ParseFrame::SortValue(form.span), form.span);
                        push_frame!(ParseFrame::Code(form, next_depth), form.span);
                    }
                }
                ParseFrame::SortValue(at) => {
                    let ty = take_code!(values, at)?;
                    push_value!(Rewritten::Sort(Sort::Value(ty)), at);
                }
                ParseFrame::SortPremise(at) => {
                    let proof = take_prop!(values, at)?;
                    push_value!(Rewritten::Sort(Sort::Premise(Box::new(proof))), at);
                }
                ParseFrame::Proposition(form, depth) => {
                    advance!(meter, meter.visit(depth, form.span));
                    let items = list(form)?;
                    match items.first().and_then(|item| atom(item).ok()) {
                        Some("Pi") if items.len() == 3 => {
                            let binding = list(&items[1])?;
                            if binding.len() != 2 {
                                return Err(reject(items[1].span, "Pi needs one typed binder"));
                            }
                            let name = atom(&binding[0])?.to_string();
                            let next_depth = deeper(depth, binding[1].span)?;
                            push_frame!(
                                ParseFrame::PiAfterSort {
                                    name,
                                    body: &items[2],
                                    depth
                                },
                                form.span
                            );
                            push_frame!(ParseFrame::Sort(&binding[1], next_depth), binding[1].span);
                        }
                        Some("Eq") if items.len() == 4 => {
                            let next_depth = deeper(depth, items[1].span)?;
                            push_frame!(ParseFrame::EqAfterCode(&items[2], &items[3]), form.span);
                            push_frame!(ParseFrame::Code(&items[1], next_depth), items[1].span);
                        }
                        _ => {
                            return Err(reject(
                                form.span,
                                "expected dependent Pi or typed Eq proposition",
                            ))
                        }
                    }
                }
                ParseFrame::PiAfterSort { name, body, depth } => {
                    let ty = take_sort!(values, body.span)?;
                    let index = context
                        .total
                        .checked_add(local.total)
                        .ok_or_else(|| exhausted(body.span))?;
                    // Each active binder belongs to a visited Pi; the derived frame
                    // cap is strictly above the source's maximum visited node count.
                    work_push!(local, Binder { name, sort: ty }, bound)
                        .map_err(|()| exhausted(body.span))?;
                    let next_depth = deeper(depth, body.span)?;
                    push_frame!(ParseFrame::PiFinish(index, body.span), body.span);
                    push_frame!(ParseFrame::Proposition(body, next_depth), body.span);
                }
                ParseFrame::PiFinish(index, at) => {
                    let body = take_prop!(values, at)?;
                    let ty = work_pop!(local).ok_or_else(|| exhausted(at))?.sort;
                    push_value!(
                        Rewritten::Proposition(Proposition::Pi(index, ty, Box::new(body))),
                        at
                    );
                }
                ParseFrame::EqAfterCode(left, right) => {
                    let ty = take_code!(values, left.span)?;
                    let left_name = atom(left)?;
                    let (first, first_sort) = binder(context, &local, left_name)
                        .ok_or_else(|| reject(left.span, "unbound equality value"))?;
                    if !matches!(first_sort,Sort::Value(value) if value == &ty) {
                        return Err(reject(left.span, "equality operand has wrong code"));
                    }
                    let right_name = atom(right)?;
                    let (second, second_sort) = binder(context, &local, right_name)
                        .ok_or_else(|| reject(right.span, "unbound equality value"))?;
                    if !matches!(second_sort,Sort::Value(value) if value == &ty) {
                        return Err(reject(right.span, "equality operand has wrong code"));
                    }
                    push_value!(
                        Rewritten::Proposition(Proposition::Eq(ty, first, second)),
                        right.span
                    );
                }
            }
        }
        work_pop!(values).ok_or_else(|| exhausted(span))
    })();
    (meter, result)
}
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-contracts; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn code<'m>(
    form: &Form,
    context: &WorkStack<Binder>,
    meter: Meter<'m>,
    depth: u32,
) -> Transition<'m, Code> {
    let (meter, result) = parsed(ParseFrame::Code(form, depth), context, meter, form.span);
    (
        meter,
        result.and_then(|value| match value {
            Rewritten::Code(code) => Ok(code),
            Rewritten::Sort(_) | Rewritten::Proposition(_) => Err(exhausted(form.span)),
        }),
    )
}
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-contracts; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn sort<'m>(
    form: &Form,
    context: &WorkStack<Binder>,
    meter: Meter<'m>,
    depth: u32,
) -> Transition<'m, Sort> {
    let (meter, result) = parsed(ParseFrame::Sort(form, depth), context, meter, form.span);
    (
        meter,
        result.and_then(|value| match value {
            Rewritten::Sort(sort) => Ok(sort),
            Rewritten::Code(_) | Rewritten::Proposition(_) => Err(exhausted(form.span)),
        }),
    )
}
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-contracts; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn proposition<'m>(
    form: &Form,
    context: &WorkStack<Binder>,
    meter: Meter<'m>,
    depth: u32,
) -> Transition<'m, Proposition> {
    let (meter, result) = parsed(
        ParseFrame::Proposition(form, depth),
        context,
        meter,
        form.span,
    );
    (
        meter,
        result.and_then(|value| match value {
            Rewritten::Proposition(proposition) => Ok(proposition),
            Rewritten::Code(_) | Rewritten::Sort(_) => Err(exhausted(form.span)),
        }),
    )
}
enum LeanFrame<'a> {
    Code(&'a Code),
    Sort(&'a Sort),
    Proposition(&'a Proposition),
    Text(&'static str),
    Number(usize),
}
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-contracts; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn lean_text(root: LeanFrame<'_>, span: Span, bounds: TreeBounds) -> Result<String, Diagnostic> {
    use core::fmt::Write;
    let mut pending = WorkStack::new();
    macro_rules! push {
        ($frame:expr) => {
            work_push!(pending, $frame, bounds.frames).map_err(|()| exhausted(span))?
        };
    }
    push!(root);
    // A small, constant initial allocation avoids reallocations for ordinary
    // composite terms; it never reserves from attacker-controlled limits.
    let mut output = String::with_capacity(64);
    macro_rules! append {
        ($part:expr) => {{
            let part = $part;
            if output
                .len()
                .checked_add(part.len())
                .is_none_or(|len| len > bounds.bytes)
            {
                return Err(exhausted(span));
            }
            output.push_str(part);
        }};
    }
    while let Some(frame) = work_pop!(pending) {
        match frame {
            LeanFrame::Text(part) => append!(part),
            LeanFrame::Number(index) => {
                let digits = if index == 0 {
                    1
                } else {
                    usize::try_from(index.ilog10())
                        .map_err(|_| exhausted(span))?
                        .checked_add(1)
                        .ok_or_else(|| exhausted(span))?
                };
                if output
                    .len()
                    .checked_add(digits)
                    .is_none_or(|len| len > bounds.bytes)
                {
                    return Err(exhausted(span));
                }
                write!(&mut output, "{index}").map_err(|_| exhausted(span))?;
            }
            LeanFrame::Code(code) => match code {
                Code::Unit => append!(".unit"),
                Code::Bool => append!(".bool"),
                Code::I64 => append!(".i64"),
                Code::Text => append!(".text"),
                Code::Variable(index) => {
                    push!(LeanFrame::Number(*index));
                    push!(LeanFrame::Text("v"));
                }
                Code::Pair(left, right) | Code::Sum(left, right) => {
                    push!(LeanFrame::Text(")"));
                    push!(LeanFrame::Code(right));
                    push!(LeanFrame::Text(" "));
                    push!(LeanFrame::Code(left));
                    push!(LeanFrame::Text(if matches!(code, Code::Pair(..)) {
                        "(.pair "
                    } else {
                        "(.sum "
                    }));
                }
                Code::List(item) => {
                    push!(LeanFrame::Text(")"));
                    push!(LeanFrame::Code(item));
                    push!(LeanFrame::Text("(.list "));
                }
            },
            LeanFrame::Sort(sort) => match sort {
                Sort::Codes => append!("PureTyCode"),
                Sort::Value(code) => {
                    push!(LeanFrame::Code(code));
                    push!(LeanFrame::Text("El "));
                }
                Sort::Premise(prop) => push!(LeanFrame::Proposition(prop)),
            },
            LeanFrame::Proposition(prop) => match prop {
                Proposition::Pi(index, sort, body) => {
                    push!(LeanFrame::Text(")"));
                    push!(LeanFrame::Proposition(body));
                    push!(LeanFrame::Text("), "));
                    push!(LeanFrame::Sort(sort));
                    push!(LeanFrame::Text(" : "));
                    push!(LeanFrame::Number(*index));
                    push!(LeanFrame::Text("(∀ (v"));
                }
                Proposition::Eq(_, left, right) => {
                    push!(LeanFrame::Number(*right));
                    push!(LeanFrame::Text("v"));
                    push!(LeanFrame::Text(" = "));
                    push!(LeanFrame::Number(*left));
                    push!(LeanFrame::Text("v"));
                }
            },
        }
    }
    Ok(output)
}
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn lean_code(code: &Code, span: Span, bounds: TreeBounds) -> Result<String, Diagnostic> {
    match code {
        Code::Unit => Ok(".unit".into()),
        Code::Bool => Ok(".bool".into()),
        Code::I64 => Ok(".i64".into()),
        Code::Text => Ok(".text".into()),
        Code::Variable(index) => Ok(format!("v{index}")),
        _ => lean_text(LeanFrame::Code(code), span, bounds),
    }
}
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn lean_sort(sort: &Sort, span: Span, bounds: TreeBounds) -> Result<String, Diagnostic> {
    match sort {
        Sort::Codes => Ok("PureTyCode".into()),
        Sort::Value(Code::Unit) => Ok("El .unit".into()),
        Sort::Value(Code::Bool) => Ok("El .bool".into()),
        Sort::Value(Code::I64) => Ok("El .i64".into()),
        Sort::Value(Code::Text) => Ok("El .text".into()),
        Sort::Value(Code::Variable(index)) => Ok(format!("El v{index}")),
        Sort::Premise(prop) => lean_proposition(prop, span, bounds),
        _ => lean_text(LeanFrame::Sort(sort), span, bounds),
    }
}
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
fn lean_proposition(
    prop: &Proposition,
    span: Span,
    bounds: TreeBounds,
) -> Result<String, Diagnostic> {
    match prop {
        Proposition::Eq(_, left, right) => Ok(format!("v{left} = v{right}")),
        _ => lean_text(LeanFrame::Proposition(prop), span, bounds),
    }
}
enum ProofGoal<'a> {
    Borrowed(&'a Proposition),
    Owned(Proposition),
}
impl ProofGoal<'_> {
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
    )]
    fn as_ref(&self) -> &Proposition {
        match self {
            Self::Borrowed(goal) => goal,
            Self::Owned(goal) => goal,
        }
    }
}
enum ProofSort<'a> {
    Borrowed(&'a Sort),
    Owned(Sort),
}
impl ProofSort<'_> {
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
    )]
    fn as_ref(&self) -> &Sort {
        match self {
            Self::Borrowed(sort) => sort,
            Self::Owned(sort) => sort,
        }
    }
}
enum ProofFrame<'a> {
    Infer(&'a Form, u32),
    Check(&'a Form, ProofGoal<'a>, u32),
    ApplyFunction {
        term: &'a Form,
        argument: &'a Form,
        depth: u32,
    },
    ApplyPremise {
        term: &'a Form,
        body: Box<Proposition>,
        index: usize,
        lowered: String,
        depth: u32,
    },
    AfterInfer {
        term: &'a Form,
        goal: ProofGoal<'a>,
        pi: bool,
    },
    IntroFinish {
        term: &'a Form,
        index: usize,
        expected: ProofSort<'a>,
    },
    SubstEquation {
        term: &'a Form,
        goal: ProofGoal<'a>,
        code: Code,
        name: String,
        motive: &'a Form,
        binder_span: Span,
        evidence: &'a Form,
        premise: &'a Form,
        depth: u32,
    },
    SubstPremise {
        term: &'a Form,
        code: Code,
        body: Proposition,
        index: usize,
        equality_lean: String,
    },
}
enum ProofValue {
    Inferred(Proposition, String),
    Checked(String),
}
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-contracts; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::foreign_enum_catch_all,
    reason = "Owner: noble-contracts; the foreign enum is matched only to translate its known variants, and a new variant is a deliberate fail-closed default"
)]
fn proof_walk<'a, 'm>(
    root: ProofFrame<'a>,
    available: &[Available],
    mut meter: Meter<'m>,
    span: Span,
) -> Transition<'m, ProofValue> {
    let result = (|| -> Result<ProofValue, Diagnostic> {
        let bound = meter.tree_bounds.frames;
        let mut frames = WorkStack::new();
        let mut context = WorkStack::new();
        let mut completed = None;
        let mut failure = None;
        macro_rules! schedule {
            ($frame:expr,$at:expr) => {
                work_push!(frames, $frame, bound).map_err(|()| exhausted($at))?
            };
        }
        macro_rules! take_inferred {
            ($at:expr) => {
                match completed.take() {
                    Some(ProofValue::Inferred(goal, lean)) => Ok((goal, lean)),
                    Some(ProofValue::Checked(_)) | None => Err(exhausted($at)),
                }
            };
        }
        macro_rules! take_checked {
            ($at:expr) => {
                match completed.take() {
                    Some(ProofValue::Checked(lean)) => Ok(lean),
                    Some(ProofValue::Inferred(..)) | None => Err(exhausted($at)),
                }
            };
        }
        macro_rules! finish_apply {
            ($term:expr,$depth:expr,$body:expr,$index:expr,$argument:expr,$lowered:expr,$expression:expr) => {{
                let (term, depth, body, index, argument, lowered, expression) = (
                    $term,
                    $depth,
                    $body,
                    $index,
                    $argument,
                    $lowered,
                    $expression,
                );
                let step = usize::try_from(depth)
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .ok_or_else(|| exhausted(term.span))?;
                advance!(meter, meter.charge(step, term.span));
                advance!(
                    meter,
                    meter.charge(
                        lowered
                            .len()
                            .checked_add(expression.len())
                            .ok_or_else(|| exhausted(term.span))?,
                        term.span
                    )
                );
                advance!(
                    meter,
                    meter.substitute(
                        proposition_size(&body, bound).ok_or_else(|| exhausted(term.span))?,
                        term.span
                    )
                );
                completed = Some(ProofValue::Inferred(
                    replace_prop(&body, index, &argument, term.span, bound)?,
                    format!("({lowered} {expression})"),
                ));
            }};
        }
        schedule!(root, span);
        while let Some(frame) = work_pop!(frames) {
            // An intro charges its wrapper even when its body failed. The charge
            // may itself fail and override that earlier diagnostic, so unwind all
            // pending intro continuations instead of returning immediately.
            if let ProofFrame::IntroFinish {
                term,
                index,
                expected,
            } = frame
            {
                work_pop!(context);
                let body = if failure.is_none() {
                    match completed.take() {
                        Some(ProofValue::Checked(body)) => Some(body),
                        _ => {
                            failure = Some(exhausted(term.span));
                            None
                        }
                    }
                } else {
                    None
                };
                let charged = (|| {
                    let sort_text = lean_sort(expected.as_ref(), term.span, meter.tree_bounds)?;
                    #[expect(
                        tigerstyle::numeric_units,
                        reason = "Owner: noble-contracts; the constant is a count of source elements, and the name states its unit at every use site through the surrounding type and call"
                    )]
                    let size = body
                        .as_ref()
                        .map_or(0, String::len)
                        .checked_add(sort_text.len())
                        .ok_or_else(|| exhausted(term.span))?;
                    advance!(meter, meter.charge(size, term.span));
                    Ok::<_, Diagnostic>(sort_text)
                })();
                match charged {
                    Err(error) => failure = Some(error),
                    Ok(sort_text) if failure.is_none() => {
                        let body = body.ok_or_else(|| exhausted(term.span))?;
                        completed = Some(ProofValue::Checked(format!(
                            "(fun (v{index} : {sort_text}) => {body})"
                        )));
                    }
                    Ok(_) => {}
                }
                continue;
            }
            if failure.is_some() {
                continue;
            }
            let step = (|| -> Result<(), Diagnostic> {
                match frame {
                    ProofFrame::Infer(term, depth) => {
                        advance!(meter, meter.visit(depth, term.span));
                        if let Ok(args) = operation(term, "use", 1) {
                            let name = atom(&args[0])?;
                            if let Some((index, sort)) = stack_binder(&context, name) {
                                let Sort::Premise(proof) = sort else {
                                    return Err(reject(
                                        term.span,
                                        "use does not name a proof premise",
                                    ));
                                };
                                completed = Some(ProofValue::Inferred(
                                    (**proof).clone(),
                                    format!("v{index}"),
                                ));
                                return Ok(());
                            }
                            let dependency = available
                                .iter()
                                .rev()
                                .find(|item| item.name == name)
                                .ok_or_else(|| {
                                    reject(
                                        term.span,
                                        "unresolved, private, forward or cyclic proof use",
                                    )
                                })?;
                            advance!(meter, meter.charge(dependency.lean_term.len(), term.span));
                            advance!(
                                meter,
                                meter.substitute(
                                    proposition_size(&dependency.goal, bound)
                                        .ok_or_else(|| exhausted(term.span))?,
                                    term.span
                                )
                            );
                            completed = Some(ProofValue::Inferred(
                                shifted_prop(&dependency.goal, context.total, term.span, bound)?,
                                dependency.lean_term.clone(),
                            ));
                            return Ok(());
                        }
                        if let Ok(args) = operation(term, "apply", 2) {
                            let child_depth = deeper(depth, args[0].span)?;
                            schedule!(
                                ProofFrame::ApplyFunction {
                                    term,
                                    argument: &args[1],
                                    depth
                                },
                                term.span
                            );
                            schedule!(ProofFrame::Infer(&args[0], child_depth), args[0].span);
                            return Ok(());
                        }
                        if let Ok(args) = operation(term, "refl", 1) {
                            let name = atom(&args[0])?;
                            let (index, sort) = stack_binder(&context, name)
                                .ok_or_else(|| reject(args[0].span, "unbound refl value"))?;
                            let Sort::Value(code) = sort else {
                                return Err(reject(term.span, "refl operand is not a total value"));
                            };
                            completed = Some(ProofValue::Inferred(
                                Proposition::Eq(code.clone(), index, index),
                                format!("(Eq.refl v{index})"),
                            ));
                            return Ok(());
                        }
                        return Err(reject(term.span, "proof term has no inferable proposition"));
                    }
                    ProofFrame::ApplyFunction {
                        term,
                        argument,
                        depth,
                    } => {
                        let (function, lowered) = take_inferred!(term.span)?;
                        let Proposition::Pi(index, domain, body) = function else {
                            return Err(reject(term.span, "apply requires dependent Pi proof"));
                        };
                        if index != context.total {
                            return Err(reject(term.span, "apply binder scope mismatch"));
                        }
                        match domain {
                            Sort::Codes => {
                                let ty = advance!(
                                    meter,
                                    code(argument, &context, meter, deeper(depth, argument.span)?)
                                );
                                let expression = lean_code(&ty, argument.span, meter.tree_bounds)?;
                                finish_apply!(
                                    term,
                                    depth,
                                    body,
                                    index,
                                    Argument::Code(ty),
                                    lowered,
                                    expression
                                );
                            }
                            Sort::Value(expected) => {
                                let name = atom(argument)?;
                                let (value, sort) =
                                    stack_binder(&context, name).ok_or_else(|| {
                                        reject(argument.span, "apply value is unbound")
                                    })?;
                                advance!(
                                    meter,
                                    meter.normalize(
                                        code_size(&expected, bound)
                                            .ok_or_else(|| exhausted(argument.span))?,
                                        argument.span
                                    )
                                );
                                if sort != &Sort::Value(expected) {
                                    return Err(reject(
                                        argument.span,
                                        "apply value has wrong code",
                                    ));
                                }
                                finish_apply!(
                                    term,
                                    depth,
                                    body,
                                    index,
                                    Argument::Value(value),
                                    lowered,
                                    format!("v{value}")
                                );
                            }
                            Sort::Premise(expected) => {
                                let child_depth = deeper(depth, argument.span)?;
                                schedule!(
                                    ProofFrame::ApplyPremise {
                                        term,
                                        body,
                                        index,
                                        lowered,
                                        depth
                                    },
                                    term.span
                                );
                                schedule!(
                                    ProofFrame::Check(
                                        argument,
                                        ProofGoal::Owned(*expected),
                                        child_depth
                                    ),
                                    argument.span
                                );
                            }
                        }
                    }
                    ProofFrame::ApplyPremise {
                        term,
                        body,
                        index,
                        lowered,
                        depth,
                    } => {
                        let expression = take_checked!(term.span)?;
                        finish_apply!(
                            term,
                            depth,
                            body,
                            index,
                            Argument::Premise,
                            lowered,
                            expression
                        );
                    }
                    ProofFrame::Check(term, goal, depth) => {
                        advance!(meter, meter.visit(depth, term.span));
                        if matches!(goal.as_ref(), Proposition::Pi(..)) {
                            let args = match operation(term, "intro", 2) {
                                Ok(args) => args,
                                Err(_) => {
                                    let child_depth = deeper(depth, term.span)?;
                                    schedule!(
                                        ProofFrame::AfterInfer {
                                            term,
                                            goal,
                                            pi: true
                                        },
                                        term.span
                                    );
                                    schedule!(ProofFrame::Infer(term, child_depth), term.span);
                                    return Ok(());
                                }
                            };
                            let binder_form = list(&args[0])?;
                            if binder_form.len() != 2 {
                                return Err(reject(args[0].span, "intro needs a typed binder"));
                            }
                            let name = atom(&binder_form[0])?.to_string();
                            let actual = advance!(
                                meter,
                                sort(
                                    &binder_form[1],
                                    &context,
                                    meter,
                                    deeper(depth, binder_form[1].span)?
                                )
                            );
                            advance!(
                                meter,
                                meter.normalize(
                                    proposition_size(goal.as_ref(), bound)
                                        .ok_or_else(|| exhausted(args[0].span))?,
                                    args[0].span
                                )
                            );
                            let (index, expected, rest) = match goal {
                                ProofGoal::Borrowed(Proposition::Pi(index, expected, rest)) => (
                                    *index,
                                    ProofSort::Borrowed(expected),
                                    ProofGoal::Borrowed(rest),
                                ),
                                ProofGoal::Owned(Proposition::Pi(index, expected, rest)) => {
                                    (index, ProofSort::Owned(expected), ProofGoal::Owned(*rest))
                                }
                                _ => return Err(exhausted(term.span)),
                            };
                            if &actual != expected.as_ref() || index != context.total {
                                return Err(reject(
                                    args[0].span,
                                    "intro binder type differs from goal",
                                ));
                            }
                            let body_depth = deeper(depth, args[1].span)?;
                            work_push!(context, Binder { name, sort: actual }, bound)
                                .map_err(|()| exhausted(args[1].span))?;
                            schedule!(
                                ProofFrame::IntroFinish {
                                    term,
                                    index,
                                    expected
                                },
                                term.span
                            );
                            schedule!(ProofFrame::Check(&args[1], rest, body_depth), args[1].span);
                            return Ok(());
                        }
                        if let Ok(parts) = operation(term, "subst", 3) {
                            let motive = operation(&parts[0], "motive", 2)?;
                            let binder_form = list(&motive[0])?;
                            if binder_form.len() != 2 {
                                return Err(reject(
                                    motive[0].span,
                                    "subst motive needs one typed value binder",
                                ));
                            }
                            let name = atom(&binder_form[0])?.to_string();
                            let Sort::Value(code) = advance!(
                                meter,
                                sort(
                                    &binder_form[1],
                                    &context,
                                    meter,
                                    deeper(depth, binder_form[1].span)?
                                )
                            ) else {
                                return Err(reject(
                                    motive[0].span,
                                    "subst motive binder must be a total value",
                                ));
                            };
                            let child_depth = deeper(depth, parts[1].span)?;
                            schedule!(
                                ProofFrame::SubstEquation {
                                    term,
                                    goal,
                                    code,
                                    name,
                                    motive: &motive[1],
                                    binder_span: motive[0].span,
                                    evidence: &parts[1],
                                    premise: &parts[2],
                                    depth,
                                },
                                term.span
                            );
                            schedule!(ProofFrame::Infer(&parts[1], child_depth), parts[1].span);
                            return Ok(());
                        }
                        {
                            let child_depth = deeper(depth, term.span)?;
                            schedule!(
                                ProofFrame::AfterInfer {
                                    term,
                                    goal,
                                    pi: false
                                },
                                term.span
                            );
                            schedule!(ProofFrame::Infer(term, child_depth), term.span);
                        }
                    }
                    ProofFrame::AfterInfer { term, goal, pi } => {
                        let (inferred, lowered) = take_inferred!(term.span)?;
                        advance!(
                            meter,
                            meter.normalize(
                                proposition_size(goal.as_ref(), bound)
                                    .ok_or_else(|| exhausted(term.span))?,
                                term.span
                            )
                        );
                        if inferred != *goal.as_ref() {
                            let reason = if pi {
                                "inferred proof does not match dependent Pi goal"
                            } else {
                                "proof does not establish exact expected equality"
                            };
                            return Err(reject(term.span, reason));
                        }
                        completed = Some(ProofValue::Checked(lowered));
                    }
                    ProofFrame::SubstEquation {
                        term,
                        goal,
                        code,
                        name,
                        motive,
                        binder_span,
                        evidence,
                        premise,
                        depth,
                    } => {
                        let (equation, equality_lean) = take_inferred!(evidence.span)?;
                        let Proposition::Eq(equation_code, left, right) = equation else {
                            return Err(reject(evidence.span, "subst requires equality evidence"));
                        };
                        advance!(
                            meter,
                            meter.normalize(
                                code_size(&code, bound).ok_or_else(|| exhausted(evidence.span))?,
                                evidence.span
                            )
                        );
                        if equation_code != code {
                            return Err(reject(
                                binder_span,
                                "subst motive has wrong equality type",
                            ));
                        }
                        let index = context.total;
                        let body_depth = deeper(depth, motive.span)?;
                        work_push!(
                            context,
                            Binder {
                                name,
                                sort: Sort::Value(code.clone())
                            },
                            bound
                        )
                        .map_err(|()| exhausted(motive.span))?;
                        let (next, body) = proposition(motive, &context, meter, body_depth);
                        meter = next;
                        work_pop!(context);
                        let body = body?;
                        if !matches!(body, Proposition::Eq(..)) {
                            return Err(reject(
                                motive.span,
                                "subst motive must be a total Eq proposition",
                            ));
                        }
                        #[expect(
                            tigerstyle::numeric_units,
                            reason = "Owner: noble-contracts; the constant is a count of source elements, and the name states its unit at every use site through the surrounding type and call"
                        )]
                        let size = proposition_size(&body, bound)
                            .and_then(|size| size.checked_mul(2))
                            .ok_or_else(|| exhausted(term.span))?;
                        advance!(meter, meter.substitute(size, term.span));
                        let before =
                            replace_prop(&body, index, &Argument::Value(left), term.span, bound)?;
                        let after =
                            replace_prop(&body, index, &Argument::Value(right), term.span, bound)?;
                        advance!(
                            meter,
                            meter.normalize(
                                proposition_size(goal.as_ref(), bound)
                                    .ok_or_else(|| exhausted(term.span))?,
                                term.span
                            )
                        );
                        if after != *goal.as_ref() {
                            return Err(reject(
                                term.span,
                                "subst result differs from expected goal",
                            ));
                        }
                        let child_depth = deeper(depth, premise.span)?;
                        schedule!(
                            ProofFrame::SubstPremise {
                                term,
                                code,
                                body,
                                index,
                                equality_lean
                            },
                            term.span
                        );
                        schedule!(
                            ProofFrame::Check(premise, ProofGoal::Owned(before), child_depth),
                            premise.span
                        );
                    }
                    ProofFrame::SubstPremise {
                        term,
                        code,
                        body,
                        index,
                        equality_lean,
                    } => {
                        let premise = take_checked!(term.span)?;
                        advance!(
                            meter,
                            meter.charge(
                                premise
                                    .len()
                                    .checked_add(equality_lean.len())
                                    .ok_or_else(|| exhausted(term.span))?,
                                term.span
                            )
                        );
                        completed = Some(ProofValue::Checked(format!(
                        "(Eq.ndrec (motive := fun (v{index} : El {}) => {}) {premise} {equality_lean})",
                        lean_code(&code,term.span,meter.tree_bounds)?,
                        lean_proposition(&body,term.span,meter.tree_bounds)?)));
                    }
                    ProofFrame::IntroFinish { .. } => unreachable!(),
                }
                Ok(())
            })();
            if let Err(error) = step {
                failure = Some(error);
                completed = None;
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }
        completed.ok_or_else(|| exhausted(span))
    })();
    (meter, result)
}
/// Checks only the closed predicative fragment; contract proofs have a separate
/// finite reviewed MC1 rule checker below. An unchecked term is never lowered.
pub fn check_batch(batch: &ProofBatch, limits: Limits) -> Result<Vec<CheckedProof>, Diagnostic> {
    check_batch_with_budgets(batch, limits, ProofBudgets::from_limits(limits))
        .map(|(checked, _)| checked)
}
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
pub fn check_batch_with_budgets(
    batch: &ProofBatch,
    limits: Limits,
    budgets: ProofBudgets,
) -> Result<(Vec<CheckedProof>, ProofWork), Diagnostic> {
    if u32::try_from(batch.source.len()).map_or(true, |len| len > limits.bytes) {
        return Err(exhausted(Span { start: 0, end: 0 }));
    }
    let config = MeterConfig {
        budgets,
        limits,
        tree_bounds: TreeBounds::new(limits).ok_or_else(|| exhausted(Span { start: 0, end: 0 }))?,
    };
    let mut meter = Meter {
        nodes: 0,
        work: 0,
        normalization: 0,
        substitution: 0,
        config: &config,
    };
    let mut result = Vec::new();
    // Each accepted obligation has already consumed a visited node. Do not
    // reserve caller-supplied fan-out before charging it.
    let result_bound =
        usize::try_from(limits.nodes).map_err(|_| exhausted(Span { start: 0, end: 0 }))?;
    let mut available = Vec::new();
    let available_bound = batch
        .dependencies
        .len()
        .checked_add(batch.obligations.len())
        .ok_or_else(|| exhausted(Span { start: 0, end: 0 }))?;
    for dependency in &batch.dependencies {
        if !dependency.exported {
            return Err(reject(
                dependency.declaration.span,
                "private proof cannot be imported",
            ));
        }
        let checked = advance!(meter, check_dependency(dependency, meter));
        if available.len() >= available_bound {
            return Err(exhausted(dependency.declaration.span));
        }
        available.push(checked);
    }
    for obligation in &batch.obligations {
        let (checked, exported) = advance!(meter, check_obligation(obligation, &available, meter));
        if let Some(goal) = exported {
            if available.len() >= available_bound {
                return Err(exhausted(obligation.span));
            }
            available.push(goal);
        }
        if result.len() >= result_bound {
            return Err(exhausted(obligation.span));
        }
        result.push(checked);
    }
    Ok((
        result,
        ProofWork {
            normalization: meter.normalization,
            substitution: meter.substitution,
            total: meter.work,
        },
    ))
}
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn check_obligation<'m>(
    obligation: &ProofObligation,
    available: &[Available],
    mut meter: Meter<'m>,
) -> Transition<'m, (CheckedProof, Option<Available>)> {
    let result = (|| -> Result<(CheckedProof, Option<Available>), Diagnostic> {
        advance!(meter, meter.visit(0, obligation.span));
        advance!(meter, validated(&obligation.term, meter, 0));
        match &obligation.goal {
            PendingGoal::Pure {
                proposition: statement,
            } => {
                if obligation.revision != 1 {
                    return Err(reject(obligation.span, "pure proof requires revision 1"));
                }
                advance!(meter, validated(statement, meter, 0));
                let goal = advance!(meter, proposition(statement, &WorkStack::new(), meter, 0));
                let lowered = match advance!(
                    meter,
                    proof_walk(
                        ProofFrame::Check(&obligation.term, ProofGoal::Borrowed(&goal), 0),
                        available,
                        meter,
                        obligation.term.span
                    )
                ) {
                    ProofValue::Checked(lowered) => lowered,
                    ProofValue::Inferred(..) => return Err(exhausted(obligation.term.span)),
                };
                let checked = CheckedProof {
                    name: obligation.name.clone(),
                    kind: ProofKind::Pure,
                    lean_term: lowered.clone(),
                    claim: lean_proposition(&goal, obligation.span, meter.tree_bounds)?,
                    model_revision: String::new(),
                    checker_revision: String::new(),
                };
                Ok((
                    checked,
                    Some(Available {
                        name: obligation.name.clone(),
                        goal,
                        lean_term: lowered,
                    }),
                ))
            }
            PendingGoal::Contract { contract } => {
                if obligation.revision != contract.revision {
                    return Err(reject(
                        obligation.span,
                        "proof and contract revisions differ",
                    ));
                }
                let checked = match contract.revision {
                    1 => advance!(meter, check_contract(obligation, contract, meter)),
                    2 => advance!(meter, named_v2::check_contract(obligation, contract, meter)),
                    _ => return Err(reject(obligation.span, "unknown contract revision")),
                };
                Ok((checked, None))
            }
        }
    })();
    (meter, result)
}
enum DependencyFrame<'a> {
    Enter {
        dependency: &'a ProofDependency,
        depth: u32,
        parent: Option<(&'a str, u32, &'a [u8], u32)>,
    },
    Children {
        dependency: &'a ProofDependency,
        depth: u32,
        next: usize,
        start: usize,
    },
}
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-contracts; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn check_dependency<'m>(
    dependency: &ProofDependency,
    mut meter: Meter<'m>,
) -> Transition<'m, Available> {
    let result = (|| -> Result<Available, Diagnostic> {
        let bound = meter.tree_bounds.frames;
        let available_bound = usize::try_from(meter.limits.nodes)
            .map_err(|_| exhausted(dependency.declaration.span))?;
        let mut frames = WorkStack::new();
        let mut active = WorkStack::new();
        let mut completed = None;
        // Contiguous siblings share one scratch buffer; a frame owns only its
        // starting offset. No per-dependency vector or eager fan-out reservation.
        let mut available = Vec::new();
        work_push!(
            frames,
            DependencyFrame::Enter {
                dependency,
                depth: 0,
                parent: None
            },
            bound
        )
        .map_err(|()| exhausted(dependency.declaration.span))?;
        while let Some(frame) = work_pop!(frames) {
            match frame {
                DependencyFrame::Enter {
                    dependency,
                    depth,
                    parent,
                } => {
                    advance!(meter, meter.visit(depth, dependency.declaration.span));
                    advance!(
                        meter,
                        meter.charge(dependency.source.len(), dependency.declaration.span)
                    );
                    let is_repeats = active.inline[..active.inline_len]
                        .iter()
                        .flatten()
                        .chain(active.spill.iter())
                        .any(|ancestor: &&ProofDependency| {
                            ancestor.module == dependency.module
                                && ancestor.version == dependency.version
                                && ancestor.declaration.name == dependency.declaration.name
                        });
                    if is_repeats {
                        return Err(reject(
                            dependency.declaration.span,
                            "cyclic proof dependency",
                        ));
                    }
                    let is_private_local = parent.is_some_and(|(module, version, source, span)| {
                        dependency.module == module
                            && dependency.version == version
                            && dependency.source == source
                            && dependency.declaration.span.start < span
                    });
                    if (!dependency.exported && !is_private_local)
                        || dependency.source.is_empty()
                        || dependency.declaration.name.is_empty()
                    {
                        return Err(reject(
                            dependency.declaration.span,
                            "unpublished or private proof dependency",
                        ));
                    }
                    work_push!(active, dependency, bound)
                        .map_err(|()| exhausted(dependency.declaration.span))?;
                    work_push!(
                        frames,
                        DependencyFrame::Children {
                            dependency,
                            depth,
                            next: 0,
                            start: available.len()
                        },
                        bound
                    )
                    .map_err(|()| exhausted(dependency.declaration.span))?;
                }
                DependencyFrame::Children {
                    dependency,
                    depth,
                    next,
                    start,
                } => {
                    if let Some(child) = completed.take() {
                        let sibling_count = available
                            .len()
                            .checked_sub(start)
                            .ok_or_else(|| exhausted(dependency.declaration.span))?;
                        if sibling_count >= dependency.dependencies.len() {
                            return Err(exhausted(dependency.declaration.span));
                        }
                        if available.len() >= available_bound {
                            return Err(exhausted(dependency.declaration.span));
                        }
                        available.push(child);
                    }
                    if let Some(prerequisite) = dependency.dependencies.get(next) {
                        let child_depth = deeper(depth, prerequisite.declaration.span)?;
                        let following = next
                            .checked_add(1)
                            .ok_or_else(|| exhausted(prerequisite.declaration.span))?;
                        work_push!(
                            frames,
                            DependencyFrame::Children {
                                dependency,
                                depth,
                                next: following,
                                start
                            },
                            bound
                        )
                        .map_err(|()| exhausted(prerequisite.declaration.span))?;
                        work_push!(
                            frames,
                            DependencyFrame::Enter {
                                dependency: prerequisite,
                                depth: child_depth,
                                parent: Some((
                                    &dependency.module,
                                    dependency.version,
                                    &dependency.source,
                                    dependency.declaration.span.start
                                )),
                            },
                            bound
                        )
                        .map_err(|()| exhausted(prerequisite.declaration.span))?;
                        continue;
                    }
                    let siblings = available
                        .get(start..)
                        .ok_or_else(|| exhausted(dependency.declaration.span))?;
                    let (_, closed) = advance!(
                        meter,
                        check_obligation(&dependency.declaration, siblings, meter)
                    );
                    available.truncate(start);
                    work_pop!(active);
                    let mut closed = closed.ok_or_else(|| {
                        reject(
                            dependency.declaration.span,
                            "contract theorem dependency is not in the pure fragment",
                        )
                    })?;
                    closed.name = dependency.name.clone();
                    completed = Some(closed);
                }
            }
        }
        completed.ok_or_else(|| exhausted(dependency.declaration.span))
    })();
    (meter, result)
}
#[expect(
    tigerstyle::compound_condition,
    reason = "Owner: noble-contracts; the guard is the admission rule verbatim; splitting it would obscure the predicate it states"
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn check_contract<'m>(
    obligation: &ProofObligation,
    contract: &ContractGoal,
    mut meter: Meter<'m>,
) -> Transition<'m, CheckedProof> {
    let result = (|| -> Result<CheckedProof, Diagnostic> {
        advance!(meter, meter.visit(0, obligation.span));
        let (_, literal) = prepare_checked_contract(contract, meter.limits)?;
        if contract.inputs.len() != 1
            || contract.outputs.len() != 1
            || contract.inputs[0].1 != "I64"
            || contract.outputs[0].1 != "I64"
            || atom(&contract.requires)? != "true"
        {
            return Err(reject(
                obligation.span,
                "selected unary I64 rule requires typed True precondition",
            ));
        }
        let literal = literal.ok_or_else(|| {
            reject(
                obligation.span,
                "selected proof rules require exact literal then wrapping-add subject slices",
            )
        })?;
        if !matches_eq_add(
            &contract.ensures,
            &contract.inputs[0].0,
            &contract.outputs[0].0,
            literal,
        ) {
            return Err(reject(
                contract.ensures.span,
                "proof consequence does not match exact typed MC1 postcondition",
            ));
        }
        let root = operation(&obligation.term, "export-unary-I64", 2)?;
        let outer = operation(&root[0], "intro", 2)?;
        let tail_binder = list(&outer[0])?;
        if tail_binder.len() != 2 || atom(&tail_binder[1])? != "Stack" {
            return Err(reject(outer[0].span, "expected explicit Stack tail binder"));
        }
        let tail_name = atom(&tail_binder[0])?;
        let inner = operation(&outer[1], "intro", 2)?;
        let x = list(&inner[0])?;
        if x.len() != 2 || atom(&x[1])? != "I64" || atom(&x[0])? == tail_name {
            return Err(reject(
                inner[0].span,
                "expected distinct explicit I64 input binder",
            ));
        }
        let sequence = operation(&inner[1], "pc-sequence", 4)?;
        let exact1 = operation(&sequence[0], "pc-exact", 1)?;
        let lit = operation(&exact1[0], "exec-literal", 1)?;
        if atom(&lit[0])?.parse::<i64>().ok() != Some(literal) {
            return Err(reject(
                lit[0].span,
                "literal rule does not match first subject slice",
            ));
        }
        let exact2 = operation(&sequence[1], "pc-exact", 1)?;
        operation(&exact2[0], "exec-add", 0)?;
        let bridge = operation(&sequence[2], "bridge", 1)?;
        operation(&bridge[0], "by-exact-append-assoc", 0)?;
        let join = operation(&sequence[3], "join", 1)?;
        operation(&join[0], "by-exact-result", 0)?;
        operation(&root[1], "mc1-true-eq-wrap", 0)?;
        let lean_term = lower_unary_add(literal);
        advance!(meter, meter.charge(lean_term.len(), obligation.span));
        Ok(CheckedProof {
            name: obligation.name.clone(),
            kind: ProofKind::Contract,
            lean_term,
            claim: "MC1Obligation.claim".into(),
            model_revision: String::new(),
            checker_revision: String::new(),
        })
    })();
    (meter, result)
}
enum Mc1Frame<'a> {
    Enter(&'a Form, u32),
    Children(&'a [Form], usize, u32),
}
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-contracts; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn mc1_expression<'m>(form: &Form, mut meter: Meter<'m>, depth: u32) -> Transition<'m, String> {
    let result = (|| -> Result<String, Diagnostic> {
        let mut pending = WorkStack::new();
        let bound = meter.tree_bounds.frames;
        work_push!(pending, Mc1Frame::Enter(form, depth), bound)
            .map_err(|()| exhausted(form.span))?;
        let mut text = String::new();
        while let Some(frame) = work_pop!(pending) {
            match frame {
                Mc1Frame::Enter(node, depth) => {
                    advance!(meter, meter.visit(depth, node.span));
                    match &node.kind {
                        FormKind::Atom(word) => {
                            if word.is_empty()
                                || !word
                                    .bytes()
                                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                            {
                                return Err(reject(node.span, "invalid MC1 logical word"));
                            }
                            text.push_str(word);
                        }
                        FormKind::List(items) => {
                            text.push('(');
                            if items.is_empty() {
                                text.push(')');
                            } else {
                                work_push!(pending, Mc1Frame::Children(items, 0, depth), bound)
                                    .map_err(|()| exhausted(node.span))?;
                            }
                        }
                    }
                }
                Mc1Frame::Children(items, index, depth) => {
                    let Some(part) = items.get(index) else {
                        text.push(')');
                        continue;
                    };
                    if index != 0 {
                        text.push(' ');
                    }
                    let child_depth = deeper(depth, part.span)?;
                    let next = index.checked_add(1).ok_or_else(|| exhausted(part.span))?;
                    work_push!(pending, Mc1Frame::Children(items, next, depth), bound)
                        .map_err(|()| exhausted(part.span))?;
                    work_push!(pending, Mc1Frame::Enter(part, child_depth), bound)
                        .map_err(|()| exhausted(part.span))?;
                }
            }
        }
        Ok(text)
    })();
    (meter, result)
}
/// Cross-checks the asserted source binding, reaccepted complete definition
/// recipe and MC1-v1 frontend. Host authority over these public fields must
/// come from the retained `ModuleSession`, not this consistency check.
pub fn prepare_contract(
    contract: &ContractGoal,
    limits: Limits,
) -> Result<crate::Prepared, Diagnostic> {
    prepare_checked_contract(contract, limits).map(|(prepared, _)| prepared)
}
#[expect(
    tigerstyle::compound_condition,
    reason = "Owner: noble-contracts; the guard is the admission rule verbatim; splitting it would obscure the predicate it states"
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn prepare_checked_contract(
    contract: &ContractGoal,
    limits: Limits,
) -> Result<(crate::Prepared, Option<i64>), Diagnostic> {
    let subject = &contract.subject;
    let config = MeterConfig {
        budgets: ProofBudgets::from_limits(limits),
        limits,
        tree_bounds: TreeBounds::new(limits).ok_or_else(|| exhausted(contract.contract_span))?,
    };
    let mut meter = Meter {
        nodes: 0,
        work: 0,
        normalization: 0,
        substitution: 0,
        config: &config,
    };
    advance!(
        meter,
        meter.charge(
            subject
                .module_source
                .len()
                .saturating_add(subject.definition_source.len()),
            contract.contract_span
        )
    );
    if !matches!(subject.source_dependencies.as_slice(), [origin] if
        origin.module == subject.module && origin.version == subject.version &&
        origin.full_source == subject.module_source)
    {
        return Err(reject(
            subject.source_span,
            "subject source dependency differs from original module",
        ));
    }
    if subject.effects != noble_kernel::types::EffSet::empty() {
        return Err(reject(
            subject.source_span,
            "MC1 contract subject has resource effects",
        ));
    }
    let accepted = &subject.accepted_submission;
    let noble_kernel::untrusted::Outcome::Accepted(interface) = noble_kernel::acceptance::check(
        &accepted.environment,
        &accepted.request,
        &accepted.body.candidate,
    ) else {
        return Err(reject(
            subject.source_span,
            "subject candidate is not independently accepted",
        ));
    };
    if interface.interface.stack_in != subject.input_types
        || interface.interface.stack_out != subject.output_types
        || interface.interface.effects != subject.effects
    {
        return Err(reject(
            subject.source_span,
            "accepted subject interface differs from contract",
        ));
    }
    let mut matching = accepted
        .definitions
        .iter()
        .filter(|item| item.identity == subject.definition_identity);
    let Some(definition) = matching.next() else {
        return Err(reject(
            subject.source_span,
            "resolved subject identity absent from accepted recipe",
        ));
    };
    if matching.next().is_some() || accepted.definitions.len() != 1 {
        return Err(reject(
            subject.source_span,
            "ambiguous resolved subject identity",
        ));
    }
    let slot = usize::try_from(definition.definition.0).map_err(|_| {
        reject(
            subject.source_span,
            "subject definition slot exceeds address space",
        )
    })?;
    if accepted.environment.definition_owners.get(slot) != Some(&Some(subject.definition_owner))
        || accepted.environment.kind(definition.definition)
            != Some(noble_kernel::contracts::Behavior::Named)
        || !matches!(accepted.body.candidate.body.as_slice(), [node] if
            matches!(usize::try_from(node.0).ok().and_then(|index| accepted.body.candidate.nodes.get(index)),
                Some(noble_kernel::untrusted::Node::Invocation {def,..}) if *def == definition.definition))
        || accepted.body.candidate.nodes.len() != 1
        || !accepted.body.texts.is_empty()
    {
        return Err(reject(
            subject.source_span,
            "resolved subject owner or invocation differs from accepted recipe",
        ));
    }
    if definition.expected.stack_in != subject.input_types
        || definition.expected.stack_out != subject.output_types
        || definition.expected.allowed_effects != subject.effects
    {
        return Err(reject(
            subject.source_span,
            "subject definition has different stack or effect bound",
        ));
    }
    let body_span = crate::source::verify_definition_origin(
        crate::source::DefinitionOrigin {
            module_source: &subject.module_source,
            module: &subject.module,
            version: subject.version,
            definition: &subject.definition,
            ordinal: subject.definition_ordinal,
            span: subject.source_span,
        },
        &subject.definition_source,
        limits,
    )?;
    let start = usize::try_from(body_span.start)
        .map_err(|_| reject(body_span, "subject body exceeds address space"))?;
    let end = usize::try_from(body_span.end)
        .map_err(|_| reject(body_span, "subject body exceeds address space"))?;
    let body = subject
        .module_source
        .get(start..end)
        .ok_or_else(|| reject(body_span, "subject body is outside original module source"))?;
    let body = core::str::from_utf8(body)
        .map_err(|_| reject(subject.source_span, "subject body must be UTF-8 for MC1"))?;
    let mut text = format!("(contract 1 {} (input", contract.contract_name);
    for (name, ty) in &contract.inputs {
        text.push_str(&format!(" ({name} {ty})"));
    }
    text.push_str(") (output");
    for (name, ty) in &contract.outputs {
        text.push_str(&format!(" ({name} {ty})"));
    }
    let requires = advance!(meter, mc1_expression(&contract.requires, meter, 0));
    let ensures = advance!(meter, mc1_expression(&contract.ensures, meter, 0));
    text.push_str(&format!(
        ") (program {body}) (requires {requires}) (ensures {ensures}))"
    ));
    let (_, charged) = meter.charge(text.len(), contract.contract_span);
    charged?;
    let result = crate::prepare(text.as_bytes(), limits)?;
    if result.inputs().len() != contract.inputs.len()
        || result.outputs().len() != contract.outputs.len()
        || result.checked().interface.stack_in != subject.input_types
        || result.checked().interface.stack_out != subject.output_types
        || result.checked().interface.effects != subject.effects
    {
        return Err(reject(
            subject.source_span,
            "MC1 typed contract differs from accepted subject interface",
        ));
    }
    // The standalone MC1 program must be the accepted definition's complete
    // operation list. Any unresolved named call or different recipe is refused.
    let expected = &definition.body.candidate;
    let actual = result.candidate();
    if !definition.body.texts.is_empty()
        || expected.format != actual.format
        || expected.revision != actual.revision
        || expected.body.len() != actual.body.len()
        || expected.nodes.len() != actual.nodes.len()
        || expected.nodes.len() != expected.body.len()
        || u32::try_from(expected.body.len()).map_or(true, |count| count > limits.nodes)
    {
        return Err(reject(
            subject.source_span,
            "MC1 program differs from resolved subject recipe",
        ));
    }
    for (a, b) in expected.body.iter().zip(&actual.body) {
        let Some(left) = usize::try_from(a.0)
            .ok()
            .and_then(|index| expected.nodes.get(index))
        else {
            return Err(reject(subject.source_span, "invalid accepted subject node"));
        };
        let Some(right) = usize::try_from(b.0)
            .ok()
            .and_then(|index| actual.nodes.get(index))
        else {
            return Err(reject(subject.source_span, "invalid MC1 subject node"));
        };
        match (left, right) {
            (
                noble_kernel::untrusted::Node::Literal { lit: a, inst: ia },
                noble_kernel::untrusted::Node::Literal { lit: b, inst: ib },
            ) if a == b && same_inst(ia, ib) => (),
            (
                noble_kernel::untrusted::Node::Invocation { def: a, inst: ia },
                noble_kernel::untrusted::Node::Invocation { def: b, inst: ib },
            ) if a == b && same_inst(ia, ib) => (),
            _ => {
                return Err(reject(
                    subject.source_span,
                    "MC1 program differs from resolved subject operation",
                ))
            }
        }
    }
    let request = noble_kernel::untrusted::Request {
        input_bytes: accepted.request.input_bytes,
        expected: definition.expected.clone(),
        limits: accepted.request.limits,
    };
    let noble_kernel::untrusted::Outcome::Accepted(checked_definition) =
        noble_kernel::acceptance::check(&accepted.environment, &request, expected)
    else {
        return Err(reject(
            subject.source_span,
            "subject definition body is not independently accepted",
        ));
    };
    if checked_definition.interface.stack_in != subject.input_types
        || checked_definition.interface.stack_out != subject.output_types
        || checked_definition.interface.effects != subject.effects
    {
        return Err(reject(
            subject.source_span,
            "accepted definition body differs from contract interface",
        ));
    }
    if checked_definition.derivations.len() != result.checked().derivations.len()
        || !checked_definition
            .derivations
            .iter()
            .zip(&result.checked().derivations)
            .all(|(left, right)| {
                left.node == right.node
                    && left.interface.stack_in == right.interface.stack_in
                    && left.interface.stack_out == right.interface.stack_out
                    && left.interface.effects == right.interface.effects
            })
    {
        return Err(reject(
            subject.source_span,
            "MC1 node derivations differ from accepted definition body",
        ));
    }
    let literal = match expected.nodes.as_slice() {
        [noble_kernel::untrusted::Node::Literal {
            lit: noble_kernel::untrusted::Lit::I64(value),
            ..
        }, noble_kernel::untrusted::Node::Invocation {
            def: noble_kernel::contracts::Definition(4),
            ..
        }] if expected.body.as_slice()
            == [
                noble_kernel::untrusted::NodeId(0),
                noble_kernel::untrusted::NodeId(1),
            ]
            && subject.input_types == [noble_kernel::types::Ty::I64]
            && subject.output_types == [noble_kernel::types::Ty::I64] =>
        {
            Some(*value)
        }
        _ => None,
    };
    if literal.is_some()
        && (!canonical_wrapping_add(&accepted.environment)
            || accepted.environment.deps.get(slot).map(Vec::as_slice)
                != Some(&[noble_kernel::contracts::Definition(4)][..]))
    {
        return Err(reject(
            subject.source_span,
            "subject addition is not the fixed MC1 builtin",
        ));
    }
    Ok((result, literal))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; this stage imports the domain types it operates on directly, matching the module vocabulary"
)]
pub(crate) fn canonical_wrapping_add(env: &noble_kernel::contracts::Env) -> bool {
    use noble_kernel::{
        contracts::{Behavior, Definition},
        shapes::Pattern,
        words::{Variable, VariableKind},
    };
    let Some(scheme) = env.scheme(Definition(4)) else {
        return false;
    };
    env.kind(Definition(4)) == Some(Behavior::Arith)
        && env.definition_owners.get(4) == Some(&None)
        && env.deps.get(4).is_some_and(Vec::is_empty)
        && scheme.var_kinds == [VariableKind::Stack]
        && matches!(
            scheme.stack_in.as_slice(),
            [Pattern::StackVar(Variable(0)), Pattern::I64, Pattern::I64]
        )
        && matches!(
            scheme.stack_out.as_slice(),
            [Pattern::StackVar(Variable(0)), Pattern::I64]
        )
        && scheme.effects.is_empty()
}

#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
)]
pub(crate) fn same_inst(a: &noble_kernel::words::Inst, b: &noble_kernel::words::Inst) -> bool {
    use noble_kernel::words::Binding;
    a.bindings.len() == b.bindings.len()
        && a.bindings
            .iter()
            .zip(&b.bindings)
            .all(|(a, b)| match (a, b) {
                (Binding::Stack(a), Binding::Stack(b)) => a == b,
                (Binding::Value(a), Binding::Value(b)) => a == b,
                (Binding::Effect(a), Binding::Effect(b)) => a == b,
                (Binding::Ref(a), Binding::Ref(b)) => a == b,
                _ => false,
            })
}
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-contracts; the same-typed parameters are positional by contract and named at every call site"
)]
fn matches_eq_add(form: &Form, input_name: &str, output_name: &str, literal: i64) -> bool {
    let Ok(eq) = operation(form, "eq", 2) else {
        return false;
    };
    let Ok(out) = operation(&eq[0], "out", 1) else {
        return false;
    };
    let Ok(add) = operation(&eq[1], "add", 2) else {
        return false;
    };
    let Ok(input) = operation(&add[0], "in", 1) else {
        return false;
    };
    atom(&out[0]).ok() == Some(output_name)
        && atom(&input[0]).ok() == Some(input_name)
        && atom(&add[1]).ok().and_then(|word| word.parse::<i64>().ok()) == Some(literal)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
/// Expansion of the checked constructors in `check_contract`. Every instance
/// of `__N__` is sourced from the independently accepted literal node. These
/// are actual PC introduction/elimination terms, not a selected theorem name.
fn lower_unary_add(literal: i64) -> String {
    const TERM: &str = r#"
(exportedClaim_consequence
  (exportedClaim_unaryI64
    (code := [.lit (.i64 __N__)] ++ [.word 4])
    (f := fun x => x + (__N__ : BitVec 64))
    (fun tail x final execution =>
      let first : PC [.lit (.i64 __N__)]
          (fun before => before = tail ++ [.i64 x])
          (fun _ middle => middle = (tail ++ [.i64 x]) ++ [.i64 __N__]) :=
        pc_exact (code := [.lit (.i64 __N__)]) (s := tail ++ [.i64 x])
          (expected := (tail ++ [.i64 x]) ++ [.i64 __N__])
          (fun actual => exec_lit_iff (v := .i64 __N__)
            (s := tail ++ [.i64 x]) (t := actual))
      let second : PC [.word 4]
          (fun middle => middle = tail ++ [.i64 x, .i64 __N__])
          (fun _ actual => actual = tail ++ [.i64 (x + (__N__ : BitVec 64))]) :=
        pc_exact (code := [.word 4]) (s := tail ++ [.i64 x, .i64 __N__])
          (expected := tail ++ [.i64 (x + (__N__ : BitVec 64))])
          (fun actual => exec_add_iff (a := x) (b := __N__)
            (s := tail) (t := actual))
      let composed : PC ([.lit (.i64 __N__)] ++ [.word 4])
          (fun before => before = tail ++ [.i64 x])
          (fun before actual => before = tail ++ [.i64 x] ∧
            actual = tail ++ [.i64 (x + (__N__ : BitVec 64))]) :=
        @pc_sequence
          [.lit (.i64 __N__)] [.word 4]
          (fun (before : Stack) => before = tail ++ [.i64 x])
          (fun (_ middle : Stack) => middle = (tail ++ [.i64 x]) ++ [.i64 __N__])
          (fun (middle : Stack) => middle = tail ++ [.i64 x, .i64 __N__])
          (fun (_ actual : Stack) => actual = tail ++ [.i64 (x + (__N__ : BitVec 64))])
          (fun (before actual : Stack) => before = tail ++ [.i64 x] ∧
            actual = tail ++ [.i64 (x + (__N__ : BitVec 64))])
          first second
          (fun _ middle _ exactMiddle =>
            exactMiddle.trans (List.append_assoc tail [.i64 x] [.i64 __N__]))
          (fun _ _ _ initial _ exactFinal => ⟨initial, exactFinal⟩)
      ((composed (tail ++ [.i64 x]) final rfl execution).2)))
  (fun _ _ _ => True.intro)
  (fun before after params _ exactPost =>
    match exactPost with
    | ⟨x, hbefore, hafter⟩ =>
      hbefore.symm ▸ hafter.symm ▸
      (holds_eq (.output 0) (.add (.input 0) (.i64 __N__))
        ⟨[.i64 x], [.i64 (x + (__N__ : BitVec 64))], params⟩
        (.i64 (x + (__N__ : BitVec 64))) (.i64 (x + (__N__ : BitVec 64)))
        rfl rfl).mpr rfl))
"#;
    TERM.replace("__N__", &literal.to_string())
}
