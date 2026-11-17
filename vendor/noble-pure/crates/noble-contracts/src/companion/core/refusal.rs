const CODE_REFUSAL_UNKNOWNRULE: &str = "unknown-rule";
const CODE_REFUSAL_UNSUPPORTEDRULESET: &str = "unsupported-ruleset";
const CODE_REFUSAL_UNSUPPORTEDSEMANTICREVISION: &str = "unsupported-semantic-revision";
const CODE_REFUSAL_CYCLICDERIVATION: &str = "cyclic-derivation";
const CODE_REFUSAL_DUPLICATEPREMISE: &str = "duplicate-premise";
const CODE_REFUSAL_MISSINGPREMISE: &str = "missing-premise";
const CODE_REFUSAL_WRONGPREMISECLASS: &str = "wrong-premise-class";
const CODE_REFUSAL_MISMATCHEDSUBJECT: &str = "mismatched-subject";
const CODE_REFUSAL_MISMATCHEDCLAIM: &str = "mismatched-claim";
const CODE_REFUSAL_MISMATCHEDCONTEXT: &str = "mismatched-context";
const CODE_REFUSAL_STALECONTEXT: &str = "stale-context";
const CODE_REFUSAL_UNRESOLVEDIMPLICATION: &str = "unresolved-implication";
const CODE_REFUSAL_WRONGINSTANTIATION: &str = "wrong-instantiation";
const CODE_REFUSAL_UNSUPPORTEDEVIDENCECLASS: &str = "unsupported-evidence-class";
const CODE_REFUSAL_FORGEDSTATUS: &str = "forged-status";
const CODE_REFUSAL_UNKNOWNCONTRACT: &str = "unknown-contract";
const CODE_REFUSAL_UNKNOWNEVIDENCE: &str = "unknown-evidence";
const CODE_REFUSAL_UNSUPPORTEDGUARDTEMPLATE: &str = "unsupported-guard-template";
const CODE_REFUSAL_INELIGIBLEGHOST: &str = "ineligible-ghost";
const CODE_REFUSAL_LIVECAPABILITYINEVIDENCE: &str = "live-capability-in-evidence";
const CODE_REFUSAL_INVALIDEVIDENCEENCODING: &str = "invalid-evidence-encoding";
const CODE_REFUSAL_CORRESPONDENCEMISMATCH: &str = "correspondence-mismatch";
const CODE_REFUSAL_EXHAUSTEDREPLAY: &str = "exhausted-replay";
const CODE_REFUSAL_EXHAUSTEDREGISTRY: &str = "exhausted-registry";
const CODE_REFUSAL_DUPLICATEENTRY: &str = "duplicate-entry";
const CODE_REFUSAL_INTERNAL: &str = "internal";

impl crate::companion::Refusal {
    /// A stable snake_case token for reports. Reports MUST NOT depend on
    /// `Debug` formatting of this enum.
    pub fn code(self) -> &'static str {
        match self {
            Self::UnknownRule => CODE_REFUSAL_UNKNOWNRULE,
            Self::UnsupportedRuleset => CODE_REFUSAL_UNSUPPORTEDRULESET,
            Self::UnsupportedSemanticRevision => CODE_REFUSAL_UNSUPPORTEDSEMANTICREVISION,
            Self::CyclicDerivation => CODE_REFUSAL_CYCLICDERIVATION,
            Self::DuplicatePremise => CODE_REFUSAL_DUPLICATEPREMISE,
            Self::MissingPremise => CODE_REFUSAL_MISSINGPREMISE,
            Self::WrongPremiseClass => CODE_REFUSAL_WRONGPREMISECLASS,
            Self::MismatchedSubject => CODE_REFUSAL_MISMATCHEDSUBJECT,
            Self::MismatchedClaim => CODE_REFUSAL_MISMATCHEDCLAIM,
            Self::MismatchedContext => CODE_REFUSAL_MISMATCHEDCONTEXT,
            Self::StaleContext => CODE_REFUSAL_STALECONTEXT,
            Self::UnresolvedImplication => CODE_REFUSAL_UNRESOLVEDIMPLICATION,
            Self::WrongInstantiation => CODE_REFUSAL_WRONGINSTANTIATION,
            Self::UnsupportedEvidenceClass => CODE_REFUSAL_UNSUPPORTEDEVIDENCECLASS,
            Self::ForgedStatus => CODE_REFUSAL_FORGEDSTATUS,
            Self::UnknownContract => CODE_REFUSAL_UNKNOWNCONTRACT,
            Self::UnknownEvidence => CODE_REFUSAL_UNKNOWNEVIDENCE,
            Self::UnsupportedGuardTemplate => CODE_REFUSAL_UNSUPPORTEDGUARDTEMPLATE,
            Self::IneligibleGhost => CODE_REFUSAL_INELIGIBLEGHOST,
            Self::LiveCapabilityInEvidence => CODE_REFUSAL_LIVECAPABILITYINEVIDENCE,
            Self::InvalidEvidenceEncoding => CODE_REFUSAL_INVALIDEVIDENCEENCODING,
            Self::CorrespondenceMismatch => CODE_REFUSAL_CORRESPONDENCEMISMATCH,
            Self::ExhaustedReplay => CODE_REFUSAL_EXHAUSTEDREPLAY,
            Self::ExhaustedRegistry => CODE_REFUSAL_EXHAUSTEDREGISTRY,
            Self::DuplicateEntry => CODE_REFUSAL_DUPLICATEENTRY,
            Self::Internal => CODE_REFUSAL_INTERNAL,
        }
    }
}
