#![no_std]
// The contracts modules are cohesive units; splitting them for a line budget
// would change module paths and the extracted subject.
#![expect(
    tigerstyle::excessive_file_length,
    reason = "noble-contracts modules are cohesive units; splitting them for a line budget would churn the extracted subject"
)]
#![expect(
    tigerstyle::module_file_count,
    reason = "noble-contracts keeps one module per pipeline stage; merging directories for a file-count budget would churn the extracted subject"
)]
#![expect(
    tigerstyle::underscore_in_module_filename,
    reason = "the named_v2 module name carries the profile revision it implements"
)]
#![feature(register_tool)]
#![register_tool(charon, octet, tigerstyle)]

extern crate alloc;

macro_rules! attempt {
    ($step:expr) => {
        match $step {
            Ok(value) => value,
            Err(failure) => return Err(failure),
        }
    };
}

pub mod companion;
pub mod component;
mod frontend;
mod inference;
pub mod intrinsic;
mod metering;
mod predicate;
mod program;
mod rendering;
pub mod source;
mod syntax;
pub mod wire;
pub use companion::{
    Admission, CaptureBinding, ClaimTemplate, ContractId, Core, Derivation, Derived, EvidenceClass,
    EvidenceId, EvidenceOffer, GuardTemplate, Outcome, Refusal, Release, RuleId, Subject,
    SubjectDigest, RULESET_V1,
};
pub use rendering::export_lean;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub bytes: u32,
    pub nodes: u32,
    pub depth: u32,
    pub work: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            bytes: 65_536,
            nodes: 16_384,
            depth: 64,
            work: 2_000_000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    Invalid,
    Unsupported,
    Exhausted,
    Internal,
}

impl DiagnosticKind {
    /// The stable machine-facing category of a diagnostic. Clients key on this
    /// rather than on the English message, and it never conflates a budget
    /// refusal with an invalid input.
    pub const fn category(self) -> &'static str {
        match self {
            Self::Invalid => "invalid",
            Self::Unsupported => "unsupported",
            Self::Exhausted => "exhausted",
            Self::Internal => "internal",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub span: Span,
    pub message: alloc::string::String,
    join: Option<alloc::boxed::Box<JoinDiagnostic>>,
    ordinary_typing: Option<alloc::boxed::Box<noble_kernel::untrusted::Checked>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinDiagnostic {
    pub word: alloc::string::String,
    pub expected_stack: alloc::string::String,
    pub actual_stack: alloc::string::String,
    pub constraint: &'static str,
    pub value_origin: Option<Span>,
    /// Where the two stacks first differ, when the mismatch is positional.
    pub difference: Option<StackDifference>,
}

/// Where the two sides of a rejected stack join first differ, counted from the top.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackDifference {
    /// The number of positions, from the top, on which both sides render the same type.
    pub shared_prefix: u32,
    /// The expected side's type at the first differing position. A side with no element there
    /// contributes its remaining stack instead (`[]`, `?stack` or `forall-stack`).
    pub expected: alloc::string::String,
    /// The actual side's type at that position, or its remaining stack.
    pub actual: alloc::string::String,
    /// The word whose input interface is the expected side, when it is known.
    pub expected_word: Option<Span>,
    /// The word that produced the actual stack, when it is known. At the start of a body the
    /// actual stack is the body's input and no word produced it.
    pub actual_word: Option<Span>,
}

impl StackDifference {
    /// The first differing position, counted from the top, which is position 1.
    #[must_use]
    pub const fn position(&self) -> u32 {
        self.shared_prefix.saturating_add(1)
    }
}

impl Diagnostic {
    pub fn join(&self) -> Option<&JoinDiagnostic> {
        self.join.as_deref()
    }

    pub fn ordinary_typing(&self) -> Option<&noble_kernel::untrusted::Checked> {
        self.ordinary_typing.as_deref()
    }

    /// Construct a diagnostic for a reporting adapter or a controlled test.
    /// Producers inside this crate keep the dedicated constructors, so a
    /// produced diagnostic always carries the span of the node it came from.
    pub fn new(kind: DiagnosticKind, span: Span, message: &str) -> Self {
        Self {
            kind,
            span,
            message: alloc::string::String::from(message),
            join: None,
            ordinary_typing: None,
        }
    }

    /// Attach join provenance to a diagnostic that a reporting adapter carries
    /// across a boundary. Producers keep using the internal constructors; this
    /// exists so transports can re-emit a captured diagnostic without
    /// reconstructing it from prose.
    pub fn join_with(mut self, join: JoinDiagnostic) -> Self {
        self.join = Some(alloc::boxed::Box::new(join));
        self
    }

