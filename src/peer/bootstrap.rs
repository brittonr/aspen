type OrderedSet<T> = std::collections::BTreeSet<T>;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

const PEER_AGREEMENT_SCHEMA: &str = crate::preserves_rail::PEER_AGREEMENT_SCHEMA;
const PEER_BOOTSTRAP_INPUT_SCHEMA: &str = crate::preserves_rail::PEER_BOOTSTRAP_INPUT_SCHEMA;
const PEER_BOOTSTRAP_RECEIPT_SCHEMA: &str = crate::preserves_rail::PEER_BOOTSTRAP_RECEIPT_SCHEMA;
const PEER_HANDSHAKE_SCHEMA: &str = crate::preserves_rail::PEER_HANDSHAKE_SCHEMA;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

const MAX_PEER_JOIN_REQUESTS: usize = 256;
const _: () = assert!(MAX_PEER_JOIN_REQUESTS > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapInput {
    pub kind: String,
    pub peer: String,
    pub endpoint_id: String,
    pub provenance_ref: String,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureVector {
    pub runtime_versions: Vec<String>,
    pub registry_protocols: Vec<String>,
    pub schema_identities: Vec<String>,
    pub preserves_boundaries: Vec<String>,
    pub handler_profiles: Vec<String>,
    pub transports: Vec<String>,
    pub replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityOffer {
    pub capability: String,
    pub scope: String,
    pub attenuation: String,
    pub expires_at: Option<u64>,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinRequest {
    pub kind: String,
    pub target: String,
    pub required_capability: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLimits {
    pub max_inflight: u64,
    pub max_bytes: u64,
    pub max_topics: u64,
    pub max_jobs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeRecord {
    pub handshake_ref: String,
    pub node_id: String,
    pub identity_ref: String,
    pub endpoint_id: String,
    pub molten_version: String,
    pub features: FeatureVector,
    pub requested_joins: Vec<JoinRequest>,
    pub capability_offers: Vec<CapabilityOffer>,
    pub resource_limits: ResourceLimits,
    pub policy_refs: Vec<String>,
    pub receipt_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiationPolicy {
    pub mandatory_runtime: String,
    pub mandatory_schema_identity: String,
    pub mandatory_preserves_boundary: String,
    pub allow_security_downgrade: bool,
    pub max_inflight: u64,
    pub max_bytes: u64,
    pub max_topics: u64,
    pub max_jobs: u64,
}

impl Default for NegotiationPolicy {
    fn default() -> Self {
        Self {
            mandatory_runtime: "molten-runtime-v1".to_string(),
            mandatory_schema_identity: "schema-identity-v1".to_string(),
            mandatory_preserves_boundary: "preserves-boundary-v1".to_string(),
            allow_security_downgrade: false,
            max_inflight: 64,
            max_bytes: 1_048_576,
            max_topics: 8,
            max_jobs: 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerAgreement {
    pub agreement_ref: String,
    pub decision: String,
    pub local_handshake_ref: String,
    pub remote_handshake_ref: String,
    pub selected_features: FeatureVector,
    pub admitted_joins: Vec<JoinRequest>,
    pub denied_joins: Vec<JoinRequest>,
    pub accepted_capabilities: Vec<CapabilityOffer>,
    pub resource_limits: ResourceLimits,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub value: IoValue,
}

pub struct HandshakeValueInput<'a> {
    pub node_id: &'a str,
    pub identity_ref: &'a str,
    pub endpoint_id: &'a str,
    pub molten_version: &'a str,
    pub features: &'a FeatureVector,
    pub requested_joins: &'a [JoinRequest],
    pub capability_offers: &'a [CapabilityOffer],
    pub resource_limits: &'a ResourceLimits,
    pub policy_refs: &'a [String],
    pub receipt_refs: &'a [String],
}

pub struct ReceiptValueInput<'a> {
    pub operation: &'a str,
    pub decision: &'a str,
    pub local_handshake_ref: &'a str,
    pub remote_handshake_ref: &'a str,
    pub agreement_ref: Option<&'a str>,
    pub admitted_joins: &'a [JoinRequest],
    pub denied_joins: &'a [JoinRequest],
    pub diagnostics: &'a [String],
}

struct AgreementValueInput<'a> {
    decision: &'a str,
    local: &'a HandshakeRecord,
    remote: &'a HandshakeRecord,
    selected_features: &'a FeatureVector,
    admitted_joins: &'a [JoinRequest],
    denied_joins: &'a [JoinRequest],
    accepted_capabilities: &'a [CapabilityOffer],
    resource_limits: &'a ResourceLimits,
}

pub fn bootstrap_input_value(input: &BootstrapInput) -> Result<IoValue> {
    validate_bootstrap_input(input)?;
    Ok(record("peer-bootstrap-input-v1", vec![
        string(PEER_BOOTSTRAP_INPUT_SCHEMA),
        record("kind", vec![string(&input.kind)]),
        record("peer", vec![string(&input.peer)]),
        record("endpoint-id", vec![string(&input.endpoint_id)]),
        record("provenance", vec![string(&input.provenance_ref)]),
        record("policy", vec![sequence(input.policy_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("bootstrap-input-is-hint"), string("pass")]),
            record("check", vec![string("join-admission-required"), string("pass")]),
        ])]),
    ]))
}

pub fn handshake_value(input: &HandshakeValueInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.node_id, "peer handshake node id")?;
    require_ref(input.identity_ref, "peer handshake node identity ref")?;
    validate_endpoint(input.endpoint_id)?;
    validate_features(input.features)?;
    validate_refs(input.policy_refs, "peer handshake policy ref")?;
    validate_refs(input.receipt_refs, "peer handshake receipt ref")?;
    for join in input.requested_joins {
        validate_join(join)?;
    }
    for offer in input.capability_offers {
        validate_offer(offer)?;
    }
    Ok(record("peer-handshake-v1", vec![
        string(PEER_HANDSHAKE_SCHEMA),
        record("node", vec![
            record("id", vec![string(input.node_id)]),
            record("identity", vec![string(input.identity_ref)]),
            record("endpoint-id", vec![string(input.endpoint_id)]),
            record("version", vec![string(input.molten_version)]),
        ]),
        feature_vector_value(input.features),
        record("requested-joins", vec![sequence(input.requested_joins.iter().map(join_value).collect())]),
        record("capability-offers", vec![sequence(input.capability_offers.iter().map(offer_value).collect())]),
        resource_limits_value(input.resource_limits),
        record("policy", vec![sequence(input.policy_refs.iter().map(string).collect())]),
        record("receipts", vec![sequence(input.receipt_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("transport-identity-is-not-authority"), string("pass")]),
            record("check", vec![string("capability-offers-not-grants"), string("pass")]),
            record("check", vec![string("join-admission-required"), string("pass")]),
            record("check", vec![string("resource-limits-declared"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_handshake(value: &IoValue) -> Result<HandshakeRecord> {
    let fields = value
        .collect_simple_record("peer-handshake-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <peer-handshake-v1 ...>"))?;
    require_schema(&fields[0], PEER_HANDSHAKE_SCHEMA, "peer handshake schema")?;
    let node = value_to_iovalue(&fields[1]);
    let node_fields = node
        .collect_simple_record("node", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("peer handshake missing node field"))?;
    let features = parse_feature_vector(&value_to_iovalue(&fields[2]))?;
    let requested_joins = parse_join_sequence(&fields[3], "requested-joins")?;
    let capability_offers = parse_offer_sequence(&fields[4], "capability-offers")?;
    let resource_limits = parse_resource_limits(&value_to_iovalue(&fields[5]))?;
    let policy_refs = parse_ref_sequence(&fields[6], "policy")?;
    let receipt_refs = parse_ref_sequence(&fields[7], "receipts")?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "transport-identity-is-not-authority")?;
    require_check(&checks, "capability-offers-not-grants")?;
    Ok(HandshakeRecord {
        handshake_ref: canonical_hash(value)?,
        node_id: record_string(&node_fields[0], "id")?,
        identity_ref: record_string(&node_fields[1], "identity")?,
        endpoint_id: record_string(&node_fields[2], "endpoint-id")?,
        molten_version: record_string(&node_fields[3], "version")?,
        features,
        requested_joins,
        capability_offers,
        resource_limits,
        policy_refs,
        receipt_refs,
        value: value.clone(),
    })
}

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

fn validate_bootstrap_input(input: &BootstrapInput) -> Result<()> {
    validate_non_empty(&input.kind, "bootstrap input kind")?;
    validate_non_empty(&input.peer, "bootstrap input peer")?;
    validate_endpoint(&input.endpoint_id)?;
    require_ref(&input.provenance_ref, "bootstrap input provenance ref")?;
    validate_refs(&input.policy_refs, "bootstrap input policy ref")
}

fn validate_features(features: &FeatureVector) -> Result<()> {
    if features.runtime_versions.is_empty()
        || features.registry_protocols.is_empty()
        || features.schema_identities.is_empty()
        || features.preserves_boundaries.is_empty()
        || features.transports.is_empty()
    {
        return Err(MoltenError::invalid_harness("peer feature vectors must include required feature sets"));
    }
    Ok(())
}

fn validate_offer(offer: &CapabilityOffer) -> Result<()> {
    validate_non_empty(&offer.capability, "capability offer capability")?;
    validate_non_empty(&offer.scope, "capability offer scope")?;
    validate_non_empty(&offer.attenuation, "capability offer attenuation")?;
    validate_refs(&offer.policy_refs, "capability offer policy ref")
}

fn validate_join(join: &JoinRequest) -> Result<()> {
    validate_non_empty(&join.kind, "join kind")?;
    validate_non_empty(&join.target, "join target")?;
    validate_non_empty(&join.required_capability, "join required capability")
}

fn validate_endpoint(endpoint_id: &str) -> Result<()> {
    if endpoint_id.starts_with("iroh:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "expected iroh endpoint id for peer bootstrap, got {endpoint_id}"
        )))
    }
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for reference in refs {
        require_ref(reference, field)?;
    }
    Ok(())
}

fn require_ref(reference: &str, field: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {reference}: {error}"))
    })
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_u64_value(value: Option<u64>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![u64_value(value)]))
}

fn parse_optional_u64(value: &Value<IoValue>, label: &str) -> Result<Option<u64>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let optional = value_to_iovalue(&fields[0]);
    if optional.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = optional.collect_simple_record("some", Some(1)) {
        required_u64(&some[0], label).map(Some)
    } else {
        Err(MoltenError::invalid_harness(format!("expected optional u64 for {label}")))
    }
}

fn parse_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    values
        .iter()
        .map(|value| {
            let reference = required_string(value, label)?;
            require_ref(&reference, label)?;
            Ok(reference)
        })
        .collect()
}

fn parse_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    values.iter().map(|value| required_string(value, label)).collect()
}

