//! Opt-in, bounded module declarations. Neither preparation nor publication invokes guest code.
mod links;
mod parsing;
mod signatures;
mod state;
pub(super) mod types;
pub use state::{
    CheckedNominal, CheckedProgramInterface, CheckedProgramOwner, DeclaredTextSourceOwner,
    ModuleKind, ModulePrepared, ModuleSession, ProgramClosureError, RebuiltImportDefinition,
    SignedSourceOwner,
};

const SPAN: crate::Span = crate::Span { start: 0, end: 0 };
const MODULE_CAP: usize = 128;
const CONSTRUCTOR_CAP: usize = 256;

#[derive(Clone, Debug, PartialEq)]
pub struct BoundOperation {
    pub module_name: alloc::string::String,
    pub module_version: u32,
    pub operation: alloc::string::String,
    pub adapter_identity: alloc::string::String,
    pub adapter_slot: u32,
    pub input: alloc::vec::Vec<noble_kernel::types::Ty>,
    pub output: alloc::vec::Vec<noble_kernel::types::Ty>,
    pub effects: alloc::vec::Vec<noble_kernel::types::EffId>,
}

#[derive(Clone, Debug)]
#[expect(
    tigerstyle::interior_mutability_in_core,
    reason = "Owner: noble-contracts; K-CHECK-07 request budget: one request-local Rc<Cell<u32>> counter a host opts into with ModuleSession::share_request_work; it is single-threaded (Rc is !Send), never outlives the session and its staged snapshots, and every meter debits it so nested checking cannot restart the request's budget. Explicit threading would turn the &self closure-walk and origin-replay APIs into &mut ones."
)]
pub(super) struct Context {
    pub environment: noble_kernel::contracts::Env,
    pub words: alloc::vec::Vec<(alloc::string::String, crate::source::Target)>,
    /// Active outside alias owners copied from the checked module namespace.
    /// Module-owned definitions are tied to their own immutable alias snapshot.
    pub aliases: alloc::vec::Vec<(alloc::string::String, u64)>,
    pub owner: Option<u64>,
    /// Source selection only. Not an authority field in public kernel Env.
    pub signed_self: bool,
    /// Only the explicitly combined declared-text source ingress exposes text.byte.
    pub declared_text: bool,
    /// Remaining work of one host-selected request shared by every stage of
    /// this session's checking. Absent unless a host opts in.
    pub request_work: Option<alloc::rc::Rc<core::cell::Cell<u32>>>,
}

#[derive(Clone, Debug, PartialEq)]
struct Export {
    name: alloc::string::String,
    ty: Option<noble_kernel::types::Ty>,
    generic: Option<noble_kernel::types::NominalTypeId>,
    words: alloc::vec::Vec<(alloc::string::String, crate::source::Target)>,
}

#[derive(Clone, Debug)]
struct Module {
    name: alloc::string::String,
    version: u32,
    identity: u64,
    exports: alloc::vec::Vec<Export>,
    adapter_slot: Option<u32>,
    source: alloc::vec::Vec<u8>,
    definition_spans: alloc::vec::Vec<DefinitionSource>,
    /// Slots backed by at least one independently checked concrete owner
    /// specialization; another instance must still be checked independently.
    verified_signed_slots: alloc::vec::Vec<u32>,
    import_aliases: alloc::vec::Vec<ImportAlias>,
    contracts: alloc::vec::Vec<LogicalContract>,
    proof_exports: alloc::vec::Vec<alloc::string::String>,
    proofs: alloc::vec::Vec<PublishedProof>,
}

#[derive(Clone, Debug)]
struct LogicalContract {
    name: alloc::string::String,
    goal: crate::intrinsic::ContractGoal,
    exported: bool,
}

