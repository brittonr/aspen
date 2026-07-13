use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;
use crate::fabric::valid_blake3_ref;

// r[impl molten.fabric_observability.integrity_readonly]
pub fn evaluate_integrity_plan(
    profile: &ObservationProfile,
    plan: &IntegrityPlan,
    observations: &[ScanObservation],
    completion: &ScanCompletion,
) -> IntegrityResult {
    let mut issues = validate_observation_profile(profile);
    validate_integrity_plan(profile, plan, &mut issues);
    validate_completion(plan, observations, completion, &mut issues);
    let target_index = plan.targets.iter().map(|target| (target.item_ref.clone(), target)).collect::<BTreeMap<_, _>>();
    let observation_index = index_observations(profile, plan, observations, &mut issues);
    let mut findings = Vec::new();
    for target in &plan.targets {
        match observation_index.get(&target.item_ref) {
            Some(observation) => classify_observation(plan, target, observation, &mut findings, &mut issues),
            None => {
                let finding_index = findings.len();
                push_finding(
                    plan,
                    &mut findings,
                    IntegrityFinding {
                        schema: INTEGRITY_FINDING_SCHEMA.to_string(),
                        finding_id: finding_id(finding_index, FindingClass::Missing),
                        item_ref: Some(target.item_ref.clone()),
                        class: FindingClass::Missing,
                        expected_ref: target.expected_content_ref.clone(),
                        observed_ref: None,
                        recommendation: RepairRecommendation::RestoreCandidate,
                        grants_mutation_authority: false,
                    },
                    &mut issues,
                );
            }
        }
    }
    for observation in observations {
        if !target_index.contains_key(&observation.item_ref) {
            let finding_index = findings.len();
            push_finding(
                plan,
                &mut findings,
                IntegrityFinding {
                    schema: INTEGRITY_FINDING_SCHEMA.to_string(),
                    finding_id: finding_id(finding_index, FindingClass::Unexpected),
                    item_ref: Some(observation.item_ref.clone()),
                    class: FindingClass::Unexpected,
                    expected_ref: None,
                    observed_ref: observation.observed_content_ref.clone(),
                    recommendation: RepairRecommendation::OperatorReview,
                    grants_mutation_authority: false,
                },
                &mut issues,
            );
        }
    }
    add_completion_findings(plan, completion, &mut findings, &mut issues);
    let complete = completion.exhausted
        && !completion.cancelled
        && !completion.unavailable
        && completion.scanned_items == observations.len()
        && completion.scanned_items == completion.declared_items
        && completion.declared_items == plan.targets.len()
        && plan.targets.iter().all(|target| observation_index.contains_key(&target.item_ref));
    let decision = integrity_decision(plan, completion, &findings, &issues, complete);
    IntegrityResult {
        plan_ref: plan.plan_ref.clone(),
        decision,
        scanned_items: completion.scanned_items,
        declared_items: completion.declared_items,
        findings,
        complete,
        mutation_performed: false,
        issues,
    }
}

// r[impl molten.fabric_observability.integrity_readonly]
pub fn admit_integrity_mutation(
    finding_ref: &str,
    authority: Option<&IntegrityMutationAuthority>,
) -> AuthorityDecision {
    let Some(authority) = authority else {
        return AuthorityDecision::Deny;
    };
    if authority.schema != INTEGRITY_MUTATION_AUTHORITY_SCHEMA
        || !valid_blake3_ref(&authority.authority_ref)
        || !valid_blake3_ref(&authority.policy_ref)
        || !valid_blake3_ref(finding_ref)
        || !authority.finding_refs.iter().any(|candidate| candidate == finding_ref)
        || authority.finding_refs.iter().any(|candidate| !valid_blake3_ref(candidate))
    {
        return AuthorityDecision::Deny;
    }
    AuthorityDecision::Admit
}

