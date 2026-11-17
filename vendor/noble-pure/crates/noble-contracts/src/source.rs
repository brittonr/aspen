//! Bounded Core-Bootstrap preparation. This module never executes guest code.
//!
//! Named bodies retain resolved dependency identities. Their type constraints
//! are instantiated afresh at each named use, while a first-class Program has
//! one shared inference term. Every executable specialization is subsequently
//! checked by the kernel against its exact concrete environment contract.
pub mod chunks;
mod declared;
pub mod editor;
mod emission;
mod inference;
mod lexer;
mod locals;
mod parsing;
mod preflight;
mod preparation;
pub mod procedures;
pub mod proof;
mod replacement;
mod resolution;
mod sizechange;

#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; this re-export is the module public surface, not a convenience import"
)]
pub(crate) use declared::{
    verify_definition_origin, verify_named_call_origin, verify_named_export, DefinitionOrigin,
};
pub use declared::{
    BoundOperation, CheckedNominal, CheckedProgramInterface, CheckedProgramOwner,
    DeclaredTextSourceOwner, ModuleKind, ModulePrepared, ModuleSession, ProgramClosureError,
    RebuiltImportDefinition, SignedSourceOwner,
};
pub use sizechange::{
    certificate, Argument, Bound, ClosureEntry, Evidence, Graph, GraphEdge, Measure, MeasureIndex,
    Relation, Site, StrictWitness, Unsupported, Verdict, Witness,
};

/// Core vocabulary resolved by the same checker as a worker's pure source.
/// Test-host, live and selected cursor operations have different authority.
pub const fn pure_builtin_spelling(identity: u32) -> Option<&'static str> {
    if identity < 22 {
        crate::program::bootstrap_spelling(identity)
    } else {
        None
    }
}

/// Immutable source authority for the versioned Result interface. The same
/// declaration bytes are checked through ordinary module registration.
pub const RESULT_LIBRARY_SOURCE: &[u8] = include_bytes!("fixtures/result.noble");

/// Host-selected aggregate of original module bytes and outside definition history.
/// This does not raise the per-submission `Limits::bytes` parser bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-contracts; the name matches the established vocabulary used by callers; renaming it would churn call sites without clarifying them."
)]
pub struct RetainedSourceLimit {
    bytes: u32,
}

impl RetainedSourceLimit {
    pub const MAX_BYTES: u32 = 262_144;

    pub const fn new(bytes: u32) -> Option<Self> {
        if bytes > Self::MAX_BYTES {
            None
        } else {
            Some(Self { bytes })
        }
    }

    pub const fn bytes(self) -> u32 {
        self.bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Parse,
    Resolve,
    Link,
    Check,
    Acceptance,
}

#[derive(Clone, Debug)]
pub struct Error {
    stage: Stage,
    diagnostic: crate::Diagnostic,
    work_used: u32,
}

impl Error {
    pub const fn stage(&self) -> Stage {
        self.stage
    }
    pub const fn diagnostic(&self) -> &crate::Diagnostic {
        &self.diagnostic
    }
    pub(crate) const fn work_used(&self) -> u32 {
        self.work_used
    }
    /// Wrap a captured diagnostic with its original source stage so a transport
    /// re-emits it rather than flattening it to prose.
    pub const fn reported(stage: Stage, diagnostic: crate::Diagnostic) -> Self {
        Self {
            stage,
            diagnostic,
            work_used: 0,
        }
    }

    const fn at(stage: Stage, diagnostic: crate::Diagnostic) -> Self {
        Self {
            stage,
            diagnostic,
            work_used: 0,
        }
    }
}

/// An independently supplied, ordered Core stack interface, not a checked
/// candidate or permission to execute the indicated values.
#[derive(Clone, Debug, PartialEq)]
pub struct CoreInterface {
    pub stack_in: alloc::vec::Vec<noble_kernel::types::Ty>,
    pub stack_out: alloc::vec::Vec<noble_kernel::types::Ty>,
}

/// A bounded, ordered descriptor of one prepared submission's expectation: the
/// required and result arities and the allowed effect count. It is a comparison
/// descriptor, not interface equality and never behavioral equivalence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefinitionInterface {
    pub stack_in: u32,
    pub stack_out: u32,
    pub effects: u32,
}

impl DefinitionInterface {
    /// Both arities and the allowed effect count are reported as absent rather
    /// than saturated when they cannot be represented, so no caller can mistake a
    /// bound for a measured value.
    fn of(expected: &noble_kernel::untrusted::Expected) -> Option<Self> {
        Some(Self {
            stack_in: u32::try_from(expected.stack_in.len()).ok()?,
            stack_out: u32::try_from(expected.stack_out.len()).ok()?,
            effects: u32::try_from(expected.allowed_effects.as_slice().len()).ok()?,
        })
    }
}

/// Whether one inferred stack side ends in a concrete stack or still admits an
/// unconstrained tail variable. An open side is reported as open rather than
/// padded with invented entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[octet::sealed_enum]
pub enum InterfaceTail {
    Closed,
    Unconstrained,
}

impl InterfaceTail {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Unconstrained => "unconstrained",
        }
    }
}

/// The ordered, statically inferred interface of one definition: both stack
/// sides in source order (bottom first), whether each still ends in an
/// unconstrained stack variable, and the finite host-effect identities its
/// latent bound admits. `unresolved_effects` says the effect set is not finite
/// and therefore not fully reported.
///
/// This is an inference result over one checked definition. It is not a runtime
/// observation, not evidence of behavioural equivalence and never acceptance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderedInterface {
    pub stack_in: alloc::vec::Vec<alloc::string::String>,
    pub stack_in_tail: InterfaceTail,
    pub stack_out: alloc::vec::Vec<alloc::string::String>,
    pub stack_out_tail: InterfaceTail,
    pub effects: alloc::vec::Vec<u32>,
    pub unresolved_effects: bool,
}

/// Why a live Core replacement preview was refused, as a typed class.
///
/// The class names the contract that refused, so a caller never infers a cause
/// from a checking stage. The diagnostics of each class stay distinct and are
/// carried unchanged by [`PreviewError`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[octet::sealed_enum]
pub enum PreviewRefusal {
    /// The session profile or the candidate is not one replaceable checked
    /// definition: a bindings or declared-modules session, another namespace
    /// snapshot, or a candidate that is not a definition.
    UnsupportedProfile,
    /// The replacement's ordered stack interface differs.
    InterfaceChanged,
    /// The replacement admits an effect the installed definition does not.
    EffectGrowth,
    /// The replacement transitively depends on its own previous identity.
    SelfRebind,
    /// A retained dependent no longer rebuilds to its own name, effects or
    /// resolved call targets.
    DependentRebuild,
    /// The dependency walk, comparison or dependent replay exhausted its work or
    /// source budget.
    WorkExhausted,
    /// A retained dependent's own retained source is missing or malformed.
    RetainedSource,
    /// The comparison itself returned a diagnostic this class cannot name; the
    /// original diagnostic is preserved.
    ComparisonFailed,
}

impl PreviewRefusal {
    /// Stable machine-facing spelling of this class.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedProfile => "unsupported-profile",
            Self::InterfaceChanged => "interface-change",
            Self::EffectGrowth => "effect-growth",
            Self::SelfRebind => "self-rebind",
            Self::DependentRebuild => "dependent-rebuild",
            Self::WorkExhausted => "work-exhausted",
            Self::RetainedSource => "retained-source",
            Self::ComparisonFailed => "comparison-failed",
        }
    }
}

/// A refused live Core preview: the typed class that refused and the unchanged
/// diagnostic that was decided on.
#[derive(Clone, Debug)]
pub struct PreviewError {
    kind: PreviewRefusal,
    error: Error,
}

impl PreviewError {
    const fn new(kind: PreviewRefusal, error: Error) -> Self {
        Self { kind, error }
    }
    pub const fn kind(&self) -> PreviewRefusal {
        self.kind
    }
    pub const fn error(&self) -> &Error {
        &self.error
    }
    pub const fn diagnostic(&self) -> &crate::Diagnostic {
        self.error.diagnostic()
    }
    /// The stage the interrupted check stopped in. It is provenance beside the
    /// class, never a substitute for it.
    pub const fn stage(&self) -> Stage {
        self.error.stage()
    }
}

/// Parse host-selected Core interface words with the source type parser.
/// The C source-only caller separately checks this against kernel acceptance
/// and still refuses value forms its emitter cannot lower.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; invalid input is rejected through typed outcomes rather than assertions, and the invariants are enforced by callers and the crate test suite."
)]
#[expect(
    tigerstyle::sentinel_fallback,
    reason = "Owner: noble-contracts; the saturated span end is a diagnostic location only; inputs that long are refused by the 4096-byte check that follows."
)]
pub fn parse_core_interface(text: &str) -> Result<CoreInterface, Error> {
    let span = crate::Span {
        start: 0,
        end: u32::try_from(text.len()).unwrap_or(u32::MAX),
    };
    if text.len() > 4096 {
        return Err(Error::at(
            Stage::Check,
            crate::invalid(span, "Core interface bytes exhausted"),
        ));
    }
    let mut stack_in = alloc::vec::Vec::new();
    let mut stack_out = alloc::vec::Vec::new();
    let mut is_separator = false;
    for word in text.split_ascii_whitespace() {
        if word == "--" {
            if is_separator {
                return Err(Error::at(
                    Stage::Check,
                    crate::invalid(span, "duplicate stack separator"),
                ));
            }
            is_separator = true;
            continue;
        }
        let stack = if is_separator {
            &mut stack_out
        } else {
            &mut stack_in
        };
        if stack.len() >= 128 {
            return Err(Error::at(
                Stage::Check,
                crate::invalid(span, "Core stack height exhausted"),
            ));
        }
        let parsed = declared::types::parse_with_families(word, &[], &[], None, span)
            .map_err(|error| Error::at(Stage::Check, error))?;
        if !core_type(&parsed.ty) {
            return Err(Error::at(
                Stage::Check,
                crate::invalid(span, "unsupported Core interface type"),
            ));
        }
        stack.push(parsed.ty);
    }
    if !is_separator {
        return Err(Error::at(
            Stage::Check,
            crate::invalid(span, "missing stack separator"),
        ));
    }
    Ok(CoreInterface {
        stack_in,
        stack_out,
    })
}

