//! Checked external-proof admission and its inert data model.

pub(crate) mod accept;
pub(crate) mod observation;
pub(crate) mod program;
pub(crate) mod recognize;
pub(crate) mod statement;
pub(crate) mod types;

const CODE_CLAIMTEMPLATE_INCREMENTBY: &str = "increment-by";
const CODE_CLAIMTEMPLATE_INCREMENTBYCAPTURE: &str = "increment-by-capture";
const CODE_CLAIMTEMPLATE_GUARDCORRESPONDENCE: &str = "guard-correspondence";
const CODE_CLAIMTEMPLATE_ADMITTED: &str = "admitted";
const CODE_OUTCOME_PROVED: &str = "proved";
const CODE_OUTCOME_DISPROVED: &str = "disproved";
const CODE_OUTCOME_UNKNOWN: &str = "unknown";
const CODE_OUTCOME_TIMEOUT: &str = "timeout";
const CODE_OUTCOME_UNSUPPORTED: &str = "unsupported";
const CODE_OUTCOME_ERROR: &str = "error";
const CODE_OUTCOME_NOTRUN: &str = "not-run";

/// A bounded claim template. Rule conclusions are recomputed in this algebra,
/// never supplied by a producer: an arbitrary MC1 statement admits as
/// `Admitted(digest)` and composes only when template-representable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[octet::sealed_enum]
pub enum ClaimTemplate {
    /// `output = wrap64(input + k)`, tail unchanged, precondition true.
    IncrementBy(i64),
    /// `forall n. output = wrap64(input + n)` for a returned program family.
    IncrementByCapture,
    /// A prechecked template-to-precondition guard correspondence.
    GuardCorrespondence(crate::companion::GuardTemplate),
    /// An exact Lean-checked statement bound by digest; composable only when
    /// also template-representable.
    Admitted(u64),
}

impl ClaimTemplate {
    /// Canonical digest of the template. For `Admitted` this is the retained
    /// statement digest itself.
    pub fn digest(self) -> u64 {
        match self {
            ClaimTemplate::Admitted(digest) => digest,
            ClaimTemplate::IncrementBy(k) => {
                let mut fold =
                    crate::companion::digest::Fold::new(crate::companion::digest::DOMAIN_TEMPLATE);
                fold.absorb(1);
                fold.absorb(u64::from_ne_bytes(k.to_ne_bytes()));
                fold.finish()
            }
            ClaimTemplate::IncrementByCapture => {
                let mut fold =
                    crate::companion::digest::Fold::new(crate::companion::digest::DOMAIN_TEMPLATE);
                fold.absorb(2);
                fold.finish()
            }
            ClaimTemplate::GuardCorrespondence(template) => {
                let mut fold =
                    crate::companion::digest::Fold::new(crate::companion::digest::DOMAIN_TEMPLATE);
                fold.absorb(3);
                fold.absorb(template.digest());
                fold.finish()
            }
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            ClaimTemplate::IncrementBy(_) => CODE_CLAIMTEMPLATE_INCREMENTBY,
            ClaimTemplate::IncrementByCapture => CODE_CLAIMTEMPLATE_INCREMENTBYCAPTURE,
            ClaimTemplate::GuardCorrespondence(_) => CODE_CLAIMTEMPLATE_GUARDCORRESPONDENCE,
            ClaimTemplate::Admitted(_) => CODE_CLAIMTEMPLATE_ADMITTED,
        }
    }
}

/// The seven claim outcomes (VC-TOOL-02).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[octet::sealed_enum]
pub enum Outcome {
    Proved,
    Disproved,
    Unknown,
    Timeout,
    Unsupported,
    Error,
    NotRun,
}

impl Outcome {
    pub fn code(self) -> &'static str {
        match self {
            Outcome::Proved => CODE_OUTCOME_PROVED,
            Outcome::Disproved => CODE_OUTCOME_DISPROVED,
            Outcome::Unknown => CODE_OUTCOME_UNKNOWN,
            Outcome::Timeout => CODE_OUTCOME_TIMEOUT,
            Outcome::Unsupported => CODE_OUTCOME_UNSUPPORTED,
            Outcome::Error => CODE_OUTCOME_ERROR,
            Outcome::NotRun => CODE_OUTCOME_NOTRUN,
        }
    }
}

