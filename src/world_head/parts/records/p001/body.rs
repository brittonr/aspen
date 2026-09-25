
pub fn world_head_artifact_statement(
    claim: &CanonicalWorldHeadClaim,
    input: WorldHeadArtifactAuthInput<'_>,
) -> Result<(ArtifactStatement, WorldHeadStatementRef)> {
    let statement = ArtifactStatement {
        schema: STATEMENT_SCHEMA_V1.to_string(),
        scope: world_head_authentication_scope(claim)?,
        producer_id: input.producer_id.to_string(),
        key_id: input.key_id.to_string(),
        key_identity: input.key_identity,
    };
    let statement_identity = artifact_auth_core::statement_identity(&statement)
        .map_err(|_| MoltenError::invalid_harness("world-head Artifact Auth statement is invalid"))?;
    let statement_ref = WorldHeadStatementRef::new(format!("blake3:{statement_identity}")).map_err(reference_error)?;
    Ok((statement, statement_ref))
}

pub fn canonical_world_head_state(state: &WorldHeadState) -> Result<(String, Vec<u8>)> {
    let value = world_head_state_value(state);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let state_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
    Ok((state_ref, bytes))
}

pub fn parse_canonical_world_head_state(bytes: &[u8]) -> Result<WorldHeadState> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields = crate::preserves_rail::simple_record_fields(
        &decoded.value,
        WORLD_HEAD_STATE_RECORD,
        WORLD_HEAD_STATE_FIELD_COUNT,
    )?;
    let schema = crate::preserves_rail::required_string_field(&fields[0], "world-head state schema")?;
    if schema != WORLD_HEAD_TRANSITION_SCHEMA {
        return Err(MoltenError::invalid_harness("unsupported world-head state schema"));
    }
    let state = WorldHeadState {
        branch_id: WorldBranchId::new(crate::preserves_rail::required_string_field(
            &named_field_value(&fields[1], "branch-id")?,
            "world-head state branch",
        )?)
        .map_err(reference_error)?,
        branch_class: WorldBranchClass::parse(&crate::preserves_rail::required_string_field(
            &named_field_value(&fields[2], "branch-class")?,
            "world-head state class",
        )?)
        .map_err(reference_error)?,
        head: WorldCommitRef::new(crate::preserves_rail::required_content_ref_string(
            &named_field_value(&fields[3], "head")?,
            "world-head state head",
        )?)
        .map_err(world_commit_reference_error)?,
        generation: required_u64(&named_field_value(&fields[4], "generation")?, "world-head state generation")?,
        policy_ref: WorldHeadPolicyRef::new(crate::preserves_rail::required_content_ref_string(
            &named_field_value(&fields[5], "policy-ref")?,
            "world-head state policy",
        )?)
        .map_err(reference_error)?,
    };
    let (_, canonical) = canonical_world_head_state(&state)?;
    if canonical != decoded.canonical_bytes {
        return Err(MoltenError::invalid_harness("world-head state bytes are not canonical"));
    }
    Ok(state)
}

pub fn canonical_world_head_conflict(conflict: &WorldHeadConflictSet) -> Result<CanonicalWorldHeadConflict> {
    let members = conflict
        .members
        .iter()
        .map(|member| {
            crate::preserves_rail::record("conflict-member", vec![
                crate::preserves_rail::string(member.claim_ref.as_str()),
                crate::preserves_rail::string(member.successor_head.as_str()),
            ])
        })
        .collect::<Vec<_>>();
    let value = crate::preserves_rail::record(WORLD_HEAD_CONFLICT_RECORD, vec![
        crate::preserves_rail::string(WORLD_HEAD_CONFLICT_SCHEMA),
        named_field("branch-id", crate::preserves_rail::string(conflict.branch_id.as_str())),
        named_field("expected-head", crate::preserves_rail::string(conflict.expected_head.as_str())),
        named_field("expected-generation", crate::preserves_rail::u64_value(conflict.expected_generation)),
        named_field("members", crate::preserves_rail::sequence(members)),
        named_field("conflict-ref", crate::preserves_rail::string(&conflict.conflict_ref)),
        non_claims_value(),
    ]);
    crate::preserves_rail::validate_boundary_schema(&value, &WORLD_HEAD_CONFLICT_BOUNDARY_SCHEMA)?;
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    Ok(CanonicalWorldHeadConflict {
        conflict_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        value,
        bytes,
    })
}