#[expect(
    tigerstyle::no_recursion,
    reason = "Owner: noble-contracts; the recursion follows the structure of an already checked, size-bounded input."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the walk recurses through boxed Ty children, and Box dereference is not const."
)]
fn core_type(ty: &noble_kernel::types::Ty) -> bool {
    #[expect(
        tigerstyle::non_trait_imports,
        reason = "Owner: noble-contracts; the module imports these domain items directly, and the qualified form at every use site would add noise without changing the dependency."
    )]
    use noble_kernel::types::Ty;
    match ty {
        Ty::I64 | Ty::Bool | Ty::Unit | Ty::Text | Ty::Syntax => true,
        Ty::List(item) => core_type(item),
        Ty::Pair(left, right) | Ty::Sum(left, right) => core_type(left) && core_type(right),
        Ty::Program(input, output, effects) => {
            effects.as_slice().is_empty()
                && input.iter().all(core_type)
                && output.iter().all(core_type)
        }
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Builtin(u32),
    Named(u32),
    /// Finite lexical self occurrence; never a namespace lookup or an
    /// installed-definition index until the entire owner is checked.
    SelfOwner,
    SlotInvoke(Option<u32>),
}

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Literal(noble_kernel::untrusted::Lit),
    Text(alloc::vec::Vec<u8>),
    Word(alloc::vec::Vec<u8>),
    Call(Target),
    Quotation(alloc::vec::Vec<u32>),
    EditorHole,
}

#[derive(Clone, Debug, PartialEq)]
struct Node {
    kind: Kind,
    span: crate::Span,
}

#[derive(Clone, Debug, PartialEq)]
struct Tree {
    nodes: alloc::vec::Vec<Node>,
    body: alloc::vec::Vec<u32>,
    span: crate::Span,
}

impl Tree {
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; node lookup validates the arena offset and constructs an owned diagnostic for an invalid ID."
    )]
    fn node(&self, id: u32) -> Result<&Node, crate::Diagnostic> {
        match self.nodes.get(attempt!(crate::offset(id, self.span))) {
            Some(node) => Ok(node),
            None => Err(crate::internal(self.span)),
        }
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::platform_dependent_cast,
    reason = "Owner: noble-contracts; the operand is a u32 table index and the cast is a widening that cannot fail where usize is at least 32 bits; the portable form would add fallible plumbing to infallible lookups"
)]
#[expect(
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-contracts; the collection is bounded by the surrounding admission limits, which are checked before this loop runs"
)]
/// Follow immutable named targets, including calls inside quotations. A
/// replacement must not enter the old identity through any historical body:
/// rebuilding current dependents would otherwise publish an apparent cycle
/// whose new root still invokes the old root.
fn reaches_index(
    tree: &Tree,
    definitions: &[Named],
    target: usize,
    mut work: u32,
) -> Result<bool, Error> {
    let span = tree.span;
    let mut visited = alloc::vec![false; definitions.len()];
    let mut pending = alloc::vec::Vec::new();
    let mut current = Some(tree);
    while let Some(body) = current {
        for node in &body.nodes {
            work = work.checked_sub(1).ok_or_else(|| {
                Error::at(
                    Stage::Acceptance,
                    exhausted(span, "live replacement dependency walk exhausted"),
                )
            })?;
            if let Kind::Call(Target::Named(index)) = &node.kind {
                let index = *index as usize;
                if index == target {
                    return Ok(true);
                }
                let seen = visited.get_mut(index).ok_or_else(|| {
                    Error::at(
                        Stage::Acceptance,
                        crate::invalid(span, "invalid retained definition target"),
                    )
                })?;
                if !*seen {
                    *seen = true;
                    pending.push(index);
                }
            }
        }
        current = pending.pop().map(|index| &definitions[index].tree);
    }
    Ok(false)
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
)]
#[expect(
    tigerstyle::platform_dependent_cast,
    reason = "Owner: noble-contracts; the operand is a u32 table index and the cast is a widening that cannot fail where usize is at least 32 bits; the portable form would add fallible plumbing to infallible lookups"
)]
/// Replay is allowed to change exactly the named edges rebuilt in this batch.
/// Source spelling alone is not authority to retarget a shadowed historical
/// call to today's same-name definition.
fn replay_preserves_targets(
    original: &Tree,
    replayed: &Tree,
    replacements: &[Option<usize>],
) -> bool {
    if original.body != replayed.body || original.nodes.len() != replayed.nodes.len() {
        return false;
    }
    original
        .nodes
        .iter()
        .zip(&replayed.nodes)
        .all(|(old, new)| match (&old.kind, &new.kind) {
            (Kind::Literal(a), Kind::Literal(b)) => a == b,
            (Kind::Text(a), Kind::Text(b)) => a == b,
            (Kind::Quotation(a), Kind::Quotation(b)) => a == b,
            (Kind::Call(Target::Builtin(a)), Kind::Call(Target::Builtin(b))) => a == b,
            (Kind::Call(Target::SlotInvoke(a)), Kind::Call(Target::SlotInvoke(b))) => a == b,
            (Kind::Call(Target::SelfOwner), Kind::Call(Target::SelfOwner)) => true,
            (Kind::Call(Target::Named(a)), Kind::Call(Target::Named(b))) => replacements
                .get(*a as usize)
                .is_some_and(|replacement| replacement.unwrap_or(*a as usize) == *b as usize),
            _ => false,
        })
}

#[derive(Clone, Debug, PartialEq)]
struct SymbolicNode {
    draft: u32,
    span: crate::Span,
    target: Option<Target>,
}

#[derive(Clone, Debug, PartialEq)]
struct SignedRelation {
    /// Source namespace slot reserved for the owner before it was installed.
    source_slot: u32,
    /// Every executable lexical node has exactly one rigid inferred draft;
    /// `None` slots are independently re-lowered, non-executable lexical
    /// binder/name syntax. The parser's final `def` envelope is omitted.
    /// Concrete environments may use different specialization slots, so this
    /// source-order relation, not a concrete Candidate, carries the rank-1
    /// owner check across publication and signed-only compiler ingress.
    symbolic_nodes: alloc::vec::Vec<Option<SymbolicNode>>,
    /// Original self occurrences, including those inside quotations.
    lexical_nodes: alloc::vec::Vec<u32>,
}

#[derive(Clone, Debug)]
struct Named {
    name: alloc::string::String,
    identity: u64,
    owner: Option<u64>,
    tree: Tree,
    signature: Option<noble_kernel::words::Scheme>,
    signed: Option<SignedRelation>,
    /// Original alias spellings and exact source nodes bound to their
    /// independently resolved module owners at definition admission.
    alias_uses: alloc::vec::Vec<AliasUse>,
    /// Exact length-prefixed retained source offset assigned at successful
    /// outside-definition publication. Module definitions instead retain
    /// their own immutable whole-module source and have no history offset.
    history_start: Option<usize>,
}

impl PartialEq for Named {
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; a PartialEq::eq trait implementation cannot be declared const."
    )]
    fn eq(&self, other: &Self) -> bool {
        let is_same_signature = match (&self.signature, &other.signature) {
            (Some(a), Some(b)) => {
                a.var_kinds == b.var_kinds
                    && a.stack_in == b.stack_in
                    && a.stack_out == b.stack_out
                    && a.effects == b.effects
            }
            (None, None) => true,
            _ => false,
        };
        self.name == other.name
            && self.identity == other.identity
            && self.owner == other.owner
            && self.tree == other.tree
            && is_same_signature
            && self.signed == other.signed
            && self.alias_uses == other.alias_uses
            && self.history_start == other.history_start
    }
}

#[derive(Clone, Debug, PartialEq)]
struct AliasUse {
    spelling: alloc::string::String,
    owner: u64,
    nodes: alloc::vec::Vec<(u32, Target)>,
}

/// An immutable preparation. A declaration deliberately has no executable root.
#[derive(Clone, Debug)]
pub struct Prepared {
    generation: u64,
    history: alloc::vec::Vec<u8>,
    hosts: bool,
    text_cursor: bool,
    live_selected: Option<alloc::string::String>,
    live_slots: bool,
    live_slot_owner: Option<alloc::sync::Arc<()>>,
    limits: crate::Limits,
    calculator: bool,
    checked_i64: bool,
    boundary: Option<alloc::vec::Vec<u8>>,
    definition: Option<Named>,
    selected_root: Option<u32>,
    addition: alloc::vec::Vec<u8>,
    submission: Option<noble_kernel::execution::Submission>,
    checked: Option<noble_kernel::untrusted::Checked>,
    work_used: u32,
    output: alloc::vec::Vec<noble_kernel::types::Ty>,
}

