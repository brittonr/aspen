//! Deterministic companion admission and replay; guest cells carry no authority.

mod admit;
mod core;
pub(crate) mod digest;
mod guard;
mod registry;
mod release;
mod rules;
mod subject;

pub use admit::statement::interface_signature;
pub use admit::{
    Admission, AdmissionRequest, CheckObservation, ClaimTemplate, EvidenceOffer, EvidencePayload,
    Outcome,
};
pub use registry::{ContractId, EvidenceClass, EvidenceId};
pub use rules::{Derivation, Derived, RuleId, RULESET_V1};
pub use subject::{observe, CAPTURE_I64_EVENT};

/// Named endpoint identities for one observed program interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterfaceSignatures {
    pub input: u64,
    pub output: u64,
}

/// The versioned rule identifier carried by an untrusted derivation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncodedRule {
    pub ruleset: u32,
    pub code: u32,
}

/// Regenerate the statement digest from the retained contract. This is the
/// trusted source of statement identity; producer text never enters.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; statement_digest folds every retained descriptor component with length-bounded indexing; every prepared statement has a defined identity, with no invalid-value assertion precondition."
)]
pub fn statement_digest(prepared: &crate::Prepared) -> u64 {
    let mut fold = digest::Fold::new(digest::DOMAIN_STATEMENT);
    fold.absorb_bytes(prepared.name().as_bytes());
    admit::statement::fold_named(&mut fold, prepared.inputs());
    admit::statement::fold_named(&mut fold, prepared.outputs());
    admit::statement::fold_named(&mut fold, prepared.params());
    fold.absorb_count(prepared.definitions().len());
    let mut index = 0usize;
    while index < prepared.definitions().len() {
        let definition = &prepared.definitions()[index];
        fold.absorb_bytes(definition.name.as_bytes());
        admit::types::fold(&mut fold, &definition.ty);
        fold.absorb(u64::from(definition.body));
        index = index.saturating_add(1);
    }
    fold.absorb_count(prepared.expressions().len());
    index = 0;
    while index < prepared.expressions().len() {
        admit::types::fold(&mut fold, &prepared.expressions()[index].ty);
        fold.absorb(u64::from(prepared.expressions()[index].total));
        fold.absorb(u64::from(prepared.expressions()[index].uses_output));
        admit::statement::absorb_expr(&mut fold, &prepared.expressions()[index].kind);
        index = index.saturating_add(1);
    }
    fold.absorb(u64::from(prepared.requires()));
    fold.absorb(u64::from(prepared.ensures()));
    admit::statement::fold_subject(&mut fold, prepared.candidate());
    fold.finish()
}

/// The plain-compose observation relation: event concat with witness index
/// offsets, endpoint interface join, and effect union.
///
/// The intermediate implication premise is the joint endpoint interface. When
/// the left output interface does not join the right input interface the
/// premise is absent and composition is unresolved (VC-LIB-02): no certified
/// result is produced.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; compose_subject rejects endpoint mismatch, excessive event/capture counts and invalid slot offsets through Refusal before joining bounded observations; arbitrary subjects must reject rather than panic."
)]
pub fn compose_subject(left: &Subject, right: &Subject) -> Result<Subject, Refusal> {
    if left.output_signature != right.input_signature {
        return Err(Refusal::UnresolvedImplication);
    }
    if left.events.len().saturating_add(right.events.len()) > subject::OBSERVATION_CAP
        || left.captures.len().saturating_add(right.captures.len()) > subject::OBSERVATION_CAP
    {
        return Err(Refusal::ExhaustedReplay);
    }
    let captures = attempt!(subject::capture_bindings(left, right));
    let mut fold = digest::Fold::new(digest::DOMAIN_COMPOSE);
    fold.absorb(left.identity.0);
    fold.absorb(right.identity.0);
    fold.absorb(left.input_signature);
    fold.absorb(right.output_signature);
    // Effect union: the pure fragment carries no effects, so the joint
    // endpoint interface is the only latent dependency folded here.
    fold.absorb(left.output_signature);
    fold.absorb_count(captures.len());
    let mut at = 0usize;
    while at < captures.len() {
        fold.absorb(u64::from(captures[at].slot));
        fold.absorb(captures[at].value);
        at = at.saturating_add(1);
    }
    let mut events = alloc::vec::Vec::with_capacity(
        left.events
            .len()
            .saturating_add(right.events.len())
            .min(subject::OBSERVATION_CAP),
    );
    at = 0usize;
    while at < left.events.len() && events.len() < subject::OBSERVATION_CAP {
        events.push(left.events[at]);
        at = at.saturating_add(1);
    }
    at = 0usize;
    while at < right.events.len() && events.len() < subject::OBSERVATION_CAP {
        events.push(right.events[at]);
        at = at.saturating_add(1);
    }
    Ok(Subject {
        identity: SubjectDigest(fold.finish()),
        input_signature: left.input_signature,
        output_signature: right.output_signature,
        captures,
        events,
    })
}

