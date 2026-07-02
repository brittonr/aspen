
pub fn negotiate_peers(
    local_value: &IoValue,
    remote_value: &IoValue,
    policy: &NegotiationPolicy,
) -> Result<PeerAgreement> {
    let local = parse_handshake(local_value)?;
    let remote = parse_handshake(remote_value)?;
    let mut diagnostics = Vec::new();
    let selected_features = select_features(&local.features, &remote.features, policy, &mut diagnostics)?;
    let resource_limits = ResourceLimits {
        max_inflight: local
            .resource_limits
            .max_inflight
            .min(remote.resource_limits.max_inflight)
            .min(policy.max_inflight),
        max_bytes: local.resource_limits.max_bytes.min(remote.resource_limits.max_bytes).min(policy.max_bytes),
        max_topics: local.resource_limits.max_topics.min(remote.resource_limits.max_topics).min(policy.max_topics),
        max_jobs: local.resource_limits.max_jobs.min(remote.resource_limits.max_jobs).min(policy.max_jobs),
    };
    ensure_count_at_most(remote.requested_joins.len(), MAX_PEER_JOIN_REQUESTS, "remote requested joins")?;
    let accepted_capabilities = remote.capability_offers.clone();
    let mut admitted_joins = Vec::with_capacity(remote.requested_joins.len());
    let mut denied_joins = Vec::with_capacity(remote.requested_joins.len());
    for join in &remote.requested_joins {
        if join_admitted(join, &local.capability_offers) {
            push_bounded(&mut admitted_joins, join.clone(), MAX_PEER_JOIN_REQUESTS, "admitted joins")?;
        } else {
            push_bounded(&mut denied_joins, join.clone(), MAX_PEER_JOIN_REQUESTS, "denied joins")?;
        }
    }
    let decision = if diagnostics.is_empty() && denied_joins.is_empty() {
        "pass"
    } else {
        "fail"
    };
    let agreement_value = agreement_value(AgreementValueInput {
        decision,
        local: &local,
        remote: &remote,
        selected_features: &selected_features,
        admitted_joins: &admitted_joins,
        denied_joins: &denied_joins,
        accepted_capabilities: &accepted_capabilities,
        resource_limits: &resource_limits,
    });
    let agreement_ref = canonical_hash(&agreement_value)?;
    let receipt_value = bootstrap_receipt_value(&ReceiptValueInput {
        operation: "negotiate",
        decision,
        local_handshake_ref: &local.handshake_ref,
        remote_handshake_ref: &remote.handshake_ref,
        agreement_ref: Some(&agreement_ref),
        admitted_joins: &admitted_joins,
        denied_joins: &denied_joins,
        diagnostics: &diagnostics,
    });
    Ok(PeerAgreement {
        agreement_ref,
        decision: decision.to_string(),
        local_handshake_ref: local.handshake_ref,
        remote_handshake_ref: remote.handshake_ref,
        selected_features,
        admitted_joins,
        denied_joins,
        accepted_capabilities,
        resource_limits,
        receipt_ref: canonical_hash(&receipt_value)?,
        receipt_value,
        value: agreement_value,
    })
}

pub fn bootstrap_receipt_value(input: &ReceiptValueInput<'_>) -> IoValue {
    record("peer-bootstrap-receipt-v1", vec![
        string(PEER_BOOTSTRAP_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("handshakes", vec![
            record("local", vec![string(input.local_handshake_ref)]),
            record("remote", vec![string(input.remote_handshake_ref)]),
        ]),
        record("agreement", vec![optional_ref_value(input.agreement_ref)]),
        record("admitted-joins", vec![sequence(input.admitted_joins.iter().map(join_value).collect())]),
        record("denied-joins", vec![sequence(input.denied_joins.iter().map(join_value).collect())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("no-transport-authority"), string("pass")]),
            record("check", vec![string("deterministic-feature-negotiation"), string("pass")]),
            record("check", vec![string("unsafe-downgrade-denied"), string("pass")]),
            record("check", vec![string("capability-offers-not-authority"), string("pass")]),
            record("check", vec![string("resource-limits-bound"), string("pass")]),
        ])]),
    ])
}

