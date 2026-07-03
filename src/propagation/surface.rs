type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;

const EVENTUAL_SURFACE_SCHEMA: &str = "molten.eventual-surface-manifest.v1";
const EVENTUAL_SURFACE_RECEIPT_SCHEMA: &str = "molten.eventual-surface-receipt.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventualSurfaceManifest {
    pub scope: String,
    pub carrier: String,
    pub payload_schema: String,
    pub idempotency_key_ref: String,
    pub merge_law: String,
    pub tombstone_policy: String,
    pub anti_entropy_policy: String,
    pub replay_evidence_refs: Vec<String>,
    pub authority_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventualSurfaceInput {
    pub manifest: EventualSurfaceManifest,
    pub delivered_refs: Vec<String>,
    pub merged_refs: Vec<String>,
    pub missing_refs: Vec<String>,
    pub live_timing_observed: bool,
    pub claims_authority: bool,
    pub receiver_verified_import: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventualSurfaceReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
    pub receipt_ref: String,
}

pub fn eventual_surface_manifest_value(manifest: &EventualSurfaceManifest) -> IoValue {
    crate::preserves_rail::record("eventual-surface-manifest-v1", vec![
        string(EVENTUAL_SURFACE_SCHEMA),
        field("scope", &manifest.scope),
        field("carrier", &manifest.carrier),
        field("payload-schema", &manifest.payload_schema),
        field("idempotency-key-ref", &manifest.idempotency_key_ref),
        field("merge-law", &manifest.merge_law),
        field("tombstone-policy", &manifest.tombstone_policy),
        field("anti-entropy-policy", &manifest.anti_entropy_policy),
        list_field("replay-evidence-refs", &manifest.replay_evidence_refs),
        list_field("authority-refs", &manifest.authority_refs),
    ])
}

pub fn validate_eventual_surface(input: &EventualSurfaceInput) -> Result<EventualSurfaceReceipt> {
    let mut diagnostics = Vec::new();
    if input.manifest.merge_law != "lww" && input.manifest.merge_law != "orset" {
        diagnostics.push("missing deterministic merge law".to_string());
    }
    if input.manifest.tombstone_policy == "none" && !input.missing_refs.is_empty() {
        diagnostics.push("stale tombstone without retention policy".to_string());
    }
    if input.live_timing_observed && input.manifest.replay_evidence_refs.is_empty() {
        diagnostics.push("unrecorded live timing cannot satisfy deterministic pass gates".to_string());
    }
    if input.claims_authority {
        diagnostics.push("propagation surface is not consensus or authority".to_string());
    }
    if !input.receiver_verified_import && !input.delivered_refs.is_empty() {
        diagnostics.push("remote sync import requires receiver-side verification".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = receipt_value(decision, &diagnostics, input);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(EventualSurfaceReceipt {
        decision: decision.to_string(),
        diagnostics,
        value,
        receipt_ref,
    })
}

pub fn anti_entropy_status_assertion(input: &EventualSurfaceInput) -> IoValue {
    crate::preserves_rail::record("eventual-surface-status-v1", vec![
        field("scope", &input.manifest.scope),
        list_field("delivered-refs", &input.delivered_refs),
        list_field("merged-refs", &input.merged_refs),
        list_field("missing-refs", &input.missing_refs),
        field("authoritative", if input.claims_authority { "false-denied" } else { "no" }),
    ])
}

fn receipt_value(decision: &str, diagnostics: &[String], input: &EventualSurfaceInput) -> IoValue {
    crate::preserves_rail::record("eventual-surface-receipt-v1", vec![
        string(EVENTUAL_SURFACE_RECEIPT_SCHEMA),
        field("decision", decision),
        field("scope", &input.manifest.scope),
        field("carrier", &input.manifest.carrier),
        list_field("delivered-refs", &input.delivered_refs),
        list_field("merged-refs", &input.merged_refs),
        list_field("diagnostics", diagnostics),
        field("boundary", "propagation-not-consensus-or-authority"),
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

    #[test]
    fn eventual_surface_passes_with_replayable_merge_law() {
        let input = input("lww", false, true, false);
        let receipt = validate_eventual_surface(&input).expect("surface");
        assert_eq!(receipt.decision, "pass");
        assert!(
            crate::preserves_rail::to_text(&anti_entropy_status_assertion(&input))
                .expect("status")
                .contains("delivered-refs")
        );
    }

    #[test]
    fn eventual_surface_denies_unrecorded_live_authority_claims() {
        let mut input = input("none", true, false, true);
        input.manifest.replay_evidence_refs.clear();
        input.missing_refs = vec![test_ref("missing")];
        let receipt = validate_eventual_surface(&input).expect("surface");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("merge law")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("live timing")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("not consensus")));
    }

    fn input(
        merge_law: &str,
        live_timing_observed: bool,
        receiver_verified_import: bool,
        claims_authority: bool,
    ) -> EventualSurfaceInput {
        EventualSurfaceInput {
            manifest: EventualSurfaceManifest {
                scope: "topic:services".to_string(),
                carrier: "iroh-gossip".to_string(),
                payload_schema: "service-ready".to_string(),
                idempotency_key_ref: test_ref("operation"),
                merge_law: merge_law.to_string(),
                tombstone_policy: "retain-tombstones".to_string(),
                anti_entropy_policy: "pull-missing".to_string(),
                replay_evidence_refs: vec![test_ref("replay")],
                authority_refs: vec![test_ref("authority")],
            },
            delivered_refs: vec![test_ref("delivered")],
            merged_refs: vec![test_ref("merged")],
            missing_refs: Vec::new(),
            live_timing_observed,
            claims_authority,
            receiver_verified_import,
        }
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("surface-test-ref", vec![string(label)]))
            .expect("test ref")
    }
}
