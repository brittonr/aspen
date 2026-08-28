use basalt::world_branch_authority;

use super::model::*;

mod conversion;

use conversion::*;

const PLAN_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-branch-authority.plan.v1";

struct BasaltEvaluation {
    decision: world_branch_authority::BranchAuthorityDecision,
    receipt: world_branch_authority::BranchAuthorityReceipt,
}

struct PlanIdentityInput<'a> {
    decision_ref: &'a str,
    capability_ref: &'a str,
    source_branch_ref: &'a str,
    destination_branch_ref: &'a str,
    mode: Option<WorldBranchMode>,
    obligations: &'a [WorldBranchObligation],
}

// r[impl molten.world_branch_authority.adoption]
// r[impl molten.world_branch_authority.derivation]
pub fn plan_world_branch_authority(
    policy_json: &str,
    facts: &WorldBranchAuthorityFacts,
    current: &CurrentAuthorityFacts,
) -> WorldBranchAuthorityPlan {
    if !valid_input(facts, current) {
        return denied_plan(facts, WorldBranchAuthorityDiagnostic::InvalidInput);
    }
    let evaluation = match evaluate_basalt_policy(policy_json, facts, current) {
        Ok(evaluation) => evaluation,
        Err(diagnostic) => return denied_plan(facts, diagnostic),
    };
    let decision = evaluation.decision;
    let mode = decision.mode().map(world_mode);
    let obligations = decision.obligations().iter().copied().map(world_obligation).collect::<Vec<_>>();
    let diagnostic = world_diagnostic(decision.diagnostic());
    let plan_ref = plan_identity(&PlanIdentityInput {
        decision_ref: evaluation.receipt.decision_ref.as_str(),
        capability_ref: facts.capability_ref.as_str(),
        source_branch_ref: facts.source_branch_ref.as_str(),
        destination_branch_ref: facts.destination_branch_ref.as_str(),
        mode,
        obligations: &obligations,
    });
    WorldBranchAuthorityPlan {
        schema: WORLD_BRANCH_AUTHORITY_PLAN_SCHEMA,
        plan_ref,
        allowed: decision.is_allowed(),
        policy_ref: decision.policy_ref().to_string(),
        request_ref: decision.request_ref().to_string(),
        authority_input_ref: decision.authority_input_ref().to_string(),
        capability_ref: facts.capability_ref.clone(),
        source_branch_ref: facts.source_branch_ref.clone(),
        destination_branch_ref: facts.destination_branch_ref.clone(),
        source_scope: facts.source_scope.clone(),
        destination_scope: facts.destination_scope.clone(),
        mode,
        obligations,
        diagnostic,
        non_claims: non_claims(),
    }
}

fn evaluate_basalt_policy(
    policy_json: &str,
    facts: &WorldBranchAuthorityFacts,
    current: &CurrentAuthorityFacts,
) -> Result<BasaltEvaluation, WorldBranchAuthorityDiagnostic> {
    let policy = world_branch_authority::parse_branch_authority_policy(policy_json)
        .map_err(|_| WorldBranchAuthorityDiagnostic::PolicyMalformed)?;
    let request = basalt_request(facts)?;
    let authority = basalt_currentness(current);
    let decision = world_branch_authority::evaluate_branch_authority(&policy, &request, &authority);
    let receipt = world_branch_authority::branch_authority_receipt(&decision)
        .map_err(|_| WorldBranchAuthorityDiagnostic::PolicyMalformed)?;
    Ok(BasaltEvaluation { decision, receipt })
}

fn basalt_request(
    facts: &WorldBranchAuthorityFacts,
) -> Result<world_branch_authority::BranchAuthorityRequest, WorldBranchAuthorityDiagnostic> {
    world_branch_authority::BranchAuthorityRequest::new(
        basalt_capability_kind(facts.capability_kind),
        basalt_action(facts.action),
        facts.source_branch_ref.clone(),
        facts.destination_branch_ref.clone(),
        facts.capability_ref.clone(),
        basalt_scope(&facts.source_scope)?,
        basalt_scope(&facts.destination_scope)?,
        facts.policy_generation,
        facts.mapping_lossless,
    )
    .map_err(|_| WorldBranchAuthorityDiagnostic::InvalidInput)
}