pub fn canonical_world_head_transition_receipt(
    input: &WorldHeadTransitionReceiptInput<'_>,
) -> Result<CanonicalWorldHeadTransitionReceipt> {
    if !matches!(
        input.decision,
        TRANSITION_DECISION_ADMITTED | TRANSITION_DECISION_DENIED | TRANSITION_DECISION_CONFLICT
    ) {
        return Err(MoltenError::invalid_harness("unknown world-head receipt decision"));
    }
    let (before_head, before_generation, after_head, after_generation, currentness) =
        input
            .plan
            .map_or((None, None, None, None, WorldHeadCurrentnessClass::WholeStoreRollbackUnproven), |plan| {
                (
                    plan.before.as_ref().map(|state| state.head.as_str()),
                    plan.before.as_ref().map(|state| state.generation),
                    Some(plan.after.head.as_str()),
                    Some(plan.after.generation),
                    plan.currentness,
                )
            });
    let value = crate::preserves_rail::record(WORLD_HEAD_TRANSITION_RECEIPT_RECORD, vec![
        crate::preserves_rail::string(WORLD_HEAD_TRANSITION_SCHEMA),
        named_field("decision", crate::preserves_rail::string(input.decision)),
        named_field("claim-ref", crate::preserves_rail::string(input.claim_ref.as_str())),
        named_field("statement-ref", crate::preserves_rail::string(input.statement_ref.as_str())),
        named_field("authentication-decision-ref", crate::preserves_rail::string(input.authentication_decision_ref)),
        named_field("authority-ref", crate::preserves_rail::string(input.authority_ref)),
        named_field("before-head", crate::preserves_rail::optional_ref_value(before_head)),
        named_field("before-generation", optional_u64_value(before_generation)),
        named_field("after-head", crate::preserves_rail::optional_ref_value(after_head)),
        named_field("after-generation", optional_u64_value(after_generation)),
        named_field("currentness", crate::preserves_rail::string(currentness.as_str())),
        named_field(
            "issues",
            crate::preserves_rail::sequence(input.issue_codes.iter().map(crate::preserves_rail::string).collect()),
        ),
        non_claims_value(),
    ]);
    crate::preserves_rail::validate_boundary_schema(&value, &WORLD_HEAD_TRANSITION_RECEIPT_BOUNDARY_SCHEMA)?;
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    Ok(CanonicalWorldHeadTransitionReceipt {
        receipt_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        value,
        bytes,
    })
}

fn world_head_claim_value(claim: &WorldHeadClaim) -> IOValue {
    let mut source_heads = claim.source_heads.iter().map(WorldCommitRef::as_str).collect::<Vec<_>>();
    source_heads.sort_unstable();
    crate::preserves_rail::record(WORLD_HEAD_CLAIM_RECORD, vec![
        crate::preserves_rail::string(WORLD_HEAD_CLAIM_SCHEMA),
        named_field("branch-id", crate::preserves_rail::string(claim.branch_id.as_str())),
        named_field("branch-class", crate::preserves_rail::string(claim.branch_class.as_str())),
        named_field(
            "expected-head",
            crate::preserves_rail::optional_ref_value(claim.expected_head.as_ref().map(WorldCommitRef::as_str)),
        ),
        named_field("successor-head", crate::preserves_rail::string(claim.successor_head.as_str())),
        named_field("expected-generation", crate::preserves_rail::u64_value(claim.expected_generation)),
        named_field("successor-generation", crate::preserves_rail::u64_value(claim.successor_generation)),
        named_field("purpose", crate::preserves_rail::string(claim.purpose.as_str())),
        named_field("policy-ref", crate::preserves_rail::string(claim.policy_ref.as_str())),
        named_field(
            "source-heads",
            crate::preserves_rail::sequence(source_heads.into_iter().map(crate::preserves_rail::string).collect()),
        ),
    ])
}

fn world_head_state_value(state: &WorldHeadState) -> IOValue {
    crate::preserves_rail::record(WORLD_HEAD_STATE_RECORD, vec![
        crate::preserves_rail::string(WORLD_HEAD_TRANSITION_SCHEMA),
        named_field("branch-id", crate::preserves_rail::string(state.branch_id.as_str())),
        named_field("branch-class", crate::preserves_rail::string(state.branch_class.as_str())),
        named_field("head", crate::preserves_rail::string(state.head.as_str())),
        named_field("generation", crate::preserves_rail::u64_value(state.generation)),
        named_field("policy-ref", crate::preserves_rail::string(state.policy_ref.as_str())),
    ])
}

fn named_field(label: &'static str, value: IOValue) -> IOValue {
    crate::preserves_rail::record(label, vec![value])
}

fn named_field_value(value: &preserves::Value<IOValue>, label: &str) -> Result<preserves::Value<IOValue>> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} VALUE>")))?;
    Ok(fields[0].clone())
}

fn artifact_ref(profile: &str, reference: &str) -> Result<ArtifactRef> {
    Ok(ArtifactRef {
        profile: profile.to_string(),
        algorithm: ALGORITHM_BLAKE3.to_string(),
        digest_hex: crate::preserves_rail::content_ref_hex(reference)?.to_string(),
    })
}

fn required_u64(value: &preserves::Value<IOValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn optional_u64_value(value: Option<u64>) -> IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(value)]),
    )
}

fn non_claims_value() -> IOValue {
    crate::preserves_rail::record("non-claims", vec![crate::preserves_rail::sequence(
        WORLD_HEAD_NON_CLAIMS.iter().map(crate::preserves_rail::string).collect(),
    )])
}

fn reference_error(error: molten_core::world_head::WorldHeadReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world-head reference: {error}"))
}

fn world_commit_reference_error(error: molten_core::world_commit::WorldCommitReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world commit reference: {error:?}"))
}