impl Prepared {
    pub(crate) const fn work_used(&self) -> u32 {
        self.work_used
    }
    pub const fn submission(&self) -> Option<&noble_kernel::execution::Submission> {
        self.submission.as_ref()
    }
    /// The kernel-accepted derivation, including immutable typed live-site
    /// descriptors. A candidate or source ordinal alone is never authority.
    pub const fn checked(&self) -> Option<&noble_kernel::untrusted::Checked> {
        self.checked.as_ref()
    }
    pub fn live_sites(&self) -> Option<&[noble_kernel::untrusted::LiveSite]> {
        self.checked
            .as_ref()
            .map(|checked| checked.live_sites.as_slice())
    }
    pub fn definition_name(&self) -> Option<&str> {
        self.definition
            .as_ref()
            .map(|definition| definition.name.as_str())
    }
    pub const fn output(&self) -> &[noble_kernel::types::Ty] {
        self.output.as_slice()
    }
    pub const fn is_definition(&self) -> bool {
        self.definition.is_some()
    }
    /// Prospective identity of this checked definition. It is not an installed
    /// identity and never publication authority.
    pub fn definition_identity(&self) -> Option<u64> {
        self.definition
            .as_ref()
            .map(|definition| definition.identity)
    }
    /// Ordered interface descriptor of this checked definition.
    /// Descriptor of this preparation's own submission expectation. A definition
    /// preparation has no deferred submission and therefore no descriptor here.
    pub fn submission_interface(&self) -> Option<DefinitionInterface> {
        self.submission()
            .and_then(|submission| DefinitionInterface::of(&submission.request.expected))
    }
}

/// A pure namespace, independent of the runtime value stack.
/// `new` explicitly configures resource-free `test.emit` and `test.abort` test
/// bindings. `without_test_hosts` is the bootstrap-only resolution environment.
#[derive(Debug)]
pub struct Session {
    definitions: alloc::vec::Vec<Named>,
    history: alloc::vec::Vec<u8>,
    generation: u64,
    hosts: bool,
    text_cursor: bool,
    live_selected: Option<alloc::string::String>,
    live_slots: Option<LiveSlots>,
    calculator: bool,
    checked_i64: bool,
    bindings: Option<crate::component::Bindings>,
    declared: Option<declared::Context>,
}

#[derive(Debug)]
struct LiveSlots {
    environment: noble_kernel::contracts::Env,
    resources: alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>,
    owner: alloc::sync::Arc<()>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub const fn new() -> Self {
        Self {
            definitions: alloc::vec::Vec::new(),
            history: alloc::vec::Vec::new(),
            generation: 0,
            hosts: true,
            text_cursor: false,
            live_selected: None,
            live_slots: None,
            calculator: false,
            checked_i64: false,
            bindings: None,
            declared: None,
        }
    }
    pub const fn without_test_hosts() -> Self {
        Self {
            definitions: alloc::vec::Vec::new(),
            history: alloc::vec::Vec::new(),
            generation: 0,
            hosts: false,
            text_cursor: false,
            live_selected: None,
            live_slots: None,
            calculator: false,
            checked_i64: false,
            bindings: None,
            declared: None,
        }
    }
    /// Separately selected pure byte cursor; Core and declared sessions remain unchanged.
    pub fn new_text_cursor() -> Self {
        Self {
            text_cursor: true,
            ..Self::without_test_hosts()
        }
    }
    /// Only this selected named body's lexical source can request a live edit.
    /// Ordinary sessions never install either live operation.
    pub fn new_live(selected_name: &str) -> Self {
        Self {
            live_selected: Some(alloc::string::String::from(selected_name)),
            ..Self::without_test_hosts()
        }
    }
    /// A separate, host-selected profile. The environment must already have
    /// been enabled and its versioned resource schemas registered by the host.
    /// Core and guarded Live-Wasm-Draft environments cannot gain slot syntax.
    pub fn new_live_slots(environment: noble_kernel::contracts::Env) -> Result<Self, Error> {
        if !environment.live_slots || environment.text_cursor {
            return Err(Error::at(
                Stage::Acceptance,
                crate::invalid(
                    crate::Span { start: 0, end: 0 },
                    "live slots require a host-enabled independent source environment",
                ),
            ));
        }
        Ok(Self {
            live_slots: Some(LiveSlots {
                environment,
                resources: alloc::vec::Vec::new(),
                owner: alloc::sync::Arc::new(()),
            }),
            ..Self::without_test_hosts()
        })
    }

    /// Only a trusted host which independently granted the bootstrap test
    /// effects may select this profile. Source text cannot enable them, and
    /// ordinary live-slot sessions remain unable to resolve `test.emit`.
    pub fn new_live_slots_with_test_hosts(
        environment: noble_kernel::contracts::Env,
    ) -> Result<Self, Error> {
        let mut session = Self::new_live_slots(environment.enable_live_test_hosts())?;
        session.hosts = true;
        Ok(session)
    }

