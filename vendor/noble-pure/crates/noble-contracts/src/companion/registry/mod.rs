pub(crate) mod application;
mod entries;

/// The evidence class a claim was offered or derived under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[octet::sealed_enum]
pub enum EvidenceClass {
    /// An exact accepted Lean declaration for the exact statement.
    LeanExact,
    /// A checked refutation of the exact statement.
    LeanRefutation,
    /// A checked derivation replayed from accepted premises.
    Replay,
    /// An assumption: a premise, never by itself an acceptance.
    Assumption,
}

impl EvidenceClass {
    /// Decode a numeric class tag from the guest/host boundary. Unknown tags
    /// fail closed with `UnsupportedEvidenceClass`.
    pub fn decode(code: u32) -> Result<EvidenceClass, crate::companion::Refusal> {
        match code {
            1 => Ok(EvidenceClass::LeanExact),
            2 => Ok(EvidenceClass::LeanRefutation),
            3 => Ok(EvidenceClass::Replay),
            4 => Ok(EvidenceClass::Assumption),
            _ => Err(crate::companion::Refusal::UnsupportedEvidenceClass),
        }
    }

    pub fn code(&self) -> u32 {
        match self {
            EvidenceClass::LeanExact => 1,
            EvidenceClass::LeanRefutation => 2,
            EvidenceClass::Replay => 3,
            EvidenceClass::Assumption => 4,
        }
    }
}

/// An opaque contract descriptor index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContractId(pub u32);

/// An opaque evidence reference index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceId(pub u32);

/// The policy and revision retained by an authority-bearing registry entry.
#[derive(Clone, Copy)]
pub(crate) struct ContextSnapshot {
    pub(crate) policy: u32,
    pub(crate) revision: u32,
}

/// A retained contract descriptor. The statement digest is regenerated from
/// the retained `Prepared`; the guard templates are the finite prechecked set
/// whose predicate shape the precondition matches.
#[derive(Clone, Debug)]
pub(crate) struct ContractEntry {
    pub(crate) statement: u64,
    /// Full independently generated statement: digests are indices, not
    /// collision-resistant evidence of equality.
    pub(crate) exact_statement: alloc::string::String,
    pub(crate) claim: crate::companion::admit::ClaimTemplate,
    pub(crate) policy: u32,
    pub(crate) revision: u32,
    pub(crate) guards: alloc::vec::Vec<crate::companion::GuardTemplate>,
    /// The contract program's canonical op-event list, folded from the
    /// retained candidate. Applicability compares observed recipe events
    /// against this, so a same-interface different body is refused.
    pub(crate) program: alloc::vec::Vec<(u32, u64)>,
    pub(crate) input_signature: u64,
    pub(crate) output_signature: u64,
    pub(crate) input: alloc::vec::Vec<noble_kernel::types::Ty>,
    pub(crate) output: alloc::vec::Vec<noble_kernel::types::Ty>,
}

/// A retained evidence entry. Its cached observations are bounded.
#[derive(Clone, Debug)]
pub(crate) struct EvidenceEntry {
    pub(crate) contract: ContractId,
    pub(crate) statement: u64,
    pub(crate) template: crate::companion::admit::ClaimTemplate,
    pub(crate) subject: crate::companion::Subject,
    pub(crate) class: EvidenceClass,
    pub(crate) outcome: crate::companion::admit::Outcome,
    pub(crate) policy: u32,
    pub(crate) revision: u32,
    pub(crate) premises: alloc::vec::Vec<EvidenceId>,
    pub(crate) rule: Option<crate::companion::rules::RuleId>,
}

/// A cached bounded observation group, keyed by subject identity.
#[derive(Clone, Debug)]
pub(crate) struct Observation {
    pub(crate) subject: crate::companion::SubjectDigest,
    pub(crate) events: alloc::vec::Vec<(u32, u64)>,
}

/// The bounded registry.
pub(crate) struct Registry {
    pub(crate) contracts: alloc::vec::Vec<ContractEntry>,
    pub(crate) evidence: alloc::vec::Vec<EvidenceEntry>,
    pub(crate) observations: alloc::vec::Vec<Observation>,
    contract_cap: Option<usize>,
    evidence_cap: Option<usize>,
    observation_cap: Option<usize>,
}