/// Every way the deterministic core refuses to certify or admit.
///
/// The list is exhaustive and fail-closed: each variant names the precise
/// reason a decision was withheld. Variants that a given operation can never
/// produce are still present so a caller can handle them without a `_` arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[octet::sealed_enum]
pub enum Refusal {
    UnknownRule,
    UnsupportedRuleset,
    UnsupportedSemanticRevision,
    CyclicDerivation,
    DuplicatePremise,
    MissingPremise,
    WrongPremiseClass,
    MismatchedSubject,
    MismatchedClaim,
    MismatchedContext,
    StaleContext,
    UnresolvedImplication,
    WrongInstantiation,
    UnsupportedEvidenceClass,
    ForgedStatus,
    UnknownContract,
    UnknownEvidence,
    UnsupportedGuardTemplate,
    IneligibleGhost,
    LiveCapabilityInEvidence,
    InvalidEvidenceEncoding,
    CorrespondenceMismatch,
    ExhaustedReplay,
    ExhaustedRegistry,
    DuplicateEntry,
    Internal,
}

/// A bounded work allowance for one core operation. `charge` fails closed
/// with the operation's exhaustion refusal instead of running unbounded.
pub(crate) struct Budget {
    work: u32,
}

#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; charge mutates only one operation-owned work counter and cannot alter caller inputs or host state"
)]
impl Budget {
    pub(crate) fn new(limits: crate::Limits) -> Self {
        Self { work: limits.work }
    }

    pub(crate) fn charge(&mut self, amount: u32, exhausted: Refusal) -> Result<(), Refusal> {
        match self.work.checked_sub(amount) {
            Some(remaining) => {
                self.work = remaining;
                Ok(())
            }
            None => Err(exhausted),
        }
    }
}

/// The deterministic admission and derivation core.
pub struct Core {
    pub(crate) limits: crate::Limits,
    pub(crate) registry: registry::Registry,
    /// Consumer-selected policy revision. A change invalidates every retained
    /// applicability decision made under the previous revision.
    pub(crate) policy: u32,
    /// Monotone context revision, bumped whenever the policy changes.
    pub(crate) revision: u32,
    semantic_revision: u32,
    host_contract: u64,
    environment_fact: u64,
}

/// The finite v1 guard template set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[octet::sealed_enum]
pub enum GuardTemplate {
    /// `x < i64::MAX` (rejecting the maximum).
    LtI64Max,
    /// `x != i64::MIN` (rejecting the minimum).
    NeI64Min,
    /// `x = literal` for one compile-time literal.
    EqI64Literal(i64),
}

/// One captured runtime value bound into a subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureBinding {
    pub slot: u32,
    pub value: u64,
}

/// A subject identity digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubjectDigest(pub u64);

/// The observable identity of a program subject: what it is, and what it can
/// be composed with. It carries no capability and no live handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    pub identity: SubjectDigest,
    pub input_signature: u64,
    pub output_signature: u64,
    pub captures: alloc::vec::Vec<CaptureBinding>,
    /// The bounded observed recipe events this subject was folded from, so
    /// applicability can compare them against a contract's retained program
    /// without re-running the guest. Admission also retains the actual program
    /// events; an interface or digest by itself never identifies a body.
    pub events: alloc::vec::Vec<(u32, u64)>,
}

/// A released obligation: the exact contract, evidence, statement digest, and
/// subject under the releasing policy. This is inspection metadata only; it
/// carries no live capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Release {
    pub contract: crate::companion::ContractId,
    pub evidence: crate::companion::EvidenceId,
    pub statement: u64,
    pub subject: crate::companion::SubjectDigest,
    pub policy: u32,
    pub ruleset: u32,
}
