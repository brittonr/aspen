
fn parse_capability_kind(value: &str) -> Result<CapabilityKind> {
    match value {
        "public-artifact" => Ok(CapabilityKind::PublicArtifact),
        "scoped-service" => Ok(CapabilityKind::ScopedService),
        "exclusive-lease" => Ok(CapabilityKind::ExclusiveLease),
        "external-effect" => Ok(CapabilityKind::ExternalEffect),
        "deferred-effect" => Ok(CapabilityKind::DeferredEffect),
        "host-secret" => Ok(CapabilityKind::HostSecret),
        "bearer-credential" => Ok(CapabilityKind::BearerCredential),
        _ => Err(MoltenError::invalid_harness("unknown world authority capability kind")),
    }
}

fn parse_action(value: &str) -> Result<WorldBranchAction> {
    match value {
        "create" => Ok(WorldBranchAction::Create),
        "activate" => Ok(WorldBranchAction::Activate),
        "promote" => Ok(WorldBranchAction::Promote),
        "simulate" => Ok(WorldBranchAction::Simulate),
        "transfer" => Ok(WorldBranchAction::Transfer),
        _ => Err(MoltenError::invalid_harness("unknown world authority action")),
    }
}

fn read_bounded(path: &Path, maximum_bytes: u64, label: &str) -> Result<Vec<u8>> {
    let metadata = std::fs::metadata(path).map_err(MoltenError::from)?;
    if metadata.len() > maximum_bytes {
        return Err(MoltenError::invalid_harness(format!("{label} exceeds the reviewed byte bound")));
    }
    let bytes = std::fs::read(path).map_err(MoltenError::from)?;
    let observed =
        u64::try_from(bytes.len()).map_err(|_| MoltenError::invalid_harness(format!("{label} length exceeds u64")))?;
    if observed > maximum_bytes {
        return Err(MoltenError::invalid_harness(format!("{label} changed beyond the reviewed byte bound")));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content_ref(label: &str) -> String {
        let mut hasher = blake3::Hasher::new_derive_key("onixresearch.molten.world-authority-cli-test.v1");
        hasher.update(label.as_bytes());
        format!("blake3:{}", hasher.finalize().to_hex())
    }

    fn request_json(extra: &str) -> Vec<u8> {
        format!(
            r#"{{
  "schema":"{OPERATOR_REQUEST_SCHEMA}",
  "capability_kind":"public-artifact",
  "action":"create",
  "source_branch_ref":"{}",
  "destination_branch_ref":"{}",
  "capability_ref":"{}",
  "source_scope":{{"resource":"artifact/root","abilities":["read"],"limit":null}},
  "destination_scope":{{"resource":"artifact/root","abilities":["read"],"limit":null}},
  "policy_generation":1,
  "mapping_lossless":true,
  "current":{{
    "observation_ref":"{}",
    "policy":true,
    "capability":true,
    "revocation":true,
    "replay":true,
    "scope":true,
    "ucan_verified":true
  }}{extra}
}}"#,
            content_ref("source"),
            content_ref("destination"),
            content_ref("capability"),
            content_ref("current"),
        )
        .into_bytes()
    }

    #[test]
    fn operator_request_maps_to_closed_public_facts() {
        let request = parse_request(&request_json("")).expect("valid request");
        let (facts, current) = request.into_facts().expect("closed facts");
        assert_eq!(facts.capability_kind, CapabilityKind::PublicArtifact);
        assert_eq!(facts.action, WorldBranchAction::Create);
        assert!(facts.mapping_lossless);
        assert!(current.all_current());
        assert!(current.ucan_verified);
    }

    #[test]
    fn unknown_fields_kinds_and_bearer_text_fail_closed() {
        let unknown = parse_request(&request_json(",\"unexpected\":true"));
        assert!(unknown.is_err());

        let mut request = parse_request(&request_json("")).expect("valid request");
        request.capability_kind = "ambient-superuser".to_string();
        assert!(request.into_facts().is_err());

        let mut request = parse_request(&request_json("")).expect("valid request");
        request.source_scope.resource = "secret=private".to_string();
        assert!(request.into_facts().is_err());
    }

    fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|window| window == needle)
    }

    #[test]
    fn runtime_denial_receipt_excludes_raw_scope_and_policy_material() {
        let request = parse_request(&request_json("")).expect("valid request");
        let (facts, current) = request.into_facts().expect("closed facts");
        let plan = plan_world_branch_authority(
            basalt::world_branch_authority::DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON,
            &facts,
            &current,
        );
        assert!(plan.allowed);
        let denied = deny_world_branch_authority_plan(plan, WorldBranchAuthorityDiagnostic::MissingObligationEvidence);
        let receipt = plan_receipt(&denied);
        let mut secret_diagnostic = receipt.clone();
        secret_diagnostic.diagnostic = "secret=private".to_string();
        assert!(encode_receipt(&secret_diagnostic).is_err());
        let mut weakened = receipt.clone();
        weakened.non_claims.pop();
        assert!(encode_receipt(&weakened).is_err());

        let (_, bytes) = encode_receipt(&receipt).expect("canonical denial receipt");
        molten::preserves_rail::strict_canonical_decode(&bytes).expect("strict canonical Preserves receipt");
        assert!(!contains_bytes(&bytes, b"artifact/root"));
        assert!(!contains_bytes(&bytes, b"secret="));
        assert!(!contains_bytes(&bytes, b"bearer-token="));
        assert!(contains_bytes(&bytes, b"missing-obligation-evidence"));
    }
}