#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-contracts; the type name is the domain term at its use sites and renaming would churn the extracted subject"
)]
#[derive(Clone, Debug, PartialEq)]
struct DefinitionSource {
    name: alloc::string::String,
    span: crate::Span,
    body_span: crate::Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ImportAlias {
    alias: alloc::string::String,
    owner: u64,
}

#[derive(Clone, Debug)]
struct PublishedProof {
    reference: crate::intrinsic::ProofDependency,
    checked: crate::intrinsic::CheckedProof,
}

impl PartialEq for Module {
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the helper reads runtime tables or calls non-const helpers, so the const suggestion does not apply"
    )]
    fn eq(&self, other: &Self) -> bool {
        // A typed goal contains a kernel Submission, which deliberately has
        // no equality. Its derivation is fixed by the exact module source,
        // prior immutable module sources, namespace aliases and host bindings
        // compared by ModuleSession::current_snapshot. Compare published host
        // results here too, never merely the number of accepted proofs.
        self.name == other.name
            && self.version == other.version
            && self.identity == other.identity
            && self.exports == other.exports
            && self.adapter_slot == other.adapter_slot
            && self.source == other.source
            && self.definition_spans == other.definition_spans
            && self.verified_signed_slots == other.verified_signed_slots
            && self.import_aliases == other.import_aliases
            && self.proof_exports == other.proof_exports
            && self.contracts.len() == other.contracts.len()
            && self.proofs.len() == other.proofs.len()
            && self.proofs.iter().zip(&other.proofs).all(|(a, b)| {
                a.reference.name == b.reference.name
                    && a.reference.module == b.reference.module
                    && a.reference.version == b.reference.version
                    && a.reference.source == b.reference.source
                    && a.reference.model_revision == b.reference.model_revision
                    && a.reference.checker_revision == b.reference.checker_revision
                    && a.checked.name == b.checked.name
                    && a.checked.kind == b.checked.kind
                    && a.checked.claim == b.checked.claim
                    && a.checked.lean_term == b.checked.lean_term
                    && a.checked.model_revision == b.checked.model_revision
                    && a.checked.checker_revision == b.checked.checker_revision
            })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Alias {
    spelling: alloc::string::String,
    module: usize,
    pending: bool,
}

fn error(stage: crate::source::Stage, reason: &str) -> crate::source::Error {
    crate::source::Error::at(stage, crate::invalid(SPAN, reason))
}
/// A budget refusal: the host-selected bound decided it, not the input, so a
/// larger budget can admit the same bytes.
fn exhausted(stage: crate::source::Stage, reason: &str) -> crate::source::Error {
    crate::source::Error::at(stage, crate::source::exhausted(SPAN, reason))
}
fn diagnostic(stage: crate::source::Stage, problem: crate::Diagnostic) -> crate::source::Error {
    crate::source::Error::at(stage, problem)
}

#[derive(Clone, Copy)]
pub(crate) struct DefinitionOrigin<'a> {
    pub module_source: &'a [u8],
    pub module: &'a str,
    pub version: u32,
    pub definition: &'a str,
    pub ordinal: u32,
    pub span: crate::Span,
}

/// Reparse a claimed module's source, not its specialization slots, to bind
/// one lexical definition to its exact original bracketed body. This proves
/// internal consistency of the supplied bytes, not their historical
/// acceptance or authenticity as a host-owned artifact.
pub(crate) fn verify_definition_origin(
    origin: DefinitionOrigin<'_>,
    definition_source: &[u8],
    limits: crate::Limits,
) -> Result<crate::Span, crate::Diagnostic> {
    verify_definition_origin_in_request(origin, definition_source, limits, None)
}

#[expect(
    tigerstyle::compound_condition,
    reason = "Owner: noble-contracts; the guard is the admission rule verbatim; splitting it would obscure the predicate it states"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::interior_mutability_in_core,
    reason = "Owner: noble-contracts; K-CHECK-07 request budget: one request-local Rc<Cell<u32>> counter a host opts into with ModuleSession::share_request_work; it is single-threaded (Rc is !Send), never outlives the session and its staged snapshots, and every meter debits it so nested checking cannot restart the request's budget. Explicit threading would turn the &self closure-walk and origin-replay APIs into &mut ones."
)]
#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-contracts; the only mutable input is the K-CHECK-07 request budget, a request-local counter debited by the meters of this checking stage; no source, published compiler state or caller value is mutated."
)]
/// `verify_definition_origin` whose reparse draws from a shared request.
pub(super) fn verify_definition_origin_in_request(
    origin: DefinitionOrigin<'_>,
    definition_source: &[u8],
    limits: crate::Limits,
    request_work: Option<&alloc::rc::Rc<core::cell::Cell<u32>>>,
) -> Result<crate::Span, crate::Diagnostic> {
    let DefinitionOrigin {
        module_source,
        module,
        version,
        definition,
        ordinal,
        span: source_span,
    } = origin;
    let parsed = attempt!(parsing::parse(module_source, limits, request_work));
    let parsing::ParsedUnit::Module {
        name,
        version: parsed_version,
        members,
    } = parsed
    else {
        return Err(crate::invalid(
            source_span,
            "subject source is not a module",
        ));
    };
    if name != module || parsed_version != version {
        return Err(crate::invalid(
            source_span,
            "subject module identity differs from source",
        ));
    }
    let mut at = 0u32;
    let mut found = None;
    for member in members {
        if let parsing::Member::Definition {
            name,
            bytes,
            span,
            body_span,
        } = member
        {
            if name == definition {
                if found.is_some()
                    || at != ordinal
                    || span != source_span
                    || bytes.as_slice() != definition_source
                {
                    return Err(crate::invalid(
                        source_span,
                        "ambiguous or mismatched lexical subject definition",
                    ));
                }
                found = Some(body_span);
            }
            at = attempt!(at.checked_add(1).ok_or_else(|| {
                crate::source::exhausted(source_span, "module definition ordinal limit exceeded")
            }));
        }
    }
    found.ok_or_else(|| {
        crate::invalid(
            source_span,
            "subject definition is absent from module source",
        )
    })
}