    pub(crate) fn with_typing(mut self, checked: noble_kernel::untrusted::Checked) -> Self {
        self.ordinary_typing = Some(alloc::boxed::Box::new(checked));
        self
    }
}

#[derive(Clone, Debug)]
pub struct NamedType {
    pub name: alloc::string::String,
    pub ty: noble_kernel::types::Ty,
}

#[derive(Clone, Debug)]
pub struct LogicDef {
    pub name: alloc::string::String,
    pub ty: noble_kernel::types::Ty,
    pub body: u32,
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: noble_kernel::types::Ty,
    pub span: Span,
    total: bool,
    uses_output: bool,
}

impl Expr {
    /// False means evaluation can be undefined (not a false Boolean value).
    pub fn is_total(&self) -> bool {
        self.total
    }

    pub fn uses_output(&self) -> bool {
        self.uses_output
    }
}

#[derive(Clone, Debug)]
#[charon::variants_suffix("Expr")]
#[octet::sealed_enum]
pub enum ExprKind {
    I64(i64),
    Bool(bool),
    Unit,
    Input(u32),
    Output(u32),
    Param(u32),
    Definition(u32),
    Not(u32),
    And(u32, u32),
    Or(u32, u32),
    Implies(u32, u32),
    Eq(u32, u32),
    Lt(u32, u32),
    Le(u32, u32),
    Add(u32, u32),
    Sub(u32, u32),
    Mul(u32, u32),
    Pair(u32, u32),
    First(u32),
    Second(u32),
    Inl(u32),
    Inr(u32),
    IsLeft(u32),
    Left(u32),
    Right(u32),
    Nil,
    Cons(u32, u32),
    IsNil(u32),
    Head(u32),
    Tail(u32),
    Length(u32),
    Maps(u32, u32, u32),
}

/// Only preparation can construct a value; every getter is immutable.
#[derive(Clone, Debug)]
pub struct Prepared {
    name: alloc::string::String,
    inputs: alloc::vec::Vec<NamedType>,
    outputs: alloc::vec::Vec<NamedType>,
    params: alloc::vec::Vec<NamedType>,
    definitions: alloc::vec::Vec<LogicDef>,
    expressions: alloc::vec::Vec<Expr>,
    requires: u32,
    ensures: u32,
    candidate: noble_kernel::untrusted::Candidate,
    request: noble_kernel::untrusted::Request,
    checked: noble_kernel::untrusted::Checked,
}

impl Prepared {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn inputs(&self) -> &[NamedType] {
        &self.inputs
    }
    pub fn outputs(&self) -> &[NamedType] {
        &self.outputs
    }
    pub fn params(&self) -> &[NamedType] {
        &self.params
    }
    pub fn definitions(&self) -> &[LogicDef] {
        &self.definitions
    }
    pub fn expressions(&self) -> &[Expr] {
        &self.expressions
    }
    pub fn requires(&self) -> u32 {
        self.requires
    }
    pub fn ensures(&self) -> u32 {
        self.ensures
    }
    pub fn candidate(&self) -> &noble_kernel::untrusted::Candidate {
        &self.candidate
    }
    pub fn request(&self) -> &noble_kernel::untrusted::Request {
        &self.request
    }
    pub fn checked(&self) -> &noble_kernel::untrusted::Checked {
        &self.checked
    }
}

pub fn prepare(source: &[u8], limits: Limits) -> Result<Prepared, Diagnostic> {
    frontend::prepare(source, limits)
}

#[expect(
    tigerstyle::interior_mutability_in_core,
    reason = "Owner: noble-contracts; K-CHECK-07 request budget: one request-local Rc<Cell<u32>> counter a host opts into with ModuleSession::share_request_work; it is single-threaded (Rc is !Send), never outlives the session and its staged snapshots, and every meter debits it so nested checking cannot restart the request's budget. Explicit threading would turn the &self closure-walk and origin-replay APIs into &mut ones."
)]
pub(crate) struct Meter {
    pub limits: Limits,
    initial_work: u32,
    work: u32,
    nodes: u32,
    /// Remaining work of one shared request. When present, every charge also
    /// draws from it, so meters cut from one request can never restart its
    /// budget; `limits.work` then only caps this meter's share.
    pool: Option<alloc::rc::Rc<core::cell::Cell<u32>>>,
}

pub(crate) fn invalid(span: Span, message: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticKind::Invalid, span, message)
}

pub(crate) fn internal(span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagnosticKind::Internal,
        span,
        "inconsistent frontend arena",
    )
}

pub(crate) fn index(value: usize, span: Span) -> Result<u32, Diagnostic> {
    match u32::try_from(value) {
        Ok(value) => Ok(value),
        Err(_) => Err(Diagnostic::new(
            DiagnosticKind::Exhausted,
            span,
            "index exceeds the finite format",
        )),
    }
}

pub(crate) fn offset(value: u32, span: Span) -> Result<usize, Diagnostic> {
    match usize::try_from(value) {
        Ok(value) => Ok(value),
        Err(_) => Err(Diagnostic::new(
            DiagnosticKind::Exhausted,
            span,
            "index exceeds the host address space",
        )),
    }
}
