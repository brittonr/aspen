type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;

const SUBSCRIPTION_GRANT_SCHEMA: &str = "molten.peer-subscription-grant.v1";
const SUBSCRIPTION_PROJECTION_SCHEMA: &str = "molten.peer-subscription-projection-receipt.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionGrant {
    pub holder_ref: String,
    pub resource_ref: String,
    pub projection_kind: String,
    pub egress_policy_ref: String,
    pub redaction_profile: String,
    pub resource_limit: u64,
    pub expires_at_tick: u64,
    pub revoked_refs: Vec<String>,
    pub read_authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionRequest {
    pub holder_ref: String,
    pub operation: String,
    pub projection_kind: String,
    pub at_tick: u64,
    pub item_count: u64,
    pub required_authority_ref: String,
    pub required_policy_ref: String,
    pub required_resource_ref: String,
    pub requested_scope: String,
    pub contains_sensitive_content: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
    pub receipt_ref: String,
}

pub fn subscription_grant_value(grant: &SubscriptionGrant) -> IoValue {
    crate::preserves_rail::record("peer-subscription-grant-v1", vec![
        string(SUBSCRIPTION_GRANT_SCHEMA),
        field("holder-ref", &grant.holder_ref),
        field("resource-ref", &grant.resource_ref),
        field("projection-kind", &grant.projection_kind),
        field("egress-policy-ref", &grant.egress_policy_ref),
        field("redaction-profile", &grant.redaction_profile),
        crate::preserves_rail::record("resource-limit", vec![crate::preserves_rail::u64_value(grant.resource_limit)]),
        crate::preserves_rail::record("expires-at-tick", vec![crate::preserves_rail::u64_value(grant.expires_at_tick)]),
        list_field("revoked-refs", &grant.revoked_refs),
        list_field("read-authority-refs", &grant.read_authority_refs),
        list_field("policy-refs", &grant.policy_refs),
        list_field("resource-refs", &grant.resource_refs),
        list_field("scopes", &grant.scopes),
    ])
}

pub fn validate_subscription_projection(
    grant: &SubscriptionGrant,
    request: &ProjectionRequest,
) -> Result<ProjectionReceipt> {
    let mut diagnostics = Vec::new();
    if grant.holder_ref != request.holder_ref {
        diagnostics.push("holder mismatch".to_string());
    }
    if request.operation != "read" && request.operation != "project" {
        diagnostics.push(format!("read-only grant cannot perform {}", request.operation));
    }
    if grant.projection_kind != request.projection_kind {
        diagnostics.push("projection kind mismatch".to_string());
    }
    if request.at_tick > grant.expires_at_tick {
        diagnostics.push("subscription grant expired".to_string());
    }
    if request.item_count > grant.resource_limit {
        diagnostics.push("subscription resource limit exceeded".to_string());
    }
    if request.contains_sensitive_content && grant.redaction_profile == "none" {
        diagnostics.push("sensitive content requires redaction".to_string());
    }
    require_member(&mut diagnostics, "read authority", &request.required_authority_ref, &grant.read_authority_refs);
    require_member(&mut diagnostics, "policy", &request.required_policy_ref, &grant.policy_refs);
    require_member(&mut diagnostics, "resource", &request.required_resource_ref, &grant.resource_refs);
    require_member(&mut diagnostics, "scope", &request.requested_scope, &grant.scopes);
    if request.operation == "relay" || request.operation == "republish" || request.operation == "sync-import" {
        require_member(&mut diagnostics, "attenuated relay scope", &request.operation, &grant.scopes);
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = receipt_value(decision, &diagnostics, grant, request);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ProjectionReceipt {
        decision: decision.to_string(),
        diagnostics,
        value,
        receipt_ref,
    })
}

fn require_member(diagnostics: &mut Vec<String>, label: &str, required: &str, available: &[String]) {
    if !available.iter().any(|value| value == required) {
        diagnostics.push(format!("missing {label} {required}"));
    }
}

fn receipt_value(
    decision: &str,
    diagnostics: &[String],
    grant: &SubscriptionGrant,
    request: &ProjectionRequest,
) -> IoValue {
    crate::preserves_rail::record("peer-subscription-projection-receipt-v1", vec![
        string(SUBSCRIPTION_PROJECTION_SCHEMA),
        field("decision", decision),
        field("holder-ref", &request.holder_ref),
        field("resource-ref", &grant.resource_ref),
        field("operation", &request.operation),
        field("projection-kind", &request.projection_kind),
        list_field("diagnostics", diagnostics),
        field("authority-boundary", "subscriber-receipt-is-not-consensus-or-write-authority"),
    ])
}

fn field(label: &'static str, value: &str) -> IoValue {
    crate::preserves_rail::record(label, vec![string(value)])
}

fn list_field(label: &'static str, values: &[String]) -> IoValue {
    crate::preserves_rail::record(label, vec![crate::preserves_rail::sequence(values.iter().map(string).collect())])
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMIT: u64 = 8;
    const CURRENT_TICK: u64 = 2;
    const STALE_TICK: u64 = 3;

    #[test]
    fn subscriber_projection_passes_with_read_evidence() {
        let receipt = validate_subscription_projection(&grant(), &request("read", CURRENT_TICK, LIMIT, false))
            .expect("projection");
        assert_eq!(receipt.decision, "pass");
    }

    #[test]
    fn subscriber_projection_denies_write_upgrade_and_sensitive_egress() {
        let mut grant = grant();
        grant.redaction_profile = "none".to_string();
        let receipt = validate_subscription_projection(&grant, &request("publish", STALE_TICK, LIMIT + 1, true))
            .expect("projection");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("read-only")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("redaction")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("expired")));
    }

    fn grant() -> SubscriptionGrant {
        SubscriptionGrant {
            holder_ref: test_ref("holder"),
            resource_ref: test_ref("resource"),
            projection_kind: "catalog".to_string(),
            egress_policy_ref: test_ref("egress"),
            redaction_profile: "public".to_string(),
            resource_limit: LIMIT,
            expires_at_tick: CURRENT_TICK,
            revoked_refs: Vec::new(),
            read_authority_refs: vec![test_ref("read-authority")],
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource-policy")],
            scopes: vec!["catalog-read".to_string()],
        }
    }

    fn request(operation: &str, at_tick: u64, item_count: u64, contains_sensitive_content: bool) -> ProjectionRequest {
        ProjectionRequest {
            holder_ref: test_ref("holder"),
            operation: operation.to_string(),
            projection_kind: "catalog".to_string(),
            at_tick,
            item_count,
            required_authority_ref: test_ref("read-authority"),
            required_policy_ref: test_ref("policy"),
            required_resource_ref: test_ref("resource-policy"),
            requested_scope: "catalog-read".to_string(),
            contains_sensitive_content,
        }
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("subscriber-test-ref", vec![string(
            label,
        )]))
        .expect("test ref")
    }
}
