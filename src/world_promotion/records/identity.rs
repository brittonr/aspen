use super::*;

pub const WORLD_PROMOTION_IDENTITY_SCHEMA: &str = "molten.world-promotion-identity.v1";
const IDENTITY_RECORD: &str = "world-promotion-identity";
const IDENTITY_FIELD_COUNT: u64 = 6;

// r[impl molten.promotion_identity.roles]
pub fn canonical_promotion_identity(identity: &WorldPromotionIdentity) -> Result<CanonicalWorldPromotionRecord> {
    canonical(crate::preserves_rail::record(IDENTITY_RECORD, vec![
        crate::preserves_rail::string(WORLD_PROMOTION_IDENTITY_SCHEMA),
        named("operation-ref", string(identity.operation_ref.as_str())),
        named("request-ref", string(identity.canonical_request_ref.as_str())),
        named("attempt-ref", string(identity.attempt_ref.as_str())),
        named("generation", number(identity.generation)),
        non_claims(),
    ]))
}

pub fn parse_promotion_identity(bytes: &[u8]) -> Result<WorldPromotionIdentity> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields = crate::preserves_rail::simple_record_fields(&decoded.value, IDENTITY_RECORD, IDENTITY_FIELD_COUNT)?;
    require_schema(&fields[0], WORLD_PROMOTION_IDENTITY_SCHEMA)?;
    let non_claim_field = named_value(&fields[5], "non-claims")?;
    let non_claim_values =
        crate::preserves_rail::required_sequence_field(&non_claim_field, "world promotion non-claims")?;
    let parsed_non_claims = non_claim_values
        .iter()
        .map(|value| crate::preserves_rail::required_string_field(value, "world promotion non-claim"))
        .collect::<Result<Vec<_>>>()?;
    require_non_claims(&parsed_non_claims)?;
    let identity = WorldPromotionIdentity {
        operation_ref: WorldPromotionOperationRef::new(content_ref(&fields[1], "operation-ref")?)
            .map_err(reference_error)?,
        canonical_request_ref: WorldPromotionRequestRef::new(content_ref(&fields[2], "request-ref")?)
            .map_err(reference_error)?,
        attempt_ref: WorldReleaseAttemptRef::new(content_ref(&fields[3], "attempt-ref")?).map_err(reference_error)?,
        generation: u64_field(&fields[4], "generation")?,
    };
    let canonical = canonical_promotion_identity(&identity)?;
    if canonical.bytes != decoded.canonical_bytes {
        return Err(Failure::invalid_harness("world promotion identity bytes are not canonical"));
    }
    Ok(identity)
}

/// Reads only the operation and plan identities from a stored promotion plan.
/// Used for records written before the identity role table existed.
pub fn parse_promotion_operation(bytes: &[u8]) -> Result<(WorldPromotionOperationRef, WorldPromotionPlanRef)> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields = crate::preserves_rail::simple_record_fields(&decoded.value, PROMOTION_PLAN_RECORD, 14)
        .or_else(|_| crate::preserves_rail::simple_record_fields(&decoded.value, PROMOTION_PLAN_RECORD, 11))?;
    let plan_ref = WorldPromotionPlanRef::new(content_ref(&fields[1], "plan-ref")?).map_err(reference_error)?;
    let operation_ref =
        WorldPromotionOperationRef::new(content_ref(&fields[2], "operation-ref")?).map_err(reference_error)?;
    Ok((operation_ref, plan_ref))
}
