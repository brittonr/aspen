pub fn crypto(
    scope: &crate::node_state::Scope,
) -> crate::error::Result<crate::fabric_crypto_identity::IrohEd25519<'_>> {
    let profile = crate::fabric_crypto_identity::profile(&crate::fabric_crypto_identity::production_ed25519_profile(
        super::transport::reference("federated-pull-live-ed25519"),
        super::transport::reference("system-entropy-v1"),
    ))?;
    crate::fabric_crypto_identity::IrohEd25519::new(
        scope,
        profile,
        super::transport::reference("federated-pull-live-key-backend"),
    )
}
pub fn content() -> crate::error::Result<molten_core::content_store_adapter::ContentAdapterProfile> {
    crate::content_store_adapter::profile(
        "federated-pull-capability-local-v1",
        super::transport::reference("federated-pull-content-profile-v1"),
        molten_core::content_store_adapter::ContentAdapterClass::CapabilityLocal,
        molten_core::content_store_adapter::ContentResourceBounds {
            max_total_bytes: super::transport::LIMIT,
            max_chunk_count: 8,
            max_chunk_bytes: super::transport::LIMIT,
            max_range_bytes: super::transport::LIMIT,
            max_concurrent_operations: 2,
            max_queued_bytes: super::transport::LIMIT * 2,
            max_memory_bytes: super::transport::LIMIT * 2,
            max_deadline_ticks: super::transport::UNTIL,
            max_retries: 0,
            max_events: 64,
            max_status_entries: 16,
        },
        vec![super::transport::reference(include_str!("profiles.rs"))],
    )
}
pub fn time() -> crate::error::Result<crate::fabric_time::Profile> {
    crate::fabric_time::admit_profile(&crate::fabric_time::TimeProfileDescriptor {
        schema: crate::fabric_time::FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: "federated-pull-live-time-v1".to_string(),
        profile_ref: super::transport::reference("federated-pull-live-time-v1"),
        kind: crate::fabric_time::TimeProfileKind::Live,
        supported_domains: crate::fabric_time::REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: super::transport::UNTIL,
        max_uncertainty_ticks: 100,
        max_timers: 8,
        max_runnables: 8,
        max_entropy_request_bytes: 64,
        max_entropy_total_bytes: 1024,
        max_scheduler_concurrency: 2,
        max_scheduler_queue_depth: 8,
        fairness_bound_turns: None,
        scheduler_policy: crate::fabric_time::SchedulerPolicy {
            ordering: crate::fabric_time::SchedulerOrdering::Fifo,
            replay: crate::fabric_time::SchedulerReplayPolicy::RecordedChoiceRequired,
            overload: crate::fabric_time::SchedulerOverloadPolicy::Backpressure,
        },
        evidence_mode: crate::fabric_time::TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: crate::fabric_time::REQUIRED_TIME_NON_CLAIMS.to_vec(),
    })
}
pub const ARTIFACT_SCHEMA: crate::preserves_rail::BoundarySchemaSpec = crate::preserves_rail::BoundarySchemaSpec {
    family: "federation-live-artifact",
    version: "v1",
    record_label: "federation-live-artifact-v1",
    schema_id: "molten.federation.live-artifact.v1",
    fields: &[crate::preserves_rail::BoundaryFieldSpec {
        label: "message",
        kind: crate::preserves_rail::BoundaryFieldKind::StringRecord,
    }],
};
pub fn artifact() -> preserves::IOValue {
    crate::preserves_rail::record("federation-live-artifact-v1", vec![crate::preserves_rail::record("message", vec![
        crate::preserves_rail::string("receiver-admitted signed pull"),
    ])])
}
pub fn local_policy(head: &str, signer: &str) -> crate::error::Result<crate::mesh::admission::rules::LocalPolicy> {
    let mut policy = crate::mesh::admission::rules::LocalPolicy {
        schema: ARTIFACT_SCHEMA,
        allowed_heads: std::collections::BTreeSet::from([head.to_string()]),
        allowed_signers: std::collections::BTreeSet::from([signer.to_string()]),
        maximum_bytes: super::transport::LIMIT,
        expires_at: super::transport::UNTIL,
        retention_policy_ref: String::new(),
        provenance_policy_ref: String::new(),
        resource_profile_ref: crate::mesh::admission::rules::resource_rule_ref(super::transport::LIMIT)?,
    };
    policy.retention_policy_ref = crate::mesh::admission::rules::retention_rule_ref(&policy)?;
    policy.provenance_policy_ref = crate::mesh::admission::rules::provenance_rule_ref(&policy)?;
    Ok(policy)
}
pub fn proofset(head: &str, holder: &str) -> crate::error::Result<crate::capability_tokens::CapabilityProofset> {
    let context = super::transport::reference("federation-live-local-authority-context");
    let session = super::transport::reference("federation-live-local-authority-session");
    let rule = crate::preserves_rail::record("federation-live-local-grant-rule-v1", vec![
        crate::preserves_rail::string(holder),
        crate::preserves_rail::string(head),
        crate::preserves_rail::string("federation:import"),
        crate::preserves_rail::string("local-registry"),
        crate::preserves_rail::u64_value(super::transport::UNTIL),
    ]);
    let policies = vec![crate::preserves_rail::canonical_hash(&rule)?];
    let resources = vec![crate::mesh::admission::rules::resource_rule_ref(
        super::transport::LIMIT,
    )?];
    let token = crate::capability_tokens::CapabilityToken {
        token_kind: "local-grant".to_string(),
        issuer_ref: holder.to_string(),
        holder_ref: holder.to_string(),
        session_ref: session.clone(),
        context_ref: context.clone(),
        resource_ref: head.to_string(),
        ability: "federation:import".to_string(),
        scope: "local-registry".to_string(),
        attenuation: "exact-ref".to_string(),
        caveats: Vec::new(),
        expires_at_tick: super::transport::UNTIL,
        revoked_refs: Vec::new(),
        policy_refs: policies.clone(),
        resource_refs: resources.clone(),
        delegation_refs: Vec::new(),
        evidence_refs: vec![holder.to_string()],
    };
    Ok(crate::capability_tokens::CapabilityProofset {
        holder_ref: holder.to_string(),
        session_ref: session,
        context_ref: context,
        tokens: vec![token],
        policy_refs: policies,
        resource_refs: resources,
        revocation_refs: Vec::new(),
        evidence_refs: vec![holder.to_string()],
    })
}