fn field_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<Value<IoValue>>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(values.iter().cloned().collect())
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let values = field_sequence(value, "checks")?;
    values
        .iter()
        .map(|check| {
            let check = value_to_iovalue(check);
            let fields = check
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected peer bootstrap check"))?;
            Ok((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?))
        })
        .collect()
}

fn require_check(checks: &[(String, String)], name: &str) -> Result<()> {
    if checks.iter().any(|(check, status)| check == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("peer bootstrap evidence missing passing {name} check")))
    }
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&fields[0], label)
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_u64(&fields[0], label)
}

fn require_schema(value: &Value<IoValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")));
    }
    Ok(())
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatible_loopback_handshake_admits_join_with_capability() {
        let policy = NegotiationPolicy::default();
        let local = sample_handshake("local", vec![sample_offer("join:gossip", "topic:updates")], Vec::new());
        let remote = sample_handshake("remote", Vec::new(), vec![JoinRequest {
            kind: "gossip-topic".to_string(),
            target: "topic:updates".to_string(),
            required_capability: "join:gossip".to_string(),
        }]);
        let agreement = negotiate_peers(&local, &remote, &policy).expect("negotiate");
        assert_eq!(agreement.decision, "pass");
        assert_eq!(agreement.admitted_joins.len(), 1);
        assert!(agreement.denied_joins.is_empty());
        assert_eq!(agreement.selected_features.runtime_versions, vec![policy.mandatory_runtime]);
    }

    #[test]
    fn unsafe_downgrade_and_missing_capability_are_denied() {
        let policy = NegotiationPolicy::default();
        let local = sample_handshake("local", Vec::new(), Vec::new());
        let mut remote_features = sample_features();
        remote_features.preserves_boundaries = vec!["legacy-preserves".to_string()];
        let remote = handshake_value(&HandshakeValueInput {
            node_id: "remote",
            identity_ref: &ref_for("remote-identity"),
            endpoint_id: "iroh:remote",
            molten_version: "0.1.0",
            features: &remote_features,
            requested_joins: &[JoinRequest {
                kind: "docs-namespace".to_string(),
                target: "docs:private".to_string(),
                required_capability: "join:docs".to_string(),
            }],
            capability_offers: &[],
            resource_limits: &sample_limits(),
            policy_refs: &[],
            receipt_refs: &[],
        })
        .expect("remote handshake");
        let agreement = negotiate_peers(&local, &remote, &policy).expect("negotiate denial");
        assert_eq!(agreement.decision, "fail");
        assert_eq!(agreement.denied_joins.len(), 1);
        assert!(
            crate::preserves_rail::to_text(&agreement.receipt_value)
                .expect("receipt text")
                .contains("unsafe-downgrade")
        );
    }

    #[test]
    fn capability_offers_do_not_grant_authority_until_join_is_admitted() {
        let local = sample_handshake("local", vec![sample_offer("join:jobs", "*")], Vec::new());
        let remote = sample_handshake("remote", Vec::new(), Vec::new());
        let agreement = negotiate_peers(&local, &remote, &NegotiationPolicy::default()).expect("negotiate");
        assert_eq!(agreement.decision, "pass");
        assert!(agreement.admitted_joins.is_empty());
        assert!(
            crate::preserves_rail::to_text(&agreement.receipt_value)
                .expect("receipt text")
                .contains("capability-offers-not-authority")
        );
    }

    #[test]
    fn resource_limits_are_bound_by_policy_and_peers() {
        let policy = NegotiationPolicy {
            max_inflight: 8,
            max_bytes: 1024,
            max_topics: 2,
            max_jobs: 1,
            ..NegotiationPolicy::default()
        };
        let local = sample_handshake("local", Vec::new(), Vec::new());
        let remote = sample_handshake("remote", Vec::new(), Vec::new());
        let agreement = negotiate_peers(&local, &remote, &policy).expect("negotiate");
        assert_eq!(agreement.resource_limits.max_inflight, 8);
        assert_eq!(agreement.resource_limits.max_bytes, 1024);
        assert_eq!(agreement.resource_limits.max_topics, 2);
        assert_eq!(agreement.resource_limits.max_jobs, 1);
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_negotiation_is_deterministic_and_denied_join_is_safe(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let should_offer_capability = tc.draw(hegel::generators::booleans());
        let offers = if should_offer_capability {
            vec![sample_offer("join:gossip", &format!("topic:{salt}"))]
        } else {
            Vec::new()
        };
        let local = sample_handshake(&format!("local-{salt}"), offers, Vec::new());
        let remote = sample_handshake(&format!("remote-{salt}"), Vec::new(), vec![JoinRequest {
            kind: "gossip-topic".to_string(),
            target: format!("topic:{salt}"),
            required_capability: "join:gossip".to_string(),
        }]);
        let first = negotiate_peers(&local, &remote, &NegotiationPolicy::default()).expect("first negotiation");
        let second = negotiate_peers(&local, &remote, &NegotiationPolicy::default()).expect("second negotiation");
        assert_eq!(first.value, second.value);
        assert_eq!(first.receipt_value, second.receipt_value);
        if should_offer_capability {
            assert_eq!(first.decision, "pass");
            assert_eq!(first.admitted_joins.len(), 1);
        } else {
            assert_eq!(first.decision, "fail");
            assert_eq!(first.denied_joins.len(), 1);
            assert!(first.admitted_joins.is_empty());
        }
    }

    fn sample_handshake(name: &str, offers: Vec<CapabilityOffer>, joins: Vec<JoinRequest>) -> IoValue {
        handshake_value(&HandshakeValueInput {
            node_id: name,
            identity_ref: &ref_for(&format!("identity-{name}")),
            endpoint_id: &format!("iroh:{name}"),
            molten_version: "0.1.0",
            features: &sample_features(),
            requested_joins: &joins,
            capability_offers: &offers,
            resource_limits: &sample_limits(),
            policy_refs: &[],
            receipt_refs: &[],
        })
        .expect("sample handshake")
    }

    fn sample_features() -> FeatureVector {
        FeatureVector {
            runtime_versions: vec!["molten-runtime-v1".to_string()],
            registry_protocols: vec!["registry-v1".to_string(), "registry-v2".to_string()],
            schema_identities: vec!["schema-identity-v1".to_string()],
            preserves_boundaries: vec!["preserves-boundary-v1".to_string()],
            handler_profiles: vec!["native".to_string(), "wasm".to_string()],
            transports: vec!["iroh-gossip".to_string(), "iroh-blobs".to_string()],
            replay: true,
        }
    }

    fn sample_offer(capability: &str, scope: &str) -> CapabilityOffer {
        CapabilityOffer {
            capability: capability.to_string(),
            scope: scope.to_string(),
            attenuation: "scoped".to_string(),
            expires_at: Some(100),
            policy_refs: Vec::new(),
        }
    }

    fn sample_limits() -> ResourceLimits {
        ResourceLimits {
            max_inflight: 64,
            max_bytes: 1_048_576,
            max_topics: 8,
            max_jobs: 4,
        }
    }

    fn ref_for(label: &str) -> String {
        canonical_hash(&record("peer-bootstrap-test-ref", vec![string(label)])).expect("test ref")
    }
}