fn validate_integrity_plan(profile: &ObservationProfile, plan: &IntegrityPlan, issues: &mut Vec<ObservabilityIssue>) {
    if plan.schema != INTEGRITY_PLAN_SCHEMA {
        issues.push(ObservabilityIssue::SchemaMismatch("integrity-plan-schema"));
    }
    for (field, reference) in [
        ("integrity-plan-ref", plan.plan_ref.as_str()),
        ("integrity-profile-ref", plan.profile_ref.as_str()),
        ("integrity-scope-ref", plan.scope_ref.as_str()),
        ("integrity-resource-ref", plan.resource_ref.as_str()),
    ] {
        if !valid_blake3_ref(reference) {
            issues.push(ObservabilityIssue::MalformedRef(field));
        }
    }
    if plan.profile_ref != profile.profile_ref {
        issues.push(ObservabilityIssue::ProfileMismatch);
    }
    if plan.generation == 0 {
        issues.push(ObservabilityIssue::ZeroBound("integrity-generation"));
    }
    if !plan.read_only {
        issues.push(ObservabilityIssue::PlanNotReadOnly);
        issues.push(ObservabilityIssue::MutationWithoutAuthority);
    }
    if plan.max_items == 0 || plan.max_items > profile.bounds.max_scan_items {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("integrity-max-items"));
    }
    if plan.max_findings == 0 || plan.max_findings > profile.bounds.max_findings {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("integrity-max-findings"));
    }
    if plan.targets.len() > plan.max_items || plan.targets.len() > profile.bounds.max_scan_items {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("integrity-targets"));
    }
    let mut target_refs = BTreeSet::new();
    for target in &plan.targets {
        if !valid_blake3_ref(&target.item_ref) {
            issues.push(ObservabilityIssue::MalformedRef("integrity-target-ref"));
        }
        if !target_refs.insert(target.item_ref.clone()) {
            issues.push(ObservabilityIssue::DuplicateValue("integrity-target-ref"));
        }
        if let Some(expected_ref) = target.expected_content_ref.as_deref()
            && !valid_blake3_ref(expected_ref)
        {
            issues.push(ObservabilityIssue::MalformedRef("integrity-expected-content-ref"));
        }
    }
    validate_ref_list("integrity-policy-ref", &plan.policy_refs, issues);
    validate_ref_list("integrity-evidence-ref", &plan.evidence_refs, issues);
    validate_plan_non_claims(&plan.non_claims, issues);
}

fn validate_completion(
    plan: &IntegrityPlan,
    observations: &[ScanObservation],
    completion: &ScanCompletion,
    issues: &mut Vec<ObservabilityIssue>,
) {
    if completion.scanned_items != observations.len() || completion.declared_items != plan.targets.len() {
        issues.push(ObservabilityIssue::PartialScan);
    }
    if completion.scanned_items > plan.max_items || completion.declared_items > plan.max_items {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("integrity-completion-items"));
    }
    if completion.cancelled {
        issues.push(ObservabilityIssue::Cancelled);
    }
    if completion.unavailable {
        issues.push(ObservabilityIssue::ObservationUnavailable("integrity-scan".to_string()));
    }
}

fn index_observations<'a>(
    profile: &ObservationProfile,
    plan: &IntegrityPlan,
    observations: &'a [ScanObservation],
    issues: &mut Vec<ObservabilityIssue>,
) -> BTreeMap<String, &'a ScanObservation> {
    let mut index = BTreeMap::new();
    if observations.len() > plan.max_items || observations.len() > profile.bounds.max_scan_items {
        issues.push(ObservabilityIssue::CollectionLimitExceeded("scan-observations"));
    }
    for observation in observations {
        validate_scan_observation_into(plan, observation, issues);
        if index.insert(observation.item_ref.clone(), observation).is_some() {
            issues.push(ObservabilityIssue::DuplicateValue("scan-observation-item"));
        }
    }
    index
}

pub fn validate_scan_observation(plan: &IntegrityPlan, observation: &ScanObservation) -> Vec<ObservabilityIssue> {
    let mut issues = Vec::new();
    validate_scan_observation_into(plan, observation, &mut issues);
    issues
}

fn validate_scan_observation_into(
    plan: &IntegrityPlan,
    observation: &ScanObservation,
    issues: &mut Vec<ObservabilityIssue>,
) {
    if observation.schema != SCAN_OBSERVATION_SCHEMA {
        issues.push(ObservabilityIssue::SchemaMismatch("scan-observation-schema"));
    }
    for (field, reference) in [
        ("scan-observation-ref", observation.observation_ref.as_str()),
        ("scan-plan-ref", observation.plan_ref.as_str()),
        ("scan-item-ref", observation.item_ref.as_str()),
    ] {
        if !valid_blake3_ref(reference) {
            issues.push(ObservabilityIssue::MalformedRef(field));
        }
    }
    if observation.plan_ref != plan.plan_ref {
        issues.push(ObservabilityIssue::ScanPlanMismatch);
    }
    if let Some(content_ref) = observation.observed_content_ref.as_deref()
        && !valid_blake3_ref(content_ref)
    {
        issues.push(ObservabilityIssue::MalformedRef("scan-observed-content-ref"));
    }
    validate_ref_list("scan-evidence-ref", &observation.evidence_refs, issues);
}

