//! Authority delegation checks for ingress envelopes.

pub(super) fn evaluate_live_authority_delegation(
    root: &crate::node_state::DirectoryRoot,
    envelope: &super::transits::ControlIngressEnvelope,
) -> crate::error::Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(envelope.authority_refs.len().saturating_add(2));
    let mut admitted_grant_ref = None;
    let candidate_authority_refs = envelope
        .authority_refs
        .iter()
        .filter(|authority_ref| envelope.request.authority_refs.contains(*authority_ref))
        .collect::<Vec<_>>();
    if candidate_authority_refs.is_empty() {
        diagnostics.push("node control live authority refs are not bound to the request".to_string());
    }
    for authority_ref in candidate_authority_refs {
        match super::storage::read_ledger_artifact(root, authority_ref) {
            Ok(value) => match super::issuance::parse_control_authority_grant(&value) {
                Ok(grant) => {
                    let grant_diagnostics = authority_grant_diagnostics(envelope, &grant);
                    if grant_diagnostics.is_empty() {
                        admitted_grant_ref = Some(grant.grant_ref);
                        break;
                    }
                    diagnostics.extend(grant_diagnostics);
                }
                Err(error) => {
                    if let Some(diagnostic) = super::newcomers::transport_evidence_not_authority_diagnostic(
                        &value,
                        authority_ref,
                        "node control authority ref",
                        "authority",
                    ) {
                        diagnostics.push(diagnostic);
                    } else {
                        diagnostics.push(format!("node control authority ref {authority_ref} is not a grant: {error}"));
                    }
                }
            },
            Err(error) => diagnostics.push(format!("node control authority grant {authority_ref} not found: {error}")),
        }
    }
    if admitted_grant_ref.is_none() {
        diagnostics.push("node control live authority delegation missing admitted grant".to_string());
    }
    let decision = if admitted_grant_ref.is_some() { "pass" } else { "deny" };
    let receipt_value = super::stamps::receipt_value(&super::stamps::AuthorityReceiptValueInput {
        decision,
        envelope,
        grant_ref: admitted_grant_ref.as_deref(),
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    super::storage::write_preserves(
        root,
        &super::paths::control_authority_receipt_path(&envelope.envelope_ref)?,
        &receipt_value,
    )?;
    super::storage::import_artifact(root, &receipt_value)?;
    if decision == "deny" {
        diagnostics.push(format!("node control authority receipt {receipt_ref} denied"));
    }
    Ok(diagnostics)
}

pub(super) fn authority_grant_diagnostics(
    envelope: &super::transits::ControlIngressEnvelope,
    grant: &super::entitlements::ControlAuthorityGrant,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(8);
    if grant.peer_id != envelope.from_peer {
        diagnostics.push(format!(
            "node control authority grant {} peer {} does not match {}",
            grant.grant_ref, grant.peer_id, envelope.from_peer
        ));
    }
    if grant.node_id != envelope.to_node {
        diagnostics.push(format!(
            "node control authority grant {} node {} does not match {}",
            grant.grant_ref, grant.node_id, envelope.to_node
        ));
    }
    if !grant
        .operations
        .iter()
        .any(|operation| operation == "*" || operation == &envelope.request.operation)
    {
        diagnostics.push(format!(
            "node control authority grant {} does not allow operation {}",
            grant.grant_ref, envelope.request.operation
        ));
    }
    if grant.epoch > envelope.sequence {
        diagnostics
            .push(format!("node control authority grant {} is not valid until epoch {}", grant.grant_ref, grant.epoch));
    }
    if let Some(expires_at) = grant.expires_at
        && expires_at < envelope.sequence
    {
        diagnostics.push(format!("node control authority grant {} expired at epoch {expires_at}", grant.grant_ref));
    }
    if !grant.revocation_refs.is_empty() {
        diagnostics.push(format!("node control authority grant {} has revocation refs", grant.grant_ref));
    }
    if !scope_matches_request(
        &grant.target_scope,
        envelope.request.target_ref.as_deref(),
        envelope.request.payload_ref.as_deref(),
    ) {
        diagnostics.push(format!(
            "node control authority grant {} target scope {} does not match request",
            grant.grant_ref, grant.target_scope
        ));
    }
    if !scope_matches_refs(&grant.resource_scope, &envelope.resource_refs, &envelope.request.resource_refs) {
        diagnostics.push(format!(
            "node control authority grant {} resource scope {} does not match request",
            grant.grant_ref, grant.resource_scope
        ));
    }
    diagnostics
}

fn scope_matches_request(scope: &str, target_ref: Option<&str>, payload_ref: Option<&str>) -> bool {
    scope == "*" || target_ref == Some(scope) || payload_ref == Some(scope)
}

fn scope_matches_refs(scope: &str, envelope_refs: &[String], request_refs: &[String]) -> bool {
    scope == "*"
        || envelope_refs.iter().any(|reference| reference == scope)
        || request_refs.iter().any(|reference| reference == scope)
}

pub(super) fn ingress_idempotency_evidence_refs(envelope: &super::transits::ControlIngressEnvelope) -> Vec<String> {
    let mut refs = Vec::with_capacity(
        envelope.peer_bootstrap_refs.len()
            + envelope.authority_refs.len()
            + envelope.resource_refs.len()
            + envelope.evidence_refs.len(),
    );
    refs.extend(envelope.peer_bootstrap_refs.iter().cloned());
    refs.extend(envelope.authority_refs.iter().cloned());
    refs.extend(envelope.resource_refs.iter().cloned());
    refs.extend(envelope.evidence_refs.iter().cloned());
    refs.sort();
    refs.dedup();
    refs
}