    #[expect(
        tigerstyle::compound_condition,
        reason = "Owner: noble-contracts; the guard is the admission rule verbatim; splitting it would obscure the predicate it states"
    )]
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-contracts; the parameter is the owned builder state this stage accumulates into, not a borrowed input"
    )]
    /// Resolve a source spelling such as `Account@1` only to an already
    /// registered exact host nominal resource schema. This does not mint a
    /// resource or a borrowed live reference.
    pub fn register_live_resource(
        &mut self,
        name: &str,
        id: noble_kernel::types::NominalTypeId,
    ) -> Result<(), Error> {
        let span = crate::Span { start: 0, end: 0 };
        let Some((base, version)) = name.rsplit_once('@') else {
            return Err(Error::at(
                Stage::Acceptance,
                crate::invalid(span, "live resource name requires a version suffix"),
            ));
        };
        if base.is_empty()
            || !base
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            || !base.as_bytes()[0].is_ascii_alphabetic()
            || version.is_empty()
            || (version.len() > 1 && version.starts_with('0'))
            || !version.bytes().all(|byte| byte.is_ascii_digit())
            || version.parse::<u32>().is_err()
        {
            return Err(Error::at(
                Stage::Acceptance,
                crate::invalid(span, "invalid versioned live resource name"),
            ));
        }
        let Some(profile) = self.live_slots.as_mut() else {
            return Err(Error::at(
                Stage::Acceptance,
                crate::invalid(
                    span,
                    "live resource registration requires the live-slot profile",
                ),
            ));
        };
        let Some(declaration) = profile.environment.nominal(id) else {
            return Err(Error::at(
                Stage::Acceptance,
                crate::invalid(span, "unregistered nominal resource schema"),
            ));
        };
        let noble_kernel::types::NominalShape::Opaque(payload) = &declaration.shape else {
            return Err(Error::at(
                Stage::Acceptance,
                crate::invalid(span, "live resource schema must be opaque"),
            ));
        };
        let noble_kernel::types::Ty::Resource(kind) = &**payload else {
            return Err(Error::at(
                Stage::Acceptance,
                crate::invalid(span, "live resource schema must own a registered resource"),
            ));
        };
        if !profile.environment.live_resource_nominals.contains(&id)
            || !profile.environment.resource_kinds.contains(kind)
            || !declaration.exported
            || declaration.public != [false, false]
            || profile.resources.iter().any(|(existing, ty)| {
                existing == name
                    || matches!(ty, noble_kernel::types::Ty::Nominal(other, _) if *other == id)
            })
        {
            return Err(Error::at(
                Stage::Acceptance,
                crate::invalid(span, "duplicate or constructible live resource schema"),
            ));
        }
        let next = self.generation.checked_add(1).ok_or_else(|| {
            Error::at(
                Stage::Acceptance,
                exhausted(span, "session generation limit exceeded"),
            )
        })?;
        profile.resources.push((
            alloc::string::String::from(name),
            noble_kernel::types::Ty::Nominal(id, alloc::boxed::Box::new(declaration.shape.clone())),
        ));
        self.generation = next;
        Ok(())
    }

    /// Parse a bounded type expression against this profile's registered
    /// versioned names; unregistered versions never alias a same-layout schema.
    pub fn parse_live_type(&self, text: &str) -> Result<noble_kernel::types::Ty, Error> {
        let span = crate::Span { start: 0, end: 0 };
        let Some(profile) = &self.live_slots else {
            return Err(Error::at(
                Stage::Check,
                crate::invalid(span, "live reference types require the live-slot profile"),
            ));
        };
        declared::types::parse_with_families(
            text,
            &profile.resources,
            &[],
            Some(&profile.environment),
            span,
        )
        .map(|parsed| parsed.ty)
        .map_err(|error| Error::at(Stage::Check, error))
    }
    /// Opt-in pure calculator profile; nominal modules use ModuleSession.
    pub fn new_calculator() -> Self {
        Self {
            calculator: true,
            ..Self::without_test_hosts()
        }
    }
    /// Opt-in checked signed-I64 arithmetic with explicit error results.
    pub fn new_checked_i64() -> Self {
        Self {
            checked_i64: true,
            ..Self::without_test_hosts()
        }
    }
    pub const fn generation(&self) -> u64 {
        self.generation
    }
    /// Immutable checked source identity, not the namespace generation or a
    /// caller-supplied name/identity pair.
    pub fn selected_definition_identity(&self, name: &str) -> Option<u64> {
        if self.live_selected.as_deref() != Some(name) {
            return None;
        }
        self.installed_definition_identity(name)
    }

    /// Immutable identity of the current definition named `name`, independent
    /// of whether the live self-edit grant selects it.
    pub fn installed_definition_identity(&self, name: &str) -> Option<u64> {
        self.definitions
            .iter()
            .rev()
            .find(|definition| definition.name == name)
            .map(|definition| definition.identity)
    }

    /// Names of current named dependents whose resolved calls reach `name`, in
    /// declaration order. Historical definitions retained for captured Programs
    /// are not dependents themselves, but a current body that reaches `name`
    /// through one is reported: the resolved hop does not hide the dependency.
    pub fn current_dependent_names(&self, name: &str) -> alloc::vec::Vec<alloc::string::String> {
        let Some(index) = self
            .definitions
            .iter()
            .rposition(|definition| definition.name == name)
        else {
            return alloc::vec::Vec::new();
        };
        self.current_dependents(index)
            .into_iter()
            .filter_map(|dependent| {
                self.definitions
                    .get(dependent)
                    .map(|definition| definition.name.clone())
            })
            .collect()
    }

    /// Only current named bodies count as live dependents. Old Programs may
    /// retain historical definitions without acquiring the guest edit grant.
    pub fn live_definition_has_dependents(&self, name: &str) -> bool {
        let Some(index) = self
            .definitions
            .iter()
            .rposition(|definition| definition.name == name)
        else {
            return true;
        };
        !self.current_dependents(index).is_empty()
    }

    #[expect(
        tigerstyle::platform_dependent_cast,
        reason = "Owner: noble-contracts; the operand is a u32 table index and the cast is a widening that cannot fail where usize is at least 32 bits; the portable form would add fallible plumbing to infallible lookups"
    )]
    #[expect(
        tigerstyle::unbounded_collection_growth,
        reason = "Owner: noble-contracts; the collection is bounded by the surrounding admission limits, which are checked before this loop runs"
    )]
    fn current_dependents(&self, original: usize) -> alloc::vec::Vec<usize> {
        // Every resolved edge points to an earlier immutable definition.
        // Current dependent bodies can therefore be visited in declaration
        // order, even though their later rebuilds are appended in a batch.
        let mut affected = alloc::vec![false; self.definitions.len()];
        affected[original] = true;
        let mut dependents = alloc::vec::Vec::new();
        for (index, definition) in self.definitions.iter().enumerate() {
            if index == original {
                continue;
            }
            let has_affected_call = definition.tree.nodes.iter().any(|node| match &node.kind {
                Kind::Call(Target::Named(target)) => {
                    affected.get(*target as usize).copied().unwrap_or(false)
                }
                _ => false,
            });
            if !has_affected_call {
                continue;
            }
            // A shadowed historical definition is still a resolved hop: a
            // current body that calls it reaches the original identity through
            // it, even though the hop itself is not a live dependent.
            affected[index] = true;
            let is_shadowed = self
                .definitions
                .iter()
                .skip(index.saturating_add(1))
                .any(|later| later.name == definition.name);
            if !is_shadowed {
                dependents.push(index);
            }
        }
        dependents
    }

    #[expect(
        tigerstyle::compound_condition,
        reason = "Owner: noble-contracts; the guard is the admission rule verbatim; splitting it would obscure the predicate it states"
    )]
    #[expect(
        tigerstyle::unchecked_division,
        reason = "Owner: noble-contracts; the divisor is a compile-time constant known non-zero"
    )]
    #[expect(
        tigerstyle::sentinel_fallback,
        reason = "Owner: noble-contracts; the fallback is the documented refusal path of this decoder"
    )]
    /// Build one private successor containing a replacement and every current
    /// definition whose immutable resolved calls reach its previous identity.
    /// Historical definitions stay in place for already captured Programs.
    pub fn preview_core_rebuild(&self, prepared: &Prepared) -> Result<Self, PreviewError> {
        let span = crate::Span { start: 0, end: 0 };
        if !prepared.is_definition()
            || self.bindings.is_some()
            || self.declared.is_some()
            || !self.is_current_namespace(prepared)
        {
            return Err(classify(
                PreviewRefusal::UnsupportedProfile,
                Error::at(
                    Stage::Acceptance,
                    crate::invalid(span, "live Core preview requires a checked definition"),
                ),
            ));
        }
        let Some(replacement) = prepared.definition.as_ref() else {
            return Err(classify(
                PreviewRefusal::UnsupportedProfile,
                Error::at(
                    Stage::Acceptance,
                    crate::invalid(span, "live Core preview requires a definition"),
                ),
            ));
        };
        let name = &replacement.name;
        let original = self
            .definitions
            .iter()
            .rposition(|definition| definition.name == *name);
        if let Some(index) = original {
            let compatibility = replacement::compatible(
                self,
                &self.definitions[index],
                replacement,
                prepared.limits,
            )
            .map_err(|diagnostic| {
                classify_diagnostic(
                    PreviewRefusal::ComparisonFailed,
                    Error::at(Stage::Check, diagnostic),
                )
            })?;
            match compatibility {
                replacement::Compatibility::Compatible => {}
                replacement::Compatibility::InterfaceChanged => {
                    return Err(classify(
                        PreviewRefusal::InterfaceChanged,
                        Error::at(
                            Stage::Acceptance,
                            crate::invalid(
                                span,
                                "live replacement changes the checked stack interface",
                            ),
                        ),
                    ));
                }
                replacement::Compatibility::EffectGrowth => {
                    return Err(classify(
                        PreviewRefusal::EffectGrowth,
                        Error::at(
                            Stage::Acceptance,
                            crate::invalid(
                                span,
                                "live replacement increases effects beyond the installed bound",
                            ),
                        ),
                    ));
                }
            }
            let is_self_rebind = reaches_index(
                &replacement.tree,
                &self.definitions,
                index,
                prepared.limits.work,
            )
            .map_err(|error| classify_diagnostic(PreviewRefusal::ComparisonFailed, error))?;
            if is_self_rebind {
                return Err(classify(
                    PreviewRefusal::SelfRebind,
                    Error::at(
                        Stage::Acceptance,
                        crate::invalid(
                            span,
                            "live replacement transitively depends on its previous identity",
                        ),
                    ),
                ));
            }
        }
        let dependents =
            original.map_or_else(alloc::vec::Vec::new, |index| self.current_dependents(index));
        let mut successor = Self {
            definitions: self.definitions.clone(),
            history: self.history.clone(),
            generation: self.generation,
            hosts: self.hosts,
            text_cursor: self.text_cursor,
            live_selected: self.live_selected.clone(),
            live_slots: None,
            calculator: self.calculator,
            checked_i64: self.checked_i64,
            bindings: None,
            declared: None,
        };
        successor
            .commit(prepared.clone())
            .map_err(|error| classify_diagnostic(PreviewRefusal::UnsupportedProfile, error))?;
        let mut replacements = alloc::vec![None; self.definitions.len()];
        if let Some(index) = original {
            replacements[index] = Some(self.definitions.len());
        }
        let replay_work =
            prepared.limits.work / u32::try_from(dependents.len().max(1)).unwrap_or(u32::MAX);
        if replay_work == 0 {
            return Err(classify(
                PreviewRefusal::WorkExhausted,
                Error::at(
                    Stage::Acceptance,
                    exhausted(span, "live dependent rebuild work limit exceeded"),
                ),
            ));
        }
        for index in dependents {
            let bytes = self
                .definition_source(index)
                .map_err(|error| classify_diagnostic(PreviewRefusal::RetainedSource, error))?;
            let limits = crate::Limits {
                work: replay_work,
                ..prepared.limits
            };
            let dependent = successor
                .prepare(bytes, &[], limits)
                .map_err(|error| classify_diagnostic(PreviewRefusal::DependentRebuild, error))?;
            let Some(rebuilt) = dependent.definition.as_ref() else {
                return Err(classify(
                    PreviewRefusal::DependentRebuild,
                    Error::at(
                        Stage::Acceptance,
                        crate::invalid(span, "retained dependent source is not a definition"),
                    ),
                ));
            };
            if rebuilt.name != self.definitions[index].name
                || successor.core_definition_requires_host_effects(&dependent)
            {
                return Err(classify(
                    PreviewRefusal::DependentRebuild,
                    Error::at(
                        Stage::Acceptance,
                        crate::invalid(
                            span,
                            "retained dependent changed name or requires unavailable host effects",
                        ),
                    ),
                ));
            }
            if !replay_preserves_targets(
                &self.definitions[index].tree,
                &rebuilt.tree,
                &replacements,
            ) {
                return Err(classify(
                    PreviewRefusal::DependentRebuild,
                    Error::at(
                        Stage::Acceptance,
                        crate::invalid(
                            span,
                            "retained dependent changed an unapproved resolved call target",
                        ),
                    ),
                ));
            }
            let compatibility =
                replacement::compatible(&successor, &self.definitions[index], rebuilt, limits)
                    .map_err(|diagnostic| {
                        classify_diagnostic(
                            PreviewRefusal::DependentRebuild,
                            Error::at(Stage::Check, diagnostic),
                        )
                    })?;
            if compatibility != replacement::Compatibility::Compatible {
                return Err(classify(
                    PreviewRefusal::DependentRebuild,
                    Error::at(
                        Stage::Acceptance,
                        crate::invalid(
                            span,
                            "rebuilt dependent changes the checked stack interface or increases effects",
                        ),
                    ),
                ));
            }
            replacements[index] = Some(successor.definitions.len());
            successor
                .commit(dependent)
                .map_err(|error| classify_diagnostic(PreviewRefusal::DependentRebuild, error))?;
        }
        Ok(successor)
    }

    /// The ordered, statically inferred interface of the current definition
    /// named `name`, if this namespace holds one.
    pub fn installed_ordered_interface(
        &self,
        name: &str,
        limits: crate::Limits,
    ) -> Result<Option<OrderedInterface>, Error> {
        let Some(definition) = self
            .definitions
            .iter()
            .rfind(|definition| definition.name == name)
        else {
            return Ok(None);
        };
        replacement::ordered(self, definition, limits)
            .map(Some)
            .map_err(|diagnostic| Error::at(Stage::Check, diagnostic))
    }

    /// The ordered, statically inferred interface of one prepared candidate
    /// definition, checked against this namespace's environment.
    pub fn candidate_ordered_interface(
        &self,
        prepared: &Prepared,
    ) -> Result<Option<OrderedInterface>, Error> {
        let Some(definition) = prepared.definition.as_ref() else {
            return Ok(None);
        };
        replacement::ordered(self, definition, prepared.limits)
            .map(Some)
            .map_err(|diagnostic| Error::at(Stage::Check, diagnostic))
    }

    /// Never reconstruct a dependent from its already-resolved tree: the
    /// source offset was fixed when its original checked definition committed.
    fn definition_source(&self, index: usize) -> Result<&[u8], Error> {
        let span = crate::Span { start: 0, end: 0 };
        let offset_bytes = self
            .definitions
            .get(index)
            .and_then(|definition| definition.history_start)
            .ok_or_else(|| {
                Error::at(
                    Stage::Acceptance,
                    crate::invalid(span, "retained dependent has no outside source provenance"),
                )
            })?;
        let header_end = offset_bytes.checked_add(4).ok_or_else(|| {
            Error::at(
                Stage::Acceptance,
                crate::invalid(span, "retained source header overflows"),
            )
        })?;
        let header = self.history.get(offset_bytes..header_end).ok_or_else(|| {
            Error::at(
                Stage::Acceptance,
                crate::invalid(span, "retained source header is missing"),
            )
        })?;
        let length_bytes = usize::try_from(u32::from_le_bytes(
            <[u8; 4]>::try_from(header).map_err(|_| {
                Error::at(
                    Stage::Acceptance,
                    crate::invalid(span, "invalid retained source length"),
                )
            })?,
        ))
        .map_err(|_| {
            Error::at(
                Stage::Acceptance,
                crate::invalid(span, "retained source length exceeds address space"),
            )
        })?;
        let end = header_end.checked_add(length_bytes).ok_or_else(|| {
            Error::at(
                Stage::Acceptance,
                crate::invalid(span, "retained source length overflows"),
            )
        })?;
        self.history.get(header_end..end).ok_or_else(|| {
            Error::at(
                Stage::Acceptance,
                crate::invalid(span, "retained dependent source is incomplete"),
            )
        })
    }

    #[expect(
        tigerstyle::platform_dependent_cast,
        reason = "Owner: noble-contracts; the operand is a u32 table index and the cast is a widening that cannot fail where usize is at least 32 bits; the portable form would add fallible plumbing to infallible lookups"
    )]
    /// A definition declaration has no executable root; checking only an
    /// empty post-definition submission would miss latent host operations in
    /// its body. Walk resolved calls (including immutable named dependencies)
    /// and the kernel's effect schemes before allowing it into a live
    /// namespace. Only that session's scoped live effects are permitted;
    /// unavailable schemes and ambient test hosts fail closed.
    pub fn core_definition_requires_host_effects(&self, prepared: &Prepared) -> bool {
        let Some(definition) = prepared.definition.as_ref() else {
            return true;
        };
        let environment = match self.environment() {
            Ok(environment) => environment,
            Err(_) => return true,
        };
        let mut seen = alloc::vec![false; self.definitions.len()];
        let mut pending = alloc::vec![&definition.tree];
        while let Some(tree) = pending.pop() {
            for node in &tree.nodes {
                match &node.kind {
                    Kind::Call(Target::Builtin(index))
                        if environment.defs.get(*index as usize).is_none_or(|scheme| {
                            scheme.effects.iter().any(|effect| {
                                !matches!(effect,
                                    noble_kernel::shapes::EffectSlot::Effect(
                                        noble_kernel::types::EffId(3 | 4)
                                    ) | noble_kernel::shapes::EffectSlot::Var(_)
                                        if self.live_selected.is_some())
                            })
                        }) =>
                    {
                        return true;
                    }
                    Kind::Call(Target::Named(index)) => {
                        let Some(prior) = self.definitions.get(*index as usize) else {
                            return true;
                        };
                        let Some(visited) = seen.get_mut(*index as usize) else {
                            return true;
                        };
                        if !*visited {
                            *visited = true;
                            pending.push(&prior.tree);
                        }
                    }
                    _ => {}
                }
            }
        }
        false
    }

    pub(crate) fn with_bindings(bindings: crate::component::Bindings) -> Self {
        Self {
            bindings: Some(bindings),
            ..Self::without_test_hosts()
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
    )]
    fn effect_universe(&self) -> u64 {
        if let Some(profile) = &self.live_slots {
            return profile
                .environment
                .effects
                .iter()
                .fold(0u64, |mask, id| mask | 1u64.checked_shl(id.0).unwrap_or(0));
        }
        if let Some(context) = &self.declared {
            // Kernel validation binds non-bootstrap IDs to exact host-selected adapters.
            return context
                .environment
                .effects
                .iter()
                .fold(0u64, |mask, id| mask | 1u64.checked_shl(id.0).unwrap_or(0));
        }
        match &self.bindings {
            Some(bindings) => bindings.effects,
            None => {
                if self.live_selected.is_some() {
                    (1 << 3) | (1 << 4)
                } else if self.hosts {
                    3
                } else {
                    0
                }
            }
        }
    }

    fn program_type_effect_universe(&self) -> u64 {
        let effects = self.effect_universe();
        effects
            | self
                .bindings
                .as_ref()
                .map_or(0, |bindings| bindings.program_type_effects)
    }

    /// Consume a preparation as an explicit deterministic namespace transition.
    ///
    /// Generation, host configuration, and complete declaration history must
    /// match; these checks and the next-generation check precede every mutation,
    /// so any returned error leaves the session unchanged. A successful
    /// declaration appends its owned definition and length-prefixed source
    /// history together. Every successful commit advances generation once,
    /// including executable submissions, which leave definitions/history intact.
    /// Host configuration is unchanged; no submission or host code is executed.
    #[expect(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers; commit is the explicit owned namespace publication transition, not preparation scratch: snapshot and generation checks precede mutation, a declaration appends its definition/history, and every success advances generation once without executing guest or host code."
    )]
    pub fn commit(&mut self, prepared: Prepared) -> Result<(), Error> {
        let span = crate::Span { start: 0, end: 0 };
        if !self.is_current_namespace(&prepared) {
            return Err(Error::at(
                Stage::Acceptance,
                crate::invalid(
                    span,
                    "stale preparation belongs to a different namespace snapshot",
                ),
            ));
        }
        let next = match self.generation.checked_add(1) {
            Some(next) => next,
            None => {
                return Err(Error::at(
                    Stage::Acceptance,
                    exhausted(span, "session generation limit exceeded"),
                ))
            }
        };
        if let Some(mut definition) = prepared.definition {
            if definition.owner.is_none() {
                definition.history_start = Some(self.history.len());
            }
            self.definitions.push(definition);
            self.history.extend_from_slice(&prepared.addition);
        }
        self.generation = next;
        Ok(())
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; namespace comparison uses non-const exact Vec content equality over retained history and optional boundary identities."
    )]
    #[expect(
        tigerstyle::compound_condition,
        reason = "Owner: noble-contracts; the guard is the admission rule verbatim; splitting it would obscure the predicate it states"
    )]
    fn is_current_namespace(&self, prepared: &Prepared) -> bool {
        let is_live_slot_owner_matches = match (&self.live_slots, &prepared.live_slot_owner) {
            (Some(slots), Some(owner)) => alloc::sync::Arc::ptr_eq(&slots.owner, owner),
            (None, None) => true,
            _ => false,
        };
        if prepared.generation != self.generation
            || prepared.hosts != self.hosts
            || prepared.text_cursor != self.text_cursor
            || prepared.live_selected != self.live_selected
            || prepared.live_slots != self.live_slots.is_some()
            || prepared.calculator != self.calculator
            || prepared.checked_i64 != self.checked_i64
            || prepared.history != self.history
            || !is_live_slot_owner_matches
        {
            return false;
        }
        match (&self.bindings, &prepared.boundary) {
            (Some(bindings), Some(boundary)) => bindings.key == *boundary,
            (None, None) => true,
            (Some(_), None) | (None, Some(_)) => false,
        }
    }
}