fn classify_observation(
    plan: &IntegrityPlan,
    target: &IntegrityTarget,
    observation: &ScanObservation,
    findings: &mut Vec<IntegrityFinding>,
    issues: &mut Vec<ObservabilityIssue>,
) {
    if target.kind != observation.kind {
        push_finding(
            plan,
            findings,
            finding(
                findings.len(),
                target,
                observation,
                FindingClass::Corrupt,
                RepairRecommendation::QuarantineCandidate,
            ),
            issues,
        );
        return;
    }
    match observation.status {
        ScanItemStatus::Present => classify_present(plan, target, observation, findings, issues),
        ScanItemStatus::Missing => push_classified(
            plan,
            target,
            observation,
            FindingClass::Missing,
            RepairRecommendation::RestoreCandidate,
            findings,
            issues,
        ),
        ScanItemStatus::Corrupt => push_classified(
            plan,
            target,
            observation,
            FindingClass::Corrupt,
            RepairRecommendation::QuarantineCandidate,
            findings,
            issues,
        ),
        ScanItemStatus::PermissionDenied => push_classified(
            plan,
            target,
            observation,
            FindingClass::PermissionDenied,
            RepairRecommendation::OperatorReview,
            findings,
            issues,
        ),
        ScanItemStatus::Unsupported => push_classified(
            plan,
            target,
            observation,
            FindingClass::Unsupported,
            RepairRecommendation::OperatorReview,
            findings,
            issues,
        ),
        ScanItemStatus::OverBound => push_classified(
            plan,
            target,
            observation,
            FindingClass::OverBound,
            RepairRecommendation::OperatorReview,
            findings,
            issues,
        ),
        ScanItemStatus::Cancelled => push_classified(
            plan,
            target,
            observation,
            FindingClass::Cancelled,
            RepairRecommendation::VerifyAgain,
            findings,
            issues,
        ),
    }
}

fn classify_present(
    plan: &IntegrityPlan,
    target: &IntegrityTarget,
    observation: &ScanObservation,
    findings: &mut Vec<IntegrityFinding>,
    issues: &mut Vec<ObservabilityIssue>,
) {
    if let Some(expected_ref) = target.expected_content_ref.as_deref()
        && observation.observed_content_ref.as_deref() != Some(expected_ref)
    {
        push_classified(
            plan,
            target,
            observation,
            FindingClass::ContentMismatch,
            RepairRecommendation::QuarantineCandidate,
            findings,
            issues,
        );
    }
    if let Some(expected_length) = target.expected_length
        && observation.observed_length != Some(expected_length)
    {
        push_classified(
            plan,
            target,
            observation,
            FindingClass::LengthMismatch,
            RepairRecommendation::RepairCandidate,
            findings,
            issues,
        );
    }
}

fn push_classified(
    plan: &IntegrityPlan,
    target: &IntegrityTarget,
    observation: &ScanObservation,
    class: FindingClass,
    recommendation: RepairRecommendation,
    findings: &mut Vec<IntegrityFinding>,
    issues: &mut Vec<ObservabilityIssue>,
) {
    push_finding(plan, findings, finding(findings.len(), target, observation, class, recommendation), issues);
}

fn finding(
    index: usize,
    target: &IntegrityTarget,
    observation: &ScanObservation,
    class: FindingClass,
    recommendation: RepairRecommendation,
) -> IntegrityFinding {
    IntegrityFinding {
        schema: INTEGRITY_FINDING_SCHEMA.to_string(),
        finding_id: finding_id(index, class),
        item_ref: Some(target.item_ref.clone()),
        class,
        expected_ref: target.expected_content_ref.clone(),
        observed_ref: observation.observed_content_ref.clone(),
        recommendation,
        grants_mutation_authority: false,
    }
}

fn add_completion_findings(
    plan: &IntegrityPlan,
    completion: &ScanCompletion,
    findings: &mut Vec<IntegrityFinding>,
    issues: &mut Vec<ObservabilityIssue>,
) {
    let completion_class = if completion.cancelled {
        Some((FindingClass::Cancelled, RepairRecommendation::VerifyAgain))
    } else if completion.unavailable {
        Some((FindingClass::Unavailable, RepairRecommendation::OperatorReview))
    } else if !completion.exhausted || completion.scanned_items != completion.declared_items {
        Some((FindingClass::PartialScan, RepairRecommendation::VerifyAgain))
    } else {
        None
    };
    if let Some((class, recommendation)) = completion_class {
        push_finding(
            plan,
            findings,
            IntegrityFinding {
                schema: INTEGRITY_FINDING_SCHEMA.to_string(),
                finding_id: finding_id(findings.len(), class),
                item_ref: None,
                class,
                expected_ref: None,
                observed_ref: None,
                recommendation,
                grants_mutation_authority: false,
            },
            issues,
        );
    }
}

