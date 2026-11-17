pub(crate) mod derive;
mod premises;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Derived {
    pub evidence: crate::companion::registry::EvidenceId,
    pub contract: crate::companion::ContractId,
    pub statement_digest: u64,
}

pub const RULESET_V1: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[octet::sealed_enum]
pub enum RuleId {
    AdmitLeanV1 = 1,
    ComposeV1 = 2,
    InstantiateV1 = 3,
    GuardV1 = 4,
    ProjectV1 = 5,
    InvokeV1 = 6,
}

impl RuleId {
    pub fn decode(
        encoded: crate::companion::EncodedRule,
    ) -> Result<RuleId, crate::companion::Refusal> {
        if encoded.ruleset != RULESET_V1 {
            return Err(crate::companion::Refusal::UnsupportedRuleset);
        }
        match encoded.code {
            1 => Ok(RuleId::AdmitLeanV1),
            2 => Ok(RuleId::ComposeV1),
            3 => Ok(RuleId::InstantiateV1),
            4 => Ok(RuleId::GuardV1),
            5 => Ok(RuleId::ProjectV1),
            6 => Ok(RuleId::InvokeV1),
            _ => Err(crate::companion::Refusal::UnknownRule),
        }
    }

    pub fn code(self) -> u32 {
        match self {
            RuleId::AdmitLeanV1 => 1,
            RuleId::ComposeV1 => 2,
            RuleId::InstantiateV1 => 3,
            RuleId::GuardV1 => 4,
            RuleId::ProjectV1 => 5,
            RuleId::InvokeV1 => 6,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derivation {
    pub rule: RuleId,
    pub contract: crate::companion::ContractId,
    pub statement: u64,
    pub subject: crate::companion::Subject,
    pub premises: alloc::vec::Vec<crate::companion::registry::EvidenceId>,
}

/// Recompute every conclusion. AdmitLeanV1 is evidence-preserving replay of
/// an already checked exact declaration, never a zero-premise admission rule.
#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; replay explicitly transitions the consumer-owned Core registry after validating immutable borrowed premises."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; replay charges the offered subject and checks retained context, the bounded premise closure, rule eligibility and the recomputed statement before insertion; malformed derivations remain typed refusals."
)]
pub(crate) fn replay(
    engine: &mut crate::companion::Core,
    derivation: &Derivation,
) -> Result<crate::companion::registry::EvidenceId, crate::companion::Refusal> {
    let mut budget = crate::companion::Budget::new(engine.limits);
    attempt!(premises::subject_work(&mut budget, &derivation.subject));
    let contract = attempt!(engine.registry.contract(derivation.contract));
    if crate::companion::registry::Registry::stale(
        engine,
        crate::companion::registry::ContextSnapshot {
            policy: contract.policy,
            revision: contract.revision,
        },
    ) {
        return Err(crate::companion::Refusal::StaleContext);
    }
    attempt!(premises::validate(
        engine,
        &derivation.premises,
        &mut budget,
        true
    ));
    let (template, statement) = attempt!(rule_checks(engine, derivation, &mut budget));
    if derivation.statement != statement {
        return Err(crate::companion::Refusal::MismatchedClaim);
    }
    let derived = attempt!(derive::insert(engine, derivation.clone(), template));
    Ok(derived.evidence)
}

#[expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; rule checks mutate only the replay operation's private budget while registry entries and the borrowed derivation remain immutable."
)]
#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; rule_checks rejects wrong arity, contract, evidence class and subject through typed refusals before transporting or recomputing a theorem. Derived RuleId/Subject equality and allocating composition/instantiation observation folds are runtime-only."
)]
fn rule_checks(
    engine: &crate::companion::Core,
    derivation: &Derivation,
    budget: &mut crate::companion::Budget,
) -> Result<(crate::companion::admit::ClaimTemplate, u64), crate::companion::Refusal> {
    let count = match derivation.rule {
        RuleId::ComposeV1 => 2,
        RuleId::AdmitLeanV1
        | RuleId::InstantiateV1
        | RuleId::GuardV1
        | RuleId::ProjectV1
        | RuleId::InvokeV1 => 1,
    };
    if derivation.premises.len() < count {
        return Err(crate::companion::Refusal::MissingPremise);
    }
    if derivation.premises.len() > count {
        return Err(crate::companion::Refusal::WrongPremiseClass);
    }
    let first = attempt!(crate::companion::registry::application::proved(
        engine,
        derivation.premises[0]
    ));
    if first.contract != derivation.contract {
        return Err(crate::companion::Refusal::MismatchedContext);
    }
    match derivation.rule {
        RuleId::ComposeV1 => {
            let template = attempt!(derive::composition(
                engine,
                derivation.premises[0],
                derivation.premises[1],
                &derivation.subject,
                budget
            ));
            Ok((template, template.digest()))
        }
        RuleId::InstantiateV1 => {
            if derivation.subject.captures.len() != 1 {
                return Err(crate::companion::Refusal::WrongInstantiation);
            }
            let capture = i64::from_ne_bytes(derivation.subject.captures[0].value.to_ne_bytes());
            let template = attempt!(derive::instantiation(
                engine,
                derivation.premises[0],
                capture,
                &derivation.subject
            ));
            Ok((template, template.digest()))
        }
        RuleId::AdmitLeanV1 | RuleId::GuardV1 | RuleId::ProjectV1 | RuleId::InvokeV1 => {
            if derivation.rule == RuleId::AdmitLeanV1
                && first.class != crate::companion::registry::EvidenceClass::LeanExact
            {
                return Err(crate::companion::Refusal::WrongPremiseClass);
            }
            if derivation.rule == RuleId::GuardV1 {
                let _ = attempt!(crate::companion::guard::templates(
                    engine,
                    derivation.contract
                ));
            }
            // These rules transport an existing theorem. No arbitrary new
            // subject, claim, context, or unconditional guard claim is minted.
            if derivation.subject != first.subject {
                return Err(crate::companion::Refusal::MismatchedSubject);
            }
            Ok((first.template, first.statement))
        }
    }
}
