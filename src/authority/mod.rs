use preserves::IOValue;
use preserves::Value;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::AUTHORITY_CONTEXT_SCHEMA;
use crate::preserves_rail::AUTHORITY_IDENTITY_SCHEMA;
use crate::preserves_rail::AUTHORITY_RECEIPT_SCHEMA;
use crate::preserves_rail::AUTHORITY_REVOCATION_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::validate_content_ref;
use crate::preserves_rail::value_to_iovalue;

type RuntimeAssertion = crate::runtime::RuntimeAssertion;
#[cfg(test)]
type RuntimeValue = crate::runtime::RuntimeValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityIdentity {
    pub identity_ref: String,
    pub identity_type: String,
    pub id: String,
    pub display_name: String,
    pub key_refs: Vec<String>,
    pub parent_refs: Vec<String>,
    pub metadata_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityCapability {
    pub capability: String,
    pub scope: String,
    pub attenuation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityContext {
    pub context_ref: String,
    pub subject_ref: String,
    pub capabilities: Vec<AuthorityCapability>,
    pub delegation_refs: Vec<String>,
    pub not_before: Option<u64>,
    pub expires_at: Option<u64>,
    pub revocation_refs: Vec<String>,
    pub key_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityRevocation {
    pub revocation_ref: String,
    pub target_kind: String,
    pub target_ref: String,
    pub reason: String,
    pub effective_at: u64,
    pub issuer_ref: String,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub authority_context_ref: Option<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityAdmission {
    pub decision: String,
    pub receipt: AuthorityReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityLiveRef {
    pub live_ref: String,
    pub authority_context_ref: String,
    pub scope: String,
    pub attenuation: String,
    pub expires_at: Option<u64>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct IdentityValueInput<'a> {
    pub identity_type: &'a str,
    pub id: &'a str,
    pub display_name: &'a str,
    pub key_refs: &'a [String],
    pub parent_refs: &'a [String],
    pub metadata_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ContextValueInput<'a> {
    pub subject_ref: &'a str,
    pub capabilities: &'a [AuthorityCapability],
    pub delegation_refs: &'a [String],
    pub not_before: Option<u64>,
    pub expires_at: Option<u64>,
    pub revocation_refs: &'a [String],
    pub key_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct RevocationValueInput<'a> {
    pub target_kind: &'a str,
    pub target_ref: &'a str,
    pub reason: &'a str,
    pub effective_at: u64,
    pub issuer_ref: &'a str,
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ReceiptValueInput<'a> {
    pub operation: &'a str,
    pub decision: &'a str,
    pub authority_context_ref: Option<&'a str>,
    pub capability: &'a str,
    pub scope: &'a str,
    pub logical_time: u64,
    pub diagnostics: &'a [&'a str],
}

pub fn authority_identity_value(input: IdentityValueInput<'_>) -> Result<IOValue> {
    validate_identity_type(input.identity_type)?;
    validate_non_empty(input.id, "authority identity id")?;
    validate_non_empty(input.display_name, "authority identity display name")?;
    validate_refs(input.key_refs, "authority identity key ref")?;
    validate_refs(input.parent_refs, "authority identity parent ref")?;
    validate_refs(input.metadata_refs, "authority identity metadata ref")?;
    Ok(record("authority-identity-v1", vec![
        string(AUTHORITY_IDENTITY_SCHEMA),
        record("identity", vec![
            record("type", vec![string(input.identity_type)]),
            record("id", vec![string(input.id)]),
            record("display-name", vec![string(input.display_name)]),
        ]),
        record("keys", vec![sequence(input.key_refs.iter().map(string).collect())]),
        record("parents", vec![sequence(input.parent_refs.iter().map(string).collect())]),
        record("metadata", vec![sequence(input.metadata_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("names-are-metadata"), string("pass")]),
            record("check", vec![string("identity-alone-grants-no-authority"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_authority_identity(value: &IOValue) -> Result<AuthorityIdentity> {
    let fields = value
        .collect_simple_record("authority-identity-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <authority-identity-v1 ...>"))?;
    require_schema(&fields[0], AUTHORITY_IDENTITY_SCHEMA, "authority identity schema")?;
    let identity = value_to_iovalue(&fields[1]);
    let identity_fields = identity
        .collect_simple_record("identity", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("authority identity missing identity field"))?;
    let checks = parse_checks(&fields[5])?;
    require_check(&checks, "identity-alone-grants-no-authority")?;
    Ok(AuthorityIdentity {
        identity_ref: canonical_hash(value)?,
        identity_type: record_string(&identity_fields[0], "type")?,
        id: record_string(&identity_fields[1], "id")?,
        display_name: record_string(&identity_fields[2], "display-name")?,
        key_refs: parse_ref_sequence(&fields[2], "keys")?,
        parent_refs: parse_ref_sequence(&fields[3], "parents")?,
        metadata_refs: parse_ref_sequence(&fields[4], "metadata")?,
        value: value.clone(),
    })
}

pub fn authority_context_value(input: ContextValueInput<'_>) -> Result<IOValue> {
    require_ref(input.subject_ref, "authority context subject ref")?;
    for capability in input.capabilities {
        validate_capability(capability)?;
    }
    validate_refs(input.delegation_refs, "authority context delegation ref")?;
    validate_refs(input.revocation_refs, "authority context revocation ref")?;
    validate_refs(input.key_refs, "authority context key ref")?;
    validate_refs(input.policy_refs, "authority context policy ref")?;
    validate_refs(input.evidence_refs, "authority context evidence ref")?;
    Ok(record("authority-context-v1", vec![
        string(AUTHORITY_CONTEXT_SCHEMA),
        record("subject", vec![string(input.subject_ref)]),
        record("capabilities", vec![sequence(input.capabilities.iter().map(capability_value).collect())]),
        record("delegations", vec![sequence(input.delegation_refs.iter().map(string).collect())]),
        record("validity", vec![
            optional_u64_value(input.not_before),
            optional_u64_value(input.expires_at),
        ]),
        record("revocations", vec![sequence(input.revocation_refs.iter().map(string).collect())]),
        record("keys", vec![sequence(input.key_refs.iter().map(string).collect())]),
        record("policy", vec![sequence(input.policy_refs.iter().map(string).collect())]),
        record("evidence", vec![sequence(input.evidence_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("scoped-capabilities"), string("pass")]),
            record("check", vec![string("attenuation-monotonic"), string("pass")]),
            record("check", vec![string("revocation-checked-at-admission"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_authority_context(value: &IOValue) -> Result<AuthorityContext> {
    let fields = value
        .collect_simple_record("authority-context-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <authority-context-v1 ...>"))?;
    require_schema(&fields[0], AUTHORITY_CONTEXT_SCHEMA, "authority context schema")?;
    let validity = value_to_iovalue(&fields[4]);
    let validity_fields = validity
        .collect_simple_record("validity", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("authority context missing validity"))?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "revocation-checked-at-admission")?;
    Ok(AuthorityContext {
        context_ref: canonical_hash(value)?,
        subject_ref: record_string(&fields[1], "subject")?,
        capabilities: parse_capability_sequence(&fields[2], "capabilities")?,
        delegation_refs: parse_ref_sequence(&fields[3], "delegations")?,
        not_before: parse_optional_u64_value(&validity_fields[0])?,
        expires_at: parse_optional_u64_value(&validity_fields[1])?,
        revocation_refs: parse_ref_sequence(&fields[5], "revocations")?,
        key_refs: parse_ref_sequence(&fields[6], "keys")?,
        policy_refs: parse_ref_sequence(&fields[7], "policy")?,
        evidence_refs: parse_ref_sequence(&fields[8], "evidence")?,
        value: value.clone(),
    })
}

pub fn revocation_value(input: RevocationValueInput<'_>) -> Result<IOValue> {
    validate_revocation_target(input.target_kind)?;
    require_ref(input.target_ref, "authority revocation target ref")?;
    validate_non_empty(input.reason, "authority revocation reason")?;
    require_ref(input.issuer_ref, "authority revocation issuer ref")?;
    validate_refs(input.evidence_refs, "authority revocation evidence ref")?;
    Ok(record("authority-revocation-v1", vec![
        string(AUTHORITY_REVOCATION_SCHEMA),
        record("target", vec![
            record("kind", vec![string(input.target_kind)]),
            record("ref", vec![string(input.target_ref)]),
        ]),
        record("reason", vec![string(input.reason)]),
        record("effective-at", vec![u64_value(input.effective_at)]),
        record("issuer", vec![string(input.issuer_ref)]),
        record("evidence", vec![sequence(input.evidence_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("revocation-target-bound"), string("pass")]),
            record("check", vec![string("authority-cleanup-required"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_revocation(value: &IOValue) -> Result<AuthorityRevocation> {
    let fields = value
        .collect_simple_record("authority-revocation-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <authority-revocation-v1 ...>"))?;
    require_schema(&fields[0], AUTHORITY_REVOCATION_SCHEMA, "authority revocation schema")?;
    let target = value_to_iovalue(&fields[1]);
    let target_fields = target
        .collect_simple_record("target", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("authority revocation missing target"))?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "authority-cleanup-required")?;
    Ok(AuthorityRevocation {
        revocation_ref: canonical_hash(value)?,
        target_kind: record_string(&target_fields[0], "kind")?,
        target_ref: record_string(&target_fields[1], "ref")?,
        reason: record_string(&fields[2], "reason")?,
        effective_at: record_u64(&fields[3], "effective-at")?,
        issuer_ref: record_string(&fields[4], "issuer")?,
        evidence_refs: parse_ref_sequence(&fields[5], "evidence")?,
        value: value.clone(),
    })
}

pub fn admit_authority(
    context_value: &IOValue,
    requested_capability: &str,
    requested_scope: &str,
    logical_time: u64,
    revocation_values: &[IOValue],
) -> Result<AuthorityAdmission> {
    let context = parse_authority_context(context_value)?;
    let has_revocation_hit = revocation_values
        .iter()
        .map(parse_revocation)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .any(|revocation| {
            revocation.effective_at <= logical_time
                && (revocation.target_ref == context.context_ref
                    || revocation.target_ref == context.subject_ref
                    || context.delegation_refs.iter().any(|delegation| delegation == &revocation.target_ref)
                    || context.key_refs.iter().any(|key| key == &revocation.target_ref)
                    || context.capabilities.iter().any(|capability| {
                        capability_ref(&context.subject_ref, capability)
                            .is_ok_and(|capability_ref| capability_ref == revocation.target_ref)
                    }))
        });
    let is_context_expired = context.expires_at.is_some_and(|expires_at| logical_time >= expires_at);
    let is_before_validity_start = context.not_before.is_some_and(|not_before| logical_time < not_before);
    let has_matching_capability = context
        .capabilities
        .iter()
        .any(|capability| capability_allows(capability, requested_capability, requested_scope));
    let has_live_authority = !has_revocation_hit && !is_context_expired && !is_before_validity_start;
    let decision = if has_live_authority && has_matching_capability {
        "pass"
    } else {
        "fail"
    };
    let mut diagnostics = Vec::new();
    if has_revocation_hit {
        diagnostics.push("revoked");
    }
    if is_context_expired {
        diagnostics.push("expired");
    }
    if is_before_validity_start {
        diagnostics.push("not-yet-valid");
    }
    if !has_matching_capability {
        diagnostics.push("capability-denied");
    }
    let receipt_value = authority_receipt_value(ReceiptValueInput {
        operation: "admission",
        decision,
        authority_context_ref: Some(&context.context_ref),
        capability: requested_capability,
        scope: requested_scope,
        logical_time,
        diagnostics: &diagnostics,
    });
    Ok(AuthorityAdmission {
        decision: decision.to_string(),
        receipt: AuthorityReceipt {
            receipt_ref: canonical_hash(&receipt_value)?,
            operation: "admission".to_string(),
            decision: decision.to_string(),
            authority_context_ref: Some(context.context_ref),
            value: receipt_value,
        },
    })
}

pub fn gatekeeper_resolve_live_ref(
    context_value: &IOValue,
    scope: &str,
    requested_capability: &str,
    logical_time: u64,
    revocation_values: &[IOValue],
) -> Result<AuthorityLiveRef> {
    let admission = admit_authority(context_value, requested_capability, scope, logical_time, revocation_values)?;
    if admission.decision != "pass" {
        return Err(MoltenError::invalid_harness("gatekeeper resolution denied by authority context"));
    }
    let context = parse_authority_context(context_value)?;
    let expires_at = context.expires_at;
    let evidence_refs = vec![admission.receipt.receipt_ref];
    let value = record("authority-live-ref-v1", vec![
        string(crate::preserves_rail::AUTHORITY_LIVE_REF_SCHEMA),
        record("authority-context", vec![string(&context.context_ref)]),
        record("scope", vec![string(scope)]),
        record("capability", vec![string(requested_capability)]),
        record("attenuation", vec![string("scoped")]),
        record("expires-at", vec![optional_u64_value(expires_at)]),
        record("evidence", vec![sequence(evidence_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("gatekeeper-resolution"), string("pass")]),
            record("check", vec![string("scoped-live-ref"), string("pass")]),
            record("check", vec![string("expiry-bound"), string("pass")]),
        ])]),
    ]);
    Ok(AuthorityLiveRef {
        live_ref: canonical_hash(&value)?,
        authority_context_ref: context.context_ref,
        scope: scope.to_string(),
        attenuation: "scoped".to_string(),
        expires_at,
        evidence_refs,
        value,
    })
}

pub fn cleanup_for_revocation(
    assertions: &[RuntimeAssertion],
    revocation_value: &IOValue,
    logical_time: u64,
) -> Result<(Vec<RuntimeAssertion>, AuthorityReceipt)> {
    let revocation = parse_revocation(revocation_value)?;
    let remaining = assertions
        .iter()
        .filter(|assertion| {
            let value = assertion.value.as_iovalue();
            let Some(record) = value.collect_simple_record("authority-bound-assertion", Some(2)) else {
                return true;
            };
            let Ok(authority_ref) = record_string(&record[0], "authority") else {
                return true;
            };
            authority_ref != revocation.target_ref
        })
        .cloned()
        .collect::<Vec<_>>();
    let removed = assertions.len().saturating_sub(remaining.len());
    let diagnostic = format!("cleanup-removed:{removed}");
    let diagnostics = [diagnostic.as_str()];
    let receipt_value = authority_receipt_value(ReceiptValueInput {
        operation: "cleanup",
        decision: "pass",
        authority_context_ref: Some(&revocation.target_ref),
        capability: "cleanup",
        scope: &revocation.target_kind,
        logical_time,
        diagnostics: &diagnostics,
    });
    Ok((remaining, AuthorityReceipt {
        receipt_ref: canonical_hash(&receipt_value)?,
        operation: "cleanup".to_string(),
        decision: "pass".to_string(),
        authority_context_ref: Some(revocation.target_ref),
        value: receipt_value,
    }))
}

pub fn replay_verify_authority_receipt(receipt: &AuthorityReceipt, context_value: &IOValue) -> Result<()> {
    let context = parse_authority_context(context_value)?;
    if receipt.authority_context_ref.as_deref() != Some(context.context_ref.as_str()) {
        return Err(MoltenError::invalid_harness("replay authority receipt does not bind recorded context"));
    }
    Ok(())
}

pub fn authority_receipt_value(input: ReceiptValueInput<'_>) -> IOValue {
    record("authority-receipt-v1", vec![
        string(AUTHORITY_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("authority-context", vec![optional_ref_value(input.authority_context_ref)]),
        record("request", vec![
            record("capability", vec![string(input.capability)]),
            record("scope", vec![string(input.scope)]),
            record("logical-time", vec![u64_value(input.logical_time)]),
        ]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("authority-context-recorded"), string("pass")]),
            record("check", vec![string("revocation-checked"), string("pass")]),
            record("check", vec![string("expiry-checked"), string("pass")]),
            record("check", vec![string("replay-does-not-mint-authority"), string("pass")]),
        ])]),
    ])
}

fn capability_value(capability: &AuthorityCapability) -> IOValue {
    record("capability", vec![
        record("name", vec![string(&capability.capability)]),
        record("scope", vec![string(&capability.scope)]),
        record("attenuation", vec![string(&capability.attenuation)]),
    ])
}

fn parse_capability(value: &IOValue) -> Result<AuthorityCapability> {
    let fields = value
        .collect_simple_record("capability", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected authority capability"))?;
    let capability = AuthorityCapability {
        capability: record_string(&fields[0], "name")?,
        scope: record_string(&fields[1], "scope")?,
        attenuation: record_string(&fields[2], "attenuation")?,
    };
    validate_capability(&capability)?;
    Ok(capability)
}

fn parse_capability_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<AuthorityCapability>> {
    let values = field_sequence(value, label)?;
    values.iter().map(|value| parse_capability(&value_to_iovalue(value))).collect()
}

fn capability_allows(capability: &AuthorityCapability, requested_capability: &str, requested_scope: &str) -> bool {
    capability.capability == requested_capability
        && (capability.scope == requested_scope || capability.scope == "*")
        && capability.attenuation != "deny"
}

fn capability_ref(subject_ref: &str, capability: &AuthorityCapability) -> Result<String> {
    canonical_hash(&record("authority-capability-ref", vec![string(subject_ref), capability_value(capability)]))
}

fn validate_identity_type(identity_type: &str) -> Result<()> {
    match identity_type {
        "principal" | "node" | "actor" | "service" | "session" | "artifact" | "execution" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported authority identity type {other}"))),
    }
}

fn validate_revocation_target(target_kind: &str) -> Result<()> {
    match target_kind {
        "key" | "principal" | "delegation" | "capability" | "live-ref" | "handler-binding" | "session" | "artifact"
        | "authority-context" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported authority revocation target {other}"))),
    }
}

fn validate_capability(capability: &AuthorityCapability) -> Result<()> {
    validate_non_empty(&capability.capability, "authority capability")?;
    validate_non_empty(&capability.scope, "authority capability scope")?;
    validate_non_empty(&capability.attenuation, "authority capability attenuation")
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

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_u64_value(value: Option<u64>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![u64_value(value)]))
}

fn parse_optional_u64_value(value: &Value<IOValue>) -> Result<Option<u64>> {
    let optional = value_to_iovalue(value);
    if optional.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = optional.collect_simple_record("some", Some(1)) {
        required_u64(&some[0], "optional u64").map(Some)
    } else {
        Err(MoltenError::invalid_harness("expected optional u64"))
    }
}

fn parse_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
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

fn field_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<Value<IOValue>>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(values.iter().cloned().collect())
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<(String, String)>> {
    let values = field_sequence(value, "checks")?;
    values
        .iter()
        .map(|check| {
            let check = value_to_iovalue(check);
            let fields = check
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected authority check"))?;
            Ok((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?))
        })
        .collect()
}

fn require_check(checks: &[(String, String)], name: &str) -> Result<()> {
    if checks.iter().any(|(check, status)| check == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("authority evidence missing passing {name} check")))
    }
}

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&fields[0], label)
}

fn record_u64(value: &Value<IOValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_u64(&fields[0], label)
}

fn require_schema(value: &Value<IOValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")));
    }
    Ok(())
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_u64(value: &Value<IOValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::*;

    #[test]
    fn identity_records_do_not_grant_authority_without_context() {
        let identity_value = authority_identity_value(IdentityValueInput {
            identity_type: "principal",
            id: "alice",
            display_name: "Alice",
            key_refs: &[],
            parent_refs: &[],
            metadata_refs: &[],
        })
        .expect("identity");
        let identity = parse_authority_identity(&identity_value).expect("parse identity");
        let receipt = authority_receipt_value(ReceiptValueInput {
            operation: "identity-check",
            decision: "fail",
            authority_context_ref: None,
            capability: "read",
            scope: "store",
            logical_time: 0,
            diagnostics: &["identity-only"],
        });
        assert_eq!(identity.identity_type, "principal");
        assert!(
            crate::preserves_rail::to_text(&identity.value)
                .expect("identity text")
                .contains("identity-alone-grants-no-authority")
        );
        assert!(crate::preserves_rail::to_text(&receipt).expect("receipt text").contains("identity-only"));
    }

    #[test]
    fn authority_context_admits_scoped_capability_and_denies_attenuation_mismatch() {
        let subject = ref_for("principal");
        let context_value = authority_context_value(ContextValueInput {
            subject_ref: &subject,
            capabilities: &[AuthorityCapability {
                capability: "read".to_string(),
                scope: "catalog:public".to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: None,
            expires_at: Some(10),
            revocation_refs: &[],
            key_refs: &[ref_for("key")],
            policy_refs: &[ref_for("policy")],
            evidence_refs: &[ref_for("evidence")],
        })
        .expect("context");
        let pass = admit_authority(&context_value, "read", "catalog:public", 1, &[]).expect("admit");
        assert_eq!(pass.decision, "pass");
        let fail = admit_authority(&context_value, "write", "catalog:public", 1, &[]).expect("deny");
        assert_eq!(fail.decision, "fail");
    }

    #[test]
    fn revocation_retracts_dependent_assertions_and_denies_future_effects() {
        let subject = ref_for("principal");
        let context_value = authority_context_value(ContextValueInput {
            subject_ref: &subject,
            capabilities: &[AuthorityCapability {
                capability: "effect:clock".to_string(),
                scope: "actor:a".to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: None,
            expires_at: None,
            revocation_refs: &[],
            key_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("context");
        let context = parse_authority_context(&context_value).expect("parse context");
        let revocation_value = revocation_value(RevocationValueInput {
            target_kind: "authority-context",
            target_ref: &context.context_ref,
            reason: "operator revoke",
            effective_at: 5,
            issuer_ref: &subject,
            evidence_refs: &[],
        })
        .expect("revocation");
        let before = admit_authority(&context_value, "effect:clock", "actor:a", 4, &[]).expect("before revoke");
        assert_eq!(before.decision, "pass");
        let after =
            admit_authority(&context_value, "effect:clock", "actor:a", 5, std::slice::from_ref(&revocation_value))
                .expect("after revoke");
        assert_eq!(after.decision, "fail");
        let assertion = RuntimeAssertion {
            actor: "authority".to_string(),
            value: RuntimeValue::new(record("authority-bound-assertion", vec![
                record("authority", vec![string(&context.context_ref)]),
                record("value", vec![string("visible")]),
            ]))
            .expect("assertion value"),
        };
        let (remaining, cleanup) = cleanup_for_revocation(&[assertion], &revocation_value, 5).expect("cleanup");
        assert!(remaining.is_empty());
        assert_eq!(cleanup.decision, "pass");
    }

    #[test]
    fn expiry_and_gatekeeper_live_refs_are_enforced() {
        let subject = ref_for("principal");
        let context_value = authority_context_value(ContextValueInput {
            subject_ref: &subject,
            capabilities: &[AuthorityCapability {
                capability: "resolve".to_string(),
                scope: "service:db".to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: Some(2),
            expires_at: Some(5),
            revocation_refs: &[],
            key_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("context");
        let early = admit_authority(&context_value, "resolve", "service:db", 1, &[]).expect("early");
        assert_eq!(early.decision, "fail");
        let live = gatekeeper_resolve_live_ref(&context_value, "service:db", "resolve", 2, &[]).expect("live ref");
        assert_eq!(live.scope, "service:db");
        let expired = admit_authority(&context_value, "resolve", "service:db", 5, &[]).expect("expired");
        assert_eq!(expired.decision, "fail");
    }

    #[test]
    fn key_rotation_preserves_historical_replay_but_not_current_authority() {
        let subject = ref_for("principal");
        let old_key = ref_for("old-key");
        let new_key = ref_for("new-key");
        let context_value = authority_context_value(ContextValueInput {
            subject_ref: &subject,
            capabilities: &[AuthorityCapability {
                capability: "sign".to_string(),
                scope: "receipt".to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: None,
            expires_at: None,
            revocation_refs: &[],
            key_refs: std::slice::from_ref(&old_key),
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("old context");
        let context = parse_authority_context(&context_value).expect("context");
        let historical = admit_authority(&context_value, "sign", "receipt", 1, &[]).expect("historical admission");
        replay_verify_authority_receipt(&historical.receipt, &context_value).expect("historical replay");
        let new_key_refs = [new_key];
        let revoke_old_key = revocation_value(RevocationValueInput {
            target_kind: "key",
            target_ref: &old_key,
            reason: "rotate",
            effective_at: 2,
            issuer_ref: &subject,
            evidence_refs: &new_key_refs,
        })
        .expect("rotate");
        let current = admit_authority(&context_value, "sign", "receipt", 2, &[revoke_old_key]).expect("current denied");
        assert_eq!(current.decision, "fail");
        assert_eq!(historical.receipt.authority_context_ref.as_deref(), Some(context.context_ref.as_str()));
    }

    #[test]
    fn storage_remote_catalog_contexts_share_admission_path() {
        let subject = ref_for("principal");
        let context_value = authority_context_value(ContextValueInput {
            subject_ref: &subject,
            capabilities: &[
                AuthorityCapability {
                    capability: "storage:read".to_string(),
                    scope: "store:typed".to_string(),
                    attenuation: "scoped".to_string(),
                },
                AuthorityCapability {
                    capability: "remote-sync:pull".to_string(),
                    scope: "catalog:public".to_string(),
                    attenuation: "scoped".to_string(),
                },
                AuthorityCapability {
                    capability: "catalog:visible".to_string(),
                    scope: "catalog:public".to_string(),
                    attenuation: "scoped".to_string(),
                },
            ],
            delegation_refs: &[],
            not_before: None,
            expires_at: None,
            revocation_refs: &[],
            key_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("context");
        assert_eq!(
            admit_authority(&context_value, "storage:read", "store:typed", 0, &[]).expect("storage").decision,
            "pass"
        );
        assert_eq!(
            admit_authority(&context_value, "remote-sync:pull", "catalog:public", 0, &[])
                .expect("remote")
                .decision,
            "pass"
        );
        assert_eq!(
            admit_authority(&context_value, "catalog:visible", "catalog:public", 0, &[])
                .expect("catalog")
                .decision,
            "pass"
        );
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_attenuation_monotonicity_identity_no_authority_and_cleanup(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let subject = ref_for(&format!("principal-{salt}"));
        let identity_id = format!("p-{salt}");
        let identity_value = authority_identity_value(IdentityValueInput {
            identity_type: "principal",
            id: &identity_id,
            display_name: "principal",
            key_refs: &[],
            parent_refs: &[],
            metadata_refs: &[],
        })
        .expect("identity");
        parse_authority_identity(&identity_value).expect("identity parses");
        let no_context_receipt = authority_receipt_value(ReceiptValueInput {
            operation: "identity-only",
            decision: "fail",
            authority_context_ref: None,
            capability: "read",
            scope: "scope",
            logical_time: salt,
            diagnostics: &[],
        });
        assert!(crate::preserves_rail::to_text(&no_context_receipt).expect("receipt text").contains("fail"));

        let scope = format!("scope:{salt}");
        let context_value = authority_context_value(ContextValueInput {
            subject_ref: &subject,
            capabilities: &[AuthorityCapability {
                capability: "read".to_string(),
                scope: scope.clone(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: None,
            expires_at: None,
            revocation_refs: &[],
            key_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("context");
        assert_eq!(admit_authority(&context_value, "read", &scope, salt, &[]).expect("same scope").decision, "pass");
        assert_eq!(
            admit_authority(&context_value, "read", "other-scope", salt, &[]).expect("other scope").decision,
            "fail"
        );
        let context = parse_authority_context(&context_value).expect("context");
        let revocation = revocation_value(RevocationValueInput {
            target_kind: "authority-context",
            target_ref: &context.context_ref,
            reason: "cleanup",
            effective_at: salt,
            issuer_ref: &subject,
            evidence_refs: &[],
        })
        .expect("revocation");
        let assertion = RuntimeAssertion {
            actor: "authority".to_string(),
            value: RuntimeValue::new(record("authority-bound-assertion", vec![
                record("authority", vec![string(&context.context_ref)]),
                record("value", vec![string("x")]),
            ]))
            .expect("assertion value"),
        };
        let (remaining, _) = cleanup_for_revocation(&[assertion], &revocation, salt).expect("cleanup");
        assert!(remaining.is_empty());
    }

    fn ref_for(label: &str) -> String {
        canonical_hash(&record("authority-test-ref", vec![string(label)])).expect("test ref")
    }
}
