use std::collections::BTreeSet;

use super::super::*;
use super::bounded_sorted_issues;
use super::valid_reference;
use super::validate_world_replay_profile;

pub fn validate_world_replay_capsule(
    capsule: &WorldReplayCapsule,
    bounds: &WorldReplayBounds,
) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(bounds.max_diagnostics);
    if capsule.schema != WORLD_REPLAY_CAPSULE_SCHEMA {
        issues.push(WorldReplayIssue::InvalidSchema("capsule"));
    }
    if !valid_reference(&capsule.capsule_ref) {
        issues.push(WorldReplayIssue::InvalidReference("capsule-ref"));
    }
    if !valid_reference(&capsule.trace_ref) {
        issues.push(WorldReplayIssue::InvalidReference("capsule-trace-ref"));
    }
    issues.extend(validate_world_replay_profile(&capsule.profile));
    if capsule.non_claims != world_replay_non_claims() {
        issues.push(WorldReplayIssue::InvalidText("capsule-non-claims"));
    }
    if capsule.members.len() > bounds.max_members {
        issues.push(WorldReplayIssue::MemberLimitExceeded);
    }
    issues.extend(validate_members(capsule, bounds));
    match identify_world_replay_capsule(capsule) {
        Ok(identity) if identity != capsule.capsule_ref => {
            issues.push(WorldReplayIssue::CapsuleIdentityMismatch);
        }
        Err(issue) => issues.push(issue),
        Ok(_) => {}
    }
    bounded_sorted_issues(issues, bounds.max_diagnostics)
}

pub fn validate_world_replay_plan(plan: &WorldReplayPlan) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(MAX_WORLD_REPLAY_DIAGNOSTICS);
    if plan.schema != WORLD_REPLAY_PLAN_SCHEMA {
        issues.push(WorldReplayIssue::InvalidSchema("plan"));
    }
    if !valid_reference(&plan.plan_ref) {
        issues.push(WorldReplayIssue::InvalidReference("plan-ref"));
    }
    if !valid_reference(&plan.trace_ref) || !valid_reference(&plan.capsule_ref) {
        issues.push(WorldReplayIssue::InvalidReference("plan-input-ref"));
    }
    if !plan.current_admission_required {
        issues.push(WorldReplayIssue::InvalidText("current-admission-required"));
    }
    if plan.non_claims != world_replay_non_claims() {
        issues.push(WorldReplayIssue::InvalidText("plan-non-claims"));
    }
    match identify_world_replay_plan(plan) {
        Ok(identity) if identity != plan.plan_ref => issues.push(WorldReplayIssue::PlanIdentityMismatch),
        Err(issue) => issues.push(issue),
        Ok(_) => {}
    }
    bounded_sorted_issues(issues, MAX_WORLD_REPLAY_DIAGNOSTICS)
}

fn validate_members(capsule: &WorldReplayCapsule, bounds: &WorldReplayBounds) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(bounds.max_diagnostics);
    let mut prior_ref: Option<&str> = None;
    let mut seen = BTreeSet::new();
    let mut total_bytes = 0_u64;
    for member in &capsule.members {
        if let Some(prior_ref) = prior_ref
            && prior_ref > member.object_ref.as_str()
        {
            issues.push(WorldReplayIssue::NonCanonicalMemberOrder);
        }
        prior_ref = Some(&member.object_ref);
        if !seen.insert(member.object_ref.as_str()) {
            issues.push(WorldReplayIssue::DuplicateMember(member.object_ref.clone()));
        }
        issues.extend(validate_member(member, bounds));
        total_bytes = match total_bytes.checked_add(member.byte_length) {
            Some(value) => value,
            None => {
                issues.push(WorldReplayIssue::TotalByteLimitExceeded);
                bounds.max_total_bytes
            }
        };
    }
    if total_bytes > bounds.max_total_bytes {
        issues.push(WorldReplayIssue::TotalByteLimitExceeded);
    }
    issues
}

fn validate_member(member: &WorldReplayCapsuleMember, bounds: &WorldReplayBounds) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(MAX_WORLD_REPLAY_DIAGNOSTICS);
    if !valid_reference(&member.object_ref) {
        issues.push(WorldReplayIssue::InvalidReference("capsule-member-ref"));
    }
    if member.roles.is_empty() {
        issues.push(WorldReplayIssue::EmptyMemberRoles(member.object_ref.clone()));
    }
    if member.roles.len() > MAX_WORLD_REPLAY_ROLES_PER_MEMBER {
        issues.push(WorldReplayIssue::TooManyMemberRoles(member.object_ref.clone()));
    }
    let mut canonical_roles = member.roles.clone();
    canonical_roles.sort();
    canonical_roles.dedup();
    if canonical_roles != member.roles {
        issues.push(WorldReplayIssue::NonCanonicalMemberRoleOrder(member.object_ref.clone()));
    }
    if member.byte_length == 0 || member.byte_length > bounds.max_member_bytes {
        issues.push(WorldReplayIssue::MemberByteLimitExceeded(member.object_ref.clone()));
    }
    if let WorldReplayMemberProtection::Ciphertext { descriptor_ref } = &member.protection
        && !valid_reference(descriptor_ref)
    {
        issues.push(WorldReplayIssue::InvalidMemberProtection(member.object_ref.clone()));
    }
    issues.extend(validate_codec_role(member));
    issues
}

fn validate_codec_role(member: &WorldReplayCapsuleMember) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(MAX_WORLD_REPLAY_DIAGNOSTICS);
    if member.codec == WorldReplayMemberCodec::ContentManifestV1
        && !member.roles.contains(&WorldReplayCapsuleMemberRole::ContentManifest)
    {
        issues.push(WorldReplayIssue::InvalidText("content-manifest-codec-role"));
    }
    if member.codec == WorldReplayMemberCodec::SealedReproductionBundleV1
        && !member.roles.contains(&WorldReplayCapsuleMemberRole::SealedReproductionBundle)
    {
        issues.push(WorldReplayIssue::InvalidText("sealed-reproduction-codec-role"));
    }
    issues
}