#[expect(
    tigerstyle::compound_condition,
    reason = "Owner: noble-contracts; the guard is the admission rule verbatim; splitting it would obscure the predicate it states"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
/// Recover the complete, independently tokenized original caller word. A
/// caller-supplied span is accepted only if it covers that whole source node,
/// not a substring or another token inside the retained definition body.
pub(crate) fn verify_named_call_origin<'a>(
    caller: DefinitionOrigin<'a>,
    source_node: u32,
    source_span: crate::Span,
    limits: crate::Limits,
) -> Result<&'a str, crate::Diagnostic> {
    let DefinitionOrigin {
        module_source,
        definition: caller_definition,
        span: caller_span,
        ..
    } = caller;
    let start = usize::try_from(caller_span.start)
        .map_err(|_| crate::invalid(caller_span, "named caller span exceeds address space"))?;
    let end = usize::try_from(caller_span.end)
        .map_err(|_| crate::invalid(caller_span, "named caller span exceeds address space"))?;
    let source = module_source
        .get(start..end)
        .ok_or_else(|| crate::invalid(caller_span, "named caller lies outside retained module"))?;
    let body_span = attempt!(verify_definition_origin(caller, source, limits));
    let mut meter = crate::Meter::new(limits);
    let (tree, parsed_name) = attempt!(crate::source::parsing::parse_declared(source, &mut meter));
    if parsed_name.as_deref() != Some(caller_definition)
        || tree.body.iter().filter(|id| **id == source_node).count() != 1
    {
        return Err(crate::invalid(
            source_span,
            "named call is not a unique original body node",
        ));
    }
    let node = attempt!(tree.node(source_node));
    let crate::source::Kind::Word(spelling) = &node.kind else {
        return Err(crate::invalid(
            source_span,
            "named call is not an original source word",
        ));
    };
    let absolute_start = attempt!(caller_span
        .start
        .checked_add(node.span.start)
        .ok_or_else(|| crate::invalid(source_span, "named call source span overflows")));
    let absolute_end = attempt!(caller_span
        .start
        .checked_add(node.span.end)
        .ok_or_else(|| crate::invalid(source_span, "named call source span overflows")));
    let source_start = usize::try_from(absolute_start)
        .map_err(|_| crate::invalid(source_span, "named call source span exceeds address space"))?;
    let source_end = usize::try_from(absolute_end)
        .map_err(|_| crate::invalid(source_span, "named call source span exceeds address space"))?;
    let original_word = module_source
        .get(source_start..source_end)
        .ok_or_else(|| crate::invalid(source_span, "named call lies outside retained module"))?;
    if source_span
        != (crate::Span {
            start: absolute_start,
            end: absolute_end,
        })
        || absolute_start <= body_span.start
        || absolute_end >= body_span.end
        || original_word != spelling.as_slice()
    {
        return Err(crate::invalid(
            source_span,
            "named call differs from exact original word token",
        ));
    }
    core::str::from_utf8(original_word)
        .map_err(|_| crate::invalid(source_span, "named call word is not UTF-8"))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::too_many_parameters,
    reason = "Owner: noble-contracts; the parameter list is the internal call shape; bundling it would change the extracted subject"
)]
#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-contracts; the same-typed parameters are positional by contract and named at every call site"
)]
/// An imported named call may target only a unique, explicitly exported
/// lexical definition in the original owner module, not a similarly named
/// later source, hidden definition, or dynamically allocated specialization.
pub(crate) fn verify_named_export(
    module_source: &[u8],
    module: &str,
    version: u32,
    definition: &str,
    ordinal: u32,
    definition_span: crate::Span,
    limits: crate::Limits,
) -> Result<(), crate::Diagnostic> {
    let parsed = attempt!(parsing::parse(module_source, limits, None));
    let parsing::ParsedUnit::Module {
        name,
        version: parsed_version,
        members,
    } = parsed
    else {
        return Err(crate::invalid(
            definition_span,
            "named export source is not a module",
        ));
    };
    if name != module || parsed_version != version {
        return Err(crate::invalid(
            definition_span,
            "named export module differs from source",
        ));
    }
    let mut at = 0u32;
    let mut is_found = false;
    let mut is_exported = false;
    for member in members {
        match member {
            parsing::Member::Export(name) if name == definition => {
                if is_exported {
                    return Err(crate::invalid(
                        definition_span,
                        "duplicate named source export",
                    ));
                }
                is_exported = true;
            }
            parsing::Member::Definition { name, span, .. } => {
                if name == definition {
                    if is_found || at != ordinal || span != definition_span {
                        return Err(crate::invalid(
                            definition_span,
                            "named source export has a different lexical definition",
                        ));
                    }
                    is_found = true;
                }
                at = attempt!(at.checked_add(1).ok_or_else(|| {
                    crate::source::exhausted(
                        definition_span,
                        "module definition ordinal limit exceeded",
                    )
                }));
            }
            _ => (),
        }
    }
    if !is_found || !is_exported {
        return Err(crate::invalid(
            definition_span,
            "named source definition is absent or not is_exported",
        ));
    }
    Ok(())
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; slice starts_with/get and str indexing are non-const on both pinned compilers; reassess when those APIs become const."
)]
fn strip_qualified<'a>(text: &'a str, prefix: &[u8], separator: u8) -> Option<&'a str> {
    let bytes = text.as_bytes();
    let boundary = prefix.len();
    if bytes.starts_with(prefix) && bytes.get(boundary) == Some(&separator) {
        // The complete UTF-8 prefix and ASCII separator end at a character boundary.
        let next = boundary.saturating_add(1);
        Some(&text[next..])
    } else {
        None
    }
}