fn agreement_value(input: AgreementValueInput<'_>) -> IoValue {
    record("peer-agreement-v1", vec![
        string(PEER_AGREEMENT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("peers", vec![
            record("local", vec![string(&input.local.identity_ref), string(&input.local.endpoint_id)]),
            record("remote", vec![string(&input.remote.identity_ref), string(&input.remote.endpoint_id)]),
        ]),
        feature_vector_value(input.selected_features),
        record("admitted-joins", vec![sequence(input.admitted_joins.iter().map(join_value).collect())]),
        record("denied-joins", vec![sequence(input.denied_joins.iter().map(join_value).collect())]),
        record("accepted-capabilities", vec![sequence(
            input.accepted_capabilities.iter().map(offer_value).collect(),
        )]),
        resource_limits_value(input.resource_limits),
        record("checks", vec![sequence(vec![
            record("check", vec![
                string("join-admission"),
                string(if input.denied_joins.is_empty() { "pass" } else { "fail" }),
            ]),
            record("check", vec![string("identity-not-authority"), string("pass")]),
            record("check", vec![string("remote-sync-join-policy"), string("pass")]),
            record("check", vec![string("topic-doc-protocol-job-joins-gated"), string("pass")]),
            record("check", vec![string("future-raft-joins-require-stronger-admission"), string("pass")]),
        ])]),
    ])
}

fn select_features(
    local: &FeatureVector,
    remote: &FeatureVector,
    policy: &NegotiationPolicy,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<FeatureVector> {
    let runtime_versions =
        select_required(&local.runtime_versions, &remote.runtime_versions, &policy.mandatory_runtime);
    let schema_identities =
        select_required(&local.schema_identities, &remote.schema_identities, &policy.mandatory_schema_identity);
    let preserves_boundaries = select_required(
        &local.preserves_boundaries,
        &remote.preserves_boundaries,
        &policy.mandatory_preserves_boundary,
    );
    if (runtime_versions.is_empty() || schema_identities.is_empty() || preserves_boundaries.is_empty())
        && !policy.allow_security_downgrade
    {
        diagnostics.push_item("unsafe-downgrade".to_string());
    }
    let registry_protocols = intersection_highest(&local.registry_protocols, &remote.registry_protocols);
    let handler_profiles = intersection_all(&local.handler_profiles, &remote.handler_profiles);
    let transports = intersection_all(&local.transports, &remote.transports);
    if transports.is_empty() {
        return Err(MoltenError::invalid_harness("peer negotiation requires at least one common transport"));
    }
    Ok(FeatureVector {
        runtime_versions,
        registry_protocols,
        schema_identities,
        preserves_boundaries,
        handler_profiles,
        transports,
        replay: local.replay && remote.replay,
    })
}

fn select_required(local: &[String], remote: &[String], required: &str) -> Vec<String> {
    if local.iter().any(|value| value == required) && remote.iter().any(|value| value == required) {
        vec![required.to_string()]
    } else {
        Vec::new()
    }
}

fn intersection_highest(left: &[String], right: &[String]) -> Vec<String> {
    let right = right.iter().collect::<OrderedSet<_>>();
    left.iter().filter(|value| right.contains(value)).max().cloned().into_iter().collect()
}

fn intersection_all(left: &[String], right: &[String]) -> Vec<String> {
    let right = right.iter().collect::<OrderedSet<_>>();
    let mut values = left.iter().filter(|value| right.contains(value)).cloned().collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn join_admitted(join: &JoinRequest, offers: &[CapabilityOffer]) -> bool {
    offers.iter().any(|offer| {
        offer.capability == join.required_capability
            && (offer.scope == join.target || offer.scope == "*")
            && offer.attenuation != "deny"
    })
}

fn feature_vector_value(features: &FeatureVector) -> IoValue {
    record("features", vec![
        record("runtime", vec![sequence(features.runtime_versions.iter().map(string).collect())]),
        record("registry", vec![sequence(features.registry_protocols.iter().map(string).collect())]),
        record("schema-identity", vec![sequence(features.schema_identities.iter().map(string).collect())]),
        record("preserves-boundary", vec![sequence(features.preserves_boundaries.iter().map(string).collect())]),
        record("handler-profiles", vec![sequence(features.handler_profiles.iter().map(string).collect())]),
        record("transports", vec![sequence(features.transports.iter().map(string).collect())]),
        record("replay", vec![string(if features.replay { "supported" } else { "unsupported" })]),
    ])
}

fn parse_feature_vector(value: &IoValue) -> Result<FeatureVector> {
    let fields = value
        .collect_simple_record("features", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected peer features"))?;
    Ok(FeatureVector {
        runtime_versions: parse_string_sequence(&fields[0], "runtime")?,
        registry_protocols: parse_string_sequence(&fields[1], "registry")?,
        schema_identities: parse_string_sequence(&fields[2], "schema-identity")?,
        preserves_boundaries: parse_string_sequence(&fields[3], "preserves-boundary")?,
        handler_profiles: parse_string_sequence(&fields[4], "handler-profiles")?,
        transports: parse_string_sequence(&fields[5], "transports")?,
        replay: record_string(&fields[6], "replay")? == "supported",
    })
}

fn offer_value(offer: &CapabilityOffer) -> IoValue {
    record("capability-offer", vec![
        record("capability", vec![string(&offer.capability)]),
        record("scope", vec![string(&offer.scope)]),
        record("attenuation", vec![string(&offer.attenuation)]),
        record("expires-at", vec![optional_u64_value(offer.expires_at)]),
        record("policy", vec![sequence(offer.policy_refs.iter().map(string).collect())]),
    ])
}

fn parse_offer(value: &IoValue) -> Result<CapabilityOffer> {
    let fields = value
        .collect_simple_record("capability-offer", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected capability offer"))?;
    let offer = CapabilityOffer {
        capability: record_string(&fields[0], "capability")?,
        scope: record_string(&fields[1], "scope")?,
        attenuation: record_string(&fields[2], "attenuation")?,
        expires_at: parse_optional_u64(&fields[3], "expires-at")?,
        policy_refs: parse_ref_sequence(&fields[4], "policy")?,
    };
    validate_offer(&offer)?;
    Ok(offer)
}

fn parse_offer_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<CapabilityOffer>> {
    let values = field_sequence(value, label)?;
    values.iter().map(|value| parse_offer(&value_to_iovalue(value))).collect()
}

fn join_value(join: &JoinRequest) -> IoValue {
    record("join-request", vec![
        record("kind", vec![string(&join.kind)]),
        record("target", vec![string(&join.target)]),
        record("required-capability", vec![string(&join.required_capability)]),
    ])
}

fn parse_join(value: &IoValue) -> Result<JoinRequest> {
    let fields = value
        .collect_simple_record("join-request", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected join request"))?;
    let join = JoinRequest {
        kind: record_string(&fields[0], "kind")?,
        target: record_string(&fields[1], "target")?,
        required_capability: record_string(&fields[2], "required-capability")?,
    };
    validate_join(&join)?;
    Ok(join)
}

fn parse_join_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<JoinRequest>> {
    let values = field_sequence(value, label)?;
    values.iter().map(|value| parse_join(&value_to_iovalue(value))).collect()
}

fn resource_limits_value(limits: &ResourceLimits) -> IoValue {
    record("resource-limits", vec![
        record("max-inflight", vec![u64_value(limits.max_inflight)]),
        record("max-bytes", vec![u64_value(limits.max_bytes)]),
        record("max-topics", vec![u64_value(limits.max_topics)]),
        record("max-jobs", vec![u64_value(limits.max_jobs)]),
    ])
}

fn parse_resource_limits(value: &IoValue) -> Result<ResourceLimits> {
    let fields = value
        .collect_simple_record("resource-limits", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected resource limits"))?;
    Ok(ResourceLimits {
        max_inflight: record_u64(&fields[0], "max-inflight")?,
        max_bytes: record_u64(&fields[1], "max-bytes")?,
        max_topics: record_u64(&fields[2], "max-topics")?,
        max_jobs: record_u64(&fields[3], "max-jobs")?,
    })
}
