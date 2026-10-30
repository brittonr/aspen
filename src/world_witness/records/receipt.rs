const RECEIPT_RECORD: &str = "molten-world-witness-receipt-v1";
const NON_CLAIMS: &[&str] = &[
    "witness-does-not-authorize-branch-mutation",
    "no-world-commit-semantic-truth",
    "no-provider-honesty-or-global-consensus",
    "no-release-eligibility-or-external-effect-completion",
    "local-profile-requires-intact-local-currentness-state",
    "receipt-does-not-grant-future-authority",
];

macro_rules! decision {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }
            fn parse(text: &str) -> crate::error::Result<Self> {
                match text { $($text => Ok(Self::$variant)),+, _ => Err(super::wire::invalid("unknown witness decision")) }
            }
        }
    };
}

decision!(RoleDecision {
    Admitted => "admitted", Denied => "denied", NotObserved => "not-observed",
    Uncertain => "uncertain", Reserved => "reserved",
});
decision!(PersistenceDecision {
    NotSubmitted => "not-submitted", Committed => "committed", Uncertain => "uncertain",
});
decision!(ReconciliationDecision {
    NotRequired => "not-required", Finalize => "finalize", AlreadyComplete => "already-complete",
    Superseded => "superseded", Conflict => "conflict", Retryable => "retryable",
    Denied => "denied", ManualReview => "manual-review",
});

pub fn required_non_claims() -> Vec<String> {
    NON_CLAIMS.iter().map(|value| (*value).to_owned()).collect()
}

// r[impl molten.world_witness.receipts]
pub fn validate(receipt: &super::Receipt) -> crate::error::Result<()> {
    let claim = receipt.claim_ref.as_str();
    let authentication = receipt.authentication_ref.as_str();
    let authority = receipt.authority_ref.as_str();
    if receipt.operation_id.is_empty()
        || claim == authentication
        || claim == authority
        || authentication == authority
        || receipt.witness_grants_authority
        || receipt.non_claims != required_non_claims()
        || receipt.authentication == RoleDecision::Reserved
        || receipt.branch_authorization == RoleDecision::Reserved
    {
        return Err(super::wire::invalid("witness receipt authority overclaim or invalid role binding"));
    }
    if receipt.local_persistence == PersistenceDecision::Committed
        && (receipt.authentication != RoleDecision::Admitted
            || receipt.branch_authorization != RoleDecision::Admitted
            || !matches!(receipt.witness_currentness, RoleDecision::Admitted | RoleDecision::Reserved))
    {
        return Err(super::wire::invalid("committed receipt lacks independently admitted decision roles"));
    }
    if let Some(highest) = &receipt.highest
        && highest.subject.claim_ref != receipt.claim_ref
    {
        return Err(super::wire::invalid("receipt highest state names another claim"));
    }
    if receipt.profile == molten_core::world_witness::AssuranceProfile::IndependentWitnessV1
        && matches!(receipt.witness_currentness, RoleDecision::Admitted | RoleDecision::Reserved)
        && receipt
            .highest
            .as_ref()
            .is_none_or(|highest| !highest.checkpoint.signature_verified || highest.quorum.is_empty())
    {
        return Err(super::wire::invalid("independent currentness lacks admitted signed highest state"));
    }
    if receipt.profile == molten_core::world_witness::AssuranceProfile::LocalGenerationV1
        && (receipt.highest.is_some() || receipt.witness_currentness == RoleDecision::Reserved)
    {
        return Err(super::wire::invalid("local receipt overclaims independent witnessing"));
    }
    Ok(())
}

pub fn canonical(receipt: &super::Receipt) -> crate::error::Result<Vec<u8>> {
    validate(receipt)?;
    crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(RECEIPT_RECORD, vec![
        crate::preserves_rail::string(&receipt.operation_id),
        crate::preserves_rail::string(receipt.claim_ref.as_str()),
        crate::preserves_rail::string(receipt.authentication_ref.as_str()),
        crate::preserves_rail::string(receipt.authority_ref.as_str()),
        crate::preserves_rail::string(receipt.profile.as_str()),
        crate::preserves_rail::string(receipt.authentication.as_str()),
        crate::preserves_rail::string(receipt.branch_authorization.as_str()),
        crate::preserves_rail::string(receipt.witness_currentness.as_str()),
        crate::preserves_rail::string(receipt.local_persistence.as_str()),
        crate::preserves_rail::string(receipt.reconciliation.as_str()),
        match &receipt.highest {
            Some(highest) => crate::preserves_rail::record("some", vec![super::highest_value(highest)]),
            None => crate::preserves_rail::record("none", vec![]),
        },
        crate::preserves_rail::bool_value(receipt.witness_grants_authority),
        crate::preserves_rail::sequence(receipt.non_claims.iter().map(crate::preserves_rail::string).collect()),
    ]))
}

pub fn identity(receipt: &super::Receipt) -> crate::error::Result<String> {
    Ok(crate::preserves_rail::content_ref_from_bytes(&canonical(receipt)?))
}

pub fn parse(bytes: &[u8]) -> crate::error::Result<super::Receipt> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields = super::wire::fields(&decoded.value, RECEIPT_RECORD, 13)?;
    let profile = match super::wire::text(&fields[4])?.as_str() {
        "local-generation-v1" => molten_core::world_witness::AssuranceProfile::LocalGenerationV1,
        "independent-witness-v1" => molten_core::world_witness::AssuranceProfile::IndependentWitnessV1,
        _ => return Err(super::wire::invalid("unknown witness assurance profile")),
    };
    let highest = if super::wire::fields(&fields[10], "none", 0).is_ok() {
        None
    } else {
        Some(super::parse_highest_value(&super::wire::fields(&fields[10], "some", 1)?[0])?)
    };
    let receipt = super::Receipt {
        operation_id: super::wire::text(&fields[0])?,
        claim_ref: molten_core::world_head::WorldHeadClaimRef::new(super::wire::text(&fields[1])?)
            .map_err(|issue| super::wire::invalid(issue.to_string()))?,
        authentication_ref: molten_core::world_head::WorldHeadAuthenticationDecisionRef::new(super::wire::text(
            &fields[2],
        )?)
        .map_err(|issue| super::wire::invalid(issue.to_string()))?,
        authority_ref: molten_core::world_head::WorldHeadAuthorityRef::new(super::wire::text(&fields[3])?)
            .map_err(|issue| super::wire::invalid(issue.to_string()))?,
        profile,
        authentication: RoleDecision::parse(&super::wire::text(&fields[5])?)?,
        branch_authorization: RoleDecision::parse(&super::wire::text(&fields[6])?)?,
        witness_currentness: RoleDecision::parse(&super::wire::text(&fields[7])?)?,
        local_persistence: PersistenceDecision::parse(&super::wire::text(&fields[8])?)?,
        reconciliation: ReconciliationDecision::parse(&super::wire::text(&fields[9])?)?,
        highest,
        witness_grants_authority: super::wire::boolean(&fields[11])?,
        non_claims: super::wire::texts(&fields[12])?,
    };
    validate(&receipt)?;
    Ok(receipt)
}