/// An offered piece of evidence. It carries no acceptable-status flag: the
/// core decides, so a serialized `certified: true` can never forge status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceOffer {
    pub class: crate::companion::registry::EvidenceClass,
    pub refutation: bool,
    pub payload: EvidencePayload,
}

/// Structured ingress preserves capability kinds instead of erasing them into
/// inert proof bytes. Neither handle grants admission authority.
#[derive(Clone, Debug, PartialEq, Eq)]
#[octet::sealed_enum]
pub enum EvidencePayload {
    Declaration(alloc::vec::Vec<u8>),
    Resource(u64),
    ServiceCapability(u64),
}

/// Opaque request created only after deterministic ingress checks. Its exact
/// retained proposition, offered source and context remain bound while the
/// trusted host runs the independent checker outside this core.
pub struct AdmissionRequest<'a> {
    expected: &'a crate::Prepared,
    offer: EvidenceOffer,
    contract: crate::companion::ContractId,
    statement: u64,
    policy: u32,
    revision: u32,
}

/// Immutable, complete normalized observation from the trusted host checker.
///
/// Soundness and authenticity of this observation are explicit premises:
/// successful classes require the actual independent Lean consumer to check
/// the declaration/type/axioms for these exact inputs. Source headers, process
/// exit, guest data or cached status are not observations of that check.
/// This is host-only API data, never a guest value or an EvidenceOffer field;
/// it does not claim unforgeability against arbitrary trusted host Rust code.
pub struct CheckObservation {
    checked_statement: alloc::string::String,
    checked_source: alloc::vec::Vec<u8>,
    result: Result<crate::companion::EvidenceClass, crate::companion::Refusal>,
}

/// The result of an admission decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Admission {
    pub outcome: Outcome,
    pub contract: Option<crate::companion::ContractId>,
    pub evidence: Option<crate::companion::EvidenceId>,
    pub statement_digest: u64,
    pub refusals: alloc::vec::Vec<crate::companion::Refusal>,
}

#[expect(
    clippy::vec_init_then_push,
    reason = "Owner: noble-maintainers; explicit owned push avoids the pinned Aeneas erased-region failure in vec! array conversion; reassess when the translator supports it."
)]
fn refused(
    outcome: Outcome,
    contract: Option<crate::companion::ContractId>,
    statement_digest: u64,
    refusal: crate::companion::Refusal,
) -> Admission {
    let mut refusals = alloc::vec::Vec::with_capacity(1);
    refusals.push(refusal);
    Admission {
        outcome,
        contract,
        evidence: None,
        statement_digest,
        refusals,
    }
}

/// Ghost parameters must remain logical data (VC-INPUT-04): a non-data ghost
/// type would let a resource hide in erased data.
pub(crate) fn check_ghost_eligibility(
    prepared: &crate::Prepared,
) -> Result<(), crate::companion::Refusal> {
    let mut index = 0usize;
    while index < prepared.params().len() {
        if !prepared.params()[index].ty.is_data() {
            return Err(crate::companion::Refusal::IneligibleGhost);
        }
        index = index.saturating_add(1);
    }
    Ok(())
}

/// Reject capability-bearing input before retention or proof checking.
pub(crate) fn check_evidence_purity(
    offer: &EvidenceOffer,
) -> Result<(), crate::companion::Refusal> {
    match &offer.payload {
        EvidencePayload::Declaration(bytes) => match core::str::from_utf8(bytes) {
            Ok(_) => Ok(()),
            Err(_) => Err(crate::companion::Refusal::InvalidEvidenceEncoding),
        },
        EvidencePayload::Resource(_) | EvidencePayload::ServiceCapability(_) => {
            Err(crate::companion::Refusal::LiveCapabilityInEvidence)
        }
    }
}
