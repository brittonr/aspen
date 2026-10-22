use super::*;

fn identity(label: &str) -> ReplayTraceMemberIdentity {
    ReplayTraceMemberIdentity {
        content_ref: digest(label),
        byte_length: 64,
    }
}

fn sharing(declared: &str) -> ReplayMemberSharingInput {
    ReplayMemberSharingInput {
        declared: identity(declared),
        source: identity(declared),
        is_source_immutable: true,
        is_sharing_admitted: true,
        destination_readback: Some(identity(declared)),
    }
}

// r[verify molten.world_replay_boundary.trace_identity]
#[test]
fn a_trace_member_is_identified_by_content_never_by_location() {
    let identified = ReplayTraceMemberEvidence {
        content_ref: Some(digest("member")),
        byte_length: Some(64),
        locator: ReplayMemberLocator::Path("traces/member".to_string()),
    };
    assert_eq!(identify_replay_trace_member(&identified), Ok(identity("member")));
    for locator in [
        ReplayMemberLocator::Path("traces/member".to_string()),
        ReplayMemberLocator::SharedInode { device: 1, inode: 7 },
        ReplayMemberLocator::None,
    ] {
        let located = ReplayTraceMemberEvidence {
            content_ref: None,
            byte_length: None,
            locator,
        };
        assert!(identify_replay_trace_member(&located).is_err(), "a locator alone is unidentified");
    }
}

// r[verify molten.world_replay_boundary.trace_identity]
#[test]
fn a_shared_copy_stands_only_after_identity_immutability_policy_and_readback() {
    assert_eq!(plan_replay_member_sharing(&sharing("member")), Ok(ReplayMemberSharing::RetainShared));
    let mut mutable = sharing("member");
    mutable.is_source_immutable = false;
    assert_eq!(plan_replay_member_sharing(&mutable), Ok(ReplayMemberSharing::CopyRequired));
    let mut unadmitted = sharing("member");
    unadmitted.is_sharing_admitted = false;
    assert_eq!(plan_replay_member_sharing(&unadmitted), Ok(ReplayMemberSharing::CopyRequired));
    let mut unread = sharing("member");
    unread.destination_readback = None;
    assert_eq!(plan_replay_member_sharing(&unread), Ok(ReplayMemberSharing::CopyRequired));
    let mut diverged = sharing("member");
    diverged.destination_readback = Some(identity("other"));
    assert!(plan_replay_member_sharing(&diverged).is_err());
    let mut substituted = sharing("member");
    substituted.source = identity("other");
    assert!(plan_replay_member_sharing(&substituted).is_err());
}

fn summary() -> ReplayBoundarySummary {
    let request = valid_request();
    let admission = admit(&request).expect("admission");
    summarize_replay_boundary(&admission, &request.boundary, &request.trace.profile, 0).expect("summary")
}

// r[verify molten.world_replay_boundary.claims]
#[test]
fn a_summary_claims_only_logical_replay_of_its_profile_with_every_non_claim() {
    let stated = summary();
    assert_eq!(stated.claims, vec![ReplayBoundaryClaim::LogicalReplayForProfile]);
    assert_eq!(stated.redaction, ReplayRedaction::SensitiveProtected);
    assert_eq!(stated.sealed_effects, 1);
    assert_eq!(validate_replay_boundary_claims(&stated), Vec::new());
}

// r[verify molten.world_replay_boundary.claims]
// r[verify molten.world_replay_boundary.verification]
#[test]
fn overclaims_dropped_non_claims_and_replay_after_a_repeated_effect_are_rejected() {
    let mut overclaim = summary();
    overclaim.claims.push(ReplayBoundaryClaim::ExternalEffectCompletion);
    overclaim.claims.push(ReplayBoundaryClaim::ReleaseEligibility);
    assert_eq!(validate_replay_boundary_claims(&overclaim), vec![
        ReplayBoundaryBlocker::Overclaim("external-effect-completion"),
        ReplayBoundaryBlocker::Overclaim("release-eligibility"),
    ]);
    let mut dropped = summary();
    dropped.non_claims.pop();
    assert_eq!(validate_replay_boundary_claims(&dropped), vec![ReplayBoundaryBlocker::InvalidRecord("non-claims")]);
    let mut repeated = summary();
    repeated.repeated_effect_attempts = 1;
    assert_eq!(validate_replay_boundary_claims(&repeated), vec![ReplayBoundaryBlocker::Overclaim(
        "replay-after-repeated-effect"
    )]);
}