fn basalt_currentness(current: &CurrentAuthorityFacts) -> world_branch_authority::AuthorityCurrentness {
    world_branch_authority::AuthorityCurrentness {
        observation_ref: current.observation_ref.clone(),
        policy: current_fact(current.policy_current),
        capability: current_fact(current.capability_current),
        revocation: current_fact(current.revocation_current),
        replay: current_fact(current.replay_current),
        scope: current_fact(current.scope_current),
        ucan: if current.ucan_verified {
            world_branch_authority::UcanComposition::Verified
        } else {
            world_branch_authority::UcanComposition::Missing
        },
    }
}

pub fn deny_world_branch_authority_plan(
    mut plan: WorldBranchAuthorityPlan,
    diagnostic: WorldBranchAuthorityDiagnostic,
) -> WorldBranchAuthorityPlan {
    let obligations = vec![WorldBranchObligation::DenyActivation];
    plan.plan_ref = plan_identity(&PlanIdentityInput {
        decision_ref: plan.policy_ref.as_str(),
        capability_ref: plan.capability_ref.as_str(),
        source_branch_ref: plan.source_branch_ref.as_str(),
        destination_branch_ref: plan.destination_branch_ref.as_str(),
        mode: plan.mode,
        obligations: &obligations,
    });
    plan.allowed = false;
    plan.obligations = obligations;
    plan.diagnostic = diagnostic;
    plan
}

fn valid_input(facts: &WorldBranchAuthorityFacts, current: &CurrentAuthorityFacts) -> bool {
    facts.policy_generation > 0
        && facts.source_branch_ref != facts.destination_branch_ref
        && valid_content_ref(&facts.source_branch_ref)
        && valid_content_ref(&facts.destination_branch_ref)
        && valid_content_ref(&facts.capability_ref)
        && valid_content_ref(&current.observation_ref)
}

fn denied_plan(
    facts: &WorldBranchAuthorityFacts,
    diagnostic: WorldBranchAuthorityDiagnostic,
) -> WorldBranchAuthorityPlan {
    let obligations = vec![WorldBranchObligation::DenyActivation];
    let plan_ref = plan_identity(&PlanIdentityInput {
        decision_ref: facts.capability_ref.as_str(),
        capability_ref: facts.capability_ref.as_str(),
        source_branch_ref: facts.source_branch_ref.as_str(),
        destination_branch_ref: facts.destination_branch_ref.as_str(),
        mode: None,
        obligations: &obligations,
    });
    WorldBranchAuthorityPlan {
        schema: WORLD_BRANCH_AUTHORITY_PLAN_SCHEMA,
        plan_ref,
        allowed: false,
        policy_ref: String::new(),
        request_ref: String::new(),
        authority_input_ref: String::new(),
        capability_ref: facts.capability_ref.clone(),
        source_branch_ref: facts.source_branch_ref.clone(),
        destination_branch_ref: facts.destination_branch_ref.clone(),
        source_scope: facts.source_scope.clone(),
        destination_scope: facts.destination_scope.clone(),
        mode: None,
        obligations,
        diagnostic,
        non_claims: non_claims(),
    }
}

fn plan_identity(input: &PlanIdentityInput<'_>) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(PLAN_IDENTITY_DOMAIN);
    for value in [
        input.decision_ref,
        input.capability_ref,
        input.source_branch_ref,
        input.destination_branch_ref,
    ] {
        update_text(&mut hasher, value);
    }
    update_text(&mut hasher, input.mode.map_or("none", WorldBranchMode::as_str));
    for obligation in input.obligations {
        update_text(&mut hasher, obligation.as_str());
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn update_text(hasher: &mut blake3::Hasher, value: &str) {
    let length = match u64::try_from(value.len()) {
        Ok(length) => length,
        Err(_) => {
            let fallback = b"length-overflow";
            let Ok(fallback_length) = u64::try_from(fallback.len()) else {
                return;
            };
            hasher.update(&fallback_length.to_le_bytes());
            hasher.update(fallback);
            return;
        }
    };
    hasher.update(&length.to_le_bytes());
    hasher.update(value.as_bytes());
}

fn non_claims() -> Vec<String> {
    WORLD_BRANCH_AUTHORITY_NON_CLAIMS.iter().map(ToString::to_string).collect()
}