fn exhausted(span: crate::Span, message: &str) -> crate::Diagnostic {
    crate::Diagnostic::new(crate::DiagnosticKind::Exhausted, span, message)
}

const fn classify(kind: PreviewRefusal, error: Error) -> PreviewError {
    classify_diagnostic(kind, error)
}

/// A budget refusal is always the same class wherever it is met, so an exhausted
/// diagnostic overrides the class its call site would otherwise name. Every
/// other diagnostic keeps the class of the contract that refused.
const fn classify_diagnostic(kind: PreviewRefusal, error: Error) -> PreviewError {
    let kind = match error.diagnostic().kind {
        crate::DiagnosticKind::Exhausted => PreviewRefusal::WorkExhausted,
        _ => kind,
    };
    PreviewError::new(kind, error)
}

/// The source test environment is explicit, resource-free, and fixed. The old
/// checker's Text--Unit host contract remains unchanged outside source.
pub(crate) fn environment() -> Result<noble_kernel::contracts::Env, crate::Diagnostic> {
    let span = crate::Span { start: 0, end: 0 };
    let env = match noble_kernel::contracts::environment() {
        Ok(env) => env,
        Err(_) => return Err(crate::internal(span)),
    };
    adapt_environment(env, true)
}

pub(crate) fn checked_i64_environment() -> Result<noble_kernel::contracts::Env, crate::Diagnostic> {
    let span = crate::Span { start: 0, end: 0 };
    let env = match noble_kernel::contracts::checked_i64_environment() {
        Ok(env) => env,
        Err(_) => return Err(crate::internal(span)),
    };
    adapt_environment(env, false)
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the fixed bootstrap environment and emit slot are checked fallibly before appending matching scheme, behavior, dependency, and effect entries."
)]
fn adapt_environment(
    mut env: noble_kernel::contracts::Env,
    source_abort: bool,
) -> Result<noble_kernel::contracts::Env, crate::Diagnostic> {
    let span = crate::Span { start: 0, end: 0 };
    let tail = noble_kernel::shapes::Pattern::StackVar(noble_kernel::words::Variable(0));
    match env.defs.get_mut(22) {
        Some(emit) => emit.stack_out = alloc::vec![tail.clone()],
        None => return Err(crate::internal(span)),
    }
    if source_abort {
        env.defs.push(noble_kernel::words::Scheme {
            var_kinds: alloc::vec![noble_kernel::words::VariableKind::Stack],
            stack_in: alloc::vec![tail.clone()],
            stack_out: alloc::vec![tail],
            effects: alloc::vec![noble_kernel::shapes::EffectSlot::Effect(
                noble_kernel::types::EffId(1)
            )],
        });
        env.kinds.push(noble_kernel::contracts::Behavior::Named);
        env.deps.push(alloc::vec::Vec::new());
        env.definition_owners.push(None);
        env.effects.push(noble_kernel::types::EffId(1));
    }
    Ok(env)
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::allocation_in_loop,
    reason = "Owner: noble-contracts; the loop body owns one row per definition and the allocation is charged to the metered budget"
)]
#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; this stage imports the domain types it operates on directly, matching the module vocabulary"
)]
fn live_environment() -> Result<noble_kernel::contracts::Env, crate::Diagnostic> {
    use noble_kernel::shapes::{EffectSlot, Pattern};
    #[expect(
        tigerstyle::non_trait_imports,
        reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
    )]
    use noble_kernel::types::EffId;
    #[expect(
        tigerstyle::non_trait_imports,
        reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
    )]
    use noble_kernel::words::{Scheme, Variable, VariableKind};
    let mut env = environment()?;
    let stack = Pattern::StackVar(Variable(0));
    let propose = Scheme {
        var_kinds: alloc::vec![VariableKind::Stack],
        stack_in: alloc::vec![
            stack.clone(),
            Pattern::I64,
            Pattern::program(
                alloc::vec![Pattern::I64],
                alloc::vec![Pattern::I64],
                alloc::vec::Vec::new()
            )
        ],
        stack_out: alloc::vec![stack.clone(), Pattern::Unit],
        effects: alloc::vec![EffectSlot::Effect(EffId(3))],
    };
    let generation = Scheme {
        var_kinds: alloc::vec![VariableKind::Stack],
        stack_in: alloc::vec![stack.clone()],
        stack_out: alloc::vec![stack, Pattern::I64],
        effects: alloc::vec![EffectSlot::Effect(EffId(4))],
    };
    for scheme in [propose, generation] {
        env.defs.push(scheme);
        env.kinds.push(noble_kernel::contracts::Behavior::Named);
        env.deps.push(alloc::vec::Vec::new());
        env.definition_owners.push(None);
    }
    env.effects.push(EffId(3));
    env.effects.push(EffId(4));
    Ok(env)
}