fn push_finding(
    plan: &IntegrityPlan,
    findings: &mut Vec<IntegrityFinding>,
    finding: IntegrityFinding,
    issues: &mut Vec<ObservabilityIssue>,
) {
    if findings.len() >= plan.max_findings {
        if !issues.contains(&ObservabilityIssue::FindingLimitExceeded) {
            issues.push(ObservabilityIssue::FindingLimitExceeded);
        }
        return;
    }
    findings.push(finding);
}

fn integrity_decision(
    plan: &IntegrityPlan,
    completion: &ScanCompletion,
    findings: &[IntegrityFinding],
    issues: &[ObservabilityIssue],
    complete: bool,
) -> IntegrityDecision {
    if issues.iter().any(|issue| {
        matches!(
            issue,
            ObservabilityIssue::PlanNotReadOnly
                | ObservabilityIssue::MutationWithoutAuthority
                | ObservabilityIssue::ProfileMismatch
                | ObservabilityIssue::SchemaMismatch(_)
                | ObservabilityIssue::MalformedRef(_)
        )
    }) {
        return IntegrityDecision::Deny;
    }
    if completion.cancelled {
        return IntegrityDecision::Cancelled;
    }
    if completion.unavailable || issues.contains(&ObservabilityIssue::FindingLimitExceeded) {
        return IntegrityDecision::Unavailable;
    }
    if plan.require_complete && !complete {
        return IntegrityDecision::Partial;
    }
    if findings.is_empty() {
        IntegrityDecision::Pass
    } else {
        IntegrityDecision::Fail
    }
}

fn finding_id(index: usize, class: FindingClass) -> String {
    format!("finding-{index}-{}", class.as_str())
}

pub fn validate_integrity_result(profile: &ObservationProfile, result: &IntegrityResult) -> Vec<ObservabilityIssue> {
    let mut issues = validate_observation_profile(profile);
    if !valid_blake3_ref(&result.plan_ref) {
        issues.push(ObservabilityIssue::MalformedRef("integrity-result-plan-ref"));
    }
    if result.findings.len() > profile.bounds.max_findings {
        issues.push(ObservabilityIssue::FindingLimitExceeded);
    }
    if result.mutation_performed {
        issues.push(ObservabilityIssue::MutationWithoutAuthority);
    }
    if result.complete && result.scanned_items != result.declared_items {
        issues.push(ObservabilityIssue::PartialScan);
    }
    if result.decision == IntegrityDecision::Pass && (!result.complete || !result.findings.is_empty()) {
        issues.push(ObservabilityIssue::PartialScan);
    }
    for finding in &result.findings {
        if finding.schema != INTEGRITY_FINDING_SCHEMA {
            issues.push(ObservabilityIssue::SchemaMismatch("integrity-finding-schema"));
        }
        if finding.grants_mutation_authority {
            issues.push(ObservabilityIssue::MutationWithoutAuthority);
        }
        if let Some(item_ref) = finding.item_ref.as_deref()
            && !valid_blake3_ref(item_ref)
        {
            issues.push(ObservabilityIssue::MalformedRef("integrity-finding-item-ref"));
        }
    }
    issues
}

fn validate_ref_list(field: &'static str, refs: &[String], issues: &mut Vec<ObservabilityIssue>) {
    if refs.len() > MAX_OBSERVATION_REFS {
        issues.push(ObservabilityIssue::CollectionLimitExceeded(field));
    }
    if refs.windows(ADJACENT_PAIR_WIDTH).any(|pair| pair[0] >= pair[1]) {
        issues.push(ObservabilityIssue::DuplicateValue(field));
    }
    for reference in refs {
        if !valid_blake3_ref(reference) {
            issues.push(ObservabilityIssue::MalformedRef(field));
        }
    }
}

fn validate_plan_non_claims(claims: &[ObservabilityNonClaim], issues: &mut Vec<ObservabilityIssue>) {
    let supplied = claims.iter().copied().collect::<BTreeSet<_>>();
    for required in REQUIRED_OBSERVABILITY_NON_CLAIMS {
        if !supplied.contains(&required) {
            issues.push(ObservabilityIssue::MissingNonClaim(required.as_str()));
        }
    }
}
