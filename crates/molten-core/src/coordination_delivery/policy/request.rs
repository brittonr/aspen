use super::*;

pub(super) fn validate_required_refs(
    request: &DeliveryRequest,
    manifest: &DeliveryManifest,
    policy: &DeliveryPolicy,
    issues: &mut Vec<DeliveryIssue>,
) {
    if request.authority_refs.is_empty() || request.authority_refs.iter().any(|value| !valid_delivery_ref(value)) {
        issues.push(DeliveryIssue::MissingAuthority);
    }
    if !request.policy_refs.contains(&manifest.policy_ref)
        || request.policy_refs.iter().any(|value| !valid_delivery_ref(value))
        || manifest.policy_ref != identify_delivery_policy(policy)
    {
        issues.push(DeliveryIssue::MissingPolicy);
    }
    if request.resource_refs.is_empty() || request.resource_refs.iter().any(|value| !valid_delivery_ref(value)) {
        issues.push(DeliveryIssue::MissingResource);
    }
    if request.evidence_refs.is_empty() || request.evidence_refs.iter().any(|value| !valid_delivery_ref(value)) {
        issues.push(DeliveryIssue::MissingEvidence);
    }
}

pub(super) fn validate_operation(
    operation: &DeliveryOperation,
    state: &DeliveryState,
    policy: &DeliveryPolicy,
    issues: &mut Vec<DeliveryIssue>,
) {
    match operation {
        DeliveryOperation::Enqueue {
            item_ref,
            content_ref,
            metadata_ref,
            metadata_bytes,
        } => {
            if !valid_delivery_ref(item_ref) || !valid_delivery_ref(content_ref) || !valid_delivery_ref(metadata_ref) {
                issues.push(DeliveryIssue::InvalidReference);
            }
            if *metadata_bytes > policy.metadata_byte_limit {
                issues.push(DeliveryIssue::MetadataLimitExceeded);
            }
        }
        DeliveryOperation::Claim => {}
        DeliveryOperation::Acknowledge { token }
        | DeliveryOperation::ExtendLease { token }
        | DeliveryOperation::ExpireLease { token }
        | DeliveryOperation::NegativeAcknowledge { token, .. } => {
            if token.queue_id != state.queue_id
                || token.policy_ref != state.policy_ref
                || token.token_ref != identify_delivery_token(token)
            {
                issues.push(DeliveryIssue::TokenMismatch);
            }
        }
        DeliveryOperation::Redrive { item_ref } => {
            if !valid_delivery_ref(item_ref) {
                issues.push(DeliveryIssue::InvalidReference);
            }
        }
        DeliveryOperation::CleanupDeadLetter { through_tick } => {
            if *through_tick > MAX_DELIVERY_TICKS {
                issues.push(DeliveryIssue::LogicalTimeRequired);
            }
        }
    }
    if let DeliveryOperation::NegativeAcknowledge { failure_class, .. } = operation
        && !valid_failure_class(failure_class)
    {
        issues.push(DeliveryIssue::FailureClassUnsupported);
    }
}
