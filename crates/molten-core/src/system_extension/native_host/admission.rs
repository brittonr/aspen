use std::collections::BTreeSet;

use super::super::valid_ref;
use super::super::valid_token;
use super::*;

const MAX_PROFILE_ISSUES: usize = 16;
const MAX_EXECUTABLE_ISSUES: usize = 24;
const MAX_INGRESS_ISSUES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeHostIssue {
    SchemaMismatch {
        field: &'static str,
        actual: String,
        expected: &'static str,
    },
    EmptyField(&'static str),
    MalformedToken {
        field: &'static str,
        value: String,
    },
    MalformedRef {
        field: &'static str,
        value: String,
    },
    DuplicateValue(&'static str),
    ZeroBound(&'static str),
    MissingNonClaim(NativeHostNonClaim),
    NotLocalLivePilot,
    ProfileMismatch(&'static str),
    IdentityMismatch(&'static str),
    StaleGeneration {
        actual: u64,
        active: u64,
    },
    CallbackBytesExceeded {
        actual: u64,
        maximum: u64,
    },
    IngressStopped,
    TooManyPortBindings {
        actual: usize,
        maximum: usize,
    },
    TooManyUnresolvedOperations {
        actual: usize,
        maximum: usize,
    },
    OperationCountOverflow,
    DuplicateOperation(String),
    OperationNotFound(String),
    OperationKindMismatch,
    InvalidOperationTransition {
        previous: NativeOperationState,
        next: NativeOperationState,
    },
    CompletionAlreadyConsumed(String),
    RemovalBlocked(Vec<String>),
}

// r[impl molten.system_extension.native_host.profile]
// r[impl molten.system_extension.native_host.nonclaims]
pub fn admit_native_host_profile(
    profile: &NativeHostProfile,
) -> Result<AdmittedNativeHostProfile, Vec<NativeHostIssue>> {
    let mut issues = Vec::with_capacity(MAX_PROFILE_ISSUES);
    if profile.schema != NATIVE_HOST_PROFILE_SCHEMA {
        issues.push(NativeHostIssue::SchemaMismatch {
            field: "profile-schema",
            actual: profile.schema.clone(),
            expected: NATIVE_HOST_PROFILE_SCHEMA,
        });
    }
    validate_token("profile-id", &profile.profile_id, &mut issues);
    for (field, value) in [
        ("profile-ref", profile.profile_ref.as_str()),
        ("execution-profile-ref", profile.execution_profile_ref.as_str()),
        ("transport-profile-ref", profile.transport_profile_ref.as_str()),
    ] {
        validate_ref(field, value, &mut issues);
    }
    if profile.alpn != NATIVE_ALPN {
        issues.push(NativeHostIssue::ProfileMismatch("alpn"));
    }
    if profile.framing != NATIVE_FRAMING {
        issues.push(NativeHostIssue::ProfileMismatch("framing"));
    }
    for (field, value) in [
        ("max-callback-input-bytes", profile.max_callback_input_bytes),
        ("max-callback-output-bytes", profile.max_callback_output_bytes),
        ("max-diagnostic-bytes", profile.max_diagnostic_bytes),
    ] {
        if value == 0 {
            issues.push(NativeHostIssue::ZeroBound(field));
        }
    }
    for (field, value) in [
        ("max-instances", profile.max_instances),
        ("max-unresolved-operations", profile.max_unresolved_operations),
        ("max-port-bindings", profile.max_port_bindings),
        ("max-policy-refs", profile.max_policy_refs),
    ] {
        if value == 0 {
            issues.push(NativeHostIssue::ZeroBound(field));
        }
    }
    if !profile.is_local_live_pilot {
        issues.push(NativeHostIssue::NotLocalLivePilot);
    }
    if has_duplicates(&profile.non_claims) {
        issues.push(NativeHostIssue::DuplicateValue("non-claims"));
    }
    for required in REQUIRED_NATIVE_HOST_NON_CLAIMS {
        if !profile.non_claims.contains(&required) {
            issues.push(NativeHostIssue::MissingNonClaim(required));
        }
    }
    if issues.is_empty() {
        Ok(AdmittedNativeHostProfile {
            profile: profile.clone(),
        })
    } else {
        Err(issues)
    }
}

// r[impl molten.system_extension.native_host.executable]
pub fn admit_native_executable(
    profile: &AdmittedNativeHostProfile,
    evidence: &NativeExecutableEvidence,
) -> Result<AdmittedNativeExecutable, Vec<NativeHostIssue>> {
    let mut issues = Vec::with_capacity(MAX_EXECUTABLE_ISSUES);
    if evidence.schema != NATIVE_EXECUTABLE_EVIDENCE_SCHEMA {
        issues.push(NativeHostIssue::SchemaMismatch {
            field: "executable-schema",
            actual: evidence.schema.clone(),
            expected: NATIVE_EXECUTABLE_EVIDENCE_SCHEMA,
        });
    }
    for (field, value) in executable_refs(evidence) {
        validate_ref(field, value, &mut issues);
    }
    if evidence.execution_profile_ref != profile.profile.execution_profile_ref {
        issues.push(NativeHostIssue::ProfileMismatch("execution-profile-ref"));
    }
    if evidence.port_binding_refs.len() > profile.profile.max_port_bindings {
        issues.push(NativeHostIssue::TooManyPortBindings {
            actual: evidence.port_binding_refs.len(),
            maximum: profile.profile.max_port_bindings,
        });
    }
    if has_duplicates(&evidence.port_binding_refs) {
        issues.push(NativeHostIssue::DuplicateValue("port-binding-refs"));
    }
    for reference in &evidence.port_binding_refs {
        validate_ref("port-binding-ref", reference, &mut issues);
    }
    if issues.is_empty() {
        Ok(AdmittedNativeExecutable {
            executable: evidence.clone(),
            profile_ref: profile.profile.profile_ref.clone(),
        })
    } else {
        Err(issues)
    }
}

// r[impl molten.system_extension.native_host.ingress]
pub fn admit_native_ingress(
    profile: &AdmittedNativeHostProfile,
    instance: &NativeInstanceRecord,
    ingress: &NativeIngressEnvelope,
) -> Result<NativeIngressAdmission, Vec<NativeHostIssue>> {
    let mut issues = Vec::with_capacity(MAX_INGRESS_ISSUES);
    if ingress.schema != NATIVE_INGRESS_SCHEMA {
        issues.push(NativeHostIssue::SchemaMismatch {
            field: "ingress-schema",
            actual: ingress.schema.clone(),
            expected: NATIVE_INGRESS_SCHEMA,
        });
    }
    for (field, value) in ingress_refs(ingress) {
        validate_ref(field, value, &mut issues);
    }
    for (field, is_matching) in [
        ("service-id", ingress.service_id == instance.service_id),
        ("manifest-ref", ingress.manifest_ref == instance.manifest_ref),
        ("transport-profile-ref", ingress.transport_profile_ref == profile.profile.transport_profile_ref),
        ("alpn", ingress.alpn == profile.profile.alpn),
        ("framing", ingress.framing == profile.profile.framing),
    ] {
        if !is_matching {
            issues.push(NativeHostIssue::IdentityMismatch(field));
        }
    }
    if ingress.generation != instance.lifecycle.generation {
        issues.push(NativeHostIssue::StaleGeneration {
            actual: ingress.generation,
            active: instance.lifecycle.generation,
        });
    }
    if ingress.accounted_bytes > profile.profile.max_callback_input_bytes {
        issues.push(NativeHostIssue::CallbackBytesExceeded {
            actual: ingress.accounted_bytes,
            maximum: profile.profile.max_callback_input_bytes,
        });
    }
    if !instance.is_accepting_ingress {
        issues.push(NativeHostIssue::IngressStopped);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(NativeIngressAdmission {
        request_ref: ingress.request_ref.clone(),
        generation: ingress.generation,
        acknowledgement_ref: native_identity_ref(&[
            "ingress-ack-v1",
            &ingress.request_ref,
            &ingress.service_id,
            &ingress.generation.to_string(),
        ]),
    })
}

fn executable_refs(evidence: &NativeExecutableEvidence) -> [(&'static str, &str); 14] {
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

fn ingress_refs(ingress: &NativeIngressEnvelope) -> [(&'static str, &str); 8] {
    [
        ("request-ref", &ingress.request_ref),
        ("endpoint-ref", &ingress.endpoint_ref),
        ("peer-ref", &ingress.peer_ref),
        ("manifest-ref", &ingress.manifest_ref),
        ("authority-ref", &ingress.authority_ref),
        ("policy-ref", &ingress.policy_ref),
        ("resource-ref", &ingress.resource_ref),
        ("payload-ref", &ingress.payload_ref),
    ]
}

fn validate_token(field: &'static str, value: &str, issues: &mut Vec<NativeHostIssue>) {
    if value.is_empty() {
        issues.push(NativeHostIssue::EmptyField(field));
    } else if !valid_token(value) {
        issues.push(NativeHostIssue::MalformedToken {
            field,
            value: value.to_string(),
        });
    }
}

fn validate_ref(field: &'static str, value: &str, issues: &mut Vec<NativeHostIssue>) {
    if value.is_empty() {
        issues.push(NativeHostIssue::EmptyField(field));
    } else if !valid_ref(value) {
        issues.push(NativeHostIssue::MalformedRef {
            field,
            value: value.to_string(),
        });
    }
}

fn has_duplicates<T: Ord + Clone>(values: &[T]) -> bool {
    let mut seen = BTreeSet::new();
    values.iter().any(|value| !seen.insert(value.clone()))
}

pub fn native_identity_ref(parts: &[&str]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"molten-native-host-v1\0");
    for part in parts {
        hasher.update(part.len().to_string().as_bytes());
        hasher.update(b":");
        hasher.update(part.as_bytes());
        hasher.update(b"\0");
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}
