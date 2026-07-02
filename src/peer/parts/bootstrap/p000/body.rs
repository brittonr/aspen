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