fn split_once_ascii(text: &str, separator: u8) -> Option<(&str, &str)> {
    let bytes = text.as_bytes();
    let mut at = 0usize;
    while at < bytes.len() && bytes[at] != separator {
        at += 1;
    }
    // Every caller supplies an ASCII separator, hence both cuts are boundaries.
    if at < bytes.len() {
        Some((&text[..at], &text[at.saturating_add(1)..]))
    } else {
        None
    }
}

/// A canonical decimal module version: digits only, no sign, no leading zero
/// and representable as `u32`. Module headers and every reference to a
/// versioned module share this one spelling, so `m@01` and `m@+1` never name
/// `m@1`.
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; str::parse and byte iteration are not const, so this cannot be a const fn."
)]
fn canonical_version(text: &str) -> Option<u32> {
    if text.is_empty() || text.as_bytes().first() == Some(&b'0') {
        return None;
    }
    if !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<u32>().ok()
}

const fn quotation_step(depth: u32, token: &crate::source::lexer::Token) -> Option<(u32, bool)> {
    if let crate::source::lexer::TokenKind::Open = &token.kind {
        let Some(next_depth) = depth.checked_add(1) else {
            return None;
        };
        return Some((next_depth, false));
    }
    if let crate::source::lexer::TokenKind::Close = &token.kind {
        let Some(next_depth) = depth.checked_sub(1) else {
            return None;
        };
        return Some((next_depth, next_depth == 0));
    }
    Some((depth, false))
}

fn append_decimal(mut out: alloc::string::String, value: u32) -> alloc::string::String {
    let mut divisor = 1u32;
    while value / divisor >= 10 {
        divisor *= 10;
    }
    let mut remaining = value;
    while divisor != 0 {
        let digit = remaining / divisor;
        remaining %= divisor;
        out.push(decimal_digit(digit));
        divisor /= 10;
    }
    out
}

const fn decimal_digit(digit: u32) -> char {
    match digit {
        0 => '0',
        1 => '1',
        2 => '2',
        3 => '3',
        4 => '4',
        5 => '5',
        6 => '6',
        7 => '7',
        8 => '8',
        _ => '9',
    }
}

struct Schema {
    name: alloc::string::String,
    kind: SchemaKind,
}
enum SchemaKind {
    Opaque {
        base: alloc::string::String,
        public: bool,
    },
    Variant {
        parameters: alloc::vec::Vec<alloc::string::String>,
        left: alloc::string::String,
        left_type: alloc::string::String,
        left_public: bool,
        right: alloc::string::String,
        right_type: alloc::string::String,
        right_public: bool,
    },
}