#[expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
)]
#[cfg(test)]
mod tests {
    use super::{DefinitionInterface, PreviewRefusal, Session};

    fn live_limits() -> crate::Limits {
        crate::Limits {
            bytes: 65_536,
            nodes: 16_384,
            depth: 64,
            work: 2_000_000,
        }
    }

    #[test]
    fn checked_i64_session_uses_its_opt_in_operation_environment() {
        let session = Session::new_checked_i64();
        let prepared = session
            .prepare(b"1 2 i64.checked-add", &[], crate::Limits::default())
            .expect("checked-I64 expression");
        let environment = &prepared.submission().expect("submission").environment;
        assert!(!environment.calculator);
        assert!(!environment.text_cursor);
        assert_eq!(
            environment.kind(noble_kernel::contracts::Definition(23)),
            Some(noble_kernel::contracts::Behavior::CheckedI64Add)
        );
    }

    fn borrowed(
        input: alloc::vec::Vec<noble_kernel::types::Ty>,
        output: alloc::vec::Vec<noble_kernel::types::Ty>,
        effects: &[noble_kernel::types::EffId],
    ) -> noble_kernel::types::Ty {
        noble_kernel::types::Ty::LiveRef(
            alloc::boxed::Box::new(input),
            alloc::boxed::Box::new(output),
            noble_kernel::types::EffSet::from_ids(effects),
        )
    }

