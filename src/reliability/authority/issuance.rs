#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema: String,
    pub generation: u64,
    pub operator_public_key: String,
    pub allowed_nodes: std::collections::BTreeSet<String>,
    pub allowed_groups: std::collections::BTreeSet<String>,
    pub allowed_source_bindings: std::collections::BTreeSet<String>,
    pub allowed_operations: std::collections::BTreeSet<String>,
    pub not_before_ms: u64,
    pub expires_at_ms: u64,
    pub max_grant_lifetime_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub schema: String,
    pub grant_id: String,
    pub policy_generation: u64,
    pub generation: u64,
    pub node_id: String,
    pub group_ref: String,
    pub membership_ref: String,
    pub source_binding_ref: String,
    pub endpoint: String,
    pub transport_session_ref: String,
    pub allowed_operations: std::collections::BTreeSet<String>,
    pub not_before_ms: u64,
    pub expires_at_ms: u64,
    pub probe_public_key: String,
    pub client_public_key: String,
    pub client_scope_ref: String,
    pub configuration: serde_json::Value,
}

/// Content identity only: computing this digest does not validate an issuer, policy, or authority.
pub fn grant_identity(grant: &Grant) -> crate::error::Result<String> {
    super::canonical::identity("operator-grant", grant)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedPolicy {
    pub policy: Policy,
    pub signature: Vec<u8>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedGrant {
    pub grant: Grant,
    pub signature: Vec<u8>,
}

/// Constructible only by checking the current protected policy and its pinned issuer.
#[derive(Debug, Clone)]
pub struct VerifiedGrant {
    grant: Grant,
    reference: String,
    policy_ref: String,
    operator_public_key: String,
}

impl VerifiedGrant {
    pub fn grant(&self) -> &Grant {
        &self.grant
    }
    pub fn configuration(&self) -> &serde_json::Value {
        &self.grant.configuration
    }
    pub fn grant_ref(&self) -> &str {
        &self.reference
    }
    pub fn generation(&self) -> u64 {
        self.grant.generation
    }
    pub fn node_id(&self) -> &str {
        &self.grant.node_id
    }
    pub fn source_binding_ref(&self) -> &str {
        &self.grant.source_binding_ref
    }
    pub fn endpoint(&self) -> &str {
        &self.grant.endpoint
    }
    pub fn probe_public_key(&self) -> &str {
        &self.grant.probe_public_key
    }
    pub fn operator_public_key(&self) -> &str {
        &self.operator_public_key
    }
    pub fn allows(&self, operation: &str) -> bool {
        self.grant.allowed_operations.contains(operation)
    }
    pub fn verify_current(&self, root: &crate::node_state::DirectoryRoot) -> crate::error::Result<()> {
        let current = super::storage::load(root)?;
        if current.reference != self.reference || current.policy_ref != self.policy_ref {
            return Err(super::denied("coordination authority changed since admission"));
        }
        Ok(())
    }
}

pub(super) fn verify(
    policy: &SignedPolicy,
    signed: &SignedGrant,
    issuer: &str,
    now: u64,
) -> crate::error::Result<VerifiedGrant> {
    super::canonical::verify("operator-policy", &policy.policy, issuer, &policy.signature)?;
    let p = &policy.policy;
    let g = &signed.grant;
    super::canonical::verify("operator-grant", g, issuer, &signed.signature)?;
    if p.schema != "molten.coordination-policy.v1"
        || g.schema != "molten.coordination-grant.v1"
        || p.operator_public_key != issuer
        || p.generation == 0
        || g.generation == 0
        || g.policy_generation != p.generation
        || now < p.not_before_ms
        || now >= p.expires_at_ms
        || now < g.not_before_ms
        || now >= g.expires_at_ms
        || g.not_before_ms < p.not_before_ms
        || g.expires_at_ms > p.expires_at_ms
        || g.expires_at_ms <= g.not_before_ms
        || g.expires_at_ms - g.not_before_ms > p.max_grant_lifetime_ms
        || !p.allowed_nodes.contains(&g.node_id)
        || !p.allowed_groups.contains(&g.group_ref)
        || !p.allowed_source_bindings.contains(&g.source_binding_ref)
        || g.allowed_operations.is_empty()
        || !g.allowed_operations.is_subset(&p.allowed_operations)
    {
        return Err(super::denied("coordination grant outside current operator policy"));
    }
    validate_identity(g, issuer)?;
    Ok(VerifiedGrant {
        grant: g.clone(),
        reference: super::canonical::identity("operator-grant", g)?,
        policy_ref: super::canonical::identity("operator-policy", p)?,
        operator_public_key: issuer.to_owned(),
    })
}

fn validate_identity(g: &Grant, issuer: &str) -> crate::error::Result<()> {
    for identity in [
        &g.grant_id,
        &g.node_id,
        &g.group_ref,
        &g.membership_ref,
        &g.source_binding_ref,
        &g.endpoint,
        &g.transport_session_ref,
        &g.client_scope_ref,
    ] {
        if identity.is_empty() || identity.len() > 4096 {
            return Err(super::denied("invalid grant identity"));
        }
    }
    let probe: iroh::PublicKey = g.probe_public_key.parse().map_err(|_| super::denied("invalid probe key"))?;
    if probe.to_string() == issuer || !g.configuration.is_object() {
        return Err(super::denied("probe purpose must use a dedicated key and configuration must be an object"));
    }
    let client: iroh::PublicKey = g.client_public_key.parse().map_err(|_| super::denied("invalid client key"))?;
    if client.to_string() == issuer || client == probe {
        return Err(super::denied("client signing key must be purpose-separated from operator and probe"));
    }
    Ok(())
}