// A payload node is either accepted, rejected, or exposes checked children.
#[octet::sealed_enum]
enum Exposure<'a> {
    Public,
    Binary(&'a noble_kernel::types::Ty, &'a noble_kernel::types::Ty),
    Unary(&'a noble_kernel::types::Ty),
    Hidden,
}

/// Whether an external signature exposes only exported nominal interfaces.
/// An exported opaque descriptor hides its own checked representation; this
/// walk deliberately does not descend into that representation.
fn exposed_payload(
    root: &noble_kernel::types::Ty,
    environment: &noble_kernel::contracts::Env,
) -> bool {
    match exposed_node(root, environment) {
        Exposure::Public => true,
        Exposure::Hidden => false,
        Exposure::Binary(left, right) => {
            walk_exposure(alloc::vec::Vec::from([right, left]), environment)
        }
        Exposure::Unary(item) => walk_exposure(alloc::vec::Vec::from([item]), environment),
    }
}

struct ExposureState<'a> {
    pending: alloc::vec::Vec<&'a noble_kernel::types::Ty>,
    remaining: usize,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the bounded owned payload worklist rejects hidden or over-budget types without panicking on untrusted shapes; reassess fallible-check density."
)]
fn walk_exposure(
    pending: alloc::vec::Vec<&noble_kernel::types::Ty>,
    environment: &noble_kernel::contracts::Env,
) -> bool {
    let Some(remaining) = CONSTRUCTOR_CAP.checked_sub(1) else {
        return false;
    };
    let mut state = Some(ExposureState { pending, remaining });
    let mut has_work = true;
    while has_work {
        state = match state {
            Some(current) if current.pending.is_empty() => {
                has_work = false;
                Some(current)
            }
            Some(current) => exposure_step(current, environment),
            None => {
                has_work = false;
                None
            }
        };
    }
    state.is_some()
}

#[expect(
    tigerstyle::missing_const_fn,
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; Vec pop/push and owned state drop are non-const on pinned compilers; budget and pending-capacity failures reject without panic; reassess const APIs and fallible-check density."
)]
fn exposure_step<'a>(
    mut state: ExposureState<'a>,
    environment: &noble_kernel::contracts::Env,
) -> Option<ExposureState<'a>> {
    let Some(ty) = state.pending.pop() else {
        return Some(state);
    };
    if state.remaining == 0 {
        return None;
    }
    state.remaining -= 1;
    match exposed_node(ty, environment) {
        Exposure::Public => Some(state),
        Exposure::Hidden => None,
        Exposure::Binary(left, right) => {
            if CONSTRUCTOR_CAP < 2 || state.pending.len() > CONSTRUCTOR_CAP.saturating_sub(2) {
                return None;
            }
            state.pending.push(right);
            state.pending.push(left);
            Some(state)
        }
        Exposure::Unary(item) => {
            if state.pending.len() == CONSTRUCTOR_CAP {
                return None;
            }
            state.pending.push(item);
            Some(state)
        }
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn exposed_node<'a>(
    ty: &'a noble_kernel::types::Ty,
    environment: &noble_kernel::contracts::Env,
) -> Exposure<'a> {
    if scalar_payload(ty) {
        return Exposure::Public;
    }
    if let noble_kernel::types::Ty::Pair(left, right) | noble_kernel::types::Ty::Sum(left, right) =
        ty
    {
        return Exposure::Binary(left, right);
    }
    if let noble_kernel::types::Ty::List(item) = ty {
        return Exposure::Unary(item);
    }
    if let noble_kernel::types::Ty::Nominal(id, _) = ty {
        return nominal_exposure(*id, environment);
    }
    if let noble_kernel::types::Ty::GenericNominal(id, arguments, _) = ty {
        if environment
            .generic_variant(*id)
            .is_some_and(|declaration| declaration.exported)
            && environment.valid_generic_instance(ty)
        {
            return Exposure::Binary(&arguments[0], &arguments[1]);
        }
    }
    Exposure::Hidden
}

fn nominal_exposure<'a>(
    id: noble_kernel::types::NominalTypeId,
    environment: &noble_kernel::contracts::Env,
) -> Exposure<'a> {
    if environment
        .nominal(id)
        .is_some_and(|declaration| declaration.exported)
    {
        Exposure::Public
    } else {
        Exposure::Hidden
    }
}

const fn scalar_payload(ty: &noble_kernel::types::Ty) -> bool {
    matches!(
        ty,
        noble_kernel::types::Ty::Unit
            | noble_kernel::types::Ty::Bool
            | noble_kernel::types::Ty::I64
            | noble_kernel::types::Ty::Text
    )
}