    #[expect(
        tigerstyle::function_length,
        reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
    )]
    #[test]
    fn live_slots_forward_ordered_borrows_and_reuse_the_same_input() {
        use noble_kernel::types::Ty;
        let session = Session::new_live_slots(
            noble_kernel::contracts::environment()
                .unwrap()
                .enable_live_slots(),
        )
        .unwrap();
        let c = borrowed(alloc::vec![Ty::I64], alloc::vec![Ty::I64], &[]);
        let b = borrowed(
            alloc::vec![Ty::I64, c.clone()],
            alloc::vec![Ty::I64],
            &[noble_kernel::contracts::LIVE_DISPATCH],
        );
        let a = borrowed(
            alloc::vec![Ty::I64, c.clone(), b.clone()],
            alloc::vec![Ty::I64],
            &[noble_kernel::contracts::LIVE_DISPATCH],
        );
        let root = session
            .prepare(
                b"slot.invoke",
                &[Ty::I64, c.clone(), b.clone(), a],
                live_limits(),
            )
            .unwrap();
        let root_submission = root.submission().unwrap();
        assert!(matches!(
            crate::wire::lower_subject(&root_submission.body.candidate),
            Err(crate::wire::ProjectionError::LiveSlot)
        ));
        assert_eq!(root_submission.request.expected.stack_in.len(), 4);
        assert_eq!(root_submission.request.expected.stack_out, [Ty::I64]);
        assert_eq!(root.live_sites().unwrap().len(), 1);
        let site = &root.live_sites().unwrap()[0];
        assert_eq!(site.ref_ordinal, 2);
        assert_eq!(site.root_logical_input_position, 3);
        assert_eq!(site.forwarded_ref_ordinals, [1, 2]);
        assert_eq!(site.stack_in, [Ty::I64, c.clone(), b.clone()]);
        assert_eq!(
            site.allowed_effects,
            noble_kernel::types::EffSet::from_ids(&[noble_kernel::contracts::LIVE_DISPATCH])
        );
        assert_eq!(
            root.checked().unwrap().interface.effects,
            noble_kernel::types::EffSet::from_ids(&[noble_kernel::contracts::LIVE_DISPATCH])
        );

        let reversed = session
            .prepare(
                b"slot.invoke.0",
                &[Ty::I64, b.clone(), c.clone()],
                live_limits(),
            )
            .unwrap();
        assert!(!site.admits_target(&reversed.checked().unwrap().interface));
        let nested = session
            .prepare(
                b"slot.invoke slot.invoke",
                &[Ty::I64, c.clone(), b],
                live_limits(),
            )
            .unwrap();
        assert!(site.admits_target(&nested.checked().unwrap().interface));
        let sites = nested.live_sites().unwrap();
        assert_eq!(sites.len(), 2);
        assert_ne!(sites[0].site_id, sites[1].site_id);
        assert_eq!(sites[0].ref_ordinal, 1);
        assert_eq!(sites[1].ref_ordinal, 1);
        assert_eq!(sites[0].root_logical_input_position, 2);
        assert_eq!(sites[1].root_logical_input_position, 2);
        assert_eq!(sites[0].forwarded_ref_ordinals, [1]);
        assert_eq!(sites[1].forwarded_ref_ordinals, [1]);
        let leaf = session
            .prepare(b"slot.invoke", &[Ty::I64, c], live_limits())
            .unwrap();
        assert!(sites[0].admits_target(&leaf.checked().unwrap().interface));
        assert_eq!(leaf.live_sites().unwrap()[0].forwarded_ref_ordinals, []);
    }

    #[expect(
        tigerstyle::non_trait_imports,
        reason = "Owner: noble-contracts; the module imports these domain types directly, and the qualified form at every use site would add noise without changing the dependency"
    )]
    #[test]
    fn live_slot_rejects_capture_forgery_ambiguity_and_legacy_profiles() {
        use noble_kernel::types::Ty;
        let session = Session::new_live_slots(
            noble_kernel::contracts::environment()
                .unwrap()
                .enable_live_slots(),
        )
        .unwrap();
        let reference = borrowed(alloc::vec![Ty::I64], alloc::vec![Ty::I64], &[]);
        let inputs = [Ty::I64, reference.clone()];
        for source in [b"[ slot.invoke ]".as_slice(), b"def f [ slot.invoke ]"] {
            assert!(session.prepare(source, &inputs, live_limits()).is_err());
        }
        assert!(session
            .prepare(
                b"slot.invoke",
                &[
                    Ty::I64,
                    Ty::Pair(
                        alloc::boxed::Box::new(Ty::I64),
                        alloc::boxed::Box::new(reference.clone()),
                    )
                ],
                live_limits(),
            )
            .is_err());
        assert!(session
            .prepare(
                b"slot.invoke",
                &[Ty::I64, reference.clone(), reference.clone()],
                live_limits(),
            )
            .is_err());
        let selected = session
            .prepare(
                b"slot.invoke.1",
                &[Ty::I64, reference.clone(), reference.clone()],
                live_limits(),
            )
            .unwrap();
        assert_eq!(selected.live_sites().unwrap()[0].ref_ordinal, 1);
        for core in [Session::new(), Session::new_live("f")] {
            assert!(core
                .prepare(b"slot.invoke", &[Ty::I64], live_limits())
                .is_err());
            assert!(core
                .prepare(b"1", core::slice::from_ref(&reference), live_limits())
                .is_err());
        }
        let escaping = borrowed(alloc::vec![Ty::I64], alloc::vec![reference.clone()], &[]);
        assert!(session
            .prepare(b"slot.invoke", &[Ty::I64, escaping], live_limits())
            .is_err());
    }

    #[expect(
        tigerstyle::function_length,
        reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
    )]
    #[test]
    fn versioned_resource_names_resolve_only_registered_exact_schema() {
        use noble_kernel::types::{NominalShape, NominalTypeId, ResourceKind, Ty};
        let mut environment = noble_kernel::contracts::environment()
            .unwrap()
            .enable_live_slots();
        let make = |module, kind| noble_kernel::contracts::NominalDecl {
            id: NominalTypeId { module, ordinal: 0 },
            shape: NominalShape::Opaque(alloc::boxed::Box::new(Ty::Resource(ResourceKind(kind)))),
            exported: true,
            public: [false, false],
        };
        let one = make(110, 41);
        let two = make(111, 42);
        environment = environment.register_live_resource(one.clone()).unwrap();
        environment = environment.register_live_resource(two.clone()).unwrap();
        let mut session = Session::new_live_slots(environment).unwrap();
        session.register_live_resource("Account@1", one.id).unwrap();
        session.register_live_resource("Account@2", two.id).unwrap();
        let account_one = session.parse_live_type("Account@1").unwrap();
        let account_two = session.parse_live_type("Account@2").unwrap();
        assert_ne!(account_one, account_two);
        assert!(session.parse_live_type("Account@3").is_err());
        assert!(session.register_live_resource("Account@3", one.id).is_err());
        let reference = borrowed(
            alloc::vec![account_one.clone(), Ty::I64],
            alloc::vec![account_one.clone(), Ty::I64],
            &[noble_kernel::contracts::TEST_EMIT],
        );
        assert_eq!(
            session
                .parse_live_type("LiveRef<Account@1+I64,Account@1+I64,test.emit>")
                .unwrap(),
            reference,
        );
        let prepared = session
            .prepare(
                b"slot.invoke",
                &[account_one.clone(), Ty::I64, reference.clone()],
                live_limits(),
            )
            .unwrap();
        assert_eq!(prepared.output(), &[account_one.clone(), Ty::I64]);
        assert_eq!(
            prepared.checked().unwrap().interface.effects,
            noble_kernel::types::EffSet::from_ids(&[
                noble_kernel::contracts::TEST_EMIT,
                noble_kernel::contracts::LIVE_DISPATCH,
            ])
        );
        let exact_target = session
            .prepare(b"1 +", &[account_one.clone(), Ty::I64], live_limits())
            .unwrap();
        assert!(prepared.live_sites().unwrap()[0]
            .admits_target(&exact_target.checked().unwrap().interface));
        let wrong_schema = session
            .prepare(b"1 +", &[account_two.clone(), Ty::I64], live_limits())
            .unwrap();
        assert!(!prepared.live_sites().unwrap()[0]
            .admits_target(&wrong_schema.checked().unwrap().interface));
        let wrong_order = session
            .prepare(b"", &[Ty::I64, account_one], live_limits())
            .unwrap();
        assert!(!prepared.live_sites().unwrap()[0]
            .admits_target(&wrong_order.checked().unwrap().interface));
        assert!(session
            .prepare(
                b"slot.invoke",
                &[account_two, Ty::I64, reference],
                live_limits()
            )
            .is_err());
    }

    #[test]
    fn live_batch_rebinds_only_effective_transitive_dependents() {
        let limits = crate::Limits {
            bytes: 65_536,
            nodes: 16_384,
            depth: 64,
            work: 2_000_000,
        };
        let mut session = Session::new();
        for source in [
            b"def addone [ 1 + ]".as_slice(),
            b"def twice [ addone addone ]",
            b"def four [ twice twice ]",
            b"def unrelated [ 10 + ]",
        ] {
            let prepared = session.prepare(source, &[], limits).unwrap();
            session.commit(prepared).unwrap();
        }
        let original = session
            .definitions
            .iter()
            .map(|definition| definition.identity)
            .collect::<alloc::vec::Vec<_>>();
        let replacement = session.prepare(b"def addone [ 2 + ]", &[], limits).unwrap();
        let preview = session.preview_core_rebuild(&replacement).unwrap();
        assert_eq!(session.definitions.len(), 4);
        assert_eq!(preview.definitions.len(), 7);
        assert_eq!(preview.definitions[3].identity, original[3]);
        assert_eq!(preview.definitions[4].name, "addone");
        assert_eq!(preview.definitions[5].name, "twice");
        assert_eq!(preview.definitions[6].name, "four");
        assert_ne!(preview.definitions[4].identity, original[0]);
        assert_ne!(preview.definitions[5].identity, original[1]);
        assert_ne!(preview.definitions[6].identity, original[2]);
        assert_eq!(
            session
                .definitions
                .iter()
                .map(|definition| definition.identity)
                .collect::<alloc::vec::Vec<_>>(),
            original
        );
    }

    #[test]
    fn current_dependents_and_submission_expectations_are_reported_without_publishing() {
        let limits = crate::Limits {
            bytes: 65_536,
            nodes: 16_384,
            depth: 64,
            work: 2_000_000,
        };
        let mut session = Session::new();
        for source in [
            b"def addone [ 1 + ]".as_slice(),
            b"def twice [ addone addone ]",
            b"def four [ twice twice ]",
            b"def unrelated [ 10 + ]",
        ] {
            let prepared = session.prepare(source, &[], limits).unwrap();
            session.commit(prepared).unwrap();
        }
        let original: alloc::vec::Vec<_> = session
            .definitions
            .iter()
            .map(|definition| definition.identity)
            .collect();
        assert_eq!(session.current_dependent_names("addone"), ["twice", "four"]);
        assert_eq!(session.current_dependent_names("twice"), ["four"]);
        assert!(session.current_dependent_names("four").is_empty());
        assert!(session.current_dependent_names("unrelated").is_empty());
        assert!(session.current_dependent_names("absent").is_empty());

        // A caller submission into the current namespace yields its expectation
        // descriptor; a definition preparation defers its submission and has none.
        let radix = alloc::vec![noble_kernel::types::Ty::I64];
        let one = session.prepare(b"1 +", &radix, limits).unwrap();
        let two = session.prepare(b"2 +", &radix, limits).unwrap();
        let expectation = DefinitionInterface {
            stack_in: 1,
            stack_out: 1,
            effects: 0,
        };
        assert_eq!(one.submission_interface(), Some(expectation));
        assert_eq!(two.submission_interface(), Some(expectation));

        let replacement = session.prepare(b"def addone [ 2 + ]", &[], limits).unwrap();
        assert!(replacement.is_definition());
        assert_eq!(replacement.definition_name(), Some("addone"));
        assert_eq!(replacement.submission_interface(), None);
        assert!(replacement.definition_identity().is_some());
        let preview = session.preview_core_rebuild(&replacement).unwrap();
        // A preview establishes a prospective successor only: the installed
        // namespace, its identities and its dependents are unchanged.
        assert_eq!(
            session
                .definitions
                .iter()
                .map(|definition| definition.identity)
                .collect::<alloc::vec::Vec<_>>(),
            original
        );
        assert_eq!(session.current_dependent_names("addone"), ["twice", "four"]);
        assert_eq!(preview.current_dependent_names("addone"), ["twice", "four"]);
    }

    #[test]
    fn ordered_interfaces_are_reported_for_installed_and_candidate_definitions() {
        let limits = crate::Limits {
            bytes: 65_536,
            nodes: 16_384,
            depth: 64,
            work: 2_000_000,
        };
        let mut session = Session::new();
        for source in [
            b"def addone [ 1 + ]".as_slice(),
            b"def twice [ addone addone ]",
        ] {
            let prepared = session.prepare(source, &[], limits).unwrap();
            session.commit(prepared).unwrap();
        }
        // One ordered I64 in and one ordered I64 out, with the unconstrained
        // stack tail the checker inferred left explicit rather than padded.
        let installed = session
            .installed_ordered_interface("addone", limits)
            .unwrap()
            .unwrap();
        assert_eq!(installed.stack_in, ["I64"]);
        assert_eq!(installed.stack_in_tail, super::InterfaceTail::Unconstrained);
        assert_eq!(installed.stack_out, ["I64"]);
        assert_eq!(
            installed.stack_out_tail,
            super::InterfaceTail::Unconstrained
        );
        assert!(installed.effects.is_empty());
        assert!(!installed.unresolved_effects);
        let installed_identity = session
            .installed_definition_identity("addone")
            .expect("the current definition has an installed identity");

        // A same-name candidate that consumes one more operand has a different
        // ordered interface, so the prose "same arities" claim is not assumed.
        let reordered = session
            .prepare(b"def addone [ 1 + 1 ]", &[], limits)
            .unwrap();
        let prospective = session
            .candidate_ordered_interface(&reordered)
            .unwrap()
            .unwrap();
        assert_eq!(prospective.stack_in, ["I64"]);
        assert_eq!(prospective.stack_out, ["I64", "I64"]);
        assert_ne!(
            installed.stack_out, prospective.stack_out,
            "an extra result is an interface change"
        );
        // The candidate's identity is prospective and distinct from the
        // installed definition's; neither accessor publishes anything.
        assert_ne!(reordered.definition_identity(), Some(installed_identity));

        assert!(session
            .installed_ordered_interface("absent", limits)
            .unwrap()
            .is_none());
        assert_eq!(session.installed_definition_identity("absent"), None);
        let submission = session
            .prepare(b"1 +", &[noble_kernel::types::Ty::I64], limits)
            .unwrap();
        assert!(session
            .candidate_ordered_interface(&submission)
            .unwrap()
            .is_none());
    }

    #[test]
    fn preview_classifies_interface_changes() {
        let limits = live_limits();
        let mut session = Session::new();
        let prepared = session.prepare(b"def addone [ 1 + ]", &[], limits).unwrap();
        session.commit(prepared).unwrap();
        let replacement = session
            .prepare(b"def addone [ 1 + 1 ]", &[], limits)
            .unwrap();
        let failure = session.preview_core_rebuild(&replacement).unwrap_err();
        assert_eq!(failure.kind(), PreviewRefusal::InterfaceChanged);
        assert_eq!(failure.kind().as_str(), "interface-change");
        assert!(
            failure.diagnostic().message.contains("stack interface"),
            "{failure:?}"
        );
    }

    #[test]
    fn preview_classifies_effect_growth_before_interface_drift() {
        let limits = live_limits();
        let mut live = Session::new_live("evolve");
        let prepared = live
            .prepare(b"def evolve [ self.generation drop 1 + ]", &[], limits)
            .unwrap();
        live.commit(prepared).unwrap();
        let growth = live
            .prepare(
                b"def evolve [ dup 18 - quote [ + ] compose self.generation \
                  swap self.propose drop 1 + ]",
                &[],
                limits,
            )
            .unwrap();
        let failure = live.preview_core_rebuild(&growth).unwrap_err();
        assert_eq!(failure.kind(), PreviewRefusal::EffectGrowth);
        assert!(
            failure
                .diagnostic()
                .message
                .contains("increases effects beyond the installed bound"),
            "{failure:?}"
        );
    }

    #[test]
    fn preview_refuses_a_transitive_self_rebind() {
        let limits = live_limits();
        let mut rebound = Session::new();
        for source in [b"def f [ 1 + ]".as_slice(), b"def g [ f ]", b"def h [ g ]"] {
            let prepared = rebound.prepare(source, &[], limits).unwrap();
            rebound.commit(prepared).unwrap();
        }
        let replacement = rebound.prepare(b"def f [ h ]", &[], limits).unwrap();
        let failure = rebound.preview_core_rebuild(&replacement).unwrap_err();
        assert_eq!(failure.kind(), PreviewRefusal::SelfRebind);
        assert!(
            failure.diagnostic().message.contains("previous identity"),
            "{failure:?}"
        );
    }

    #[test]
    fn preview_refuses_dependent_rebuild_drift() {
        let limits = live_limits();
        let mut drift = Session::new();
        for source in [
            b"def x [ 1 + ]".as_slice(),
            b"def h [ x ]",
            b"def c [ h ]",
            b"def h [ 2 + ]",
        ] {
            let prepared = drift.prepare(source, &[], limits).unwrap();
            drift.commit(prepared).unwrap();
        }
        let replacement = drift.prepare(b"def x [ 3 + ]", &[], limits).unwrap();
        let failure = drift.preview_core_rebuild(&replacement).unwrap_err();
        assert_eq!(failure.kind(), PreviewRefusal::DependentRebuild);
        assert!(
            failure
                .diagnostic()
                .message
                .contains("resolved call target"),
            "{failure:?}"
        );
    }

    #[test]
    fn preview_classifies_exhausted_replay_as_work_exhausted() {
        let limits = live_limits();
        let mut budget = Session::new();
        let prepared = budget.prepare(b"def addone [ 1 + ]", &[], limits).unwrap();
        budget.commit(prepared).unwrap();
        let dependent = alloc::format!(
            "def twice [ addone addone \"{}\" drop ]",
            "x".repeat(40_000)
        );
        let prepared = budget.prepare(dependent.as_bytes(), &[], limits).unwrap();
        budget.commit(prepared).unwrap();
        let replacement = budget.prepare(b"def addone [ 2 + ]", &[], limits).unwrap();
        let failure = budget.preview_core_rebuild(&replacement).unwrap_err();
        assert_eq!(failure.kind(), PreviewRefusal::WorkExhausted);
        assert_eq!(
            failure.diagnostic().kind,
            crate::DiagnosticKind::Exhausted,
            "{failure:?}"
        );
        assert!(
            failure.diagnostic().message.contains("byte limit"),
            "{failure:?}"
        );
    }

    #[test]
    fn preview_rejects_a_preparation_from_a_stale_namespace() {
        let limits = live_limits();
        let mut snapshot = Session::new();
        let prepared = snapshot
            .prepare(b"def addone [ 1 + ]", &[], limits)
            .unwrap();
        snapshot.commit(prepared).unwrap();
        let stale = snapshot
            .prepare(b"def addone [ 2 + ]", &[], limits)
            .unwrap();
        let prepared = snapshot.prepare(b"def other [ 5 + ]", &[], limits).unwrap();
        snapshot.commit(prepared).unwrap();
        let failure = snapshot.preview_core_rebuild(&stale).unwrap_err();
        assert_eq!(failure.kind(), PreviewRefusal::UnsupportedProfile);
    }

    #[test]
    fn a_dependency_through_a_shadowed_definition_is_still_a_dependent() {
        let limits = crate::Limits {
            bytes: 65_536,
            nodes: 16_384,
            depth: 64,
            work: 2_000_000,
        };
        let mut session = Session::new();
        for source in [
            b"def x [ 1 + ]".as_slice(),
            b"def h [ x ]",
            b"def c [ h ]",
            b"def h [ 2 + ]",
        ] {
            let prepared = session.prepare(source, &[], limits).unwrap();
            session.commit(prepared).unwrap();
        }
        // `c` is current and its immutable resolved call reaches the old `x`
        // through the shadowed historical `h`; `h` itself is not live.
        assert_eq!(session.current_dependent_names("x"), ["c"]);
        assert!(session.live_definition_has_dependents("x"));
        assert!(session.current_dependent_names("h").is_empty());
        // The successor cannot retarget `c`'s shadowed hop, so the batch is
        // refused instead of silently leaving `c` on the old `x`.
        let replacement = session.prepare(b"def x [ 3 + ]", &[], limits).unwrap();
        let failure = session.preview_core_rebuild(&replacement).unwrap_err();
        assert!(
            failure
                .diagnostic()
                .message
                .contains("unapproved resolved call target"),
            "{failure:?}"
        );
    }

    #[test]
    fn replay_refuses_drift_to_a_shadowed_unchanged_target() {
        let limits = crate::Limits {
            bytes: 65_536,
            nodes: 16_384,
            depth: 64,
            work: 2_000_000,
        };
        // Ordinary preparation can retain a historical target when another
        // name is shadowed. The live batch must not silently retarget it.
        for source in [
            b"def use [ f offset ]".as_slice(),
            b"def use [ [ f offset ] run ]",
        ] {
            let mut session = Session::new_live("f");
            for definition in [
                b"def f [ 1 + ]".as_slice(),
                b"def offset [ 10 + ]",
                source,
                b"def offset [ 100 + ]",
            ] {
                let prepared = session.prepare(definition, &[], limits).unwrap();
                session.commit(prepared).unwrap();
            }
            let replacement = session.prepare(b"def f [ 2 + ]", &[], limits).unwrap();
            let failure = session.preview_core_rebuild(&replacement).unwrap_err();
            assert!(
                failure
                    .diagnostic()
                    .message
                    .contains("unapproved resolved call target"),
                "{failure:?}",
            );
            assert_eq!(session.definitions.len(), 4);
        }
    }

    #[test]
    fn replacement_rejects_indirect_old_identity_even_in_quotation() {
        let limits = crate::Limits {
            bytes: 65_536,
            nodes: 16_384,
            depth: 64,
            work: 2_000_000,
        };
        let mut session = Session::new();
        for source in [b"def f [ 1 + ]".as_slice(), b"def g [ f ]", b"def h [ g ]"] {
            let prepared = session.prepare(source, &[], limits).unwrap();
            session.commit(prepared).unwrap();
        }
        for source in [b"def f [ h ]".as_slice(), b"def f [ [ h ] run ]"] {
            let replacement = session.prepare(source, &[], limits).unwrap();
            let failure = session.preview_core_rebuild(&replacement).unwrap_err();
            assert!(
                failure.diagnostic().message.contains("previous identity"),
                "{failure:?}",
            );
            assert_eq!(session.definitions.len(), 3);
        }
    }
}
