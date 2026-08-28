use std::collections::BTreeSet;

use super::super::super::valid_ref;
use super::super::super::valid_token;
use super::super::NativeExecutableEvidence;
use super::super::NativeIngressEnvelope;
use super::NativeHostIssue;

pub(super) fn executable_refs(evidence: &NativeExecutableEvidence) -> [(&'static str, &str); 14] {
    [
        ("executable-ref", &evidence.executable_ref),
        ("executable-bytes-ref", &evidence.executable_bytes_ref),
        ("artifact-kind-ref", &evidence.artifact_kind_ref),
        ("target-ref", &evidence.target_ref),
        ("dependency-closure-ref", &evidence.dependency_closure_ref),
        ("materialization-ref", &evidence.materialization_ref),
        ("provenance-ref", &evidence.provenance_ref),
        ("source-gate-ref", &evidence.source_gate_ref),
        ("policy-ref", &evidence.policy_ref),
        ("authority-ref", &evidence.authority_ref),
        ("resource-ref", &evidence.resource_ref),
        ("execution-profile-ref", &evidence.execution_profile_ref),
        ("manifest-ref", &evidence.manifest_ref),
        ("state-schema-ref", &evidence.state_schema_ref),
    ]
}

pub(super) fn ingress_refs(ingress: &NativeIngressEnvelope) -> [(&'static str, &str); 8] {
    [
        ("request-ref", &ingress.request_ref),
        ("endpoint-ref", &ingress.endpoint_ref),
        ("peer-ref", &ingress.peer_ref),
        ("manifest-ref", &ingress.manifest_ref),
        ("authority-ref", &ingress.authority_ref),
        ("policy-ref", &ingress.policy_ref),
        ("resource-ref", &ingress.resource_ref),
        ("payload-ref", &ingress.payload.value_ref),
    ]
}

pub(super) fn validate_token(field: &'static str, value: &str, issues: &mut Vec<NativeHostIssue>) {
    if value.is_empty() {
        issues.push(NativeHostIssue::EmptyField(field));
    } else if !valid_token(value) {
        issues.push(NativeHostIssue::MalformedToken {
            field,
            value: value.to_string(),
        });
    }
}

pub(super) fn validate_ref(field: &'static str, value: &str, issues: &mut Vec<NativeHostIssue>) {
    if value.is_empty() {
        issues.push(NativeHostIssue::EmptyField(field));
    } else if !valid_ref(value) {
        issues.push(NativeHostIssue::MalformedRef {
            field,
            value: value.to_string(),
        });
    }
}

pub(super) fn has_duplicates<T: Ord + Clone>(values: &[T]) -> bool {
    let mut seen = BTreeSet::new();
    values.iter().any(|value| !seen.insert(value.clone()))
}

pub fn native_identity_ref(parts: &[&str]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"molten-native-host-v2\0");
    for part in parts {
        hasher.update(part.len().to_string().as_bytes());
        hasher.update(b":");
        hasher.update(part.as_bytes());
        hasher.update(b"\0");
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}
